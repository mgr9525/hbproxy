use std::{env, io, path::Path};

use serde::de::DeserializeOwned;

pub async fn remote_version(addrs: &str) -> io::Result<String> {
    let mut req = hbtp::Request::new(addrs, 1);
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