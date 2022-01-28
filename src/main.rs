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

mod app;
// mod case;

use clap::{App, Arg, SubCommand};

use crate::app::Application;

fn main() {
    let matches = App::new("My Super Program")
  .version("1.0")
  .author("Linsk Ruis. <mgr9525@gmail.com>")
  .about("Does awesome things")
  .arg(
    Arg::with_name("debug")
        .long("debug")
        .hidden(true)
)
.arg(
  Arg::with_name("workpath")
  .short("w")
      .long("work")
      .help("process work path.(def:$HOME/.$(name))")
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
                .help("bind video stream address.(def:0.0.0.0:6139)"),
        )
        .arg(
          Arg::with_name("web")
              .long("web")
              .value_name("IP:PORT")
              .help("bind web host address.(def:0.0.0.0:8080)"),
      )
          .arg(
              Arg::with_name("vpreset")
                  .long("vpreset")
                  .value_name("VALUE")
                  .help("video encode speed.(0:fast,1:ultrafast,2:superfast,3:veryfast,4:faster,5:medium)"),
          ).arg(
              Arg::with_name("dev_video_size")
                  .long("vdevsz")
                  .value_name("VALUE")
                  .help("video device size.(eg:1280x720)"),
          ).arg(
              Arg::with_name("dev_video_framerate")
                  .long("vdevrate")
                  .value_name("VALUE")
                  .help("video device frame rate.(eg:22,25,30)"),
          ).arg(
              Arg::with_name("dev_video_inputfmt")
                  .long("vdevfmt")
                  .value_name("VALUE")
                  .help("video device input format.(eg:mjpeg)"),
          ),
  )
  .get_matches();

  let mut dup = flexi_logger::Duplicate::Info;
  let logs = if matches.is_present("debug") {
      dup = flexi_logger::Duplicate::Debug;
      "debug"
  } else {
      "info"
  };
  let loger = flexi_logger::Logger::try_with_str(logs).unwrap()
    .duplicate_to_stderr(dup);
  loger.start();
  log::info!("Hello, world!");
  if Application::init("/data".into(), matches) {
      let rt = async_std::task::block_on(cmds());
      //println!("block on:{}", rt);
      std::process::exit(rt);
  } else {
      log::error!("application init err!");
      std::process::exit(-1);
  }
}

async fn cmds() -> i32 {
  if let Some(v) = Application::get().cmdargs.subcommand_matches("test") {
      if v.is_present("debug") {
          println!("Printing debug info...");
      } else {
          println!("Printing normally...");
      }
      0
  } else if let Some(v) = Application::get().cmdargs.subcommand_matches("run") {
      runs(v).await
  } else {
      -2
  }
}
async fn runs<'a>(args: &clap::ArgMatches<'a>) -> i32 {
  // case::ServerCase::new(ruisutil::Context::);
  let addrs = if let Some(vs) = args.value_of("bind") {
      vs
  } else {
      "0.0.0.0:6543"
  };
  let serv = hbtp::Engine::new(Some(Application::context()), addrs);
  // serv.reg_fun(1, handle_room);
  log::info!("server start on:{}", addrs);
  if let Err(e) = serv.run().await {
      log::error!("server run err:{}", e);
      return 1;
  }
  log::debug!("run end!");
  0
}