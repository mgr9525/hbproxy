use std::{
    collections::{HashMap, LinkedList},
    io,
    sync::RwLock,
};

use ruisutil::ArcMut;

use crate::entity::proxy::ProxyListRep;

use super::{rule::RuleProxy, NodeEngine, RuleCfg};

#[derive(Clone)]
pub struct ProxyEngine {
    inner: ArcMut<Inner>,
}
struct Inner {
    ctx: ruisutil::Context,
    node: NodeEngine,
    proxys: RwLock<LinkedList<RuleProxy>>,
}

impl ProxyEngine {
    pub fn new(ctx: ruisutil::Context, node: NodeEngine) -> Self {
        Self {
            inner: ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                node: node,
                proxys: RwLock::new(LinkedList::new()),
            }),
        }
    }

    pub fn add_check(&self, cfg: &RuleCfg) -> i8 {
        if let Ok(lkv) = self.inner.proxys.read() {
            for v in lkv.iter() {
                if v.conf().name == cfg.name {
                    return 1;
                }
                if v.conf().bind_port == cfg.bind_port {
                    return 2;
                }
            }
            0
        } else {
            -1
        }
    }
    pub async fn add_proxy(&self, cfg: RuleCfg) -> io::Result<()> {
        let proxy = RuleProxy::new(
            self.inner.ctx.clone(),
            self.clone(),
            self.inner.node.clone(),
            cfg,
        );
        proxy.start().await?;
        if let Ok(mut lkv) = self.inner.proxys.write() {
            lkv.push_back(proxy);
            Ok(())
        } else {
            Err(ruisutil::ioerr("lock err", None))
        }
    }

    pub fn show_list(&self) -> io::Result<ProxyListRep> {
        let mut rts = ProxyListRep { list: Vec::new() };
        match &self.inner.proxys.read() {
            Err(e) => return Err(ruisutil::ioerr("lock err", None)),
            Ok(lkv) => {
                for v in lkv.iter() {
                    // v.conf().name
                    rts.list.push(crate::entity::proxy::ProxyListIt {
                        name: v.conf().name.clone(),
                        remote: format!("{}:{}", v.conf().bind_host.as_str(), v.conf().bind_port),
                        proxy: format!("{}:{}", v.conf().proxy_host.as_str(), v.conf().proxy_port),
                        status: v.status(),
                        msg: v.msg(),
                    });
                }
            }
        };
        Ok(rts)
    }
    pub fn remove(&self, name: &String) -> io::Result<()> {
        if let Ok(mut lkv) = self.inner.proxys.write() {
            let mut cursor = lkv.cursor_front_mut();
            loop {
                match cursor.current() {
                    None => break,
                    Some(v) => {
                        if v.conf().name.eq(name) {
                            v.stop();
                            cursor.remove_current();
                            log::debug!("proxy remove:{}!!!!", name.as_str());
                            return Ok(());
                        }
                        cursor.move_next();
                    }
                }
            }
        }
        Err(ruisutil::ioerr("lock err", None))
    }
}
