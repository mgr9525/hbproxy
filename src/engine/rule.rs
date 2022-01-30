use std::{sync::RwLock, collections::{LinkedList, HashMap}};

use ruisutil::ArcMut;



#[derive(Clone)]
pub struct RuleProxy{
  inner:ArcMut<Inner>
}
struct Inner{
  
}


impl RuleProxy{
  pub fn new()->Self{
    Self{
      inner:ArcMut::new(Inner{

      })
    }
  }
}