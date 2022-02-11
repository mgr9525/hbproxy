use std::{
    collections::{HashMap, VecDeque},
    io,
    sync::{Mutex, RwLock},
    time::Duration,
};

use async_std::{net::TcpStream, task};

use crate::{
    entity::node::NodeConnMsg,
    utils::{self, msg::Messages},
};

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
    conn: TcpStream,
    shuted: bool,
    ctmout: ruisutil::Timer,

    msgs: Mutex<VecDeque<Messages>>,
    waits: RwLock<HashMap<String, Mutex<Option<TcpStream>>>>,
}

impl NodeServer {
    pub fn new(
        ctx: ruisutil::Context,
        egn: NodeEngine,
        conn: TcpStream,
        cfg: NodeServerCfg,
    ) -> Self {
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                egn: egn,
                cfg: cfg,
                conn: conn,
                shuted: false,
                ctmout: ruisutil::Timer::new(Duration::from_secs(30)),

                msgs: Mutex::new(VecDeque::new()),
                waits: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub fn conf(&self) -> &NodeServerCfg {
        &self.inner.cfg
    }

    pub fn peer_addr(&self) -> io::Result<String> {
        let addr = self.inner.conn.peer_addr()?;
        Ok(addr.to_string())
    }

    fn close(&self) {
        if self.inner.shuted {
            return;
        }
        let ins = unsafe { self.inner.muts() };
        ins.shuted = true;
        if let Err(e) = ins.conn.shutdown(std::net::Shutdown::Both) {
            log::error!("close shutdown err:{}", e);
        }
    }
    pub fn stop(&self) {
        self.inner.ctx.stop();
        self.close();
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
        let c = self.clone();
        task::spawn(async move {
            while !c.inner.ctx.done() {
                c.run_send().await;
            }
        });
        log::debug!("NodeServer run_recv start:{}", self.inner.cfg.name.as_str());
        self.run_recv().await;
        log::debug!("NodeServer run_recv end:{}", self.inner.cfg.name.as_str());
        // self.inner.case.rm_node(self.inner.cfg.id);
        // });
    }
    async fn run_recv(&self) {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if !self.inner.shuted {
                match utils::msg::parse_msg(&self.inner.ctx, &mut ins.conn).await {
                    Err(e) => {
                        log::error!("NodeServer parse_msg err:{:?}", e);
                        // self.stop();
                        self.close();
                        task::sleep(Duration::from_millis(100)).await;
                    }
                    Ok(v) => {
                        // self.push(data);
                        // self.inner.room.push(data);
                        let c = self.clone();
                        // task::spawn(c.on_msg(v));
                        task::spawn(async move { c.on_msg(v).await });
                        continue;
                    }
                }
            }
            task::sleep(Duration::from_millis(10)).await;
        }
    }
    async fn run_send(&self) {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if !self.inner.shuted {
                let mut msg = None;
                match self.inner.msgs.lock() {
                    Err(e) => log::error!("run_send err:{}", e),
                    Ok(mut lkv) => msg = lkv.pop_front(),
                }
                if let Some(v) = msg {
                    let ctrl = v.control;
                    if let Err(e) = utils::msg::send_msgs(&self.inner.ctx, &mut ins.conn, v).await {
                        log::error!("run_send send_msgs err:{}", e);
                        /* if let Ok(mut lkv) = self.inner.waits.write() {
                            lkv.remove(&xids);
                        } */
                    } else {
                        log::debug!("run_send send_msgs ok:{}", ctrl);
                        continue;
                    }
                }
            }
            task::sleep(Duration::from_millis(10)).await;
        }
    }
    async fn run_check(&self) {
        if self.inner.ctmout.tick() {
            if !self.inner.shuted {
                self.close();
            }
        }
    }
    async fn on_msg(&self, mut msg: utils::msg::Message) {
        match msg.control {
            0 => {
                self.inner.ctmout.reset();
                log::debug!("{} heart", self.inner.cfg.name.as_str());
                if let Ok(mut lkv) = self.inner.msgs.lock() {
                    lkv.push_front(Messages {
                        control: 0,
                        cmds: Some("heart".into()),
                        heads: None,
                        bodys: None,
                    })
                }
            }
            _ => {}
        }
    }

    pub fn online(&self) -> bool {
        if self.inner.shuted {
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
        // let ins = unsafe { self.inner.muts() };
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
        if !self.inner.shuted {
            match self.inner.msgs.lock() {
                Err(e) => {
                    log::error!("wait_conn send_msg {} err:{}", xids.as_str(), e);
                    if let Ok(mut lkv) = self.inner.waits.write() {
                        lkv.remove(&xids);
                    }
                    return Err(ruisutil::ioerr("lock err", None));
                }
                Ok(mut lkv) => lkv.push_back(Messages {
                    control: 1,
                    cmds: None,
                    heads: None,
                    bodys: Some(bds.into_boxed_slice()),
                }),
            }

            let ctx = ruisutil::Context::with_timeout(
                Some(self.inner.ctx.clone()),
                Duration::from_secs(10),
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
        if let Ok(mut lkv) = self.inner.waits.write() {
            lkv.remove(&xids);
        }
        Err(ruisutil::ioerr("timeout", None))
    }
}
