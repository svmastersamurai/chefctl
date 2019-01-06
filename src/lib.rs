#![allow(dead_code)]

extern crate actix_web;
extern crate rand;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;

pub mod api;
pub mod health;
pub mod platform;
pub mod process;
pub mod state;
pub mod symlink;

pub const VERSION: &str = "0.0.1";
