use std::{
    io,
    time::{Duration, SystemTime},
};

use async_std::task;

use crate::{
    app::Application,
    engine::{NodeEngine, NodeServerCfg, ProxyEngine, RuleCfg},
    entity::{
        node::{NodeConnMsg, ProxyGoto, RegNodeRep, RegNodeReq, ProxyGotoReq},
        proxy::RuleConfReq,
    },
    utils,
};

#[derive(Clone)]
pub struct ServerCase {
    inner: ruisutil::ArcMut<Inner>,
}
struct Inner {
    proxy: ProxyEngine,
    node: NodeEngine,
    // nodes: RwLock<HashMap<String, NodeServer>>,
    time_check: bool,
}

impl ServerCase {
    pub fn new(ctx: ruisutil::Context) -> Self {
        let nd = NodeEngine::new(ctx.clone());
        let pxy = ProxyEngine::new(ctx.clone(), nd.clone());
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                proxy: pxy,
                node: nd,
                // nodes: RwLock::new(HashMap::new()),
                time_check: match &Application::get().conf {
                    None => false,
                    Some(v) => match &v.server.key_time_check {
                        None => false,
                        Some(v) => *v,
                    },
                },
            }),
        }
    }

    pub async fn start(&self) {
        let c = self.clone();
        task::spawn(async move {
            match c.inner.proxy.reload().await {
                Err(e) => log::error!("proxy init reload err:{}", e),
                Ok(_) => log::info!("proxy init reload ok"),
            }
        });
    }

    pub fn authed_server(&self, c: &hbtp::Context) -> Option<&str> {
        self.autheds(c, &Application::get().keys)
    }
    pub fn authed_api(&self, c: &hbtp::Context) -> Option<&str> {
        self.autheds(c, &Application::get().apikeys)
    }
    fn autheds(&self, c: &hbtp::Context, key: &Option<String>) -> Option<&str> {
        match key {
            None => return None,
            Some(vs) => {
                if vs.is_empty() {
                    return None;
                }
                let tms = match c.get_arg("times") {
                    None => return Some("param times is nil"),
                    Some(v) => v,
                };
                let rands = match c.get_arg("random") {
                    None => return Some("param random is nil"),
                    Some(v) => v,
                };
                let signs = match c.get_arg("sign") {
                    None => return Some("param sign is nil"),
                    Some(v) => v,
                };
                if tms.is_empty() || rands.len() < 20 || signs.is_empty() {
                    return Some("params has empty");
                }
                match ruisutil::strptime(tms.as_str(), "%+") {
                    Err(_) => return Some("parse times err"),
                    Ok(v) => match SystemTime::now().duration_since(v) {
                        Err(e) => {
                            if self.inner.time_check {
                                return Some("duration times err");
                            } else {
                                let addrs = match c.peer_addr() {
                                    Err(_) => "<nil>".to_string(),
                                    Ok(vs) => vs,
                                };

                                log::warn!(
                                    "client {} time since err but not check:{},err:{}",
                                    addrs.as_str(),
                                    tms.as_str(),
                                    e
                                );
                            }
                        }
                        Ok(tm) => {
                            // println!("time since:{}", tm.as_secs_f32());
                            if tm > Duration::from_secs(120) {
                                if self.inner.time_check {
                                    return Some("time check err: since>120s");
                                } else {
                                    let addrs = match c.peer_addr() {
                                        Err(_) => "<nil>".to_string(),
                                        Ok(vs) => vs,
                                    };
                                    log::warn!(
                                        "client {} time err but not check:{}",
                                        addrs.as_str(),
                                        tms.as_str()
                                    );
                                }
                            }
                        }
                    },
                }
                // println!("tms:{},rands:{},signs:{}",tms,rands,signs);
                let sign = ruisutil::md5str(format!(
                    "{}{}{}{}",
                    c.command(),
                    tms.as_str(),
                    rands.as_str(),
                    vs.as_str()
                ));
                if sign.eq(&signs) {
                    return None;
                } else {
                    log::debug!("check sign err:{}!={}", sign.as_str(), signs.as_str());
                    return Some("check sign err");
                }
            }
        };
        // Some("auths not match end!!")
    }

    pub async fn node_reg(&self, c: hbtp::Context) -> io::Result<()> {
        let data: RegNodeReq = c.body_json()?;
        if data.name.is_empty() {
            return c.res_string(hbtp::ResCodeErr, "name err").await;
        }
        match self.inner.node.reg_check(&data).await {
            0 => {}
            1 => log::debug!("replace node:{}", data.name.as_str()),
            3 => return c.res_string(hbtp::ResCodeErr, "lock err").await,
            _ => return c.res_string(utils::HBTP_TOKEN_ERR, "token err").await, //已存在同名node
        };

        let cfg = NodeServerCfg {
            id: String::new(),
            name: data.name.clone(),
            version: data.version.clone(),
            token: ruisutil::random(32),
        };

        c.res_json(
            hbtp::ResCodeOk,
            &RegNodeRep {
                token: cfg.token.clone(),
            },
        )
        .await?;
        self.inner.node.register(cfg, c.own_conn()).await?;
        Ok(())
    }

    pub async fn node_conn(&self, c: hbtp::Context) -> io::Result<()> {
        let data: NodeConnMsg = c.body_json()?;
        if data.name.is_empty() || data.xids.is_empty() {
            return Err(ruisutil::ioerr("param errs", None));
        }
        c.res_string(hbtp::ResCodeOk, "ok").await?;
        self.inner
            .node
            .put_conn(&data.name, &data.xids, Some(c.own_conn()))
            .await
    }

    pub async fn node_conns(&self, c: hbtp::Context) -> io::Result<()> {
        let name = match c.get_arg("name") {
            None => return Err(ruisutil::ioerr("param err:name", None)),
            Some(v) => v,
        };
        let xids = match c.get_arg("xid") {
            None => return Err(ruisutil::ioerr("param err:name", None)),
            Some(v) => v,
        };
        let iserr = match c.get_arg("err") {
            None => false,
            Some(v) => v == "1",
        };
        if name.is_empty() || xids.is_empty() {
            return Err(ruisutil::ioerr("param errs", None));
        }
        c.res_string(hbtp::ResCodeOk, "ok").await?;

        self.inner
            .node
            .put_conn(&name, &xids, if iserr { None } else { Some(c.own_conn()) })
            .await
    }

    pub async fn node_list(&self, c: hbtp::Context) -> io::Result<()> {
        let rts = self.inner.node.show_list().await?;
        c.res_json(hbtp::ResCodeOk, &rts).await
    }
    pub async fn node_info(&self, c: hbtp::Context) -> io::Result<()> {
        let nms = match c.get_arg("name") {
            None => return c.res_string(hbtp::ResCodeErr, "param err:name").await,
            Some(v) => {
                if v.is_empty() {
                    return c.res_string(hbtp::ResCodeErr, "param err:name").await;
                } else {
                    v
                }
            }
        };
        if let Some(rts) = self.inner.node.get_info(&nms).await {
            c.res_json(hbtp::ResCodeOk, &rts).await
        } else {
            c.res_string(hbtp::ResCodeNotFound, "Not found node").await
        }
    }

    pub async fn node_proxy(&self, c: hbtp::Context) -> io::Result<()> {
        let data: ProxyGotoReq = c.body_json()?;
        for v in &data.proxys {
            match self.inner.node.wait_connlc(&v).await {
                Err(e) => log::error!("run_cli node.proxy err:{}", e),
                Ok(connlc) => {
                    c.res_string(hbtp::ResCodeOk, "ok").await?;
                    self.inner.node.proxy(&v, c.own_conn(), connlc).await;
                    return Ok(());
                }
            }
        }
        c.res_string(hbtp::ResCodeErr, "all goto is err").await
    }

    pub async fn proxy_reload(&self, c: hbtp::Context) -> io::Result<()> {
        if let Err(e) = self.inner.proxy.reload().await {
            log::debug!("proxy reload err:{}", e);
            c.res_string(hbtp::ResCodeErr, format!("reload failed:{}", e).as_str())
                .await
        } else {
            c.res_string(hbtp::ResCodeOk, "ok").await
        }
    }

    pub async fn proxy_info(&self, c: hbtp::Context) -> io::Result<()> {
        let nms = match c.get_arg("name") {
            None => return c.res_string(hbtp::ResCodeErr, "param err:name").await,
            Some(v) => {
                if v.is_empty() {
                    return c.res_string(hbtp::ResCodeErr, "param err:name").await;
                } else {
                    v
                }
            }
        };
        if let Some(rts) = self.inner.proxy.get_info(&nms).await {
            c.res_json(hbtp::ResCodeOk, &rts).await
        } else {
            c.res_string(hbtp::ResCodeNotFound, "Not found node").await
        }
    }
    pub async fn proxy_add(&self, c: hbtp::Context) -> io::Result<()> {
        let data: RuleConfReq = c.body_json()?;
        if data.bind_host.is_empty() {
            return c.res_string(hbtp::ResCodeErr, "bind host err").await;
        }
        if data.bind_port <= 0 {
            return c.res_string(hbtp::ResCodeErr, "bind port err").await;
        }
        let mut gotols = Vec::new();
        for gov in &data.goto {
            if gov.proxy_host.is_empty() {
                return c.res_string(hbtp::ResCodeErr, "proxy host err").await;
            }
            if gov.proxy_port <= 0 {
                return c.res_string(hbtp::ResCodeErr, "proxy port err").await;
            }
            gotols.push(ProxyGoto {
                proxy_host: gov.proxy_host.clone(),
                proxy_port: gov.proxy_port,
                localhost: None,
                limit: gov.limit.clone(),
            })
        }
        let cfg = RuleCfg {
            name: match &data.name {
                None => format!("b{}{}", data.bind_port, ruisutil::random(5).as_str()),
                Some(vs) => vs.clone(),
            },
            bind_host: data.bind_host.clone(),
            bind_port: data.bind_port,
            goto: gotols,
        };
        match self.inner.proxy.add_check(&cfg).await {
            0 => {}
            1 => return c.res_string(hbtp::ResCodeErr, "proxy name is exsit").await,
            2 => return c.res_string(hbtp::ResCodeErr, "proxy port is exsit").await,
            _ => return c.res_string(hbtp::ResCodeErr, "add check err").await,
        }
        let nms = cfg.name.clone();
        self.inner.proxy.add_proxy(cfg, false).await?;
        c.res_string(hbtp::ResCodeOk, nms.as_str()).await
    }
    pub async fn proxy_list(&self, c: hbtp::Context) -> io::Result<()> {
        let rts = self.inner.proxy.show_list().await?;
        c.res_json(hbtp::ResCodeOk, &rts).await
    }
    pub async fn proxy_start(&self, c: hbtp::Context) -> io::Result<()> {
        let nms = if let Some(vs) = c.get_arg("name") {
            vs
        } else {
            return c.res_string(hbtp::ResCodeOk, "param name err").await;
        };
        self.inner.proxy.start(&nms).await?;
        c.res_string(hbtp::ResCodeOk, "ok").await
    }
    pub async fn proxy_stop(&self, c: hbtp::Context) -> io::Result<()> {
        let nms = if let Some(vs) = c.get_arg("name") {
            vs
        } else {
            return c.res_string(hbtp::ResCodeOk, "param name err").await;
        };
        self.inner.proxy.stop(&nms).await?;
        c.res_string(hbtp::ResCodeOk, "ok").await
    }
    pub async fn proxy_remove(&self, c: hbtp::Context) -> io::Result<()> {
        let nms = if let Some(vs) = c.get_arg("name") {
            vs
        } else {
            return c.res_string(hbtp::ResCodeOk, "param name err").await;
        };
        self.inner.proxy.remove(&nms).await;
        c.res_string(hbtp::ResCodeOk, "ok").await
    }
}
