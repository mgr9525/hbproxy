use std::{io, net::Shutdown, sync::Arc, time::Duration};

use async_std::{net::TcpStream, sync::RwLock, task};
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
    bufw: RwLock<ByteBoxBuf>,
    buflcw: RwLock<ByteBoxBuf>,

    endr1: bool,
    endr2: bool,
}
const PROXY_BUF_SIZE_MAX: usize = 1024 * 1024;
impl Proxyer {
    pub fn new(ctx: ruisutil::Context, ids: String, conn: TcpStream, connlc: TcpStream) -> Self {
        Self {
            inner: ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                conn: conn,
                connlc: connlc,

                ids: ids,
                bufw: RwLock::new(ByteBoxBuf::new()),
                buflcw: RwLock::new(ByteBoxBuf::new()),

                endr1: false,
                endr2: false,
            }),
        }
    }

    fn closer(&self) {
        if let Err(e) = self.inner.conn.shutdown(Shutdown::Read) {
            log::debug!("closer err:{}", e);
        }
    }
    fn closelcr(&self) {
        if let Err(e) = self.inner.connlc.shutdown(Shutdown::Read) {
            log::debug!("closelcr err:{}", e);
        }
    }
    fn stop(&self) {
        self.inner.ctx.stop();
        if let Err(e) = self.inner.conn.shutdown(Shutdown::Both) {
            log::debug!("stop conn.shutdown err:{}", e);
        }
        if let Err(e) = self.inner.connlc.shutdown(Shutdown::Both) {
            log::debug!("stop connlc.shutdown err:{}", e);
        }
    }
    pub async fn start(self) {
        log::debug!("Proxyer({}) start", self.inner.ids.as_str());
        let wg = ruisutil::WaitGroup::new();
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count = 0;
            if let Err(e) = c.read1(&mut count).await {
                log::warn!("Proxyer({}) read1 err:{}", c.inner.ids.as_str(), e);
            }
            // c.closer();
            unsafe { c.inner.muts().endr1 = true };
            std::mem::drop(wgc);
            log::debug!(
                "Proxyer({}) read1 end!byte count:{}",
                c.inner.ids.as_str(),
                count
            );
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count = 0;
            if let Err(e) = c.write1(&mut count).await {
                log::warn!("Proxyer({}) write1 err:{}", c.inner.ids.as_str(), e);
            }
            c.closer();
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) write1 end!", c.inner.ids.as_str());
            log::debug!(
                "Proxyer({}) write1 end!byte count:{}",
                c.inner.ids.as_str(),
                count
            );
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count = 0;
            if let Err(e) = c.read2(&mut count).await {
                log::warn!("Proxyer({}) read2 err:{}", c.inner.ids.as_str(), e);
            }
            // c.closer();
            unsafe { c.inner.muts().endr2 = true };
            std::mem::drop(wgc);
            log::debug!(
                "Proxyer({}) read2 end!byte count:{}",
                c.inner.ids.as_str(),
                count
            );
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count = 0;
            if let Err(e) = c.write2(&mut count).await {
                log::warn!("Proxyer({}) write2 err:{}", c.inner.ids.as_str(), e);
            }
            c.closelcr();
            std::mem::drop(wgc);
            log::debug!(
                "Proxyer({}) write2 end!byte count:{}",
                c.inner.ids.as_str(),
                count
            );
        });

        wg.waits().await;
        self.stop();
        log::debug!("Proxyer({}) end", self.inner.ids.as_str());
    }
    async fn max_wait(&self, fs: i8) {
        while !self.inner.ctx.done() {
            let ln = if fs == 1 {
                self.inner.buflcw.read().await.len()
            } else {
                self.inner.bufw.read().await.len()
            };
            if ln <= PROXY_BUF_SIZE_MAX {
                break;
            }
            task::sleep(Duration::from_millis(2)).await;
        }
    }
    pub async fn read1(&self, count: &mut usize) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let mut buf: Box<[u8]> = vec![0u8; 10240].into_boxed_slice();
            let n = ins.conn.read(&mut buf).await?;
            if n <= 0 {
                return Err(ruisutil::ioerr("read size=0", None));
            }
            self.max_wait(1).await;
            let mut lkv = self.inner.buflcw.write().await;
            lkv.pushs(Arc::new(buf), 0, n);
            *count += n;
        }
        Ok(())
    }

    pub async fn write1(&self, count: &mut usize) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            {
                let lkv = self.inner.bufw.read().await;
                if lkv.len() <= 0 {
                    if self.inner.endr2 {
                        break;
                    }
                    task::sleep(Duration::from_millis(2)).await;
                    continue;
                }
            }
            let bts = {
                let mut lkv = self.inner.bufw.write().await;
                lkv.pull()
            };
            if let Some(v) = bts {
                ruisutil::tcp_write_async(&self.inner.ctx, &mut ins.conn, &v).await?;
                *count += v.len();
            } else if self.inner.endr2 {
                break;
            } else {
                task::sleep(Duration::from_millis(2)).await;
            }
        }
        Ok(())
    }
    pub async fn read2(&self, count: &mut usize) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let mut buf: Box<[u8]> = vec![0u8; 1024 * 10].into_boxed_slice();
            let n = ins.connlc.read(&mut buf).await?;
            if n <= 0 {
                return Err(ruisutil::ioerr("read size=0", None));
            }
            self.max_wait(2).await;
            let mut lkv = self.inner.bufw.write().await;
            lkv.pushs(Arc::new(buf), 0, n);
            *count += n;
        }
        Ok(())
    }

    pub async fn write2(&self, count: &mut usize) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            {
                let lkv = self.inner.buflcw.read().await;
                if lkv.len() <= 0 {
                    if self.inner.endr1 {
                        break;
                    }
                    task::sleep(Duration::from_millis(2)).await;
                    continue;
                }
            }
            let bts = {
                let mut lkv = self.inner.buflcw.write().await;
                lkv.pull()
            };
            if let Some(v) = bts {
                ruisutil::tcp_write_async(&self.inner.ctx, &mut ins.connlc, &v).await?;
                *count += v.len();
            } else if self.inner.endr1 {
                break;
            } else {
                task::sleep(Duration::from_millis(2)).await;
            }
        }
        Ok(())
    }
}
