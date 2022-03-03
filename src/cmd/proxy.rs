use crate::{
    app::Application,
    entity::proxy::{ProxyListRep, RuleConfReq},
};

pub async fn runs<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    if let Some(v) = args.subcommand_matches("reload") {
        reloads(v).await
    } else if let Some(v) = args.subcommand_matches("add") {
        adds(v).await
    } else if let Some(v) = args.subcommand_matches("ls") {
        lss(v).await
    } else if let Some(v) = args.subcommand_matches("start") {
        starts(v).await
    } else if let Some(v) = args.subcommand_matches("stop") {
        stops(v).await
    } else if let Some(v) = args.subcommand_matches("rm") {
        rms(v).await
    } else {
        -2
    }
}

async fn reloads<'a>(_: &clap::ArgMatches<'a>) -> i32 {
    let mut req = Application::new_reqs(3, "ProxyReload");
    match req.dors(None, None).await {
        Err(e) => {
            eprintln!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                if let Some(bs) = res.get_bodys() {
                    if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                        println!("reload:{}", vs);
                    }
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
async fn adds<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let names = if let Some(vs) = args.value_of("name") {
        Some(vs.to_string())
    } else {
        None
    };
    let binds = if let Some(vs) = args.value_of("bind") {
        vs.to_string()
    } else {
        eprintln!("bind?");
        return -1;
    };
    let gotos = if let Some(vs) = args.value_of("goto") {
        vs.to_string()
    } else {
        eprintln!("goto?");
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
        limit: None,
    };
    let mut req = Application::new_reqs(3, "ProxyAdd");
    match req.do_json(None, &data).await {
        Err(e) => {
            eprintln!("request do err:{}", e);
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
                        eprintln!("res err:{}", vs);
                    }
                }
                return -3;
            }
        }
    }
    0
}

async fn lss<'a>(_: &clap::ArgMatches<'a>) -> i32 {
    let mut req = Application::new_reqs(3, "ProxyList");
    match req.dors(None, None).await {
        Err(e) => {
            eprintln!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                // println!("ls");
                let data: ProxyListRep = match res.body_json() {
                    Err(e) => {
                        eprintln!("response body err:{}", e);
                        return -3;
                    }
                    Ok(v) => v,
                };
                println!(
                    "{:<30}{:<20}{:<20}{:<20}{:^10}{:<25}",
                    "Name", "Bind", "Proxy", "Localhost", "Status", "Msg"
                );
                for v in &data.list {
                    let msgs = match &v.msg {
                        None => "<nil>".to_string(),
                        Some(v) => v.clone(),
                    };
                    let loccals = match &v.goto.localhost {
                        None => "<localhost>".to_string(),
                        Some(v) => v.clone(),
                    };
                    println!(
                        "{:<30}{:<20}{:<20}{:<20}{:^10}{:<25}",
                        v.name.as_str(),
                        v.remote.as_str(),
                        v.proxy.as_str(),
                        loccals.as_str(),
                        v.status,
                        msgs.as_str()
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

async fn starts<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let names = if let Some(vs) = args.value_of("name") {
        vs
    } else {
        println!("name is required");
        return -1;
    };
    let mut req = Application::new_reqs(3, "ProxyStart");
    req.add_arg("name", names);
    match req.dors(None, None).await {
        Err(e) => {
            eprintln!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                if let Some(bs) = res.get_bodys() {
                    if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                        println!("start:{}", vs);
                    }
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
async fn stops<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let names = if let Some(vs) = args.value_of("name") {
        vs
    } else {
        println!("name is required");
        return -1;
    };
    let mut req = Application::new_reqs(3, "ProxyStop");
    req.add_arg("name", names);
    match req.dors(None, None).await {
        Err(e) => {
            eprintln!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                if let Some(bs) = res.get_bodys() {
                    if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                        println!("stop:{}", vs);
                    }
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
async fn rms<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let names = if let Some(vs) = args.value_of("name") {
        vs
    } else {
        println!("name is required");
        return -1;
    };
    let mut req = Application::new_reqs(3, "ProxyRemove");
    req.add_arg("name", names);
    match req.dors(None, None).await {
        Err(e) => {
            eprintln!("request do err:{}", e);
            return -2;
        }
        Ok(res) => {
            if res.get_code() == hbtp::ResCodeOk {
                if let Some(bs) = res.get_bodys() {
                    if let Ok(vs) = std::str::from_utf8(&bs[..]) {
                        println!("remove:{}", vs);
                    }
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
