use std::{collections::{HashMap, LinkedList}, sync::RwLock};

use ruisutil::ArcMut;

use super::rule::RuleProxy;


#[derive(Clone)]
pub struct ProxyEngine{
  inner:ArcMut<Inner>
}
struct Inner{
  proxys:RwLock<LinkedList<RuleProxy>>
}

impl ProxyEngine{
  pub fn new()->Self{
    Self{
      inner:ArcMut::new(Inner{
        proxys:RwLock::new(LinkedList::new()),
      })
    }
  }
}