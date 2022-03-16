use std::time::Duration;

use async_std::task;

use crate::{
    app::Application,
    engine::{self, NodeClientCfg},
    entity::node::NodeListRep,
    utils,
};

pub async fn runs<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    if let Some(v) = args.subcommand_matches("join") {
        joins(v).await
    } else if let Some(v) = args.subcommand_matches("ls") {
        lss(v).await
    } else {
        -2
    }
}

async fn joins<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let names = if let Some(vs) = args.value_of("name") {
        vs
    } else {
        "unkown"
    };
    if let Some(vs) = args.value_of("hosts") {
        if !vs.is_empty() {
            Application::get_mut().addrs = utils::host_defport(vs.to_string(), 6573);
        }
    };
    if let Some(vs) = args.value_of("keys") {
        if !vs.is_empty() {
            Application::get_mut().keys = Some(vs.to_string());
        }
    };

    let cfg = NodeClientCfg {
        name: names.to_string(),
        token: None,
        remote_version: String::new(),
    };
    while !Application::context().done() {
        if let Err(e) = engine::NodeClient::runs(&cfg).await {
            log::error!("NodeClient::runs err:{}", e);
            if e.kind() == std::io::ErrorKind::Interrupted {
                task::sleep(Duration::from_secs(1)).await;
                eprintln!("client will be interrupted:{}", e);
                return -3;
            }
            task::sleep(Duration::from_secs(2)).await;
        }
    }
    0
}

async fn lss<'a>(_: &clap::ArgMatches<'a>) -> i32 {
    let mut req = Application::new_reqs(2, "NodeList");
    match req.dors(None, None).await {
        Err(e) => {
            eprintln!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                // println!("ls");
                let data: NodeListRep = match res.body_json() {
                    Err(e) => {
                        eprintln!("response body err:{}", e);
                        return -3;
                    }
                    Ok(v) => v,
                };
                println!(
                    "{:<30}{:<25}{:^10}{:^12}{:^10}",
                    "Name", "Addr", "Online", "Duration", "Version"
                );
                for v in &data.list {
                    let frms = match &v.addrs {
                        None => "<nil>".to_string(),
                        Some(v) => v.clone(),
                    };
                    let vers = match &v.version {
                        None => "<nil>".to_string(),
                        Some(v) => v.clone(),
                    };
                    let tms = match v.outline_times {
                        None => utils::mytimes(v.online_times),
                        Some(v) => format!("OUT:{}", utils::mytimes(v)),
                    };
                    println!(
                        "{:<30}{:<25}{:^10}{:^12}{:^10}",
                        v.name.as_str(),
                        frms.as_str(),
                        v.online,
                        tms,
                        vers.as_str(),
                    );
                }
            } else {
                if let Some(bs) = res.get_bodys() {
                    if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                        eprintln!("res err:{}", vs);
                    }
                }
                return -3;
            }
        }
    }
    0
}
