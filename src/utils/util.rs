use std::{env, io, path::Path};

use serde::de::DeserializeOwned;

pub async fn remote_version(mut req: hbtp::Request) -> io::Result<String> {
    req.command("version");
    match req.dors(None, None).await {
        Err(e) => return Err(ruisutil::ioerr(e, None)),
        Ok(res) => {
            if let Some(bs) = res.get_bodys() {
                match std::str::from_utf8(&bs[..]) {
                    Err(e) => return Err(ruisutil::ioerr(e, None)),
                    Ok(vs) => return Ok(String::from(vs)),
                }
            }
        }
    };
    Err(ruisutil::ioerr("not found", None))
}

pub fn envs(key: &str, defs: &str) -> String {
    match env::var(key) {
        Err(_) => String::from(defs),
        Ok(vs) => {
            if vs.is_empty() {
                String::from(defs)
            } else {
                vs
            }
        }
    }
}

pub fn ymlfile<T: DeserializeOwned, P: AsRef<Path>>(path: P) -> io::Result<T> {
    let v = std::fs::read_to_string(path)?;
    match serde_yaml::from_str(v.as_str()) {
        Err(e) => Err(ruisutil::ioerr(format!("yml err:{}", e), None)),
        Ok(v) => Ok(v),
    }
}

pub fn mytimes(sec: u64) -> String {
    if sec >= 86400 {
        let day = sec / 86400;
        let hour = (sec % 86400) / 3600;
        let min = (sec % 86400 % 3600) / 60;
        let sec = sec % 86400 % 3600 & 60;
        format!("{}d{}h{}m{}s", day, hour, min, sec)
    } else if sec >= 3600 {
        let hour = sec / 3600;
        let min = (sec % 3600) / 60;
        let sec = sec % 3600 & 60;
        format!("{}h{}m{}s", hour, min, sec)
    } else if sec >= 60 {
        let min = sec / 60;
        let sec = sec % 60;
        format!("{}m{}s", min, sec)
    } else {
        format!("{}s", sec)
    }
}
