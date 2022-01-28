use std::io;



pub struct ServerConf{
  pub bind:String,
}
pub struct ServerCase{
  ctx: ruisutil::Context,
  conf:ServerConf,
  hbeg:hbtp::Engine,
}

impl ServerCase {
  pub fn new(ctx: ruisutil::Context,conf:ServerConf)->Self{
    let hbeg=hbtp::Engine::new(Some(ctx.clone()), conf.bind.as_str());
    Self{
      ctx:ctx,
      conf:conf,
      hbeg:hbeg,
    }
  }

  pub async fn run(&self)->io::Result<()>{
    self.hbeg.reg_fun(1, Self::handle1);
    // self.hbeg.run().await
    Ok(())
  }

  async fn handle1(c: hbtp::Context) -> io::Result<()>{
    Ok(())
  }
}