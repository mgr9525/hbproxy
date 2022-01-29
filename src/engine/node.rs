use std::time::Duration;

use async_std::{net::TcpStream, task};

use crate::{case::ServerCase, utils};

pub struct NodeEngineCfg {
    pub id: u32,
    pub name: Option<String>,
    pub token: String,
}
#[derive(Clone)]
pub struct NodeEngine {
    inner: ruisutil::ArcMutBox<Inner>,
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
            inner: ruisutil::ArcMutBox::new(Inner {
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
        ins.conn = None;
    }
    pub async fn start(self) {
        let c = self.clone();
        task::spawn(async move {
            while !c.inner.ctx.done() {
                c.run_check().await;
                task::sleep(Duration::from_millis(100)).await;
            }
        });
        log::info!("NodeEngine run_recv start:id:{}", self.inner.cfg.id);
        self.run_recv().await;
        log::info!("NodeEngine run_recv end:id:{}", self.inner.cfg.id);
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
            unsafe { self.inner.muts().conn = None };
        }
    }
    async fn on_msg(&self, mut msg: utils::msg::Message) {
        self.inner.ctmout.tick();
        match msg.control {
            _ => {}
        }
    }

    pub fn online(&self) -> bool {
        !self.inner.ctmout.tick()
    }
}
