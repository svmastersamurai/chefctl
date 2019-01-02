#![feature(generators, generator_trait)]

extern crate chrono;

use crate::platform::CHEF_RUN_CURRENT_PATH;
use chrono::prelude::*;
use std::ops::{Generator, GeneratorState};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file;

#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::symlink as symlink_file;

fn timestamp() -> String {
    let now: DateTime<Utc> = Utc::now();

    format!(
        "{}{}{}.{}{}.{}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.timestamp(),
    )
}

fn chef_run_log_path() -> String {
    format!("chef.{}.out", timestamp())
}

#[cfg(target_os = "windows")]
fn output_path() -> String {
    let file_name = chef_run_log_path();
    let mut p = PathBuf::new();

    match std::env::var("SYSTEMROOT") {
        Ok(r) => p.push(r),
        Err(e) => p.push("C:"),
    }

    vec!["chef", "outputs", file_name.as_str()]
        .into_iter()
        .map(|s| p.push(s));

    p.into()
}

#[cfg(not(target_os = "windows"))]
fn output_path() -> String {
    let file_name = chef_run_log_path();
    let p: PathBuf = ["/", "var", "log", "chef", "outputs", file_name.as_str()]
        .iter()
        .collect();

    match p.to_str() {
        Some(p) => p.to_owned().into(),
        None => panic!("wtf"),
    }
}

pub fn with_symlink<F>(p: &str, f: F)
where
    F: Fn() + Sized,
{
    let mut generator = || {
        let output_path = output_path();
        println!("create symlink {} -> {}", p, &output_path);
        yield &f;
        println!("done with yield");
    };

    loop {
        match unsafe { generator.resume() } {
            GeneratorState::Yielded(_) => f(),
            GeneratorState::Complete(()) => {
                println!("the job is finished!");
                break;
            }
        }
    }
}
