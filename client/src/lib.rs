#![allow(clippy::type_complexity)]

// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

pub mod client;
pub use client::EventHandler;

pub mod utils;

#[macro_export]
macro_rules! display_debug {
    ($commands:expr, $($tt:tt)*) => {
        $commands.send_event(LogEvent::Push(Message::debug(
            format!($($tt)*)
        )))
    };
}

#[macro_export]
macro_rules! display_info {
    ($commands:expr, $($tt:tt)*) => {
        $commands.send_event(LogEvent::Push(Message::info(
            format!($($tt)*)
        )))
    };
}

#[macro_export]
macro_rules! display_warn {
    ($commands:expr, $($tt:tt)*) => {
        $commands.send_event(LogEvent::Push(Message::warn(
            format!($($tt)*)
        )))
    };
}

#[macro_export]
macro_rules! display_error {
    ($commands:expr, $($tt:tt)*) => {
        $commands.send_event(LogEvent::Push(Message::error(
            format!($($tt)*)
        )))
    };
}

#[macro_export]
macro_rules! display_success {
    ($commands:expr, $($tt:tt)*) => {
        $commands.send_event(LogEvent::Push(Message::success(
            format!($($tt)*)
        )))
    };
}

pub mod log_macros {
    pub use crate::{display_debug, display_error, display_info, display_success, display_warn};
}
