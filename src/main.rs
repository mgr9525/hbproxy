extern crate async_std;
extern crate clap;
extern crate flexi_logger;
extern crate futures;
extern crate hbtp;
extern crate hex;
extern crate libc;
extern crate log;
extern crate once_cell;
extern crate ruisutil;
extern crate serde;
extern crate serde_json;
extern crate serde_yaml;
extern crate xid;

#[cfg(windows)]
extern crate winapi;

mod app;
mod cmd;
mod engine;
mod entity;
#[macro_use]
mod utils;

use clap::{App, Arg, SubCommand};
use flexi_logger::Duplicate;

use crate::app::Application;

fn main() {
    let matches = App::new("Hbproxy")
        .version(app::VERSION)
        .author("Linsk Ruis. <mgr9525@gmail.com>")
        .about("Does awesome things")
        .arg(Arg::with_name("debug").long("debug").hidden(true))
        .arg(
            Arg::with_name("conf")
                .multiple(true)
                .short("c")
                .long("conf")
                .help("yml config file(def:/etc/hbproxy/hbproxy.yml)"),
        )
        .arg(
            Arg::with_name("addr")
                .short("a")
                .long("addr")
                .value_name("IP:PORT")
                .help("server address.(def:0.0.0.0:6573)"),
        )
        .arg(
            Arg::with_name("key")
                .short("k")
                .long("key")
                .value_name("KEY")
                .help("server key"),
        )
        .arg(
            Arg::with_name("apiaddr")
                .long("apiaddr")
                .value_name("IP:PORT")
                .help("server api address.(def:localhost:6574)"),
        )
        .arg(
            Arg::with_name("apikey")
                .long("apikey")
                .value_name("KEY")
                .help("server api key"),
        )
        /* .arg(
            Arg::with_name("proxys-path")
                .long("proxys-path")
                .value_name("PATH")
                .help("proxy config path"),
        ) */
        /* .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file"),
        )
        .arg(
            Arg::with_name("a")
                .short("a")
                .value_name("FILE")
                .help("Sets the level of verbosity"),
        ) */
        .subcommand(
            SubCommand::with_name("version")
                .about("get application version.")
                .arg(
                    Arg::with_name("remote")
                        .long("remote")
                        .help("show remote server version."),
                ),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("start hbproxy server.")
                .arg(
                    Arg::with_name("hosts")
                        .value_name("HOST")
                        .help("start server host"),
                )
                .arg(
                    Arg::with_name("keys")
                        .value_name("KEY")
                        .help("start server by key"),
                ),
        )
        .subcommand(
            SubCommand::with_name("node")
                .about("node command")
                .subcommand(
                    SubCommand::with_name("join")
                        .about("start node and join to server")
                        .arg(
                            Arg::with_name("keyignore")
                                .long("keyignore")
                                .help("ignore key err"),
                        )
                        .arg(
                            Arg::with_name("name")
                                .required(true)
                                .value_name("NAME")
                                .help("node name"),
                        )
                        .arg(
                            Arg::with_name("hosts")
                                .value_name("HOST")
                                .help("join to server host"),
                        )
                        .arg(
                            Arg::with_name("keys")
                                .value_name("KEY")
                                .help("join to server by key"),
                        ),
                )
                .subcommand(SubCommand::with_name("ls").about("node list")),
        )
        .subcommand(
            SubCommand::with_name("proxy")
                .about("proxy command")
                .subcommand(SubCommand::with_name("reload").about("reload proxy config file"))
                .subcommand(
                    SubCommand::with_name("add")
                        .about("add new proxy rule")
                        .arg(
                            Arg::with_name("bind")
                                .required(true)
                                .value_name("LISTEN")
                                .help("listen on(example:0.0.0.0:1080)"),
                        )
                        .arg(
                            Arg::with_name("goto")
                                .required(true)
                                .value_name("PROXY")
                                .help("proxy to(example:xxx_node:1081)"),
                        )
                        .arg(
                            Arg::with_name("name")
                                .long("name")
                                .value_name("NAME")
                                .help("proxy rule name"),
                        ),
                )
                .subcommand(SubCommand::with_name("ls").about("proxy list"))
                .subcommand(
                    SubCommand::with_name("start").about("proxy start").arg(
                        Arg::with_name("name")
                            .required(true)
                            .value_name("NAME")
                            .help("proxy name"),
                    ),
                )
                .subcommand(
                    SubCommand::with_name("stop").about("proxy stop").arg(
                        Arg::with_name("name")
                            .required(true)
                            .value_name("NAME")
                            .help("proxy name"),
                    ),
                )
                .subcommand(
                    SubCommand::with_name("rm").about("proxy remove").arg(
                        Arg::with_name("name")
                            .required(true)
                            .value_name("NAME")
                            .help("proxy name"),
                    ),
                ),
        )
        .get_matches();

    let conf: Option<crate::entity::conf::ServerConf> =
        match std::fs::read_to_string(match matches.value_of("conf") {
            Some(v) => v.to_string(),
            None => utils::envs("HBPROXY_CONF", "/etc/hbproxy/hbproxy.yml"),
        }) {
            Err(_) => None,
            Ok(v) => match serde_yaml::from_str(v.as_str()) {
                Err(_) => None,
                Ok(v) => Some(v),
            },
        };
    let mut dup = Duplicate::Info;
    let logs = if matches.is_present("debug") {
        dup = Duplicate::Debug;
        "debug"
    } else {
        "info"
    };
    let mut loger = flexi_logger::Logger::try_with_str(logs)
        .unwrap()
        .duplicate_to_stderr(Duplicate::Warn)
        .duplicate_to_stdout(dup)
        .write_mode(flexi_logger::WriteMode::BufferAndFlush);
    if let Some(cfg) = &conf {
        if let Some(vs) = &cfg.server.log_path {
            loger = loger.log_to_file(
                flexi_logger::FileSpec::default()
                    .directory(std::path::PathBuf::from(vs))
                    .suppress_timestamp(),
            );
        }
    }
    if let Err(e) = loger.start() {
        println!("logger err:{}", e);
    }
    log::debug!("Hello, world!");
    if Application::init(conf) {
        initCmdApp(&matches);
        let rt = async_std::task::block_on(cmd::cmds(matches));
        //println!("block on:{}", rt);
        std::process::exit(rt);
    } else {
        log::error!("application init err!");
        std::process::exit(-1);
    }
}

