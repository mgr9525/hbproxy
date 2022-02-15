use serde::{Deserialize, Serialize};

use super::util::ProxyLimit;

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
    pub bind: String,
    pub proxy: String,
    pub localhost: Option<String>,
    pub limit:Option<ProxyLimit>,
}
