use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ServerConf {
    pub server: ServerInfoConf,
}

#[derive(Serialize, Deserialize)]
pub struct ServerInfoConf {
    // #[serde(rename = "name")]
    pub bind: Option<String>,
    pub key: Option<String>,
    pub log_path: Option<String>,
    pub proxys_path: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyInfoConf {
    // #[serde(rename = "name")]
    pub name: Option<String>,
    pub bind: String,
    pub proxy: String,
}
