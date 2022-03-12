use serde::{Deserialize, Serialize};

use super::{node::ProxyGoto, util::ProxyLimit};

#[derive(Serialize, Deserialize)]
pub struct RuleConfReq {
    pub name: Option<String>,
    pub bind_host: String,
    pub bind_port: i32,
    pub goto: Vec<RuleConfGoto>,
}

#[derive(Serialize, Deserialize)]
pub struct RuleConfGoto {
    pub proxy_host: String,
    pub proxy_port: i32,
    pub limit: Option<ProxyLimit>,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyListIt {
    pub name: String,
    pub remote: String,
    // pub proxy:String,
    pub goto: Vec<ProxyGoto>,
    pub status: i32,
    pub msg: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ProxyListRep {
    pub list: Vec<ProxyListIt>,
}

impl ProxyListIt {
    pub fn proxystr(&self) -> String {
        let mut rts = Vec::new();
        for v in &self.goto {
            // rts+=format!("{}:{},",v.proxy_host,v.proxy_port)
            let lcls = match &v.localhost {
                Some(vs) => format!("({})",vs),
                None => "".to_string(),
            };
            rts.push(format!("{}{}:{}", v.proxy_host, lcls, v.proxy_port));
        }
        rts.join(",")
    }
}
