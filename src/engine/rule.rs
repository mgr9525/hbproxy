use std::{
    collections::{HashMap, LinkedList},
    io,
    net::Shutdown,
    sync::{Arc, RwLock},
    time::Duration,
};

use async_std::{
    io::WriteExt,
    net::{TcpListener, TcpStream},
    task,
};
use futures::{AsyncReadExt, StreamExt};
use ruisutil::{
    bytes::{ByteBox, ByteBoxBuf},
    ArcMut,
};

use super::{NodeEngine, NodeServer, ProxyEngine};

pub struct RuleCfg {
    pub name: String,
    pub bind_host: String,
    pub bind_port: i32,
    pub proxy_host: String,
    pub proxy_port: i32,
}
#[derive(Clone)]
pub struct RuleProxy {
    inner: ArcMut<Inner>,
}
struct Inner {
    ctx: ruisutil::Context,
    egn: ProxyEngine,
    node: NodeEngine,
    cfg: RuleCfg,
    stat: i32,
    msgs: Option<String>,
    lsr: Option<TcpListener>,
}

impl RuleProxy {
    pub fn new(ctx: ruisutil::Context, egn: ProxyEngine, node: NodeEngine, cfg: RuleCfg) -> Self {
        Self {
            inner: ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                egn: egn,
                node: node,
                cfg: cfg,
                stat: 0,
                msgs: None,
                lsr: None,
            }),
        }
    }

    pub async fn start(&self) -> io::Result<()> {
        let c = self.clone();
        task::spawn(async move {
            if let Err(e) = c.run().await {
                log::error!("run err:{}", e);
                let ins = unsafe { c.inner.muts() };
                ins.lsr = None;
                ins.stat = -1;
                ins.msgs = Some(format!("bind err:{}", e));
            }
        });
        Ok(())
    }
    pub fn stop(&self) {
        unsafe { self.inner.muts().lsr = None };
        self.inner.ctx.done();
    }
    pub async fn run(&self) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        let addr = format!("{}:{}", self.inner.cfg.bind_host, self.inner.cfg.bind_port);
        let lsr = TcpListener::bind(addr.as_str()).await?;
        ins.lsr = Some(lsr);
        ins.stat = 1;

        if let Some(lsr) = &self.inner.lsr {
            let mut incom = lsr.incoming();
            while !self.inner.ctx.done() {
                match incom.next().await {
                    None => break,
                    Some(v) => {
                        if let Ok(conn) = v {
                            let c = self.clone();
                            task::spawn(async move {
                                c.run_cli(conn).await;
                            });
                        } else {
                            println!("stream conn err!!!!")
                        }
                    }
                }
            }
        }
        log::debug!(
            "{}:{} proxy stop!!",
            self.inner.cfg.bind_host.as_str(),
            self.inner.cfg.bind_port
        );
        self.stop();
        Ok(())
    }
    async fn run_cli(&self, conn: TcpStream) {
        match self.inner.node.find_node(&self.inner.cfg.proxy_host) {
            Err(e) => log::error!("{} proxy err:{}", self.inner.cfg.proxy_host.as_str(), e),
            Ok(v) => {
                let px = Proxyer::new(
                    self.inner.ctx.clone(),
                    self.clone(),
                    conn,
                    v,
                    self.inner.cfg.proxy_port,
                );
                task::spawn(px.start());
            }
        }
    }

    pub fn conf(&self) -> &RuleCfg {
        &self.inner.cfg
    }
    pub fn status(&self) -> i32 {
        self.inner.stat
    }
    pub fn msg(&self) -> Option<String> {
        self.inner.msgs.clone()
    }
}

#[derive(Clone)]
struct Proxyer {
    inner: ArcMut<Innerp>,
}
struct Innerp {
    ctx: ruisutil::Context,
    rulep: RuleProxy,
    node: NodeServer,
    conn: TcpStream,
    connlc: Option<TcpStream>,
    port: i32,

    bufw: ByteBoxBuf,
    buflcw: ByteBoxBuf,
}
impl Proxyer {
    pub fn new(
        ctx: ruisutil::Context,
        rulep: RuleProxy,
        conn: TcpStream,
        node: NodeServer,
        port: i32,
    ) -> Self {
        Self {
            inner: ArcMut::new(Innerp {
                ctx: ruisutil::Context::background(Some(ctx)),
                rulep: rulep,
                node: node,
                conn: conn,
                connlc: None,
                port: port,

                bufw: ByteBoxBuf::new(),
                buflcw: ByteBoxBuf::new(),
            }),
        }
    }

    pub fn stop(self) {
        self.inner.ctx.stop();
        self.inner.conn.shutdown(Shutdown::Both);
        if let Some(conn) = &self.inner.connlc {
            conn.shutdown(Shutdown::Both);
        }
    }
    pub async fn start(self) {
        // self.node.on_msg(msg)
        let conn = match self.inner.node.wait_conn(self.inner.port).await {
            Ok(v) => v,
            Err(e) => {
                log::error!("wait_conn err:{}", e);
                return;
            }
        };

        unsafe { self.inner.muts().connlc = Some(conn) };

        let c = self.clone();
        task::spawn(async move {
            if let Err(e) = c.read1().await {
                log::error!("Proxyer read1 err:{}", e);
            }
            c.stop();
        });
        let c = self.clone();
        task::spawn(async move {
            if let Err(e) = c.write1().await {
                log::error!("Proxyer write1 err:{}", e);
            }
            c.stop();
        });
        let c = self.clone();
        task::spawn(async move {
            if let Err(e) = c.read2().await {
                log::error!("Proxyer read2 err:{}", e);
            }
            c.stop();
        });
        let c = self.clone();
        task::spawn(async move {
            if let Err(e) = c.write2().await {
                log::error!("Proxyer write2 err:{}", e);
            }
            c.stop();
        });
    }
    pub async fn read1(&self) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let mut buf: Box<[u8]> = Vec::with_capacity(1024 * 10).into_boxed_slice();
            let n = ins.conn.read(&mut buf).await?;
            ins.buflcw.pushs(Arc::new(buf), 0, n);
        }
        Ok(())
    }

    pub async fn write1(&self) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
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
            if let Some(conn) = &mut ins.connlc {
                let mut buf: Box<[u8]> = Vec::with_capacity(1024 * 10).into_boxed_slice();
                let n = conn.read(&mut buf).await?;
                ins.bufw.pushs(Arc::new(buf), 0, n);
            }
        }
        Ok(())
    }

    pub async fn write2(&self) -> io::Result<()> {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if let Some(conn) = &mut ins.connlc {
                if let Some(v) = ins.buflcw.pull() {
                    ruisutil::tcp_write_async(&self.inner.ctx, conn, &v).await?;
                } else {
                    task::sleep(Duration::from_millis(1)).await;
                }
            }
        }
        Ok(())
    }
}
