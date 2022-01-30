use std::io;

use crate::{case::{ServerCase, ServerConf}, app::Application};



pub async fn runs<'a>(args: &clap::ArgMatches<'a>) -> i32 {
    let keys = if let Some(vs) = args.value_of("key") {
        vs
    } else {
        ""
    };
    let cs = ServerCase::new(
        Application::context(),
        ServerConf {
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
        "version" => c.res_string(hbtp::ResCodeOk, crate::app::VERSION).await,
        "NodeJoin" => cs.node_reg(c).await,
        "NodeList" => cs.node_list(c).await,
        // "regdev2room" => route::regdev2room(c).await,
        // "roomplay" => route::roomplay(c).await,
        _ => Err(ruisutil::ioerr("Not found Method", None)),
    }
}