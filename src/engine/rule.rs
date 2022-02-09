use std::{
    collections::{HashMap, LinkedList},
    io,
    net::Shutdown,
    os::unix::prelude::AsRawFd,
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

use super::{proxyer::Proxyer, NodeEngine, NodeServer, ProxyEngine};

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
        //unsafe { self.inner.muts().lsr = None };
        self.inner.ctx.done();
        if let Some(lsr) = &self.inner.lsr {
            let fd = lsr.as_raw_fd();
            unsafe { libc::shutdown(fd, libc::SHUT_RD) };
        }
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
                    Some(v) => match v {
                        Ok(conn) => {
                            let c = self.clone();
                            task::spawn(async move {
                                c.run_cli(conn).await;
                            });
                        }
                        Err(e) => {
                            println!("stream conn err:{}!!!!", e);
                            break;
                        }
                    },
                }
            }
        }
        log::debug!(
            "{}:{} proxy stop!!",
            self.inner.cfg.bind_host.as_str(),
            self.inner.cfg.bind_port
        );
        self.stop();
        self.inner.egn.remove(&self.inner.cfg.name);
        Ok(())
    }
    async fn run_cli(&self, conn: TcpStream) {
        match self.inner.node.find_node(&self.inner.cfg.proxy_host) {
            Err(e) => log::error!("{} proxy err:{}", self.inner.cfg.proxy_host.as_str(), e),
            Ok(v) => {
                let connlc = match v.wait_conn(self.inner.cfg.proxy_port).await {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("run_cli wait_conn err:{}", e);
                        return;
                    }
                };
                let px = Proxyer::new(
                    self.inner.ctx.clone(),
                    format!(
                        "{}:{}",
                        self.inner.cfg.proxy_host, self.inner.cfg.proxy_port
                    ),
                    conn,
                    connlc,
                );
                /* let px = Proxyer::new(
                    self.inner.ctx.clone(),
                    self.clone(),
                    conn,
                    v,
                    self.inner.cfg.proxy_port,
                ); */
                px.start().await;
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
