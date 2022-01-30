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
    nodes: RwLock<HashMap<String, NodeServer>>,
}

impl ServerCase {
    pub fn new(ctx: ruisutil::Context, conf: ServerConf) -> Self {
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ctx,
                conf: conf,
                proxy: ProxyEngine::new(),
                node: NodeEngine::new(),
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

    pub async fn node_reg(&self, c: hbtp::Context) -> io::Result<()> {
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
        if let Ok(mut lkv) = self.inner.nodes.write() {
            let node = NodeServer::new(self.inner.ctx.clone(), self.clone(), cfg);
            node.set_conn(c.own_conn());
            lkv.insert(data.name.clone(), node.clone());
            task::spawn(node.start());
        }

        Ok(())
    }

    pub async fn node_list(&self, c: hbtp::Context) -> io::Result<()> {
        if !self.authed(&c) {
            return c.res_string(hbtp::ResCodeAuth, "auth failed").await;
        }

        let mut rts = NodeListRep { list: Vec::new() };
        match &self.inner.nodes.read() {
            Err(e) => return Err(ruisutil::ioerr("lock err", None)),
            Ok(lkv) => {
                for k in lkv.keys() {
                    if let Some(v) = lkv.get(k) {
                        // v.conf().name
                        rts.list.push(crate::entity::node::NodeListIt {
                            name: k.clone(),
                            online: v.online(),
                            addrs: match v.peer_addr() {
                                Err(e) => {
                                    log::error!("peer_addr err:{}", e);
                                    None
                                }
                                Ok(v) => Some(v),
                            },
                        });
                    }
                }
            }
        };
        c.res_json(hbtp::ResCodeOk, &rts).await
        // Ok(())
        // Err(ruisutil::ioerr("data err", None))
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
