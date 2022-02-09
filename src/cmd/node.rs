use crate::{
    app::Application,
    engine,
    entity::node::{NodeListRep, RegNodeRep, RegNodeReq},
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
    let addrs = Application::get().addrs.clone();
    match utils::remote_version(addrs.as_str()).await {
        Err(e) => {
            log::error!("remote [{}] version err:{}", addrs, e);
            // return -1;
        }
        Ok(v) => log::info!("remote [{}] version:{}", addrs, v.as_str()),
    };

    let mut cfg = engine::NodeClientCfg {
        addr: addrs.clone(),
        key: None,
        name: names.into(),
        token: None,
    };
    if let Some(vs) = &Application::get().keys {
        cfg.key = Some(vs.into());
    }
    let cli = engine::NodeClient::new(Application::context(), cfg);
    match cli.start().await {
        Err(e) => {
            log::error!("client run err:{}", e);
            -3
        }
        Ok(_) => 0,
    }
}

async fn lss<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let mut req = Application::new_req(2);
    req.command("NodeList");
    match req.dors(None, None).await {
        Err(e) => {
            log::error!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                // println!("ls");
                let data: NodeListRep = match res.body_json() {
                    Err(e) => {
                        log::error!("response body err:{}", e);
                        return -3;
                    }
                    Ok(v) => v,
                };
                println!("{:<30}{:<25}{:^5}", "Name", "Addr", "Online");
                for v in &data.list {
                    let frms = match &v.addrs {
                        None => "<nil>".to_string(),
                        Some(v) => v.clone(),
                    };
                    println!(
                        "{:<30}{:<25}{:^5}",
                        v.name.as_str(),
                        frms.as_str(),
                        v.online
                    );
                }
            } else {
                if let Some(bs) = res.get_bodys() {
                    if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                        log::error!("response err:{}", vs);
                    }
                }
                return -3;
            }
        }
    }
    0
}
