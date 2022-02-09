use crate::{
    app::Application,
    engine,
    entity::proxy::{ProxyListRep, RuleConfReq},
    utils,
};

pub async fn runs<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    if let Some(v) = args.subcommand_matches("add") {
        adds(v).await - 1
    } else if let Some(v) = args.subcommand_matches("ls") {
        lss(v).await
    } else {
        -2
    }
}

async fn adds<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let names = if let Some(vs) = args.value_of("name") {
        Some(vs.to_string())
    } else {
        None
    };
    let binds = if let Some(vs) = args.value_of("bind") {
        vs.to_string()
    } else {
        log::error!("bind?");
        return -1;
    };
    let gotos = if let Some(vs) = args.value_of("goto") {
        vs.to_string()
    } else {
        log::error!("goto?");
        return -1;
    };
    let bindls: Vec<&str> = binds.split(":").collect();
    if bindls.len() != 2 {
        println!("bind len err");
        return -2;
    }
    let bindport = if let Ok(v) = bindls[1].parse::<i32>() {
        if v <= 0 {
            println!("bind port err:<=0");
            return -2;
        }
        v
    } else {
        println!("bind port err");
        return -2;
    };
    let gotols: Vec<&str> = gotos.split(":").collect();
    if gotols.len() != 2 {
        println!("goto len err");
        return -2;
    }
    let gotoport = if let Ok(v) = gotols[1].parse::<i32>() {
        if v <= 0 {
            println!("goto port err:<=0");
            return -2;
        }
        v
    } else {
        println!("goto port err");
        return -2;
    };

    let data = RuleConfReq {
        name: names,
        bind_host: if bindls[0].is_empty() {
            "0.0.0.0".to_string()
        } else {
            bindls[0].to_string()
        },
        bind_port: bindport,
        proxy_host: if gotols[0].is_empty() {
            "localhost".to_string()
        } else {
            gotols[0].to_string()
        },
        proxy_port: gotoport,
    };
    let mut req = Application::new_req(3, "ProxyAdd");
    match req.do_json(None, &data).await {
        Err(e) => {
            log::error!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                if let Some(bs) = res.get_bodys() {
                    if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                        println!("ok:{}", vs);
                    }
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

async fn lss<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let mut req = Application::new_req(3, "ProxyList");
    match req.dors(None, None).await {
        Err(e) => {
            log::error!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                // println!("ls");
                let data: ProxyListRep = match res.body_json() {
                    Err(e) => {
                        log::error!("response body err:{}", e);
                        return -3;
                    }
                    Ok(v) => v,
                };
                println!("{:<30}{:^10}{:<25}", "Name", "Status", "Msg");
                for v in &data.list {
                    let msgs = match &v.msg {
                        None => "<nil>".to_string(),
                        Some(v) => v.clone(),
                    };
                    println!(
                        "{:<30}{:^10}{:<25}",
                        v.name.as_str(),
                        v.status,
                        msgs.as_str()
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
