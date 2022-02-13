use std::{
    io,
    net::Shutdown,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_std::{net::TcpStream, task};
use futures::{AsyncReadExt, AsyncWriteExt};
use ruisutil::{
    bytes::CircleBuf,
    ArcMut,
};

#[derive(Clone)]
pub struct Proxyer {
    inner: ArcMut<Inner>,
}
struct Inner {
    ctx: ruisutil::Context,
    conn: TcpStream,
    connlc: TcpStream,

    ids: String,
    bufw: CircleBuf,   //Mutex<ByteBoxBuf>,
    buflcw: CircleBuf, //Mutex<ByteBoxBuf>,

    endr1: bool,
    endr2: bool,
    endw1: bool,
    endw2: bool,
}
impl Proxyer {
    pub fn new(ctx: ruisutil::Context, ids: String, conn: TcpStream, connlc: TcpStream) -> Self {
        let bufw = CircleBuf::new(&ctx, 1024 * 1024*2);
        let buflcw = CircleBuf::new(&ctx, 1024 * 1024*2);
        Self {
            inner: ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                conn: conn,
                connlc: connlc,

                ids: ids,
                bufw: bufw,     //Mutex::new(ByteBoxBuf::new()),
                buflcw: buflcw, //Mutex::new(ByteBoxBuf::new()),

                endr1: false,
                endr2: false,
                endw1: false,
                endw2: false,
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
        let ids = xid::new().to_string();
        log::debug!(
            "Proxyer({}-{}) start",
            ids.as_str(),
            self.inner.ids.as_str()
        );
        let wg = ruisutil::WaitGroup::new();
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count=0;
            if let Err(e) = c.read1(&mut count).await {
                log::warn!("Proxyer({}) read1 err:{}", c.inner.ids.as_str(), e);
            }
            // c.closer();
            unsafe { c.inner.muts().endr1 = true };
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) read1 end!byte count:{}", c.inner.ids.as_str(),count);
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count=0;
            if let Err(e) = c.write1(&mut count).await {
                log::warn!("Proxyer({}) write1 err:{}", c.inner.ids.as_str(), e);
            }
            unsafe { c.inner.muts().endw1 = true };
            c.closer();
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) write1 end!byte count:{}", c.inner.ids.as_str(),count);
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count=0;
            if let Err(e) = c.read2(&mut count).await {
                log::warn!("Proxyer({}) read2 err:{}", c.inner.ids.as_str(), e);
            }
            // c.closer();
            unsafe { c.inner.muts().endr2 = true };
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) read2 end!byte count:{}", c.inner.ids.as_str(),count);
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count=0;
            if let Err(e) = c.write2(&mut count).await {
                log::warn!("Proxyer({}) write2 err:{}", c.inner.ids.as_str(), e);
            }
            unsafe { c.inner.muts().endw2 = true };
            c.closelcr();
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) write2 end!byte count:{}", c.inner.ids.as_str(),count);
        });

        wg.waits().await;
        self.stop();
        log::debug!("Proxyer({}-{}) end", ids.as_str(), self.inner.ids.as_str());
    }
    pub async fn read1(&self,count:&mut usize) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let ln = match ins.buflcw.borrow_write_buf(10240) {
                Err(e) => {
                    if e.kind() == io::ErrorKind::InvalidData {
                        if self.inner.endw2 {
                            break;
                        }
                        log::debug!("err read1 no data space({}):InvalidData=>{}",self.inner.buflcw.len(),e);
                        task::sleep(Duration::from_millis(1)).await;
                        continue;
                    } else {
                        return Err(e);
                    }
                }
                Ok(buf) => {
                    let n = ins.conn.read(buf).await?;
                    if n <= 0 {
                        return Err(ruisutil::ioerr(
                            format!("read size=0,bufsz={}", buf.len()),
                            None,
                        ));
                    }
                    n
                }
            };
            ins.buflcw.borrow_write_ok(ln)?;
            *count+=ln;
            /* if self.inner.buflcw.avail() <= 0 {
                task::sleep(Duration::from_millis(1)).await;
                continue;
            }
            let buf = ins.buflcw.borrow_write_buf(10240)?;
            let n = ins.conn.read(buf).await?;
            if n <= 0 {
                return Err(ruisutil::ioerr("read size=0", None));
            }
            std::mem::drop(buf);
            ins.buflcw.borrow_write_ok(n)?; */
        }
        Ok(())
    }

    pub async fn write1(&self,count:&mut usize) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            match self.inner.bufw.borrow_read_buf(10240) {
                Err(e) => {
                    if e.kind() == io::ErrorKind::InvalidData {
                        // log::debug!("err write1(len:{}):InvalidData=>{}",self.inner.bufw.len(),e);
                        if self.inner.endr2 {
                            self.inner.bufw.close();
                            break;
                        }
                        task::sleep(Duration::from_millis(1)).await;
                        continue;
                    } else {
                        return Err(e);
                    }
                }
                Ok(buf) => {
                    let n = ins.conn.write(buf).await?;
                    if n <= 0 {
                        return Err(ruisutil::ioerr(
                            format!("write size=0,bufsz={}", buf.len()),
                            None,
                        ));
                    }
                    ins.bufw.borrow_read_ok(n)?;
                    *count+=n;
                    // log::debug!("write1 borrow_read_ok ln:{},len:{}", n,self.inner.bufw.len());
                }
            }
        }
        Ok(())
    }
    pub async fn read2(&self,count:&mut usize) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let ln = match ins.bufw.borrow_write_buf(10240) {
                Err(e) => {
                    if e.kind() == io::ErrorKind::InvalidData {
                        if self.inner.endw1 {
                            break;
                        }
                        log::debug!("err read2(len:{}):InvalidData=>{}",self.inner.bufw.len(),e);
                        // log::debug!("err read2 no data space({}):InvalidData=>{}",self.inner.bufw.len(),e);
                        task::sleep(Duration::from_millis(1)).await;
                        continue;
                    } else {
                        return Err(e);
                    }
                }
                Ok(buf) => {
                    let n = ins.connlc.read(buf).await?;
                    if n <= 0 {
                        return Err(ruisutil::ioerr(
                            format!("read size=0,bufsz={}", buf.len()),
                            None,
                        ));
                    }
                    n
                }
            };
            ins.bufw.borrow_write_ok(ln)?;
            *count+=ln;
            // log::debug!("read2 borrow_write_ok ln:{},len:{}", ln,self.inner.bufw.len());
        }
        Ok(())
    }

    pub async fn write2(&self,count:&mut usize) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            match self.inner.buflcw.borrow_read_buf(10240) {
                Err(e) => {
                    if e.kind() == io::ErrorKind::InvalidData {
                        if self.inner.endr1 {
                            self.inner.buflcw.close();
                            break;
                        }
                        task::sleep(Duration::from_millis(1)).await;
                        continue;
                    } else {
                        return Err(e);
                    }
                }
                Ok(buf) => {
                    let n = ins.connlc.write(buf).await?;
                    if n <= 0 {
                        return Err(ruisutil::ioerr(
                            format!("write size=0,bufsz={}", buf.len()),
                            None,
                        ));
                    }
                    ins.buflcw.borrow_read_ok(n)?;
                    *count+=n;
                    // log::debug!("write2 borrow_read_ok ln:{},len:{}", n,self.inner.buflcw.len());
                }
            }
        }
        Ok(())
    }
}
