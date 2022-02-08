use once_cell::sync::OnceCell;

use crate::utils;

static mut APPONE: OnceCell<Application> = OnceCell::new();

pub const VERSION: &str = "1.0.0";
pub struct Application {
    ctx: ruisutil::Context,
    pub id: String,
    pub workpath: String,
    pub cmdargs: clap::ArgMatches<'static>,
    pub addrs: String,
    pub keys: Option<String>,

    pub server_case: Option<crate::case::ServerCase>,
}
impl Application {
    pub fn init(workpath: String, args: clap::ArgMatches<'static>) -> bool {
        let app = Self {
            ctx: ruisutil::Context::background(None),
            id: String::new(),
            workpath: workpath,
            addrs: if let Some(vs) = args.value_of("addr") {
                vs.to_string()
            } else {
                utils::envs("HBPROXY_ADDR", "0.0.0.0:6573")
            },
            keys: if let Some(vs) = args.value_of("key") {
                // req.add_arg("node_key", vs);
                Some(vs.to_string())
            } else if let Ok(vs) = std::env::var("HBPROXY_KEY") {
                // req.add_arg("node_key", vs.as_str());
                Some(vs)
            } else {
                None
            },
            cmdargs: args,

            server_case: None,
        };
        unsafe {
            match APPONE.set(app) {
                Ok(_) => return true,
                Err(_) => return false,
            }
        }
    }
    pub fn get() -> &'static Application {
        unsafe { APPONE.get().unwrap() }
    }
    pub fn get_mut() -> &'static mut Application {
        unsafe { APPONE.get_mut() }.unwrap()
    }
    pub fn stop() {
        Self::get().ctx.stop();
        unsafe {
            APPONE = OnceCell::new();
        }
    }
    pub fn context() -> ruisutil::Context {
        Self::get().ctx.clone()
    }

    pub fn new_req(ctrl:i32) -> hbtp::Request {
      let mut req = hbtp::Request::new(Self::get().addrs.as_str(), ctrl);
      if let Some(vs) = &Self::get().keys {
          req.add_arg("node_key", vs.as_str());
      }
      req
    }
}
