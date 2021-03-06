use crate::{
    platform::{CHEF_PATH, CHEF_RUN_CURRENT_PATH, CHEF_RUN_LAST_PATH},
    state::APP_STATE,
    symlink::create_symlink,
};
use chrono::prelude::{DateTime, Datelike, Local, Timelike};
use rand::{thread_rng, Rng};
use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    io::{stderr, stdout, BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    thread::sleep,
    time::Duration,
};

const BUFFER_CAPACITY: usize = 4096;

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
    let p: PathBuf = ["C:\\", "chef", "outputs", file_name.as_str()]
        .iter()
        .collect();

    match p.to_str() {
        Some(p) => p.to_owned(),
        None => panic!("wtf"),
    }
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

lazy_static! {
    // Subsequent calls to `output_path()` will create new timestamps, so we need to
    // create a lazily initialized filename that is consistent across the entire run
    // of the application.
    pub static ref LOG_FILE_PATH: String = { output_path() };
}

// A simple splay from the thread local random number generator.
// Since we're barely using the `rand` crate we can use other random number
// generators if we need to that have different types of distributions.
pub fn splay(max: u32) -> Duration {
    Duration::from_secs(thread_rng().gen_range(0, max).into())
}

fn pump<'a>(mut opts: &'a mut OpenOptions, reader: &'a mut BufRead, writer: &'a mut Write) {
    let mut log_file = open_log(LOG_FILE_PATH.to_string(), &mut opts);
    let mut buf = String::with_capacity(BUFFER_CAPACITY);

    loop {
        match reader.read_line(&mut buf) {
            Ok(_) => {
                let b = buf.to_owned();
                buf.clear();
                match log_file.write(b.as_bytes()) {
                    Ok(_) => {}
                    Err(e) => panic!("could not write to chef.cur.out: {}", e),
                };
                match writer.write(b.as_bytes()) {
                    Ok(_) => {}
                    Err(e) => panic!("could not write to stdout: {}", e),
                };
            }
            Err(e) => eprintln!("{}", e),
        };
    }
}

fn open_log(path: String, opts: &mut OpenOptions) -> File {
    match opts.open(&path) {
        Ok(h) => h,
        Err(e) => panic!("error opening \"{}\": {}", path, e),
    }
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
    child: Option<Result<RefCell<Child>, ExitStatus>>,
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

        // Create the log file ahead of time so that we can open it in
        // append mode later.
        match File::create(PathBuf::from(&*LOG_FILE_PATH)) {
            Ok(_) => {}
            Err(e) => panic!("could not create \"{}\": {}", &*LOG_FILE_PATH, e),
        }

        let inner = RefCell::new(Box::new(cmd_line));

        println!("created process: {}", cmd);

        Self { inner, child: None }
    }
}

// Represents the state of the chef process:
//      PreRun  - Initial State, only creates an empty `Command`.
//      Waiting - Pausing execution for the `splay` value returned between a user
//                specified interval.
//      Running - The `chef-client` process is executed. Logs are piped to disk.
//      PostRun - `chef-client` has finished execution and will bubble up the
//                exit code.
#[derive(Debug)]
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
        let _ = create_symlink(chef_cur_out, &LOG_FILE_PATH);
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

    pub fn spawn(&mut self) -> Child {
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
                sleep(one_sec);
            });

        let mut countdown = val.state.splay.as_secs();
        while let Ok(_) = rx.recv() {
            if countdown > 0 {
                countdown -= 1;
                APP_STATE.update_splay_countdown(countdown);
            } else {
                break;
            }
            sleep(one_sec);
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
    child: Child,
}

impl Running {
    fn new(child: Child) -> Self {
        Self { child }
    }

    pub fn pump_stdout(&mut self) -> std::io::Result<()> {
        let mut opts = OpenOptions::new();
        opts.append(true);
        opts.create(false);

        let stdout_handle = match self.child.stdout.take() {
            Some(s) => s,
            None => panic!("no handle to stdout :("),
        };
        let mut writer = stdout();
        let mut reader = BufReader::new(stdout_handle);

        std::thread::spawn(move || {
            writer.lock();
            pump(&mut opts, &mut reader, &mut writer);
        });

        Ok(())
    }

    pub fn pump_stderr(&mut self) -> std::io::Result<()> {
        let mut opts = OpenOptions::new();
        opts.append(true);
        opts.create(false);

        let stderr_handle = match self.child.stderr.take() {
            Some(s) => s,
            None => panic!("no handle to stderr :("),
        };
        let mut writer = stderr();
        let mut reader = BufReader::new(stderr_handle);

        std::thread::spawn(move || {
            writer.lock();
            pump(&mut opts, &mut reader, &mut writer);
        });

        Ok(())
    }

    pub fn run(&mut self) -> std::io::Result<Option<ExitStatus>> {
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
                    APP_STATE.update_process_state("post-run".into());

                    return StateMachine {
                        state: PostRun { exit_status },
                    };
                }
            }
            sleep(Duration::from_millis(500));
        }
    }
}

#[derive(Debug)]
pub struct PostRun {
    exit_status: ExitStatus,
}

impl PostRun {
    fn new(exit_status: ExitStatus) -> Self {
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
