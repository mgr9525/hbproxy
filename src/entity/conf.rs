use std::io;

use serde::{Deserialize, Serialize};

use super::{node::ProxyGoto, util::ProxyLimit};

#[derive(Serialize, Deserialize)]
pub struct ServerConf {
    pub server: ServerInfoConf,
    pub api_server: Option<ApiServerInfoConf>,
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfoConf {
    // #[serde(rename = "name")]
    pub host: Option<String>,
    pub key: Option<String>,
    pub log_path: Option<String>,
    pub proxys_path: Option<String>,
    pub key_time_check: Option<bool>,
}
#[derive(Serialize, Deserialize)]
pub struct ApiServerInfoConf {
    // #[serde(rename = "name")]
    pub host: Option<String>,
    pub key: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyInfoConf {
    // #[serde(rename = "name")]
    pub name: Option<String>,
    pub stop: Option<bool>,
    pub bind: String,
    pub proxys: Vec<ProxyInfoGoto>,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyInfoGoto {
    pub proxy: String,
    pub localhost: Option<String>,
    pub limit: Option<ProxyLimit>,
}

impl ProxyInfoConf {
    pub fn convs_proxy_goto(&self) -> io::Result<Vec<ProxyGoto>> {
        let mut ls = Vec::new();
        for v in &self.proxys {
            ls.push(v.conv_proxy_goto()?);
        }
        Ok(ls)
    }
}
impl ProxyInfoGoto {
    pub fn conv_proxy_goto(&self) -> io::Result<ProxyGoto> {
        let gotols: Vec<&str> = self.proxy.split(":").collect();
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
        Ok(ProxyGoto {
            proxy_host: if gotols[0].is_empty() {
                "localhost".to_string()
            } else {
                gotols[0].to_string()
            },
            proxy_port: gotoport,
            localhost: self.localhost.clone(),
            limit: self.limit.clone(),
        })
    }
}

impl Default for ServerConf {
    fn default() -> Self {
        Self {
            server: ServerInfoConf {
                host: None,
                key: None,
                log_path: None,
                proxys_path: None,
                key_time_check: None,
            },
            api_server: None,
        }
    }
}
