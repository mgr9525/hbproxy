use std::{collections::HashMap, io, sync::RwLock};

use async_std::task;

use crate::{
    engine::{NodeEngine, NodeEngineCfg},
    entity::node::{RegNodeRep, RegNodeReq},
};

pub struct ServerConf {
    pub node_key: String,
}
#[derive(Clone)]
pub struct ServerCase {
    inner: ruisutil::ArcMutBox<Inner>,
}
struct Inner {
    ctx: ruisutil::Context,
    conf: ServerConf,
    nodes: RwLock<HashMap<u32, NodeEngine>>,
}

impl ServerCase {
    pub fn new(ctx: ruisutil::Context, conf: ServerConf) -> Self {
        Self {
            inner: ruisutil::ArcMutBox::new(Inner {
                ctx: ctx,
                conf: conf,
                nodes: RwLock::new(HashMap::new()),
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

    pub async fn reg_node(&self, c: hbtp::Context) -> io::Result<()> {
        if !self.authed(&c) {
            return c.res_string(hbtp::ResCodeAuth, "auth failed").await;
        }

        let data: RegNodeReq = c.body_json()?;

        if let Some(id) = data.id {
            let mut st = 1;
            if let Ok(lkv) = self.inner.nodes.read() {
                log::debug!("body ids:{},token:{}", id, data.token.as_str());
                if let Some(v) = lkv.get(&id) {
                    if data.token == v.conf().token {
                        st = 2;
                    }
                }
            } else {
                st = 3;
            }
            match st {
                2 => {
                    let rt = c.res_string(hbtp::ResCodeOk, "ok").await;
                    if let Ok(lkv) = self.inner.nodes.read() {
                        if let Some(v) = lkv.get(&id) {
                            v.set_conn(c.own_conn());
                        }
                    }
                    return rt;
                }
                3 => return c.res_string(hbtp::ResCodeErr, "lock err").await,
                _ => return c.res_string(hbtp::ResCodeErr, "token err").await,
            };
        }

        let id = self.new_id();
        let cfg = NodeEngineCfg {
            id: id,
            name: data.name,
            token: ruisutil::random(32),
        };

        c.res_json(
            hbtp::ResCodeOk,
            &RegNodeRep {
                id: id,
                token: cfg.token.clone(),
            },
        )
        .await?;
        if let Ok(mut lkv) = self.inner.nodes.write() {
            let node = NodeEngine::new(self.inner.ctx.clone(), self.clone(), cfg);
            node.set_conn(c.own_conn());
            lkv.insert(id, node.clone());
            task::spawn(node.start());
        }

        Ok(())
    }
    fn new_id(&self) -> u32 {
        if let Ok(lkv) = self.inner.nodes.read() {
            let mut n = lkv.len() as u32;
            loop {
                if lkv.contains_key(&n) {
                    n += 1;
                } else {
                    return n;
                }
            }
        }
        0
    }

    pub fn rm_node(&self, id: u32) {
        log::info!("ServerCase rm_node:id:{}", id);
        if let Ok(mut lkv) = self.inner.nodes.write() {
            if let Some(v) = lkv.get(&id) {
                v.stop();
                lkv.remove(&id);
            }
        }
    }
}
