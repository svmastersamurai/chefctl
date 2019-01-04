use crate::{
    platform::{CHEF_PATH, CHEF_RUN_CURRENT_PATH, CHEF_RUN_LAST_PATH},
    state::APP_STATE,
    symlink::create_symlink,
};
use chrono::prelude::{DateTime, Datelike, Local, Timelike};
use rand::{thread_rng, Rng};
use std::{
    cell::RefCell,
    fmt::Display,
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    ops::DerefMut,
    path::PathBuf,
    process::{Command, Stdio},
    rc::Rc,
    time::Duration,
};

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
pub fn output_path() -> String {
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
pub fn output_path() -> String {
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
// A simple splay from the thread local random number generator.
// Since we're barely using the `rand` crate we can use other random number
// generators if we need to that have different types of distributions.
pub fn splay(max: u32) -> Duration {
    Duration::from_secs(thread_rng().gen_range(0, max).into())
}

#[derive(Debug, Default)]
// A simple struct for constructing the command line arguments passed into the
// local installation of `chef-client`.
pub struct ChefClientArgs {
    cmd: Vec<String>,
}

impl ChefClientArgs {
    pub fn new() -> Self {
        Self { cmd: Vec::new() }
    }

    pub fn insert(&mut self, opt: &str) {
        self.cmd.push(opt.into());
    }
}

impl Into<String> for ChefClientArgs {
    fn into(mut self) -> String {
        self.cmd.insert(0, CHEF_PATH.into());

        self.cmd.join(" ")
    }
}

impl<S> From<S> for ChefClientArgs
where
    S: ToString,
{
    fn from(_: S) -> Self {
        Self::new()
    }
}

#[derive(Debug)]
// Represents a handle to the chef process to-be-launched on the client.
pub struct ChefProcess {
    inner: RefCell<Box<Command>>,
    child: Option<Result<RefCell<std::process::Child>, std::process::ExitStatus>>,
}

impl ChefProcess {
    // Creates the local process but does not execute it yet. The initial
    // bookkeeping is to setup the piped `stderr` and `stdout` so output can be
    // logged to both the console as well as a log file.
    pub fn new(cmd: String) -> Self {
        let v: Vec<_> = cmd.split(' ').collect();
        let absolute_path: String = String::from(v[0]);
        let args = &v[1..];
        let mut cmd_line = Command::new(absolute_path);

        cmd_line.args(args);
        cmd_line.stdout(Stdio::piped());
        cmd_line.stderr(Stdio::piped());

        let inner = RefCell::new(Box::new(cmd_line));

        Self { inner, child: None }
    }
}

#[derive(Debug)]
// Represents the state of the chef process:
//      PreRun  - Initial State, only creates an empty `Command`.
//      Waiting - Pausing execution for the `splay` value returned between a user
//                specified interval.
//      Running - The `chef-client` process is executed. Logs are piped to disk.
//      PostRun - `chef-client` has finished execution and will bubble up the
//                exit code.
pub struct StateMachine<S> {
    state: S,
}

impl StateMachine<PreRun> {
    pub fn new(val: String) -> Self {
        Self {
            state: PreRun::new(val),
        }
    }
}

#[derive(Debug)]
pub struct PreRun {
    process: ChefProcess,
}

impl PreRun {
    fn new(val: String) -> Self {
        let process = ChefProcess::new(val);
        APP_STATE.update_process_state("pre-run".into());

        Self { process }
    }
}

impl From<StateMachine<PreRun>> for StateMachine<Waiting> {
    fn from(val: StateMachine<PreRun>) -> StateMachine<Waiting> {
        println!("chef pre-run");
        let chef_cur_out = &String::from(CHEF_RUN_CURRENT_PATH);
        let chef_prev_out = &String::from(CHEF_RUN_LAST_PATH);
        let prev_path = match std::fs::read_link(CHEF_RUN_CURRENT_PATH) {
            Ok(s) => Some(s),
            Err(_) => None,
        };

        if let Some(p) = prev_path {
            if p.to_str().unwrap() != chef_prev_out {
                let update_symlink = &String::from(p.to_str().unwrap());

                let _ = create_symlink(chef_prev_out, update_symlink);
            }
        }
        let _ = create_symlink(chef_cur_out, &output_path());
        let duration = splay(10);

        APP_STATE.update_process_state("waiting".into());
        StateMachine {
            state: Waiting {
                process: val.state.process,
                splay: duration,
            },
        }
    }
}

#[derive(Debug)]
pub struct Waiting {
    process: ChefProcess,
    splay: Duration,
}

impl Waiting {
    fn new(process: ChefProcess, splay: Duration) -> Self {
        Self { process, splay }
    }

    pub fn spawn(&mut self) -> std::process::Child {
        self.process.inner.borrow_mut().spawn().unwrap()
    }
}

impl From<StateMachine<Waiting>> for StateMachine<Running> {
    fn from(mut val: StateMachine<Waiting>) -> StateMachine<Running> {
        let one_sec = Duration::from_secs(1);
        let (tx, rx) = std::sync::mpsc::channel::<u64>();
        let _ = std::thread::Builder::new()
            .name("ticker-tx".into())
            .spawn(move || loop {
                let _ = tx.send(1);
                std::thread::sleep(one_sec);
            });

        let mut countdown = val.state.splay.as_secs();
        while let Ok(_) = rx.recv() {
            if countdown > 0 {
                countdown -= 1;
                APP_STATE.update_splay_countdown(countdown);
            } else {
                break;
            }
            std::thread::sleep(one_sec);
        }

        APP_STATE.update_process_state("running".into());
        StateMachine {
            state: Running {
                child: val.state.spawn(),
            },
        }
    }
}

#[derive(Debug)]
pub struct Running {
    child: std::process::Child,
}

impl Running {
    fn new(child: std::process::Child) -> Self {
        Self { child }
    }

    pub fn pump_stdout(&mut self) -> std::io::Result<()> {
        let stdout_handle = match self.child.stdout.take() {
            Some(s) => s,
            None => panic!("no handle to stdout :("),
        };
        let mut stdout = std::io::stdout();
        let mut buffered_stdout = BufReader::new(stdout_handle);

        // This thead will pump all of the data from the process' stdout
        // handle into the log as well as the console.
        std::thread::spawn(move || {
            stdout.lock();
            let mut log_file = match OpenOptions::new()
                .append(true)
                .create(true)
                .open(crate::platform::CHEF_RUN_CURRENT_PATH)
            {
                Ok(h) => h,
                Err(e) => panic!("error: {}", e),
            };
            let mut buf = String::new();

            loop {
                match buffered_stdout.read_line(&mut buf) {
                    Ok(_) => {
                        let b = buf.to_owned();
                        buf.clear();
                        match log_file.write(b.as_bytes()) {
                            Ok(_) => {}
                            Err(e) => panic!("could not write to chef.cur.out: {}", e),
                        };
                        match stdout.write(b.as_bytes()) {
                            Ok(_) => {}
                            Err(e) => panic!("could not write to stdout: {}", e),
                        };
                    }
                    Err(e) => eprintln!("{}", e),
                };
            }
        });

        Ok(())
    }

    pub fn pump_stderr(&mut self) -> std::io::Result<()> {
        let stderr_handle = match self.child.stderr.take() {
            Some(s) => s,
            None => panic!("no handle to stdout :("),
        };
        let mut stderr = std::io::stderr();
        let mut buffered_stderr = BufReader::new(stderr_handle);

        // This thead will pump all of the data from the process' stdout
        // handle into the log as well as the console.
        std::thread::spawn(move || {
            stderr.lock();
            let mut log_file = match OpenOptions::new()
                .append(true)
                .create(false)
                .open(crate::platform::CHEF_RUN_CURRENT_PATH)
            {
                Ok(h) => h,
                Err(e) => panic!("error: {}", e),
            };
            let mut buf = String::new();

            loop {
                match buffered_stderr.read_line(&mut buf) {
                    Ok(_) => {
                        let b = buf.to_owned();
                        buf.clear();
                        match log_file.write(b.as_bytes()) {
                            Ok(_) => {}
                            Err(e) => panic!("could not write to chef.cur.out: {}", e),
                        };
                        match stderr.write(b.as_bytes()) {
                            Ok(_) => {}
                            Err(e) => panic!("could not write to stdout: {}", e),
                        };
                    }
                    Err(e) => eprintln!("{}", e),
                };
            }
        });

        Ok(())
    }

    pub fn run(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }
}

impl From<StateMachine<Running>> for StateMachine<PostRun> {
    fn from(mut val: StateMachine<Running>) -> StateMachine<PostRun> {
        let _ = val.state.pump_stdout();
        let _ = val.state.pump_stderr();

        loop {
            if let Ok(s) = val.state.run() {
                if let Some(exit_status) = s {
                    let _ = APP_STATE.update_process_state("post-run".into());
                    return StateMachine {
                        state: PostRun { exit_status },
                    };
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

#[derive(Debug)]
pub struct PostRun {
    exit_status: std::process::ExitStatus,
}

impl PostRun {
    fn new(exit_status: std::process::ExitStatus) -> Self {
        Self { exit_status }
    }
}

mod test {
    #[test]
    #[cfg(not(target_os = "windows"))]
    fn renders_cmd_line() {
        use super::ChefClientArgs;

        let expected = String::from("/opt/chef/embedded/bin/chef-client --force");
        let mut opts = ChefClientArgs::new();
        opts.insert("--force");

        let s: String = opts.into();

        assert_eq!(s, expected);
    }
}
