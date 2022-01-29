use serde::{Serialize, Deserialize};



#[derive(Serialize, Deserialize)]
pub struct RegNodeReq{
  pub id:Option<u32>,
  pub name:Option<String>,
  pub token:String,
}
#[derive(Serialize, Deserialize)]
pub struct RegNodeRep{
  pub id:u32,
  pub token:String,
}
