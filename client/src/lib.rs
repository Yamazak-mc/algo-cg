// #![allow(unused)]
// #![warn(unused_mut, unused_must_use)]

pub mod button;

pub mod log_display;

pub mod util;

pub mod client;
pub use client::EventHandler;

pub const DEFAULT_SERVER_PORT: u16 = 54345;
