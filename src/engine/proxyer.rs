use std::{io, net::Shutdown, sync::Arc, time::Duration};

use async_std::{net::TcpStream, task};
use futures::AsyncReadExt;
use ruisutil::{bytes::ByteBoxBuf, ArcMut};

#[derive(Clone)]
pub struct Proxyer {
    inner: ArcMut<Inner>,
}
struct Inner {
    ctx: ruisutil::Context,
    conn: TcpStream,
    connlc: TcpStream,

    ids: String,
    bufw: ByteBoxBuf,
    buflcw: ByteBoxBuf,

    endr1: bool,
    endr2: bool,
}
impl Proxyer {
    pub fn new(ctx: ruisutil::Context, ids: String, conn: TcpStream, connlc: TcpStream) -> Self {
        Self {
            inner: ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                conn: conn,
                connlc: connlc,

                ids: ids,
                bufw: ByteBoxBuf::new(),
                buflcw: ByteBoxBuf::new(),

                endr1: false,
                endr2: false,
            }),
        }
    }

    fn closer(&self) {
        self.inner.conn.shutdown(Shutdown::Read);
    }
    fn closelcr(&self) {
        self.inner.connlc.shutdown(Shutdown::Read);
    }
    fn stop(&self) {
        self.inner.ctx.stop();
        self.inner.conn.shutdown(Shutdown::Both);
        self.inner.connlc.shutdown(Shutdown::Both);
    }
    pub async fn start(self) {
        log::debug!("Proxyer({}) start", self.inner.ids.as_str());
        let wg = ruisutil::WaitGroup::new();
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            if let Err(e) = c.read1().await {
                log::warn!("Proxyer({}) read1 err:{}", c.inner.ids.as_str(), e);
            }
            // c.closer();
            unsafe { c.inner.muts().endr1 = true };
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) read1 end!", c.inner.ids.as_str());
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            if let Err(e) = c.write1().await {
                log::warn!("Proxyer({}) write1 err:{}", c.inner.ids.as_str(), e);
            }
            c.closer();
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) write1 end!", c.inner.ids.as_str());
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            if let Err(e) = c.read2().await {
                log::warn!("Proxyer({}) read2 err:{}", c.inner.ids.as_str(), e);
            }
            // c.closer();
            unsafe { c.inner.muts().endr2 = true };
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) read2 end!", c.inner.ids.as_str());
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            if let Err(e) = c.write2().await {
                log::warn!("Proxyer({}) write2 err:{}", c.inner.ids.as_str(), e);
            }
            c.closelcr();
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) write2 end!", c.inner.ids.as_str());
        });

        wg.waits().await;
        self.stop();
        log::debug!("Proxyer({}) end", self.inner.ids.as_str());
    }
    pub async fn read1(&self) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let mut buf: Box<[u8]> = vec![0u8; 1024 * 10].into_boxed_slice();
            let n = ins.conn.read(&mut buf).await?;
            if n <= 0 {
                return Err(ruisutil::ioerr("read size=0", None));
            }
            ins.buflcw.pushs(Arc::new(buf), 0, n);
        }
        Ok(())
    }

    pub async fn write1(&self) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if self.inner.endr2 && self.inner.bufw.len() <= 0 {
                break;
            }
            if let Some(v) = ins.bufw.pull() {
                ruisutil::tcp_write_async(&self.inner.ctx, &mut ins.conn, &v).await?;
            } else {
                task::sleep(Duration::from_millis(1)).await;
            }
        }
        Ok(())
    }
    pub async fn read2(&self) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let mut buf: Box<[u8]> = vec![0u8; 1024 * 10].into_boxed_slice();
            let n = ins.connlc.read(&mut buf).await?;
            if n <= 0 {
                return Err(ruisutil::ioerr("read size=0", None));
            }
            ins.bufw.pushs(Arc::new(buf), 0, n);
        }
        Ok(())
    }

    pub async fn write2(&self) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if self.inner.endr1 && self.inner.buflcw.len() <= 0 {
                break;
            }
            if let Some(v) = ins.buflcw.pull() {
                ruisutil::tcp_write_async(&self.inner.ctx, &mut ins.connlc, &v).await?;
            } else {
                task::sleep(Duration::from_millis(1)).await;
            }
        }
        Ok(())
    }
}
