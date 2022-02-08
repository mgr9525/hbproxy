use std::{
    collections::HashMap,
    io,
    sync::{Mutex, RwLock},
    time::Duration,
};

use async_std::{net::TcpStream, task};

use crate::{case::ServerCase, entity::node::NodeConnMsg, utils};

use super::NodeEngine;

pub struct NodeServerCfg {
    pub name: String,
    pub token: String,
}
#[derive(Clone)]
pub struct NodeServer {
    inner: ruisutil::ArcMut<Inner>,
}

struct Inner {
    ctx: ruisutil::Context,
    egn: NodeEngine,
    cfg: NodeServerCfg,
    conn: Option<TcpStream>,
    ctmout: ruisutil::Timer,

    waits: RwLock<HashMap<String, Mutex<Option<TcpStream>>>>,
}

impl NodeServer {
    pub fn new(ctx: ruisutil::Context, egn: NodeEngine, cfg: NodeServerCfg) -> Self {
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                egn: egn,
                cfg: cfg,
                conn: None,
                ctmout: ruisutil::Timer::new(Duration::from_secs(30)),
                waits: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub fn conf(&self) -> &NodeServerCfg {
        &self.inner.cfg
    }
    pub fn set_conn(&self, conn: TcpStream) {
        let ins = unsafe { self.inner.muts() };
        ins.conn = Some(conn);
    }

    pub fn peer_addr(&self) -> io::Result<String> {
        if let Some(conn) = &self.inner.conn {
            let addr = conn.peer_addr()?;
            Ok(addr.to_string())
        } else {
            Err(ruisutil::ioerr("conn nil", None))
        }
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
        log::debug!("NodeServer run_recv start:{}", self.inner.cfg.name.as_str());
        self.run_recv().await;
        log::debug!("NodeServer run_recv end:{}", self.inner.cfg.name.as_str());
        // self.inner.case.rm_node(self.inner.cfg.id);
        // });
    }
    pub async fn run_recv(&self) {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if let Some(conn) = &mut ins.conn {
                match utils::msg::parse_msg(&self.inner.ctx, conn).await {
                    Err(e) => {
                        log::error!("NodeServer parse_msg err:{:?}", e);
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

    pub fn put_conn(&self, xids: &String, conn: TcpStream) -> io::Result<()> {
        if let Ok(lkv) = self.inner.waits.read() {
            if let Some(mkv) = lkv.get(xids) {
                if let Ok(mut v) = mkv.lock() {
                    *v = Some(conn);
                    return Ok(());
                }
            }
        }
        Err(ruisutil::ioerr("timeout", None))
    }
    pub async fn wait_conn(&self, port: i32) -> io::Result<TcpStream> {
        let ins = unsafe { self.inner.muts() };
        let mut xids = format!("{}-{}", xid::new().to_string().as_str(), port);
        if let Ok(lkv) = self.inner.waits.read() {
            while lkv.contains_key(&xids) {
                xids = format!("{}-{}", xid::new().to_string().as_str(), port);
            }
        }
        if let Ok(mut lkv) = self.inner.waits.write() {
            lkv.insert(xids.clone(), Mutex::new(None));
        }
        let bds = match serde_json::to_vec(&NodeConnMsg {
            name: self.inner.cfg.name.clone(),
            xids: xids.clone(),
            port: port,
        }) {
            Err(e) => return Err(ruisutil::ioerr("to json err", None)),
            Ok(v) => v,
        };
        if let Some(conn) = &mut ins.conn {
            if let Err(e) = utils::msg::send_msg(
                &self.inner.ctx,
                conn,
                1,
                None,
                None,
                Some(bds.into_boxed_slice()),
            )
            .await
            {
                log::error!("wait_conn send_msg {} err:{}", xids.as_str(), e);
                if let Ok(mut lkv) = self.inner.waits.write() {
                    lkv.remove(&xids);
                }
            } else {
                let ctx = ruisutil::Context::with_timeout(
                    Some(self.inner.ctx.clone()),
                    Duration::from_secs(5),
                );
                let mut rets = None;
                while !ctx.done() {
                    let mut hased = false;
                    if let Ok(lkv) = self.inner.waits.read() {
                        if let Some(mkv) = lkv.get(&xids) {
                            if let Ok(mut v) = mkv.lock() {
                                if let Some(_) = &mut *v {
                                    hased = true;
                                }
                            }
                        }
                    }
                    if hased {
                        if let Ok(mut lkv) = self.inner.waits.write() {
                            if let Some(mkv) = lkv.remove(&xids) {
                                if let Ok(mut v) = mkv.lock() {
                                    // rets=*v;
                                    // *v=None;
                                    rets = std::mem::replace(&mut v, None);
                                    break;
                                }
                            }
                        }
                    }
                    task::sleep(Duration::from_millis(10)).await;
                }
                if let Some(conn) = rets {
                    return Ok(conn);
                }
            }
        }
        Err(ruisutil::ioerr("timeout", None))
    }
}
