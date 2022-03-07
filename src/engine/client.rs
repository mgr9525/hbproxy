use std::{collections::LinkedList, io, time::Duration};

use async_std::{net::TcpStream, sync::Mutex, task};

use crate::{
    app::Application,
    engine::proxyer::{Proxyer, ProxyerCfg},
    entity::node::{NodeConnMsg, RegNodeRep, RegNodeReq},
    utils::{self, msg::Messages},
};

#[derive(Clone)]
pub struct NodeClientCfg {
    pub name: String,
    pub token: Option<String>,
    pub remote_version: String,
}
#[derive(Clone)]
pub struct NodeClient {
    inner: ruisutil::ArcMut<Inner>,
}

struct Inner {
    ctx: ruisutil::Context,
    cfg: NodeClientCfg,
    conn: TcpStream,
    ctms: ruisutil::Timer,
    ctmout: ruisutil::Timer,
    msgs: Mutex<LinkedList<Messages>>,

    connhost: String,
    isoldconn: bool,
}

impl NodeClient {
    pub fn new(ctx: ruisutil::Context, cfg: NodeClientCfg, conn: TcpStream) -> Self {
        let isold = match utils::compare_version(&cfg.remote_version, "0.2.3".into()) {
            utils::CompareVersion::Less | utils::CompareVersion::Eq => true,
            _ => false,
        };
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                cfg: cfg,
                conn: conn,
                ctms: ruisutil::Timer::new(Duration::from_secs(20)),
                ctmout: ruisutil::Timer::new(Duration::from_secs(30)),
                msgs: Mutex::new(LinkedList::new()),

                connhost: utils::envs("HBPROXY_CLI2HOST", "localhost"),
                isoldconn: isold,
            }),
        }
    }
    fn stop(&self) {
        self.inner.ctx.stop();
        let ins = unsafe { self.inner.muts() };
        if let Err(e) = ins.conn.shutdown(std::net::Shutdown::Both) {
            log::error!("stop shutdown err:{}", e);
        }
    }
    pub async fn run(self) {
        self.inner.ctmout.reset();
        let wg = ruisutil::WaitGroup::new();
        let c = self.clone();
        let wgs = wg.clone();
        task::spawn(async move {
            while !c.inner.ctx.done() {
                c.run_check().await;
                task::sleep(Duration::from_millis(100)).await;
            }
            std::mem::drop(wgs);
            println!("client run_check end!!");
        });
        let c = self.clone();
        let wgs = wg.clone();
        task::spawn(async move {
            c.run_send().await;
            std::mem::drop(wgs);
            println!("client run_send end!!");
        });
        let c = self.clone();
        let wgs = wg.clone();
        task::spawn(async move {
            c.run_recv().await;
            std::mem::drop(wgs);
            println!("client run_recv end!!");
        });
        log::debug!(
            "NodeClient run waits start:{}",
            self.inner.cfg.name.as_str()
        );
        wg.waits().await;
        log::debug!("NodeClient run waits end:{}", self.inner.cfg.name.as_str());
    }

    pub async fn run_recv(&self) {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            match utils::msg::parse_msg(&self.inner.ctx, &mut ins.conn).await {
                Err(e) => {
                    log::error!(
                        "NodeClient({}) parse_msg err:{:?}",
                        self.inner.cfg.name.as_str(),
                        e
                    );
                    self.stop();
                }
                Ok(v) => {
                    let c = self.clone();
                    log::debug!("run_recv msg ctrl:{}", v.control);
                    task::spawn(async move { c.on_msg(v).await });
                    continue;
                }
            }
            task::sleep(Duration::from_millis(10)).await;
        }
    }
    async fn run_send(&self) {
        let ins = unsafe { self.inner.muts() };
        while !self.inner.ctx.done() {
            let msg = {
                let mut lkv = self.inner.msgs.lock().await;
                lkv.pop_front()
            };
            if let Some(v) = msg {
                if let Err(e) = utils::msg::send_msgs(&self.inner.ctx, &mut ins.conn, v).await {
                    log::error!("run_send send_msgs err:{}", e);
                    /* if let Ok(mut lkv) = self.inner.waits.write() {
                        lkv.remove(&xids);
                    } */
                } else {
                    // log::debug!("run_send send_msgs ok:{}", ctrl);
                    continue;
                }
            }
            task::sleep(Duration::from_millis(10)).await;
        }
    }

    async fn run_check(&self) {
        if self.inner.ctmout.tick() {
            log::debug!(
                "NodeClient({}) timeout->stop!!",
                self.inner.cfg.name.as_str()
            );
            self.stop();
        }
        if self.inner.ctms.tick() {
            let mut lkv = self.inner.msgs.lock().await;
            lkv.push_front(Messages {
                control: 0,
                cmds: Some("heart".into()),
                heads: None,
                bodys: None,
            })
        }
    }
    async fn on_msg(&self, msg: utils::msg::Message) {
        match msg.control {
            0 => {
                self.inner.ctmout.reset();
                log::debug!("remote reply heart")
            }
            1 => {
                if let Some(bds) = msg.bodys {
                    let data: NodeConnMsg = match serde_json::from_slice(&bds) {
                        Err(_) => return,
                        Ok(v) => v,
                    };
                    log::debug!("need new conn:{}", data.xids.as_str());

                    let c = self.clone();
                    task::spawn(async move {
                        if c.inner.isoldconn {
                            c.new_conn(data).await;
                        } else {
                            c.new_conns(data).await;
                        }
                    });
                }
            }
            _ => {}
        }
    }
    async fn new_conn(&self, data: NodeConnMsg) {
        // log::debug!("start new_conn -> :{}",data.port);
        let mut req = Application::new_req(1, "NodeConn", false);
        match req.do_json(None, &data).await {
            Err(e) => {
                log::error!("new_conn request do err:{}", e);
            }
            Ok(res) => {
                self.start_conn(res, &data.host, data.port).await;
            }
        }
    }
    async fn new_conns(&self, data: NodeConnMsg) {
        log::debug!("start new_conns -> :{}",data.port);
        let mut req = Application::new_req(1, "NodeConns", false);
        req.add_arg("name", data.name.as_str());
        req.add_arg("xid", data.xids.as_str());
        match req.dors(None, None).await {
            Err(e) => {
                log::error!("new_conn request do err:{}", e);
            }
            Ok(res) => {
                self.start_conn(res, &data.host, data.port).await;
            }
        }
    }
    async fn start_conn(&self, mut res: hbtp::Response, host: &Option<String>, port: i32) {
        if res.get_code() == hbtp::ResCodeOk {
            if let Some(bs) = res.get_bodys() {
                if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                    log::debug!("start_conn ok:{}", vs);
                }
            }

            let hosts = match host {
                None => self.inner.connhost.as_str(),
                Some(v) => v.as_str(),
            };
            let addrs = format!("{}:{}", hosts, port);
            let connlc = match TcpStream::connect(addrs.as_str()).await {
                Ok(v) => v,
                Err(e) => {
                    log::error!("start_conn Proxyer err:{}", e);
                    return;
                }
            };
            log::debug!("client Proxyer start on -> {}", addrs.as_str());
            let px = Proxyer::new(
                self.inner.ctx.clone(),
                ProxyerCfg {
                    ids: addrs,
                    limit: None,
                },
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

    pub async fn runs(name: String) -> io::Result<()> {
        let mut cfg = NodeClientCfg {
            name: name,
            token: None,
            remote_version: String::new(),
        };
        // let mut conns = None;
        while !Application::context().done() {
            let vers = match utils::remote_version(Application::new_req(1, "version", false)).await
            {
                Err(e) => {
                    log::error!("remote version err:{}", e);
                    task::sleep(Duration::from_secs(3)).await;
                    continue;
                    // return -1;
                }
                Ok(v) => v,
            };
            println!("remote version:{}", vers.as_str());
            match Self::connect(&cfg).await {
                Ok((conn, data)) => {
                    cfg.token = Some(data.token.clone());
                    cfg.remote_version = vers;
                    // conns = Some(conn);
                    let cli = Self::new(Application::context(), cfg.clone(), conn);
                    cli.run().await;
                }
                Err(e) => {
                    if e.kind() == io::ErrorKind::AlreadyExists {
                        log::error!("已存在相同名称的节点");
                        task::sleep(Duration::from_secs(1)).await;
                        Application::context().stop();
                        return Err(e);
                    } else if e.kind() == io::ErrorKind::InvalidInput {
                        log::error!("授权失败,请检查key是否正确");
                        task::sleep(Duration::from_secs(1)).await;
                        if !Application::get().keyignore {
                            Application::context().stop();
                            return Err(e);
                        }
                        task::sleep(Duration::from_secs(3)).await;
                    } else {
                        log::error!("connect err:{}", e);
                        task::sleep(Duration::from_secs(2)).await;
                    }
                }
            }
        }
        Ok(())
    }
    async fn connect(cfg: &NodeClientCfg) -> io::Result<(TcpStream, RegNodeRep)> {
        log::debug!("NodeClient connect start:{}", cfg.name.as_str());
        let mut req = Application::new_req(1, "NodeJoin", false);
        let data = RegNodeReq {
            name: cfg.name.clone(),
            token: cfg.token.clone(),
            version: Some(crate::app::VERSION.into()),
        };
        match req.do_json(None, &data).await {
            Err(e) => {
                log::error!("conntion request do err:{}", e);
                task::sleep(Duration::from_secs(5)).await;
            }
            Ok(mut res) => {
                if res.get_code() == utils::HBTP_TOKEN_ERR {
                    return Err(ruisutil::ioerr(
                        "name is exists",
                        Some(io::ErrorKind::AlreadyExists),
                    ));
                }
                if res.get_code() == hbtp::ResCodeAuth {
                    if let Some(bs) = res.get_bodys() {
                        if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                            log::error!("response err:{}", vs);
                        }
                    }

                    return Err(ruisutil::ioerr(
                        "name is exists",
                        Some(io::ErrorKind::InvalidInput),
                    ));
                }
                if res.get_code() == hbtp::ResCodeOk {
                    let data: RegNodeRep = match res.body_json() {
                        Err(e) => {
                            log::error!("response body err:{}", e);
                            return Err(ruisutil::ioerr("json data err", None));
                        }
                        Ok(v) => v,
                    };
                    return Ok((res.own_conn(), data));
                } else {
                    if let Some(bs) = res.get_bodys() {
                        if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                            log::error!("response err:{}", vs);
                        }
                    }
                }
            }
        }
        Err(ruisutil::ioerr("conn end err", None))
    }
}
