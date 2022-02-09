use std::{
    collections::{HashMap, LinkedList},
    io,
    sync::RwLock,
};

use async_std::task;

use crate::{
    engine::{NodeEngine, NodeServer, NodeServerCfg, ProxyEngine, RuleCfg},
    entity::{
        node::{NodeListRep, RegNodeRep, RegNodeReq, NodeConnMsg},
        proxy::RuleConfReq,
    },
    utils,
};

pub struct ServerConf {
    pub node_key: Option<String>,
}
#[derive(Clone)]
pub struct ServerCase {
    inner: ruisutil::ArcMut<Inner>,
}
struct Inner {
    ctx: ruisutil::Context,
    conf: ServerConf,
    proxy: ProxyEngine,
    node: NodeEngine,
    // nodes: RwLock<HashMap<String, NodeServer>>,
}

impl ServerCase {
    pub fn new(ctx: ruisutil::Context, conf: ServerConf) -> Self {
        let nd = NodeEngine::new(ctx.clone());
        let pxy = ProxyEngine::new(ctx.clone(), nd.clone());
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ctx,
                conf: conf,
                proxy: pxy,
                node: nd,
                // nodes: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub fn authed(&self, c: &hbtp::Context) -> bool {
        match &self.inner.conf.node_key {
            None => return true,
            Some(vs) => {
                if vs.is_empty() {
                    return true;
                }
                if let Some(key) = c.get_arg("node_key") {
                    if vs.eq(&key) {
                        return true;
                    }
                }
            }
        };
        false
    }

    pub async fn node_reg(&self, c: hbtp::Context) -> io::Result<()> {
        let data: RegNodeReq = c.body_json()?;
        if data.name.is_empty() {
            return c.res_string(hbtp::ResCodeErr, "name err").await;
        }
        match self.inner.node.reg_check(&data) {
            0 => {}
            1 => log::debug!("replace node:{}", data.name.as_str()),
            3 => return c.res_string(hbtp::ResCodeErr, "lock err").await,
            _ => return c.res_string(utils::HbtpTokenErr, "token err").await, //已存在同名node
        };

        let cfg = NodeServerCfg {
            name: data.name.clone(),
            token: ruisutil::random(32),
        };

        c.res_json(
            hbtp::ResCodeOk,
            &RegNodeRep {
                token: cfg.token.clone(),
            },
        )
        .await?;
        self.inner.node.register(cfg, c.own_conn())
        // Ok(())
    }

    pub async fn node_list(&self, c: hbtp::Context) -> io::Result<()> {
        let rts = self.inner.node.show_list()?;
        c.res_json(hbtp::ResCodeOk, &rts).await
        // Ok(())
        // Err(ruisutil::ioerr("data err", None))
    }
    pub async fn node_conn(&self, c: hbtp::Context) -> io::Result<()> {
        let data: NodeConnMsg = c.body_json()?;
        c.res_string(hbtp::ResCodeOk, "ok").await?;
        self.inner.node.put_conn(data,c.own_conn())
    }

    pub async fn proxy_add(&self, c: hbtp::Context) -> io::Result<()> {
        let data: RuleConfReq = c.body_json()?;
        if data.bind_host.is_empty() {
            return c.res_string(hbtp::ResCodeErr, "bind host err").await;
        }
        if data.bind_port <= 0 {
            return c.res_string(hbtp::ResCodeErr, "bind port err").await;
        }
        if data.proxy_host.is_empty() {
            return c.res_string(hbtp::ResCodeErr, "proxy host err").await;
        }
        if data.proxy_port <= 0 {
            return c.res_string(hbtp::ResCodeErr, "proxy port err").await;
        }
        let cfg = RuleCfg {
            name: match &data.name {
                None => format!("b{}_{}", data.bind_port, ruisutil::random(5).as_str()),
                Some(vs) => vs.clone(),
            },
            bind_host: data.bind_host.clone(),
            bind_port: data.bind_port,
            proxy_host: data.proxy_host.clone(),
            proxy_port: data.proxy_port,
        };
        match self.inner.proxy.add_check(&cfg) {
            0 => {}
            1 => return c.res_string(hbtp::ResCodeErr, "proxy name is exsit").await,
            2 => return c.res_string(hbtp::ResCodeErr, "proxy port is exsit").await,
            _ => return c.res_string(hbtp::ResCodeErr, "check err").await,
        }
        let nms = cfg.name.clone();
        self.inner.proxy.add_proxy(cfg).await?;
        c.res_string(hbtp::ResCodeOk, nms.as_str()).await
    }
    pub async fn proxy_list(&self, c: hbtp::Context) -> io::Result<()> {
        let rts = self.inner.proxy.show_list()?;
        c.res_json(hbtp::ResCodeOk, &rts).await
    }
    pub async fn proxy_remove(&self, c: hbtp::Context) -> io::Result<()> {
        let nms = if let Some(vs) = c.get_arg("name") {
            vs
        } else {
            return c.res_string(hbtp::ResCodeOk, "param name err").await;
        };
        self.inner.proxy.remove(nms)?;
        c.res_string(hbtp::ResCodeOk, "ok").await
    }
}