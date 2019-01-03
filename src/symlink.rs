#![feature(generators, generator_trait)]

extern crate chrono;

use crate::platform::CHEF_RUN_CURRENT_PATH;
use chrono::prelude::*;
use std::ops::{Generator, GeneratorState};
use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as std_symlink;

#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::symlink as std_symlink;

fn timestamp() -> String {
    let now: DateTime<Local> = Local::now();

    format!(
        "{}{:02}{:02}.{:02}{:02}.{}",
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
    // for prod:
    // let p: PathBuf = ["/", "var", "chef", "outputs", file_name.as_str()]
    //     .iter()
    //     .collect();
    let p: PathBuf = ["/", "tmp", file_name.as_str()].iter().collect();

    match p.to_str() {
        Some(p) => p.to_owned(),
        None => panic!("wtf"),
    }
}

pub fn with_symlink<F>(p: &str, f: F)
where
    F: Fn() + Sized,
{
    let mut generator = || {
        let output_path = output_path();

        match std::fs::File::create(&output_path) {
            Ok(f) => f,
            Err(e) => panic!("create(\"{}\") {}", &output_path, e),
        };

        ensure_path(p);

        println!("create symlink {} -> {}", &p, &output_path);

        let path = PathBuf::from(p);

        if path.exists() {
            match std::fs::remove_file(p) {
                Ok(_) => {}
                Err(e) => panic!("remove_file(\"{}\"): {}", p, e),
            };
        }

        match std_symlink(&output_path, p) {
            Ok(_) => yield &f,
            Err(e) => panic!("io::Error not handled: {}", e),
        };

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

// Validates that the directory structure needed for the file about to be written
// exists.
fn ensure_path<P>(p: P)
where
    PathBuf: From<P>,
{
    let path = PathBuf::from(p);

    if !path.parent().unwrap().exists() {
        match std::fs::create_dir_all(path.parent().unwrap()) {
            Ok(_) => {}
            Err(e) => panic!("could not create_dir_all: {}", e),
        }
    }
}
