use serde::{Deserialize, Serialize};

#[derive(Clone,Serialize, Deserialize)]
pub struct ProxyLimit {
  pub up: usize,   // kb/s
  pub down: usize, // kb/s
}
