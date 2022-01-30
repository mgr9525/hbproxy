use std::time::Duration;

use async_std::{net::TcpStream, task};

use crate::{case::ServerCase, utils};

pub struct NodeEngineCfg {
    pub name: String,
    pub token: String,
}
#[derive(Clone)]
pub struct NodeEngine {
    inner: ruisutil::ArcMut<Inner>,
}

struct Inner {
    ctx: ruisutil::Context,
    case: ServerCase,
    cfg: NodeEngineCfg,
    conn: Option<TcpStream>,
    ctmout: ruisutil::Timer,
}

impl<'a> NodeEngine {
    pub fn new(ctx: ruisutil::Context, case: ServerCase, cfg: NodeEngineCfg) -> Self {
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                case: case,
                cfg: cfg,
                conn: None,
                ctmout: ruisutil::Timer::new(Duration::from_secs(30)),
            }),
        }
    }

    pub fn conf(&self) -> &NodeEngineCfg {
        &self.inner.cfg
    }
    pub fn set_conn(&self, conn: TcpStream) {
        let ins = unsafe { self.inner.muts() };
        ins.conn = Some(conn);
    }

    pub fn stop(&self) {
        let ins = unsafe { self.inner.muts() };
        self.inner.ctx.stop();
        if let Some(conn) = &mut ins.conn {
            conn.shutdown(std::net::Shutdown::Both);
        }
        ins.conn = None;
    }
    pub async fn start(self) {
        self.inner.ctmout.reset();
        let c = self.clone();
        task::spawn(async move {
            while !c.inner.ctx.done() {
                c.run_check().await;
                task::sleep(Duration::from_millis(100)).await;
            }
        });
        log::debug!("NodeEngine run_recv start:{}", self.inner.cfg.name.as_str());
        self.run_recv().await;
        log::debug!("NodeEngine run_recv end:{}", self.inner.cfg.name.as_str());
        // self.inner.case.rm_node(self.inner.cfg.id);
        // });
    }
    pub async fn run_recv(&self) {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if let Some(conn) = &mut ins.conn {
                match utils::msg::parse_msg(&self.inner.ctx, conn).await {
                    Err(e) => {
                        log::error!("NodeEngine parse_msg err:{:?}", e);
                        // self.stop();
                        ins.conn = None;
                        task::sleep(Duration::from_millis(100)).await;
                    }
                    Ok(v) => {
                        // self.push(data);
                        // self.inner.room.push(data);
                        let c = self.clone();
                        // task::spawn(c.on_msg(v));
                        task::spawn(async move { c.on_msg(v).await });
                    }
                }
            } else {
                task::sleep(Duration::from_millis(10)).await;
            }
        }
    }
    async fn run_check(&self) {
        if self.inner.ctmout.tick() {
            if let Some(_) = self.inner.conn {
                unsafe { self.inner.muts().conn = None };
            }
        }
    }
    async fn on_msg(&self, mut msg: utils::msg::Message) {
        let ins = unsafe { self.inner.muts() };
        self.inner.ctmout.reset();
        match msg.control {
            0 => {
                log::debug!("{} heart", self.inner.cfg.name.as_str());
                if let Some(conn) = &mut ins.conn {
                    if let Err(e) = utils::msg::send_msg(
                        &self.inner.ctx,
                        conn,
                        0,
                        Some("heart".into()),
                        None,
                        None,
                    )
                    .await
                    {
                        log::error!("send_msg heart err:{}", e);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn online(&self) -> bool {
        if let None = self.inner.conn {
            false
        } else {
            !self.inner.ctmout.tmout()
        }
    }
}
