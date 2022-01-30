#![feature(linked_list_cursors)]
extern crate async_std;
extern crate clap;
extern crate flexi_logger;
extern crate futures;
extern crate hbtp;
extern crate hex;
extern crate log;
extern crate once_cell;
extern crate ruisutil;
extern crate serde;
extern crate serde_json;

mod app;
mod case;
mod cmd;
mod engine;
mod entity;
mod utils;

use std::io;

use clap::{App, Arg, SubCommand};

use crate::app::Application;

fn main() {
    let matches = App::new("My Super Program")
        .version("1.0")
        .author("Linsk Ruis. <mgr9525@gmail.com>")
        .about("Does awesome things")
        .arg(Arg::with_name("debug").long("debug").hidden(true))
        .arg(
            Arg::with_name("workpath")
                .short("w")
                .long("work")
                .help("process work path.(def:$HOME/.$(name))"),
        )
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
            SubCommand::with_name("test")
                .about("controls testing features")
                .version("1.3")
                .author("Someone E. <someone_else@other.com>")
                .arg(
                    Arg::with_name("debug")
                        .short("d")
                        .help("print debug information verbosely"),
                ),
        )
        .subcommand(SubCommand::with_name("listcodec").about("controls testing features"))
        .subcommand(
            SubCommand::with_name("run")
                .about("controls testing features")
                .arg(
                    Arg::with_name("bind")
                        .short("b")
                        .long("bind")
                        .value_name("IP:PORT")
                        .help("bind node server address.(def:0.0.0.0:6573)"),
                )
                .arg(
                    Arg::with_name("key")
                        .short("k")
                        .long("key")
                        .value_name("KEY")
                        .help("node server key"),
                ),
        )
        .subcommand(
            SubCommand::with_name("node")
                .about("start node and join to server")
                .subcommand(
                    SubCommand::with_name("join")
                        .about("start node and join to server")
                        .arg(
                            Arg::with_name("name")
                                .required(true)
                                .value_name("NAME")
                                .help("node name"),
                        )
                        .arg(
                            Arg::with_name("addr")
                                .short("a")
                                .long("addr")
                                .value_name("IP:PORT")
                                .help("join server address.(def:hbproxy.server:6573)"),
                        )
                        .arg(
                            Arg::with_name("key")
                                .short("k")
                                .long("key")
                                .value_name("KEY")
                                .help("node server key"),
                        ),
                )
                .subcommand(SubCommand::with_name("ls").about("node list")),
        )
        .get_matches();

    let mut dup = flexi_logger::Duplicate::Info;
    let logs = if matches.is_present("debug") {
        dup = flexi_logger::Duplicate::Debug;
        "debug"
    } else {
        "info"
        // dup = flexi_logger::Duplicate::Debug;
        // "debug"
    };
    let loger = flexi_logger::Logger::try_with_str(logs)
        .unwrap()
        .duplicate_to_stderr(dup);
    loger.start();
    log::debug!("Hello, world!");
    if Application::init("/data".into(), matches) {
        let rt = async_std::task::block_on(cmd::cmds());
        //println!("block on:{}", rt);
        std::process::exit(rt);
    } else {
        log::error!("application init err!");
        std::process::exit(-1);
    }
}

#[cfg(test)]
mod tests {
    use hbtp::Request;

    use crate::entity::node::RegNodeReq;

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
            let mut req = Request::new("localhost:6573", 1);
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
