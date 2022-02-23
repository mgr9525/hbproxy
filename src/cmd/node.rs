use crate::{app::Application, engine, entity::node::NodeListRep, utils};

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
    match utils::remote_version(Application::new_req(1, "version", false)).await {
        Err(e) => {
            eprintln!("remote version err:{}", e);
            // return -1;
        }
        Ok(v) => println!("remote version:{}", v.as_str()),
    };

    match engine::NodeClient::runs(names.into()).await {
        Err(e) => {
            eprintln!("client run err:{}", e);
            -3
        }
        Ok(_) => 0,
    }
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
                    "{:<30}{:<25}{:^10}{:^10}",
                    "Name", "Addr", "Online", "Version"
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
                    println!(
                        "{:<30}{:<25}{:^10}{:^10}",
                        v.name.as_str(),
                        frms.as_str(),
                        v.online,
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
