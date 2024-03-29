pub mod msg;
mod util;

pub use util::{
    compare_version, envs, host_defport, mytimes, remote_version, ymlfile, CompareVersion,
};

pub const HBTP_TOKEN_ERR: i32 = 100;

/// Declares Unix-specific items.
#[allow(unused_macros)]
#[macro_export]
macro_rules! cfg_unix {
    ($($item:item)*) => {
        $(
            #[cfg(any(unix, feature = "docs"))]
            #[cfg_attr(feature = "docs", doc(cfg(unix)))]
            $item
        )*
    }
}

/// Declares Windows-specific items.
#[allow(unused_macros)]
#[macro_export]
macro_rules! cfg_windows {
    ($($item:item)*) => {
        $(
            #[cfg(any(windows, feature = "docs"))]
            #[cfg_attr(feature = "docs", doc(cfg(windows)))]
            $item
        )*
    }
}
