pub mod env;
pub mod error;
// pub mod eval;
pub mod value;

macro_rules! debug_info {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)] {
            println!("\x1b[90mINFO: {}\x1b[0m", format_args!($($args)*));
        }
    };
}
pub(crate) use debug_info;

macro_rules! debug_warn {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)] {
            println!("\x1b[93mWARNING: {}\x1b[0m", format_args!($($args)*));
        }
    };
}
pub(crate) use debug_warn;

macro_rules! debug_unsafe {
    ($($args:tt)*) => {
        #[cfg(debug_assertions)] {
            println!("\x1b[95mUNSAFE: {}\x1b[0m", format_args!($($args)*));
        }
    };
}
pub(crate) use debug_unsafe;
