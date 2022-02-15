use std::{
    io,
    net::Shutdown,
    sync::Arc,
    time::{Duration, SystemTime},
};

use async_std::{net::TcpStream, sync::RwLock, task};
use futures::AsyncReadExt;
use ruisutil::{bytes::ByteBoxBuf, ArcMut};

use crate::entity::util::ProxyLimit;

pub struct ProxyerCfg {
    pub ids: String,
    pub limit: Option<ProxyLimit>,
}

#[derive(Clone)]
pub struct Proxyer {
    inner: ArcMut<Inner>,
}
struct Inner {
    ctx: ruisutil::Context,
    cfg: ProxyerCfg,
    conn: TcpStream,
    connlc: TcpStream,

    bufw: RwLock<ByteBoxBuf>,
    buflcw: RwLock<ByteBoxBuf>,
    speedtm: Duration,

    endr1: bool,
    endr2: bool,
}
const PROXY_BUF_SIZE_MAX: usize = 1024 * 1024;
impl Proxyer {
    pub fn new(
        ctx: ruisutil::Context,
        cfg: ProxyerCfg,
        conn: TcpStream,
        connlc: TcpStream,
    ) -> Self {
        Self {
            inner: ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                cfg: cfg,
                conn: conn,
                connlc: connlc,

                bufw: RwLock::new(ByteBoxBuf::new()),
                buflcw: RwLock::new(ByteBoxBuf::new()),
                speedtm: Duration::from_millis(100),

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
        log::debug!("Proxyer({}) start", self.inner.cfg.ids.as_str());
        let wg = ruisutil::WaitGroup::new();
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count = 0;
            if let Err(e) = c.read1(&mut count).await {
                log::warn!("Proxyer({}) read1 err:{}", c.inner.cfg.ids.as_str(), e);
            }
            // c.closer();
            unsafe { c.inner.muts().endr1 = true };
            std::mem::drop(wgc);
            log::debug!(
                "Proxyer({}) read1 end!byte count:{}",
                c.inner.cfg.ids.as_str(),
                count
            );
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count = 0;
            if let Err(e) = c.write1(&mut count).await {
                log::warn!("Proxyer({}) write1 err:{}", c.inner.cfg.ids.as_str(), e);
            }
            c.closer();
            std::mem::drop(wgc);
            log::debug!("Proxyer({}) write1 end!", c.inner.cfg.ids.as_str());
            log::debug!(
                "Proxyer({}) write1 end!byte count:{}",
                c.inner.cfg.ids.as_str(),
                count
            );
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count = 0;
            if let Err(e) = c.read2(&mut count).await {
                log::warn!("Proxyer({}) read2 err:{}", c.inner.cfg.ids.as_str(), e);
            }
            // c.closer();
            unsafe { c.inner.muts().endr2 = true };
            std::mem::drop(wgc);
            log::debug!(
                "Proxyer({}) read2 end!byte count:{}",
                c.inner.cfg.ids.as_str(),
                count
            );
        });
        let c = self.clone();
        let wgc = wg.clone();
        task::spawn(async move {
            let mut count = 0;
            if let Err(e) = c.write2(&mut count).await {
                log::warn!("Proxyer({}) write2 err:{}", c.inner.cfg.ids.as_str(), e);
            }
            c.closelcr();
            std::mem::drop(wgc);
            log::debug!(
                "Proxyer({}) write2 end!byte count:{}",
                c.inner.cfg.ids.as_str(),
                count
            );
        });

        wg.waits().await;
        self.stop();
        log::debug!("Proxyer({}) end", self.inner.cfg.ids.as_str());
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
        let mut ts = SystemTime::now();
        let mut ln = 0;
        let lmt = if let Some(lmt) = &self.inner.cfg.limit {
            Some(lmt.up * 1024 / 10)
        } else {
            None
        };
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let mut buf: Box<[u8]> = vec![0u8; 10240].into_boxed_slice();
            let n = ins.conn.read(&mut buf).await?;
            if n <= 0 {
                return Err(ruisutil::ioerr("read size=0", None));
            }
            {
                self.max_wait(1).await;
                let mut lkv = self.inner.buflcw.write().await;
                lkv.pushs(Arc::new(buf), 0, n);
                *count += n;
            }
            if let Some(lmv) = lmt {
                ln += n;
                if lmv > 0 && ln >= lmv {
                    if let Ok(t) = SystemTime::now().duration_since(ts) {
                        if t < self.inner.speedtm {
                            let wt = self.inner.speedtm - t;
                            log::debug!(
                                "read1 limit({}b/100ms) up ({}) uses:{}ms, waits:{}ms",
                                lmv,
                                ln,
                                t.as_millis(),
                                wt.as_millis()
                            );
                            task::sleep(wt*10).await;
                        }
                    };
                    ts = SystemTime::now();
                    ln = 0;
                }
            }
        }
        Ok(())
    }

    pub async fn write1(&self, count: &mut usize) -> io::Result<()> {
        let mut ts = SystemTime::now();
        let mut ln = 0;
        let lmt = if let Some(lmt) = &self.inner.cfg.limit {
            Some(lmt.down * 1024 / 10)
        } else {
            None
        };
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
                
                if let Some(lmv) = lmt {
                    ln += v.len();
                    if lmv > 0 && ln >= lmv {
                        if let Ok(t) = SystemTime::now().duration_since(ts) {
                            if t < self.inner.speedtm {
                                let wt = self.inner.speedtm - t;
                                log::debug!(
                                    "write1 limit({}b/100ms) down ({}) uses:{}ms, waits:{}ms",
                                    lmv,
                                    ln,
                                    t.as_millis(),
                                    wt.as_millis()
                                );
                                task::sleep(wt).await;
                            }
                        };
                        ts = SystemTime::now();
                        ln = 0;
                    }
                }
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
