mod node;
mod proxy;
mod server;

use crate::{app::Application, utils};

pub async fn cmds(cmdargs: clap::ArgMatches<'static>) -> i32 {
    if let Some(v) = cmdargs.subcommand_matches("test") {
        if v.is_present("debug") {
            println!("Printing debug info...");
        } else {
            println!("Printing normally...");
        }
        0
    } else if let Some(v) = cmdargs.subcommand_matches("server") {
        server::runs(v).await
    } else if let Some(v) = cmdargs.subcommand_matches("node") {
        node::runs(v).await
    } else if let Some(v) = cmdargs.subcommand_matches("proxy") {
        proxy::runs(v).await
    } else if let Some(v) = cmdargs.subcommand_matches("version") {
        if v.is_present("remote") {
            match utils::remote_version(Application::new_req(1, "version", false)).await {
                Err(e) => {
                    log::error!("remote version err:{}", e);
                    // return -1;
                }
                Ok(v) => log::info!("remote version:{}", v.as_str()),
            };
        } else {
            println!("Application version is:{}", crate::app::VERSION);
        }
        0
    } else {
        println!("Application version is:{}", crate::app::VERSION);
        println!("Not found command!Please input help or '-h'!");
        -2
    }
}
