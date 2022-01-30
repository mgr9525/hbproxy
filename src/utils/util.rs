use std::io;


pub async fn remote_version(addrs: &str) -> io::Result<String> {
    let mut req = hbtp::Request::new(addrs, 1);
    req.command("version");
    match req.dors(None, None).await {
        Err(e) => return Err(ruisutil::ioerr(e, None)),
        Ok(res) => {
            if let Some(bs) = res.get_bodys() {
              match std::str::from_utf8(&bs[..]){
                Err(e) => return Err(ruisutil::ioerr(e, None)),
                Ok(vs)=>{
                  return Ok(String::from(vs))
                }
              }
            }
        }
    };
    Err(ruisutil::ioerr("not found", None))
}
