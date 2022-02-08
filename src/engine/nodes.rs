use std::{collections::HashMap, io, sync::RwLock};

use async_std::{net::TcpStream, task};

use crate::{
    case::ServerCase,
    entity::node::{NodeConnMsg, NodeListRep, RegNodeReq},
};

use super::{NodeServer, NodeServerCfg};

#[derive(Clone)]
pub struct NodeEngine {
    inner: ruisutil::ArcMut<Inner>,
}

struct Inner {
    ctx: ruisutil::Context,
    nodes: RwLock<HashMap<String, NodeServer>>,
}

impl NodeEngine {
    pub fn new(ctx: ruisutil::Context) -> Self {
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                nodes: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub fn reg_check(&self, data: &RegNodeReq) -> i32 {
        let mut st = 0;
        if let Ok(lkv) = self.inner.nodes.read() {
            // println!("123:{}",data.name.as_str());
            if let Some(v) = lkv.get(&data.name) {
                if v.online() {
                    st = 2;
                    // println!("456:{}",data.name.as_str());
                    if let Some(token) = &data.token {
                        log::debug!(
                            "get body name:{},token:{}",
                            data.name.as_str(),
                            token.as_str()
                        );
                        if token.eq(&v.conf().token) {
                            st = 1;
                        }
                    }
                }
            }
        } else {
            st = 3;
        }
        /* match st {
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
        }; */
        st
    }
    pub fn register(&self, cfg: NodeServerCfg, conn: TcpStream) -> io::Result<()> {
        if let Ok(mut lkv) = self.inner.nodes.write() {
            let name = cfg.name.clone();
            if let Some(v) = lkv.get(&name) {
                v.stop();
            }
            let node = NodeServer::new(self.inner.ctx.clone(), self.clone(), conn,cfg);
            lkv.insert(name, node.clone());
            task::spawn(node.start());
            Ok(())
        } else {
            Err(ruisutil::ioerr("lock err", None))
        }
    }
    pub fn rm_node(&self, nm: &String) {
        log::info!("rm_node:{}", nm.as_str());
        if let Ok(mut lkv) = self.inner.nodes.write() {
            if let Some(v) = lkv.get(nm) {
                v.stop();
                lkv.remove(nm);
            }
        }
    }
    pub fn show_list(&self) -> io::Result<NodeListRep> {
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
        Ok(rts)
    }

    pub fn find_node(&self, k: &String) -> io::Result<NodeServer> {
        match &self.inner.nodes.read() {
            Err(e) => return Err(ruisutil::ioerr("lock err", None)),
            Ok(lkv) => {
                if let Some(v) = lkv.get(k) {
                    if v.online() {
                        return Ok(v.clone());
                    }
                }
            }
        }
        Err(ruisutil::ioerr("node not found", None))
    }

    pub fn put_conn(&self, data: NodeConnMsg, conn: TcpStream) -> io::Result<()> {
        match &self.inner.nodes.read() {
            Err(e) => return Err(ruisutil::ioerr("lock err", None)),
            Ok(lkv) => {
                if let Some(v) = lkv.get(&data.name) {
                    v.put_conn(&data.xids, conn)?;
                }
            }
        };
        Ok(())
    }
}
