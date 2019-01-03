use crate::platform::CHEF_PATH;
use crate::state::APP_STATE;
use rand::{thread_rng, Rng};
use std::cell::RefCell;
use std::fmt::Display;
use std::fs::File;
use std::io::BufRead;
use std::io::{BufReader, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

pub fn splay(max: u32) -> Duration {
    Duration::from_secs(thread_rng().gen_range(0, max).into())
}

#[derive(Debug, Default)]
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

#[derive(Debug)]
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

impl State {
    pub fn run(self) -> State {
        match self {
            State::PreRun(s) => {
                println!("chef pre-run");
                let mut transition = State::Running(s.inner.borrow_mut().spawn().unwrap());
                let _ = APP_STATE.update_process_state(&mut transition);

                transition
            }
            State::Running(mut s) => {
                println!("chef run");

                let stdout_handle = match s.stdout.take() {
                    Some(s) => s,
                    None => panic!("no handle to stdout :("),
                };
                let mut stdout = std::io::stdout();
                let mut buffered_stdout = BufReader::new(stdout_handle);
                std::thread::spawn(move || {
                    stdout.lock();
                    let mut log_file = match File::create(crate::platform::CHEF_RUN_CURRENT_PATH) {
                        Ok(h) => h,
                        Err(e) => panic!("error: {}", e),
                    };
                    let mut buf = String::new();

                    loop {
                        match buffered_stdout.read_line(&mut buf) {
                            Ok(0) => {
                                match log_file.write(buf.as_bytes()) {
                                    Ok(0) => {}
                                    Ok(b) => println!("log flushed {} bytes", b),
                                    Err(e) => panic!("could not flush to chef.cur.out: {}", e),
                                };
                                match stdout.write(buf.as_bytes()) {
                                    Ok(0) => {}
                                    Ok(b) => println!("stdout flushed {} bytes", b),
                                    Err(e) => panic!("could not flush to stdout: {}", e),
                                };
                                break;
                            }
                            Ok(_) => {
                                let b = buf.to_owned();
                                buf.clear();
                                match log_file.write(b.as_bytes()) {
                                    Ok(0) => {}
                                    Ok(b) => println!("log wrote {} bytes", b),
                                    Err(e) => panic!("could not write to chef.cur.out: {}", e),
                                };
                                match stdout.write(b.as_bytes()) {
                                    Ok(0) => {}
                                    Ok(b) => println!("stdout wrote {} bytes", b),
                                    Err(e) => panic!("could not write to stdout: {}", e),
                                };
                            }
                            Err(e) => eprintln!("{}", e),
                        };
                    }
                });

                loop {
                    if let Ok(s) = s.try_wait() {
                        if let Some(p) = s {
                            let mut transition = State::PostRun(p);
                            let _ = APP_STATE.update_process_state(&mut transition);

                            return transition;
                        }
                    }

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

impl Display for State {
    fn fmt(&self, w: &mut std::fmt::Formatter) -> std::fmt::Result {
        let my_type = match self {
            State::PreRun(_) => "PreRun",
            State::Running(_) => "Running",
            State::PostRun(_) => "PostRun",
        };

        write!(w, "State({})", my_type)
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
