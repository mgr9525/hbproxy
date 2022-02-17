use std::io;

crate::cfg_unix! {
  use std::os::unix::io::{AsRawFd, FromRawFd, IntoRawFd, RawFd};
}
crate::cfg_windows! {
  use winapi::um::winsock2;
  use std::os::windows::io::{
      AsRawSocket, FromRawSocket, IntoRawSocket, RawSocket,
  };
}

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

use crate::entity::node::ProxyGoto;

use super::{proxyer::Proxyer, NodeEngine, NodeServer, ProxyEngine};

pub struct RuleCfg {
    pub name: String,
    pub bind_host: String,
    pub bind_port: i32,
    pub goto: ProxyGoto,
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
                msgs: Some("wait start...".to_string()),
                lsr: None,
            }),
        }
    }

    pub fn stopd(&self) -> bool {
        self.inner.ctx.done()
    }
    pub async fn start(&self) -> io::Result<()> {
        let c = self.clone();
        let ins = unsafe { c.inner.muts() };
        ins.stat = 0;
        ins.msgs = None;
        task::spawn(async move {
            if let Err(e) = c.run().await {
                log::error!("run err:{}", e);
                let ins = unsafe { c.inner.muts() };
                ins.stat = -1;
                ins.msgs = Some(format!("bind err:{}", e));
            } else {
                let ins = unsafe { c.inner.muts() };
                ins.stat = 2;
                ins.msgs = Some("bind is closed!".to_string());
            }
        });
        Ok(())
    }
    crate::cfg_unix! {
      pub fn stop(&self) {
          //unsafe { self.inner.muts().lsr = None };
          self.inner.ctx.stop();
          if let Some(lsr) = &self.inner.lsr {
              let fd = lsr.as_raw_fd();
              if fd != 0 {
                  // std::net::TcpListener::set_nonblocking(lsr, true);
                  unsafe { libc::shutdown(fd, libc::SHUT_RDWR) };
              }
            }
        }
    }
    crate::cfg_windows! {
      pub fn stop(&self) {
          //unsafe { self.inner.muts().lsr = None };
          self.inner.ctx.done();
          if let Some(lsr) = &self.inner.lsr {
              let fd = lsr.as_raw_socket();
              if fd != 0 {
                  // std::net::TcpListener::set_nonblocking(lsr, true);
                  unsafe { winsock2::closesocket(fd as winsock2::SOCKET) };
              }
            }
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
                            log::error!("stream conn err:{}!!!!", e);
                            break;
                        }
                    },
                }
            }
        }
        // self.inner.egn.remove(&self.inner.cfg.name).await;
        log::debug!(
            "{}:{} proxy stop!!",
            self.inner.cfg.bind_host.as_str(),
            self.inner.cfg.bind_port
        );
        self.stop();
        ins.lsr = None;
        Ok(())
    }
    async fn run_cli(&self, conn: TcpStream) {
        if let Ok(addr) = conn.peer_addr() {
            let locals = match &self.inner.cfg.goto.localhost {
                None => "<nil>",
                Some(v) => v.as_str(),
            };
            log::debug!(
                "listen {}:{} incoming from:{}->{}({}):{}",
                self.inner.cfg.bind_host.as_str(),
                self.inner.cfg.bind_port,
                addr,
                self.inner.cfg.goto.proxy_host.as_str(),
                locals,
                self.inner.cfg.goto.proxy_port
            );
        }
        if let Err(e) = self.inner.node.proxy(&self.inner.cfg.goto, conn).await {
            log::error!("run_cli node.proxy err:{}", e);
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
