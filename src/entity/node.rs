use serde::{Deserialize, Serialize};

use super::util::ProxyLimit;

#[derive(Serialize, Deserialize)]
pub struct RegNodeReq {
    pub name: String,
    pub token: Option<String>,
    pub version: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct RegNodeRep {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct NodeListIt {
    pub name: String,
    pub version: Option<String>,
    pub addrs: Option<String>,
    pub online: bool,
}

#[derive(Serialize, Deserialize)]
pub struct NodeListRep {
    pub list: Vec<NodeListIt>,
}

#[derive(Serialize, Deserialize)]
pub struct NodeConnMsg {
    pub name: String,
    pub xids: String,
    // pub token: String,
    pub host:Option<String>,
    pub port: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyGoto {
  pub proxy_host: String,
  pub proxy_port: i32,
  pub localhost: Option<String>,
  pub limit:Option<ProxyLimit>,
}