use std::{collections::HashMap, sync::RwLock};

use crate::case::ServerCase;

use super::NodeServer;

#[derive(Clone)]
pub struct NodeEngine {
    inner: ruisutil::ArcMut<Inner>,
}

struct Inner {
    // ctx: ruisutil::Context,
    nodes: RwLock<HashMap<String, NodeServer>>,
}

impl NodeEngine {
    pub fn new() -> Self {
        Self {
            inner: ruisutil::ArcMut::new(Inner {
                // ctx: ruisutil::Context::background(Some(ctx)),
                nodes: RwLock::new(HashMap::new()),
            }),
        }
    }
}
