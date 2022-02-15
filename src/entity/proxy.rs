use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RuleConfReq {
    pub name: Option<String>,
    pub bind_host: String,
    pub bind_port: i32,
    pub proxy_host: String,
    pub proxy_port: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyListIt {
    pub name: String,
    pub remote:String,
    pub proxy:String,
    pub status: i32,
    pub msg:Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyListRep {
    pub list: Vec<ProxyListIt>,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyGoto {
  pub proxy_host: String,
  pub proxy_port: i32,
  pub localhost: Option<String>,
}