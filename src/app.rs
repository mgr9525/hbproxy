use std::time::SystemTime;

use once_cell::sync::OnceCell;

use crate::utils;

static mut APPONE: OnceCell<Application> = OnceCell::new();

pub const VERSION: &str = "0.1.2";
pub struct Application {
    ctx: ruisutil::Context,
    pub cmdargs: clap::ArgMatches<'static>,
    pub conf: Option<crate::entity::conf::ServerConf>,
    pub addrs: String,
    pub keys: Option<String>,
    pub apiaddrs: String,
    pub apikeys: Option<String>,
    pub keyignore:bool,

    pub server_case: Option<crate::engine::ServerCase>,
}
impl Application {
    pub fn init(
        conf: Option<crate::entity::conf::ServerConf>,
        args: clap::ArgMatches<'static>,
    ) -> bool {
        let addr_confs = match &conf {
            None => None,
            Some(v) => match &v.server.host {
                None => None,
                Some(v) => Some(v.clone()),
            },
        };
        let apiaddr_confs = match &conf {
            None => None,
            Some(v) => match &v.api_server {
                None => None,
                Some(vc) => match &vc.host {
                    None => None,
                    Some(vs) => Some(vs.clone()),
                },
            },
        };
        let key_confs = match &conf {
            None => None,
            Some(v) => match &v.server.key {
                None => None,
                Some(v) => Some(v.clone()),
            },
        };
        let apikey_confs = match &conf {
            None => None,
            Some(v) => match &v.api_server {
                None => None,
                Some(vc) => match &vc.key {
                    None => None,
                    Some(vs) => Some(vs.clone()),
                },
            },
        };
        let app = Self {
            ctx: ruisutil::Context::background(None),
            conf: conf,

            addrs: if let Some(vs) = args.value_of("addr") {
                vs.to_string()
            } else if let Some(vs) = addr_confs {
                vs
            } else {
                utils::envs("HBPROXY_ADDR", "0.0.0.0:6573")
            },
            keys: if let Some(vs) = args.value_of("key") {
                // req.add_arg("node_key", vs);
                Some(vs.to_string())
            } else if let Some(vs) = key_confs {
                // req.add_arg("node_key", vs.as_str());
                Some(vs)
            } else if let Ok(vs) = std::env::var("HBPROXY_KEY") {
                Some(vs)
            } else {
                None
            },

            apiaddrs: if let Some(vs) = args.value_of("apiaddr") {
                vs.to_string()
            } else if let Some(vs) = apiaddr_confs {
                vs
            } else {
                utils::envs("HBPROXY_APIADDR", "localhost:6574")
            },
            apikeys: if let Some(vs) = args.value_of("apikey") {
                // req.add_arg("node_key", vs);
                Some(vs.to_string())
            } else if let Some(vs) = apikey_confs {
                // req.add_arg("node_key", vs.as_str());
                Some(vs)
            } else if let Ok(vs) = std::env::var("HBPROXY_APIKEY") {
                Some(vs)
            } else {
                None
            },
            keyignore:args.is_present("keyignore"),

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

    pub fn new_reqs(ctrl: i32, cmds: &str) -> hbtp::Request {
        Self::new_req(ctrl, cmds, true)
    }
    pub fn new_req(ctrl: i32, cmds: &str, is_api: bool) -> hbtp::Request {
        let addrs = if is_api {
            &Self::get().apiaddrs
        } else {
            &Self::get().addrs
        };
        let keys = if is_api {
            &Self::get().apikeys
        } else {
            &Self::get().keys
        };
        let mut req = hbtp::Request::new(addrs.as_str(), ctrl);
        req.command(cmds);
        if let Some(vs) = keys {
            let tms = ruisutil::strftime(SystemTime::now(), "%+");
            let rands = ruisutil::random(20);
            let sign = ruisutil::md5str(format!(
                "{}{}{}{}",
                cmds,
                tms.as_str(),
                rands.as_str(),
                vs.as_str()
            ));
            /* log::debug!(
                "sginos:{}{}{}{}",
                cmds,
                tms.as_str(),
                rands.as_str(),
                vs.as_str()
            );
            log::debug!("signs:{}",sign.as_str()); */
            req.add_arg("times", tms.as_str());
            req.add_arg("random", rands.as_str());
            req.add_arg("sign", sign.as_str());
        }
        req
    }
}
