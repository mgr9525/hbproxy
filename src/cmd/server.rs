use std::io;

use crate::{
    app::Application,
    case::{ServerCase, ServerConf},
};

pub async fn runs<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let addrs = Application::get().addrs.clone();
    let cs = ServerCase::new(
        Application::context(),
        ServerConf {
            node_key: Application::get().keys.clone(),
        },
    );
    Application::get_mut().server_case = Some(cs);
    let serv = hbtp::Engine::new(Some(Application::context()), addrs.as_str());
    serv.reg_fun(1, handle1);
    log::info!("server start on:{}", addrs.as_str());
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
    if c.control() != 1 && !cs.authed(&c) {
        return c.res_string(hbtp::ResCodeAuth, "auth failed").await;
    }
    match c.control() {
        1 => match c.command() {
            "version" => c.res_string(hbtp::ResCodeOk, crate::app::VERSION).await,
            _ => Err(ruisutil::ioerr("Not found Method", None)),
        },
        2 => {
            match c.command() {
                "NodeJoin" => cs.node_reg(c).await,
                "NodeList" => cs.node_list(c).await,
                "NodeConn" => cs.node_conn(c).await,
                _ => Err(ruisutil::ioerr("Not found Method", None)),
            }
        }
        3=>{
          match c.command() {
              "ProxyAdd" => cs.proxy_add(c).await,
              "ProxyList" => cs.proxy_list(c).await,
              "ProxyRemove" => cs.proxy_remove(c).await,
              _ => Err(ruisutil::ioerr("Not found Method", None)),
          }
        }
        _ => Err(ruisutil::ioerr("Not found Control", None)),
    }
}
