#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;
extern crate rand;

pub mod api;
pub mod platform;
pub mod process;
pub mod state;
pub mod symlink;

pub const VERSION: &str = "0.0.1";
