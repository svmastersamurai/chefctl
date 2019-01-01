#![feature(generators, generator_trait)]
extern crate chrono;

use chrono::prelude::*;
use std::ops::{Generator, GeneratorState};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file;

#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::symlink as symlink_file;

#[cfg(target_os = "windows")]
fn output_path() -> String {
    let mut p = PathBuf::new();
    let now: DateTime<Utc> = Utc::now();
    let file_name = format!("outputs-{}.log", now);

    match std::env::var("SYSTEMROOT") {
        Ok(r) => p.push(r),
        Err(e) => p.push("C:"),
    }

    vec!["chef", "outputs"].into_iter().map(|s| p.push(s));

    p.into()
}

#[cfg(not(target_os = "windows"))]
fn output_path() -> String {
    let now: DateTime<Utc> = Utc::now();
    let file_name = format!("chef_run-{}.log", now);
    let p: PathBuf = ["var", "log", "chef", "outputs", file_name.as_str()]
        .iter()
        .collect();

    match p.to_str() {
        Some(p) => p.to_owned().into(),
        None => panic!("wtf"),
    }
}

pub fn with_symlink<F>(p: &str, f: F)
where
    F: Fn(String) + Sized,
{
    let mut generator = || {
        let output_path = output_path();
        println!("opening symlink path to {}", &output_path);
        yield &f;
        println!("done with yield");
    };

    loop {
        match unsafe { generator.resume() } {
            GeneratorState::Yielded(_) => {
                f("a".to_string());
            }
            GeneratorState::Complete(()) => {
                println!("the job is finished!");
                break;
            }
        }
    }
}
