use std::{collections::HashMap, io, sync::RwLock};

use async_std::task;

use crate::{
    engine::{NodeEngine, NodeEngineCfg},
    entity::node::{RegNodeRep, RegNodeReq},
    utils,
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
    nodes: RwLock<HashMap<String, NodeEngine>>,
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
        if data.name.is_empty() {
            return c.res_string(hbtp::ResCodeErr, "name err").await;
        }
        let mut st = 0;
        if let Ok(lkv) = self.inner.nodes.read() {
            // println!("123:{}",data.name.as_str());
            if let Some(v) = lkv.get(&data.name) {
                if v.online() {
                    st = 2;
                    // println!("456:{}",data.name.as_str());
                    if let Some(token) = data.token {
                        log::debug!(
                            "get body name:{},token:{}",
                            data.name.as_str(),
                            token.as_str()
                        );
                        if token == v.conf().token {
                            st = 1;
                            v.stop();
                        }
                    }
                }
            }
        } else {
            st = 3;
        }
        match st {
            0 => {}
            1 => {
                log::debug!("replace node:{}", data.name.as_str());
                /* let rt = c.res_string(hbtp::ResCodeOk, "ok").await;
                if let Ok(lkv) = self.inner.nodes.read() {
                    if let Some(v) = lkv.get(&name) {
                        v.set_conn(c.own_conn());
                    }
                }
                return rt; */
            }
            3 => return c.res_string(hbtp::ResCodeErr, "lock err").await,
            _ => return c.res_string(utils::HbtpTokenErr, "token err").await, //已存在同名node
        };

        let cfg = NodeEngineCfg {
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
        if let Ok(mut lkv) = self.inner.nodes.write() {
            let node = NodeEngine::new(self.inner.ctx.clone(), self.clone(), cfg);
            node.set_conn(c.own_conn());
            lkv.insert(data.name.clone(), node.clone());
            task::spawn(node.start());
        }

        Ok(())
    }
    /* fn new_id(&self) -> u32 {
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
    } */

    pub fn rm_node(&self, nm: &String) {
        log::info!("ServerCase rm_node:{}", nm.as_str());
        if let Ok(mut lkv) = self.inner.nodes.write() {
            if let Some(v) = lkv.get(nm) {
                v.stop();
                lkv.remove(nm);
            }
        }
    }
}
