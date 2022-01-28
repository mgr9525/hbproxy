use once_cell::sync::OnceCell;

static mut APPONE: OnceCell<Application> = OnceCell::new();

pub struct Application {
    ctx: ruisutil::Context,
    pub id: String,
    pub workpath: String,
    pub cmdargs: clap::ArgMatches<'static>,

    // pub roomsrv: Option<crate::videos::RoomServ>,
}
impl Application {
    pub fn init(workpath: String, args: clap::ArgMatches<'static>) -> bool {
        let app = Self {
            ctx: ruisutil::Context::background(None),
            id: String::new(),
            workpath: workpath,
            cmdargs: args,

            // roomsrv: None,
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
}