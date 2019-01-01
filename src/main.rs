extern crate actix_web;
extern crate chefctl;
extern crate clap;
#[macro_use]
extern crate lazy_static;

use chefctl::{
    api::start_api_server,
    platform::{CONFIG_FILE_PATH, LOCK_FILE_PATH},
    symlink::with_symlink,
    VERSION,
};
use clap::Arg;
use std::collections::HashMap;

lazy_static! {
    static ref APP_TO_CHEF: HashMap<&'static str, &'static str> = {
        let mut v = HashMap::new();
        v.insert("force", "--force");
        v.insert("immediate", "--once");
        v.insert("lock-file", "--lockfile");
        v.insert("lock-timeout", "--run-lock-timeout");
        v.insert("splay", "--splay");
        v.insert("why-run", "--why-run");

        v
    };
}

fn main() {
    std::thread::spawn(|| {
        start_api_server("127.0.0.1:6666")
            .map_err(|e| {
                eprintln!("api server disabled because {}", e);

                std::process::exit(1);
            })
            .expect("api creation failed, exiting");
    });

    let _matches = clap::App::new("chefctl")
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
        .get_matches();

    with_symlink("/tmp/my_file", |a| println!("ohai friend! {:?}", a));

    for (k, v) in &(*APP_TO_CHEF) {
        println!("{} = {}", k, v);
    }
}
