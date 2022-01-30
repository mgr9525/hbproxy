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
    } else if let Some(v) = Application::get().cmdargs.subcommand_matches("run") {
        runs(v).await
    } else if let Some(v) = Application::get().cmdargs.subcommand_matches("node") {
        nodes(v).await
    } else {
        -2
    }
}
async fn runs<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let keys = if let Some(vs) = args.value_of("key") {
        vs
    } else {
        ""
    };
    let cs = case::ServerCase::new(
        Application::context(),
        case::ServerConf {
            node_key: String::from(keys),
        },
    );
    Application::get_mut().server_case = Some(cs);
    let addrs = if let Some(vs) = args.value_of("bind") {
        vs
    } else {
        "0.0.0.0:6573"
    };
    let serv = hbtp::Engine::new(Some(Application::context()), addrs);
    serv.reg_fun(1, handle1);
    log::info!("server start on:{}", addrs);
    if let Err(e) = serv.run().await {
        log::error!("server run err:{}", e);
        return 1;
    }
    log::debug!("run end!");
    0
}

async fn handle1(c: hbtp::Context) -> io::Result<()> {
    let cs = match &Application::get().server_case {
        Some(v) => v,
        None => {
            return Err(ruisutil::ioerr("not init ok!!!", None));
        }
    };
    match c.command() {
        "version" => c.res_string(hbtp::ResCodeOk, app::VERSION).await,
        "RegNode" => cs.node_reg(c).await,
        // "regdev2room" => route::regdev2room(c).await,
        // "roomplay" => route::roomplay(c).await,
        _ => Err(ruisutil::ioerr("Not found Method", None)),
    }
}

async fn nodes<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let names = if let Some(vs) = args.value_of("name") {
        vs
    } else {
        "unkown"
    };
    let addrs = if let Some(vs) = args.value_of("addr") {
        vs
    } else {
        "localhost:6573"
    };
    match utils::remote_version(addrs).await {
        Err(e) => {
            log::error!("remote [{}] version err:{}", addrs, e);
            return -1;
        }
        Ok(v) => log::info!("remote [{}] version:{}", addrs, v.as_str()),
    };

    let mut cfg = engine::NodeClientCfg {
        addr: addrs.into(),
        key: None,
        name: names.into(),
        token: String::new(),
    };

    let mut req = hbtp::Request::new(addrs, 1);
    req.command("RegNode");
    if let Some(vs) = args.value_of("key") {
        req.add_arg("node_key", vs);
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
