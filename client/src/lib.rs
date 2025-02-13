#![allow(clippy::type_complexity)]

// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

pub mod client;
pub use client::EventHandler;

pub mod utils;

#[macro_export]
macro_rules! display_debug {
    ($commands:expr, $($tt:tt)*) => {
        $crate::display_impl!(debug, debug, $commands, $($tt)*)
    };
}

#[macro_export]
macro_rules! display_info {
    ($commands:expr, $($tt:tt)*) => {
        $crate::display_impl!(info, info, $commands, $($tt)*)
    };
}

#[macro_export]
macro_rules! display_warn {
    ($commands:expr, $($tt:tt)*) => {
        $crate::display_impl!(warn, warn, $commands, $($tt)*)
    };
}

#[macro_export]
macro_rules! display_error {
    ($commands:expr, $($tt:tt)*) => {
        $crate::display_impl!(error, error, $commands, $($tt)*)
    };
}

#[macro_export]
macro_rules! display_success {
    ($commands:expr, $($tt:tt)*) => {
        $crate::display_impl!(success, info, $commands, $($tt)*)
    };
}

#[macro_export]
macro_rules! display_impl {
    ($constructor:ident, $log_chan:ident, $commands:expr, $($tt:tt)*) => {
        {
            let msg = format!($($tt)*);
            $commands.send_event(LogEvent::Push(Message::$constructor(&msg)));
            $log_chan!("[LogDisp]: {}", msg);
        }
    };
}

pub mod log_macros {
    pub use crate::{display_debug, display_error, display_info, display_success, display_warn};
}
