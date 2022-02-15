use std::{collections::HashMap, io};

use async_std::{net::TcpStream, sync::RwLock, task};

use crate::{
    engine::proxyer::{Proxyer, ProxyerCfg},
    entity::node::{NodeConnMsg, NodeListRep, ProxyGoto, RegNodeReq},
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
    pub async fn register(&self, cfg: NodeServerCfg, conn: TcpStream) {
        let mut lkv = self.inner.nodes.write().await;
        let name = cfg.name.clone();
        if let Some(v) = lkv.get(&name) {
            v.stop();
        }
        let node = NodeServer::new(self.inner.ctx.clone(), self.clone(), conn, cfg);
        lkv.insert(name, node.clone());
        task::spawn(node.start());
    }
    /* pub async fn rm_node(&self, nm: &String) {
        log::info!("rm_node:{}", nm.as_str());
        let mut lkv = self.inner.nodes.write().await;
        if let Some(v) = lkv.get(nm) {
            v.stop();
            lkv.remove(nm);
        }
    } */
    pub async fn show_list(&self) -> io::Result<NodeListRep> {
        let mut rts = NodeListRep { list: Vec::new() };
        let lkv = self.inner.nodes.read().await;
        for k in lkv.keys() {
            if let Some(v) = lkv.get(k) {
                // v.conf().name
                rts.list.push(crate::entity::node::NodeListIt {
                    name: v.conf().name.clone(),
                    version: v.conf().version.clone(),
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
        Ok(rts)
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

    pub async fn put_conn(&self, data: NodeConnMsg, conn: TcpStream) -> io::Result<()> {
        let lkv = self.inner.nodes.read().await;
        if let Some(v) = lkv.get(&data.name) {
            v.put_conn(&data.xids, conn).await?;
        }
        Ok(())
    }
    pub async fn proxy(&self, data: &ProxyGoto, conn: TcpStream) -> io::Result<()> {
        match self.find_node(&data.proxy_host).await {
            Err(e) => {
                log::error!("goto {} proxy err:{}", data.proxy_host.as_str(), e);
                let addrs = format!("{}:{}", data.proxy_host.as_str(), data.proxy_port);
                let connlc = match TcpStream::connect(addrs.as_str()).await {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("new_conn connect err:{}", e);
                        return Err(e);
                    }
                };
                log::debug!("rule Proxyer start on -> {}", addrs.as_str());
                let px = Proxyer::new(
                    self.inner.ctx.clone(),
                    ProxyerCfg {
                        ids: addrs,
                        limit: data.limit.clone(),
                    },
                    conn,
                    connlc,
                );
                px.start().await;
            }
            Ok(v) => {
                let connlc = match v.wait_conn(&data.localhost, data.proxy_port).await {
                    Ok(v) => v,
                    Err(e) => {
                        log::error!("run_cli wait_conn err:{}", e);
                        return Err(e);
                    }
                };
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
        Ok(())
    }
}
