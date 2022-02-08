use std::{io, net::Shutdown, sync::Arc, time::Duration};

use async_std::{net::TcpStream, task};
use futures::AsyncReadExt;
use ruisutil::{bytes::ByteBoxBuf, ArcMut};

use crate::{
    app::Application,
    engine::proxyer::Proxyer,
    entity::node::{NodeConnMsg, RegNodeRep, RegNodeReq},
    utils,
};

pub struct NodeClientCfg {
    pub addr: String,
    pub name: String,
    pub key: Option<String>,
    pub token: String,
}
#[derive(Clone)]
pub struct NodeClient {
    inner: ruisutil::ArcMut<Inner>,
}

struct Inner {
    ctx: ruisutil::Context,
    cfg: NodeClientCfg,
    conn: Option<TcpStream>,
    conntm: ruisutil::Timer,
    ctms: ruisutil::Timer,
    ctmout: ruisutil::Timer,
}

impl NodeClient {
    pub fn new(ctx: ruisutil::Context, conn: TcpStream, cfg: NodeClientCfg) -> Self {
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ctx,
                cfg: cfg,
                conn: Some(conn),
                conntm: ruisutil::Timer::new(Duration::from_secs(2)),
                ctms: ruisutil::Timer::new(Duration::from_secs(20)),
                ctmout: ruisutil::Timer::new(Duration::from_secs(30)),
            }),
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
        });
        log::debug!("NodeClient run_recv start:{}", self.inner.cfg.name.as_str());
        self.run_recv().await;
        log::debug!("NodeClient run_recv end:{}", self.inner.cfg.name.as_str());
        Ok(())
    }

    pub async fn run_recv(&self) {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            if let Some(conn) = &mut ins.conn {
                match utils::msg::parse_msg(&self.inner.ctx, conn).await {
                    Err(e) => {
                        log::error!("NodeClient parse_msg err:{:?}", e);
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
    async fn reconn(&self) {
        let mut req = hbtp::Request::new(self.inner.cfg.addr.as_str(), 2);
        req.command("NodeJoin");
        if let Some(vs) = &self.inner.cfg.key {
            req.add_arg("node_key", vs.as_str());
        }
        let data = RegNodeReq {
            name: self.inner.cfg.name.clone(),
            token: Some(self.inner.cfg.token.clone()),
        };
        match req.do_json(None, &data).await {
            Err(e) => {
                log::error!("conntion request do err:{}", e);
            }
            Ok(mut res) => {
                if res.get_code() == utils::HbtpTokenErr {
                    log::error!("已存在相同名称的节点");
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
                    ins.cfg.token = data.token.clone();
                    ins.conn = Some(res.own_conn());
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
        let ins = unsafe { self.inner.muts() };
        if self.inner.ctms.tick() {
            if let Some(conn) = &mut ins.conn {
                match utils::msg::send_msg(
                    &self.inner.ctx,
                    conn,
                    0,
                    Some("heart".into()),
                    None,
                    None,
                )
                .await
                {
                    Err(e) => log::error!("send_msg heart err:{}", e),
                    Ok(_) => self.inner.ctmout.reset(),
                };
            }
        }
        if self.inner.ctmout.tick() {
            if let Some(_) = self.inner.conn {
                unsafe { self.inner.muts().conn = None };
            }
        }
        if let None = self.inner.conn {
            if self.inner.conntm.tick() {
                self.reconn().await;
            }
        }
    }
    async fn on_msg(&self, mut msg: utils::msg::Message) {
        self.inner.ctmout.reset();
        match msg.control {
            0 => log::debug!("remote reply heart"),
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
        let mut req = Application::new_req(2);
        req.command("NodeConn");
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

                    let addrs = format!("localhost:{}", data.port);
                    let connlc = match TcpStream::connect(addrs.as_str()).await {
                        Ok(v) => v,
                        Err(e) => {
                            log::error!("new_conn Proxyer err:{}", e);
                            return;
                        }
                    };
                    log::debug!("client Proxyer start on -> {}", addrs.as_str());
                    let px = Proxyer::new(self.inner.ctx.clone(),addrs.clone(), res.own_conn(), connlc);
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
