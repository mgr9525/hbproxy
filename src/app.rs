use std::time::SystemTime;

use once_cell::sync::OnceCell;

use crate::utils;

static mut APPONE: OnceCell<Application> = OnceCell::new();

pub const VERSION: &str = "1.0.0";
pub struct Application {
    ctx: ruisutil::Context,
    pub id: String,
    pub workpath: String,
    pub cmdargs: clap::ArgMatches<'static>,
    pub conf: Option<crate::entity::conf::ServerConf>,
    pub addrs: String,
    pub keys: Option<String>,

    pub server_case: Option<crate::engine::ServerCase>,
}
impl Application {
    pub fn init(workpath: String, args: clap::ArgMatches<'static>) -> bool {
        let conf: Option<crate::entity::conf::ServerConf> =
            match std::fs::read_to_string(match args.value_of("conf") {
                Some(v) => v.to_string(),
                None => utils::envs("HBPROXY_CONF", "/etc/hbproxy/hbproxy.yml"),
            }) {
                Err(e) => None,
                Ok(v) => match serde_yaml::from_str(v.as_str()) {
                    Err(e) => None,
                    Ok(v) => Some(v),
                },
            };
        let addr_confs = match &conf {
            None => None,
            Some(v) => match &v.server.bind {
                None => None,
                Some(v) => Some(v.clone()),
            },
        };
        let key_confs = match &conf {
            None => None,
            Some(v) => match &v.server.key {
                None => None,
                Some(v) => Some(v.clone()),
            },
        };
        let app = Self {
            ctx: ruisutil::Context::background(None),
            id: String::new(),
            workpath: workpath,
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

    pub fn new_req(ctrl: i32, cmds: &str) -> hbtp::Request {
        let mut req = hbtp::Request::new(Self::get().addrs.as_str(), ctrl);
        req.command(cmds);
        if let Some(vs) = &Self::get().keys {
            let tms = ruisutil::strftime(SystemTime::now(), "%+");
            let rands = ruisutil::random(20);
            let sign = ruisutil::md5str(format!(
                "{}{}{}{}",
                cmds,
                tms.as_str(),
                rands.as_str(),
                vs.as_str()
            ));
            req.add_arg("times", tms.as_str());
            req.add_arg("random", rands.as_str());
            req.add_arg("sign", sign.as_str());
        }
        req
    }
}
