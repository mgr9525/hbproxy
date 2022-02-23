use std::{collections::HashMap, io, path::Path, time::Duration};

use async_std::{sync::RwLock, task};
use ruisutil::ArcMut;

use crate::{
    app::Application,
    entity::{conf::ProxyInfoConf, node::ProxyGoto, proxy::ProxyListRep},
    utils,
};

use super::{rule::RuleProxy, NodeEngine, RuleCfg};

#[derive(Clone)]
pub struct ProxyEngine {
    inner: ArcMut<Inner>,
}
struct Inner {
    ctx: ruisutil::Context,
    node: NodeEngine,
    proxys: RwLock<HashMap<String, RuleProxy>>,
}

impl ProxyEngine {
    pub fn new(ctx: ruisutil::Context, node: NodeEngine) -> Self {
        Self {
            inner: ArcMut::new(Inner {
                ctx: ruisutil::Context::background(Some(ctx)),
                node: node,
                proxys: RwLock::new(HashMap::new()),
            }),
        }
    }

    pub async fn wait_proxys_clear(&self) {
        while !self.inner.ctx.done() {
            task::sleep(Duration::from_millis(500)).await;
            let lkv = self.inner.proxys.read().await;
            if lkv.len() <= 0 {
                return;
            }
            let mut alled = true;
            for (k, v) in lkv.iter() {
                if !v.stopd() {
                    log::debug!("{} not stop!!!!!!", k.as_str());
                    alled = false;
                }
            }
            if alled {
                break;
            }
        }
        {
            let mut lkv = self.inner.proxys.write().await;
            lkv.clear();
        }
    }
    pub async fn reload(&self) -> io::Result<()> {
        log::debug!("proxy reload start");
        let path = match &Application::get().conf {
            None => return Err(ruisutil::ioerr("not found proxys path", None)),
            Some(v) => match &v.server.proxys_path {
                None => "/etc/hbproxy/proxys".to_string(),
                Some(v) => v.clone(),
            },
        };
        let pth = Path::new(path.as_str());
        if !pth.exists() || !pth.is_dir() {
            return Err(ruisutil::ioerr(
                format!(
                    "proxys path ({}) not exists",
                    match pth.to_str() {
                        None => "xxx",
                        Some(vs) => vs,
                    }
                ),
                None,
            ));
        }
        {
            let lkv = self.inner.proxys.read().await;
            for (_, v) in lkv.iter() {
                v.stop();
            }
        }
        self.wait_proxys_clear().await;

        for e in std::fs::read_dir(pth)? {
            let dir = e?;
            let dpth = dir.path();
            let dpths = if let Some(vs) = dpth.to_str() {
                vs
            } else {
                "xxx"
            };
            if dpth.is_file() {
                match self.load_confs(&dpth).await {
                    Err(e) => log::error!("load conf({}) faild:{}", dpths, e),
                    Ok(_) => log::info!("load conf({}) success", dpths),
                }
            }
        }

        Ok(())
    }

