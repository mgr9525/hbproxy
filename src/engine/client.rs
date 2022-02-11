use std::{
    collections::LinkedList,
    io,
    net::Shutdown,
    sync::{Arc, Mutex},
    time::Duration,
};

use async_std::{net::TcpStream, task};
use futures::AsyncReadExt;
use ruisutil::{bytes::ByteBoxBuf, ArcMut};

use crate::{
    app::Application,
    engine::proxyer::Proxyer,
    entity::node::{NodeConnMsg, RegNodeRep, RegNodeReq},
    utils::{self, msg::Messages},
};

pub struct NodeClientCfg {
    pub name: String,
    pub token: Option<String>,
}
#[derive(Clone)]
pub struct NodeClient {
    inner: ruisutil::ArcMut<Inner>,
}

struct Inner {
    ctx: ruisutil::Context,
    cfg: NodeClientCfg,
    conn: Option<TcpStream>,
    shuted: bool,
    conntm: ruisutil::Timer,
    ctms: ruisutil::Timer,
    ctmout: ruisutil::Timer,
    msgs: Mutex<LinkedList<Messages>>,

    connhost: String,
}

impl NodeClient {
    pub fn new(ctx: ruisutil::Context, cfg: NodeClientCfg) -> Self {
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ctx,
                cfg: cfg,
                conn: None,
                shuted: true,
                conntm: ruisutil::Timer::new(Duration::from_secs(2)),
                ctms: ruisutil::Timer::new(Duration::from_secs(20)),
                ctmout: ruisutil::Timer::new(Duration::from_secs(30)),
                msgs: Mutex::new(LinkedList::new()),

                connhost: utils::envs("HBPROXY_CLI2HOST", "localhost"),
            }),
        }
    }
    fn close(&self) {
        if self.inner.shuted {
            return;
        }
        let ins = unsafe { self.inner.muts() };
        ins.shuted = true;
        if let Some(conn) = &mut ins.conn {
            if let Err(e) = conn.shutdown(std::net::Shutdown::Both) {
                log::error!("close shutdown err:{}", e);
            }
        }
    }
    pub async fn start(self) -> io::Result<()> {
        self.inner.ctmout.reset();
        let c = self.clone();
        task::spawn(async move {
            while !c.inner.ctx.done() {
                c.run_check().await;
                task::sleep(Duration::from_millis(100)).await;
            }
            println!("client run_check end!!");
        });
        let c = self.clone();
        task::spawn(async move {
            c.run_send().await;
            println!("client run_send end!!");
        });
        log::debug!("NodeClient run_recv start:{}", self.inner.cfg.name.as_str());
        self.run_recv().await;
        log::debug!("NodeClient run_recv end:{}", self.inner.cfg.name.as_str());
        Ok(())
    }

    pub async fn run_recv(&self) {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if !self.inner.shuted {
                if let Some(conn) = &mut ins.conn {
                    match utils::msg::parse_msg(&self.inner.ctx, conn).await {
                        Err(e) => {
                            log::error!(
                                "NodeClient({}) parse_msg err:{:?}",
                                self.inner.cfg.name.as_str(),
                                e
                            );
                            // self.stop();
                            self.close();
                            log::debug!("NodeClient({}) close!!", self.inner.cfg.name.as_str());
                            task::sleep(Duration::from_millis(100)).await;
                        }
                        Ok(v) => {
                            // self.push(data);
                            // self.inner.room.push(data);
                            let c = self.clone();
                            // task::spawn(c.on_msg(v));
                            log::debug!("run_recv msg ctrl:{}", v.control);
                            task::spawn(async move { c.on_msg(v).await });
                            continue;
                        }
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
                    if let Some(conn) = &mut ins.conn {
                        let ctrl = v.control;
                        if let Err(e) = utils::msg::send_msgs(&self.inner.ctx, conn, v).await {
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
            }
            task::sleep(Duration::from_millis(10)).await;
        }
    }
    async fn reconn(&self) {
        log::debug!("NodeClient reconn start:{}", self.inner.cfg.name.as_str());
        let mut req = Application::new_req(1, "NodeJoin", false);
        let data = RegNodeReq {
            name: self.inner.cfg.name.clone(),
            token: self.inner.cfg.token.clone(),
        };
        match req.do_json(None, &data).await {
            Err(e) => {
                log::error!("conntion request do err:{}", e);
                task::sleep(Duration::from_secs(5)).await;
            }
            Ok(mut res) => {
                if res.get_code() == utils::HbtpTokenErr {
                    log::error!("已存在相同名称的节点");
                    task::sleep(Duration::from_secs(1)).await;
                    Application::context().stop();
                    return;
                }
                if res.get_code() == hbtp::ResCodeAuth {
                    /* let keys = match &Application::get().keys {
                        None => "[nil]",
                        Some(vs) => vs.as_str(),
                    }; */
                    log::error!("授权失败,请检查key是否正确");
                    task::sleep(Duration::from_secs(1)).await;
                    if !Application::get().keyignore {
                        Application::context().stop();
                        return;
                    }
                    task::sleep(Duration::from_secs(3)).await;
                    return;
                }
                if res.get_code() == hbtp::ResCodeOk {
                    let ins = unsafe { self.inner.muts() };
                    let data: RegNodeRep = match res.body_json() {
                        Err(e) => {
                            log::error!("response body err:{}", e);
                            return;
                        }
                        Ok(v) => v,
                    };
                    self.inner.ctmout.reset();
                    ins.shuted = false;
                    ins.conn = Some(res.own_conn());
                    ins.cfg.token = Some(data.token.clone());
                } else {
                    if let Some(bs) = res.get_bodys() {
                        if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                            log::error!("response err:{}", vs);
                        }
                    }
                }
            }
        }
    }
    async fn run_check(&self) {
        if self.inner.shuted {
            if self.inner.conntm.tick() {
                self.reconn().await;
            }
        } else {
            if self.inner.ctmout.tick() {
                log::debug!(
                    "NodeClient({}) timeout->close!!",
                    self.inner.cfg.name.as_str()
                );
                self.close();
            }
            if self.inner.ctms.tick() {
                if let Ok(mut lkv) = self.inner.msgs.lock() {
                    lkv.push_back(Messages {
                        control: 0,
                        cmds: Some("heart".into()),
                        heads: None,
                        bodys: None,
                    })
                }
            }
        }
    }
    async fn on_msg(&self, mut msg: utils::msg::Message) {
        self.inner.ctmout.reset();
        match msg.control {
            0 => {
                log::debug!("remote reply heart")
            }
            1 => {
                if let Some(bds) = msg.bodys {
                    let data: NodeConnMsg = match serde_json::from_slice(&bds) {
                        Err(e) => return,
                        Ok(v) => v,
                    };
                    log::debug!("need new conn:{}", data.xids.as_str());

                    let c = self.clone();
                    task::spawn(async move { c.new_conn(data).await });
                }
            }
            _ => {}
        }
    }
    async fn new_conn(&self, data: NodeConnMsg) {
        let mut req = Application::new_req(1, "NodeConn", false);
        match req.do_json(None, &data).await {
            Err(e) => {
                log::error!("new_conn request do err:{}", e);
            }
            Ok(mut res) => {
                if res.get_code() == hbtp::ResCodeOk {
                    if let Some(bs) = res.get_bodys() {
                        if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                            log::debug!("new_conn ok:{}", vs);
                        }
                    }

                    let addrs = format!("{}:{}", self.inner.connhost.as_str(), data.port);
                    let connlc = match TcpStream::connect(addrs.as_str()).await {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("new_conn Proxyer err:{}", e);
                            return;
                        }
                    };
                    log::debug!("client Proxyer start on -> {}", addrs.as_str());
                    let px = Proxyer::new(
                        self.inner.ctx.clone(),
                        addrs.clone(),
                        res.own_conn(),
                        connlc,
                    );
                    // let px = Proxyer::new(self.inner.ctx.clone(), res.own_conn(), data.port);
                    px.start().await;
                } else {
                    if let Some(bs) = res.get_bodys() {
                        if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                            log::error!("response err:{}", vs);
                        }
                    }
                }
            }
        }
    }
}
