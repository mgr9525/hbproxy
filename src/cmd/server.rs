use std::io;

use async_std::task;

use crate::{app::Application, engine::ServerCase};

pub async fn runs<'a>(_: &clap::ArgMatches<'a>) -> i32 {
    let addrs = Application::get().apiaddrs.clone();
    let cs = ServerCase::new(Application::context());
    cs.start().await;
    Application::get_mut().server_case = Some(cs);
    task::spawn(async move {
        let serv = hbtp::Engine::new(Some(Application::context()), addrs.as_str());
        serv.set_lmt_max(hbtp::LmtMaxConfig {
            max_ohther: 1024 * 10,       //10K
            max_heads: 1024 * 100,       //100K
            max_bodys: 1024 * 1024 * 10, //10M
        });
        serv.reg_fun(1, handle1, None).await;
        serv.reg_fun(2, handle2, None).await;
        serv.reg_fun(3, handle3, None).await;
        log::info!("server api start on:{}", addrs.as_str());
        if let Err(e) = serv.run().await {
            log::error!("server api run err:{}", e);
        }
        log::debug!("server api end!");
    });
    let addrs = Application::get().addrs.clone();
    let serv = hbtp::Engine::new(Some(Application::context()), addrs.as_str());
    serv.set_lmt_max(hbtp::LmtMaxConfig {
        max_ohther: 1024 * 10,  //10K
        max_heads: 1024 * 100,  //100K
        max_bodys: 1024 * 1024, //1M
    });
    serv.reg_fun(1, handles, None).await;
    log::info!("server start on:{}", addrs.as_str());
    if let Err(e) = serv.run().await {
        log::error!("server run err:{}", e);
        return 1;
    }
    log::debug!("server end!");
    0
}

async fn handles(c: hbtp::Context) -> io::Result<()> {
    let cs = match &Application::get().server_case {
        Some(v) => v,
        None => {
            return Err(ruisutil::ioerr("not init ok!!!", None));
        }
    };
    if c.command() != "version" {
        if let Some(vs) = cs.authed_server(&c) {
            return c.res_string(hbtp::ResCodeAuth, vs).await;
        }
    }
    match c.command() {
        "version" => c.res_string(hbtp::ResCodeOk, crate::app::VERSION).await,
        "NodeJoin" => cs.node_reg(c).await,
        "NodeConn" => cs.node_conn(c).await,
        "NodeConns" => cs.node_conns(c).await,
        _ => Err(ruisutil::ioerr("Not found Method", None)),
    }
}

async fn handle1(c: hbtp::Context) -> io::Result<()> {
    match c.command() {
        "version" => c.res_string(hbtp::ResCodeOk, crate::app::VERSION).await,
        _ => Err(ruisutil::ioerr("Not found Method", None)),
    }
}
async fn handle2(c: hbtp::Context) -> io::Result<()> {
    let cs = match &Application::get().server_case {
        Some(v) => v,
        None => {
            return Err(ruisutil::ioerr("not init ok!!!", None));
        }
    };
    if let Some(vs) = cs.authed_api(&c) {
        return c.res_string(hbtp::ResCodeAuth, vs).await;
    }
    match c.command() {
        "NodeList" => cs.node_list(c).await,
        "NodeInfo" => cs.node_info(c).await,
        "NodeProxy" => cs.node_proxy(c).await,
        _ => Err(ruisutil::ioerr("Not found Method", None)),
    }
}
async fn handle3(c: hbtp::Context) -> io::Result<()> {
    let cs = match &Application::get().server_case {
        Some(v) => v,
        None => {
            return Err(ruisutil::ioerr("not init ok!!!", None));
        }
    };
    if let Some(vs) = cs.authed_api(&c) {
        return c.res_string(hbtp::ResCodeAuth, vs).await;
    }
    match c.command() {
        "ProxyInfo" => cs.proxy_info(c).await,
        "ProxyAdd" => cs.proxy_add(c).await,
        "ProxyList" => cs.proxy_list(c).await,
        "ProxyStart" => cs.proxy_start(c).await,
        "ProxyStop" => cs.proxy_stop(c).await,
        "ProxyRemove" => cs.proxy_remove(c).await,
        "ProxyReload" => cs.proxy_reload(c).await,
        _ => Err(ruisutil::ioerr("Not found Method", None)),
    }
}
