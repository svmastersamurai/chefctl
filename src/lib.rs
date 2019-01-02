#![allow(dead_code)]
#![feature(generators, generator_trait)]

extern crate rand;

pub mod api;
pub mod platform;
pub mod process;
pub mod symlink;

pub const VERSION: &str = "0.0.1";