    async fn load_confs(&self, dpth: &Path) -> io::Result<()> {
        let rs: io::Result<Vec<ProxyInfoConf>> = utils::ymlfile(&dpth);
        match rs {
            Err(e) => log::warn!("load confs faild:{}", e),
            Ok(vs) => {
                for v in vs {
                    self.load_conf(v).await?;
                }
                return Ok(());
            }
        }
        let rs: io::Result<ProxyInfoConf> = utils::ymlfile(&dpth);
        match rs {
            Err(e) => log::warn!("load conf faild:{}", e),
            Ok(v) => {
                self.load_conf(v).await?;
                return Ok(());
            }
        }
        Err(ruisutil::ioerr("conf yml err", None))
    }
    async fn load_conf(&self, cfg: ProxyInfoConf) -> io::Result<()> {
        /* let cfg: ProxyInfoConf = match utils::ymlfile(&dpth) {
            Err(e) => return Err(ruisutil::ioerr(format!("ymlfile err:{}", e), None)),
            Ok(v) => v,
        }; */
        let bindls: Vec<&str> = cfg.bind.split(":").collect();
        if bindls.len() != 2 {
            return Err(ruisutil::ioerr("bind len err", None));
        }
        let bindport = if let Ok(v) = bindls[1].parse::<i32>() {
            if v <= 0 {
                return Err(ruisutil::ioerr("bind port err:<=0", None));
            }
            v
        } else {
            return Err(ruisutil::ioerr("bind port err", None));
        };
        let gotols: Vec<&str> = cfg.proxy.split(":").collect();
        if gotols.len() != 2 {
            return Err(ruisutil::ioerr("goto len err", None));
        }
        let gotoport = if let Ok(v) = gotols[1].parse::<i32>() {
            if v <= 0 {
                return Err(ruisutil::ioerr("goto port err:<=0", None));
            }
            v
        } else {
            return Err(ruisutil::ioerr("goto port err", None));
        };
        let data = RuleCfg {
            name: match &cfg.name {
                None => format!("b{}{}", bindport, ruisutil::random(5).as_str()),
                Some(vs) => vs.clone(),
            },
            bind_host: if bindls[0].is_empty() {
                "0.0.0.0".to_string()
            } else {
                bindls[0].to_string()
            },
            bind_port: bindport,
            goto: ProxyGoto {
                proxy_host: if gotols[0].is_empty() {
                    "localhost".to_string()
                } else {
                    gotols[0].to_string()
                },
                proxy_port: gotoport,
                localhost: cfg.localhost.clone(),
                limit: cfg.limit.clone(),
            },
        };
        match self.add_check(&data).await {
            0 => {}
            1 => return Err(ruisutil::ioerr("proxy name is exsit", None)),
            2 => return Err(ruisutil::ioerr("proxy port is exsit", None)),
            _ => return Err(ruisutil::ioerr("add check err", None)),
        }
        let stopd = if let Some(v) = &cfg.stop { *v } else { false };
        self.add_proxy(data, stopd).await?;
        Ok(())
    }

    pub async fn add_check(&self, cfg: &RuleCfg) -> i8 {
        let lkv = self.inner.proxys.read().await;
        if let Some(v) = lkv.get(&cfg.name) {
            if !v.stopd() {
                if v.conf().name == cfg.name {
                    return 1;
                }
                if v.conf().bind_port == cfg.bind_port {
                    return 2;
                }
            }
        }
        0
    }
    pub async fn add_proxy(&self, cfg: RuleCfg, stopd: bool) -> io::Result<RuleProxy> {
        let nms = cfg.name.clone();
        if nms.is_empty() {
            return Err(ruisutil::ioerr("name is empty!", None));
        }
        if let Err(e) = self.stop(&nms).await {
            log::debug!("add {} stop err:{}", nms.as_str(), e);
        }
        let proxy = RuleProxy::new(self.clone(), self.inner.node.clone(), cfg);
        if !stopd {
            proxy.start(self.inner.ctx.clone()).await?;
        }
        let mut lkv = self.inner.proxys.write().await;
        lkv.insert(nms, proxy.clone());
        Ok(proxy)
    }

    pub async fn show_list(&self) -> io::Result<ProxyListRep> {
        let mut rts = ProxyListRep { list: Vec::new() };
        let lkv = self.inner.proxys.read().await;
        for (_, v) in lkv.iter() {
            // v.conf().name
            rts.list.push(crate::entity::proxy::ProxyListIt {
                name: v.conf().name.clone(),
                remote: format!("{}:{}", v.conf().bind_host.as_str(), v.conf().bind_port),
                proxy: format!(
                    "{}:{}",
                    v.conf().goto.proxy_host.as_str(),
                    v.conf().goto.proxy_port
                ),
                goto: v.conf().goto.clone(),
                status: v.status(),
                msg: v.msg(),
            });
        }
        Ok(rts)
    }
    pub async fn start(&self, name: &String) -> io::Result<()> {
        let lkv = self.inner.proxys.read().await;
        if let Some(v) = lkv.get(name) {
            v.start(self.inner.ctx.clone()).await?;
            log::debug!("proxy start:{}!!!!", name.as_str());
            Ok(())
        } else {
            Err(ruisutil::ioerr("not found proxy", None))
        }
    }
    pub async fn stop(&self, name: &String) -> io::Result<()> {
        let lkv = self.inner.proxys.read().await;
        if let Some(v) = lkv.get(name) {
            v.stop();
            log::debug!("proxy stop:{}!!!!", name.as_str());
            Ok(())
        } else {
            Err(ruisutil::ioerr("not found proxy", None))
        }
    }
    pub async fn remove(&self, name: &String) {
        let mut lkv = self.inner.proxys.write().await;
        if let Some(v) = lkv.remove(name) {
            v.stop();
            log::debug!("proxy remove:{}!!!!", name.as_str());
        }
    }
}
