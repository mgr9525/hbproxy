use std::io;



pub struct ServerConf{
}
pub struct ServerCase{
  ctx: ruisutil::Context,
  conf:ServerConf,
}

impl ServerCase {
  pub fn new(ctx: ruisutil::Context,conf:ServerConf)->Self{
    Self{
      ctx:ctx,
      conf:conf,
    }
  }

  pub async fn check(&self,c: hbtp::Context) -> io::Result<()> {
    Ok(())
  }
}