mod node;
mod proxy;
mod server;

use std::io;

use crate::{
    app::{self, Application},
    case, engine,
    entity::node::{RegNodeRep, RegNodeReq},
    utils,
};

pub async fn cmds() -> i32 {
    if let Some(v) = Application::get().cmdargs.subcommand_matches("test") {
        if v.is_present("debug") {
            println!("Printing debug info...");
        } else {
            println!("Printing normally...");
        }
        0
    } else if let Some(v) = Application::get().cmdargs.subcommand_matches("server") {
        server::runs(v).await
    } else if let Some(v) = Application::get().cmdargs.subcommand_matches("node") {
        node::runs(v).await
      } else if let Some(v) = Application::get().cmdargs.subcommand_matches("proxy") {
        proxy::runs(v).await
    } else {
        -2
    }
}

