extern crate actix_web;
extern crate chefctl;
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate ctrlc;

use chefctl::{
    api::start_api_server,
    platform::{CONFIG_FILE_PATH, LOCK_FILE_PATH},
    process::{ChefClientArgs, PostRun, PreRun, Running, Waiting},
    VERSION,
};
use clap::Arg;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
};

lazy_static! {
    static ref APP_TO_CHEF: HashMap<&'static str, &'static str> = {
        let mut v = HashMap::new();

        v.insert("force", "--force");
        v.insert("immediate", "--once");
        v.insert("lock-file", "--lockfile");
        v.insert("lock-timeout", "--run-lock-timeout");
        v.insert("splay", "--splay");
        v.insert("why-run", "--why-run");
        v.insert("human", "-l auto");

        v
    };
}

fn handle_signals() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("error setting up signal trap");

    while running.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    println!("SIGINT received");
    std::process::exit(1);
}

fn args_from_clap(matches: clap::ArgMatches) -> String {
    let mut opts = ChefClientArgs::new();

    for (k, v) in &(*APP_TO_CHEF) {
        if let Some(value) = matches.value_of(k) {
            if k == &"splay" && matches.is_present("immediate") {
                continue;
            }

            opts.insert(&format!("{} {}", v, value));
        } else if matches.occurrences_of(k) > 0 {
            opts.insert(v);
        }
    }

    opts.insert("--no-fork");
    opts.insert("--force-formatter");

    // For some reason chef-client still double logs and I have to do this :(
    opts.insert("-L /dev/null");

    opts.into()
}

fn main() -> Result<(), std::io::Error> {
    // Start REST API server.
    std::thread::spawn(|| {
        start_api_server("127.0.0.1:6666")
            .map_err(|e| {
                eprintln!("api server disabled because {}", e);

                std::process::exit(1);
            })
            .expect("api creation failed, exiting");
    });

    // This is currently the best Rust has to offer for signal handling.
    std::thread::spawn(handle_signals);

    let args = args_from_clap(
        clap::App::new("chefctl")
            .about("a rust wrapper around chef-client")
            .version(VERSION)
            .arg(
                Arg::with_name("config")
                    .short("C")
                    .help("config file")
                    .default_value(CONFIG_FILE_PATH),
            )
            .arg(
                Arg::with_name("verbose")
                    .short("v")
                    .help("verbose output from chefctl"),
            )
            .arg(Arg::with_name("color").short("c").help("enable colors"))
            .arg(
                Arg::with_name("debug")
                    .short("d")
                    .help("enable chef debugging"),
            )
            .arg(
                Arg::with_name("human")
                    .short("H")
                    .help("human readable output"),
            )
            .arg(
                Arg::with_name("why-run")
                    .short("n")
                    .help("enable why-run mode"),
            )
            .arg(
                Arg::with_name("immediate")
                    .short("i")
                    .help("execute immediately. no splay. safely stop other chefctl processes.")
                    .conflicts_with("splay"),
            )
            .arg(
                Arg::with_name("splay")
                    .short("s")
                    .help("maximum number of seconds for a random splay.")
                    .default_value("870"),
            )
            .arg(
                Arg::with_name("lock-timeout")
                    .short("l")
                    .help("lock timeout in seconds")
                    .default_value("1800"),
            )
            .arg(
                Arg::with_name("lock-file")
                    .short("L")
                    .help("lock file location")
                    .default_value(LOCK_FILE_PATH),
            )
            .arg(
                Arg::with_name("quiet")
                    .short("q")
                    .help("do not print output to terminal"),
            )
            .get_matches(),
    );

    // Run the state machine.
    let pre_run = chefctl::process::StateMachine::<PreRun>::new(args);
    let waiting = chefctl::process::StateMachine::<Waiting>::from(pre_run);
    let running = chefctl::process::StateMachine::<Running>::from(waiting);
    let ___done = chefctl::process::StateMachine::<PostRun>::from(running);

    Ok(())
}
