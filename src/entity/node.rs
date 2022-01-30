use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RegNodeReq {
    pub name: String,
    pub token: Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct RegNodeRep {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct NodeListRep {
    pub list: Vec<String>,
}
