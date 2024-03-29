use std::{collections::HashMap, io};

use async_std::{net::TcpStream, sync::RwLock, task};

use crate::{
    engine::proxyer::{Proxyer, ProxyerCfg},
    entity::node::{NodeConnMsg, NodeListIt, NodeListRep, ProxyGoto, RegNodeReq},
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

    pub async fn reg_check(&self, data: &RegNodeReq) -> i32 {
        let mut st = 0;
        let lkv = self.inner.nodes.read().await;
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
    pub async fn register(&self, cfg: NodeServerCfg, conn: TcpStream) -> io::Result<()> {
        let nms = cfg.name.clone();
        if nms.is_empty() {
            return Err(ruisutil::ioerr("name is empty!", None));
        }
        log::info!("node register:{}", nms.as_str());
        let mut lkv = self.inner.nodes.write().await;
        if let Some(v) = lkv.get(&nms) {
            v.stop();
        }
        let node = NodeServer::new(self.inner.ctx.clone(), self.clone(), conn, cfg);
        lkv.insert(nms, node.clone());
        task::spawn(async move {
            node.start().await;
        });
        Ok(())
    }
    /* pub async fn rm_node(&self, nm: &String) {
        log::info!("rm_node:{}", nm.as_str());
        let mut lkv = self.inner.nodes.write().await;
        if let Some(v) = lkv.get(nm) {
            v.stop();
            lkv.remove(nm);
        }
    } */

    pub async fn get_info(&self, name: &String) -> Option<NodeListIt> {
        //let mut rts = NodeListIt {  };
        let lkv = self.inner.nodes.read().await;
        let v = lkv.get(name)?;
        Some(NodeListIt {
            name: v.conf().name.clone(),
            version: v.conf().version.clone(),
            online: v.online(),
            online_times: match v.online_time() {
                Err(_) => 0,
                Ok(v) => v.as_secs(),
            },
            outline_times: match v.outline_time() {
                Err(_) => None,
                Ok(v) => Some(v.as_secs()),
            },
            addrs: match v.peer_addr() {
                Err(e) => {
                    log::error!("peer_addr err:{}", e);
                    None
                }
                Ok(v) => Some(v),
            },
        })
    }

    pub async fn show_list(&self) -> io::Result<NodeListRep> {
        let mut rts = NodeListRep { list: Vec::new() };
        let lkv = self.inner.nodes.read().await;
        for k in lkv.keys() {
            if let Some(v) = lkv.get(k) {
                // v.conf().name
                rts.list.push(NodeListIt {
                    name: v.conf().name.clone(),
                    version: v.conf().version.clone(),
                    online: v.online(),
                    online_times: match v.online_time() {
                        Err(_) => 0,
                        Ok(v) => v.as_secs(),
                    },
                    outline_times: match v.outline_time() {
                        Err(_) => None,
                        Ok(v) => Some(v.as_secs()),
                    },
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
        Ok(rts)
    }
    pub async fn remove(&self, name: &String, id: &String) {
        let mut lkv = self.inner.nodes.write().await;
        if let Some(v) = lkv.get(name) {
            if !v.conf().id.eq(id) {
                log::debug!("skip remove node,the {} is replaced", name.as_str());
                return;
            }
            if let Some(v) = lkv.remove(name) {
                v.stop();
                log::debug!("proxy remove:{}!!!!", name.as_str());
            }
        }
    }

    pub async fn find_node(&self, k: &String) -> io::Result<NodeServer> {
        let lkv = self.inner.nodes.read().await;
        if let Some(v) = lkv.get(k) {
            if v.online() {
                return Ok(v.clone());
            }
        }
        Err(ruisutil::ioerr("node not found", None))
    }

    pub async fn put_conn(
        &self,
        name: &String,
        xids: &String,
        conn: Option<TcpStream>,
    ) -> io::Result<()> {
        let lkv = self.inner.nodes.read().await;
        if let Some(v) = lkv.get(name) {
            v.put_conn(xids, conn).await?;
        }
        Ok(())
    }
    pub async fn wait_connlc(&self, data: &ProxyGoto) -> io::Result<TcpStream> {
        let v = self.find_node(&data.proxy_host).await?;
        let connlc = match v.wait_conn(&data.localhost, data.proxy_port).await {
            Ok(v) => v,
            Err(e) => {
                return Err(ruisutil::ioerr(
                    format!("run_cli wait_conn err:{}", e),
                    None,
                ))
            }
        };
        Ok(connlc)
    }
    pub async fn proxy(&self, data: &ProxyGoto, conn: TcpStream, connlc: TcpStream) {
        let px = Proxyer::new(
            self.inner.ctx.clone(),
            ProxyerCfg {
                ids: format!("{}:{}", data.proxy_host.as_str(), data.proxy_port),
                limit: data.limit.clone(),
            },
            conn,
            connlc,
        );
        px.start().await;
    }
}
