use crate::platform::CHEF_PATH;
use rand::{thread_rng, Rng};
use std::cell::RefCell;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::fs::File;
use std::io::{Read, BufReader, Write};

pub fn splay(max: u32) -> Duration {
    Duration::from_secs(thread_rng().gen_range(0, max).into())
}

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

pub struct ChefProcess {
    inner: RefCell<Box<Command>>,
    child: Option<Result<std::process::Child, std::process::ExitStatus>>,
}

impl ChefProcess {
    pub fn new(cmd: &str) -> Self {
        let v: Vec<_> = cmd.split(' ').collect();
        let absolute_path: String = String::from(v[0]);
        let args = &v[1..];
        let mut cmd_line = Command::new(absolute_path);

        cmd_line.args(args);
        cmd_line.stdout(Stdio::piped());
        cmd_line.stderr(Stdio::piped());

        let inner: RefCell<Box<Command>> = RefCell::new(Box::new(cmd_line));

        Self { inner, child: None }
    }
}

pub enum State {
    PreRun(ChefProcess),
    Running(std::process::Child),
    PostRun(std::process::ExitStatus),
}

pub fn create<T>(cmd: T) -> State
where
    T: Into<String>,
{
    let inner = ChefProcess::new(&cmd.into());

    State::PreRun(inner)
}

// `pump` will take a standard I/O stream and then pump its contents into both
// the log file and the console.
fn pump<T>(mut fd: T) where T: Read {
    let mut log_file = match File::create(crate::platform::CHEF_RUN_CURRENT_PATH) {
        Ok(h) => h,
        Err(e) => panic!("error: {}", e),
    };

    loop {
        let mut buf = [0 as u8; 4096];
        let mut stdout = std::io::stdout();
        stdout.lock();

        match fd.read(&mut buf) {
            Ok(0) => { break }
            Ok(v) => {
                println!("flushed {}", v);
                log_file.write(&buf);
                stdout.write(&buf);
            }
            Err(e) => eprintln!("{}", e),
        };

        std::thread::sleep_ms(500);
    }
}

impl State {
    pub fn run(self) -> State {
        match self {
            State::PreRun(s) => {
                println!("chef pre-run");

                State::Running(s.inner.borrow_mut().spawn().unwrap())
            }
            State::Running(mut s) => {
                println!("chef run");

                let stdout_handle = match s.stdout.take() {
                    Some(s) => s,
                    None => panic!("no handle to stdout :("),
                };
                let buffered_stdout = BufReader::new(stdout_handle);
                std::thread::spawn(move || pump(buffered_stdout));

                let stderr_handle = match s.stderr.take() {
                    Some(s) => s,
                    None => panic!("no handle to stderr"),
                };
                let buffered_stderr = BufReader::new(stderr_handle);
                std::thread::spawn(move || pump(buffered_stderr));

                loop {
                    match s.try_wait() {
                        Ok(s) => match s {
                            Some(p) => {
                                return State::PostRun(p);
                            }
                            _ => (),
                        },
                        _ => (),
                    };

                    // Pump stdout/stderr into the file.
                    println!("waiting for process to finish");
                    std::thread::sleep(Duration::from_secs(1));
                }
            }
            State::PostRun(s) => {
                println!("chef post-run");

                println!("chef-client exited with {}", s.code().unwrap());

                State::PostRun(s)
            }
        }
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