fn initCmdApp(args: &clap::ArgMatches<'static>) {
    let app = Application::get_mut();
    if let Some(vs) = args.value_of("addr") {
      app.addrs = utils::host_defport(vs.to_string(), 6573)
    };
    if let Some(vs) = args.value_of("key") {
      app.keys = Some(vs.to_string())
    };
    if let Some(vs) = args.value_of("apiaddr") {
      app.apiaddrs = utils::host_defport(vs.to_string(), 6573)
    };
    if let Some(vs) = args.value_of("apikey") {
      app.apikeys = Some(vs.to_string())
    };
}

#[cfg(test)]
mod tests {
    use hbtp::Request;

    #[test]
    fn versions() {
        async_std::task::block_on(async {
            let mut req = Request::new("localhost:6573", 1);
            req.command("version");
            req.add_arg("hehe1", "123456789");
            match req.do_string(None, "dedededede").await {
                Err(e) => println!("do err:{}", e),
                Ok(res) => {
                    println!("res code:{}", res.get_code());
                    if let Some(bs) = res.get_bodys() {
                        println!("res data:{}", std::str::from_utf8(&bs[..]).unwrap())
                    }
                }
            };
        });
    }
    #[test]
    fn node_reg() {
        async_std::task::block_on(async {
            let mut req = Request::new("localhost:6573", 2);
            req.command("reg_node");
            /* let mut data = RegNodeReq{
              id:1234,
              token:"".into(),
            }; */
            let mut data = serde_json::Value::default();
            data["token"] = serde_json::Value::String("ihaha".to_string());
            // data["id"] = serde_json::Value::Number(serde_json::Number::from(123i64));
            match req.do_json(None, &data).await {
                Err(e) => println!("do err:{}", e),
                Ok(res) => {
                    println!("res code:{}", res.get_code());
                    if let Some(bs) = res.get_bodys() {
                        println!("res data:{}", std::str::from_utf8(&bs[..]).unwrap())
                    }
                }
            };
        });
    }
}
