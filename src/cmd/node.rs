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

fn req_addrs<'a>(args: &clap::ArgMatches<'a>) -> String {
    if let Some(vs) = args.value_of("addr") {
        vs.to_string()
    } else {
        utils::envs("HBPROXY_ADDR", "localhost:6573")
    }
}
fn req_keys<'a>(args: &clap::ArgMatches<'a>) -> Option<String> {
    if let Some(vs) = args.value_of("key") {
        // req.add_arg("node_key", vs);
        Some(vs.to_string())
    } else if let Ok(vs) = std::env::var("HBPROXY_KEY") {
        // req.add_arg("node_key", vs.as_str());
        Some(vs)
    } else {
        None
    }
}
fn new_req<'a>(args: &clap::ArgMatches<'a>) -> hbtp::Request {
    let mut req = hbtp::Request::new(req_addrs(args).as_str(), 1);
    if let Some(vs) = req_keys(args) {
        req.add_arg("node_key", vs.as_str());
        // cfg.key = Some(vs.into());
    }
    req
}
async fn joins<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let names = if let Some(vs) = args.value_of("name") {
        vs
    } else {
        "unkown"
    };
    let addrs = req_addrs(args);
    match utils::remote_version(addrs.as_str()).await {
        Err(e) => {
            log::error!("remote [{}] version err:{}", addrs, e);
            return -1;
        }
        Ok(v) => log::info!("remote [{}] version:{}", addrs, v.as_str()),
    };

    let mut cfg = engine::NodeClientCfg {
        addr: addrs.clone(),
        key: None,
        name: names.into(),
        token: String::new(),
    };

    let mut req = hbtp::Request::new(addrs.as_str(), 1);
    req.command("NodeJoin");
    if let Some(vs) = req_keys(args) {
        req.add_arg("node_key", vs.as_str());
        cfg.key = Some(vs.into());
    }
    let data = RegNodeReq {
        name: cfg.name.clone(),
        token: None,
    };
    match req.do_json(None, &data).await {
        Err(e) => {
            log::error!("request do err:{}", e);
            -2
        }
        Ok(mut res) => {
            if res.get_code() == utils::HbtpTokenErr {
                log::error!("已存在相同名称的节点");
            }
            if res.get_code() == hbtp::ResCodeOk {
                let data: RegNodeRep = match res.body_json() {
                    Err(e) => {
                        log::error!("response body err:{}", e);
                        return -3;
                    }
                    Ok(v) => v,
                };
                cfg.token = data.token.clone();
                let cli = engine::NodeClient::new(Application::context(), res.own_conn(), cfg);
                match cli.start().await {
                    Err(e) => {
                        log::error!("client run err:{}", e);
                        -3
                    }
                    Ok(_) => 0,
                }
            } else {
                if let Some(bs) = res.get_bodys() {
                    if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                        log::error!("response err:{}", vs);
                    }
                }
                -4
            }
        }
    }
}

async fn lss<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let mut req = new_req(args);
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
                println!("{:<20}{:<30}{:^5}", "Addr","Name","Online");
                for v in &data.list {
                    let frms = match &v.addrs {
                        None => "<nil>".to_string(),
                        Some(v) => v.clone(),
                    };
                    println!("{:<20}{:<30}{:^5}", frms.as_str(), v.name.as_str(),v.online);
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
