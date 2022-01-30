use std::{
    collections::{HashMap, LinkedList},
    io,
    sync::RwLock,
};

use async_std::task;

use crate::{
    engine::{NodeEngine, NodeServer, NodeServerCfg, ProxyEngine},
    entity::node::{NodeListRep, RegNodeRep, RegNodeReq},
    utils,
};

pub struct ServerConf {
    pub node_key: String,
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
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ctx.clone(),
                conf: conf,
                proxy: ProxyEngine::new(),
                node: NodeEngine::new(ctx),
                // nodes: RwLock::new(HashMap::new()),
            }),
        }
    }

    fn authed(&self, c: &hbtp::Context) -> bool {
        if self.inner.conf.node_key.is_empty() {
            return true;
        }
        if let Some(key) = c.get_arg("node_key") {
            if key == self.inner.conf.node_key {
                return true;
            }
        }
        false
    }

    pub async fn node_reg(&self, c: hbtp::Context) -> io::Result<()> {
        if !self.authed(&c) {
            return c.res_string(hbtp::ResCodeAuth, "auth failed").await;
        }

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
        if !self.authed(&c) {
            return c.res_string(hbtp::ResCodeAuth, "auth failed").await;
        }
        let rts = self.inner.node.node_list()?;
        c.res_json(hbtp::ResCodeOk, &rts).await
        // Ok(())
        // Err(ruisutil::ioerr("data err", None))
    }
}
