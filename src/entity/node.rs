use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize)]
pub struct RegNodeReq{
  pub name:String,
  pub token:Option<String>,
}
#[derive(Serialize, Deserialize)]
pub struct RegNodeRep{
  pub token:String,
}
