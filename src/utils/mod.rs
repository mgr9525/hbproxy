mod util;
pub mod msg;

pub use util::{remote_version,envs,ymlfile,mytimes};

pub const HbtpTokenErr: i32 = 100;



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
