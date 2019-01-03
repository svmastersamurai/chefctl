use crate::platform::CHEF_PATH;
use rand::{thread_rng, Rng};
use std::cell::RefCell;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::fs::File;
use std::io::{Read, BufReader, Write, Cursor};

pub fn splay(max: u32) -> Duration {
    Duration::from_secs(thread_rng().gen_range(0, max).into())
}

#[derive(Debug)]
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

#[derive(Debug)]
struct LogCursor(Cursor<Vec<u8>>);

impl LogCursor {
    pub fn clear(&mut self) {
        self.0.get_mut().clear();
    }

    pub fn get_mut(&mut self) -> &mut [u8] {
        self.as_mut()
    }

    pub fn get_ref(&self) -> &[u8] {
        self.as_ref()
    }
}

impl AsMut<[u8]> for LogCursor {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.get_mut().as_mut_slice()
    }
}

impl AsRef<[u8]> for LogCursor {
    fn as_ref(&self) -> &[u8] {
        self.0.get_ref().as_slice()
    }
}

#[derive(Debug)]
enum IoHandle {
    Stdout(std::io::Stdout),
    Stderr(std::io::Stderr),
}

impl IoHandle {
    fn lock(&self) -> IoHandleLocks {
        match self {
            IoHandle::Stdout(s) => IoHandleLocks::Stdout(s.lock()),
            IoHandle::Stderr(s) => IoHandleLocks::Stderr(s.lock()),
        }
    }
}

#[derive(Debug)]
enum IoHandleLocks<'a> {
    Stdout(std::io::StdoutLock<'a>),
    Stderr(std::io::StderrLock<'a>),
}

impl<'a> Write for IoHandleLocks<'a> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            IoHandleLocks::Stderr(s) => s.write(buf),
            IoHandleLocks::Stdout(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            IoHandleLocks::Stderr(s) => s.flush(),
            IoHandleLocks::Stdout(s) => s.flush(),
        }
    }
}

// `pump` will take a standard I/O stream and then pump its contents into both
// the log file and the console.
fn pump<T>(mut fd: T, handle: IoHandle) where T: Read {
    let mut log_file = match File::create(crate::platform::CHEF_RUN_CURRENT_PATH) {
        Ok(h) => h,
        Err(e) => panic!("error: {}", e),
    };
    let mut l = handle.lock();
    let mut buf = LogCursor(std::io::Cursor::new(Vec::new()));

    loop {
        match fd.read(&mut buf.get_mut()) {
            Ok(0) => { break }
            Ok(_) => {
                match log_file.write(buf.get_ref()) {
                    Ok(0) => {}
                    Ok(b) => println!("log flushed {}", b),
                    Err(e) => panic!("could not write to chef.cur.out: {}", e)
                };
                match l.write(buf.get_ref()) {
                    Ok(0) => {}
                    Ok(b) => println!("stdout flushed {}", b),
                    Err(e) => panic!("could not write to stdout: {}", e)
                };
                &buf.clear();
            }
            Err(e) => eprintln!("{}", e),
        };
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
                let stdout = std::io::stdout();
                let buffered_stdout = BufReader::new(stdout_handle);
                std::thread::spawn(move || pump(buffered_stdout, IoHandle::Stdout(stdout)));

                let stderr_handle = match s.stderr.take() {
                    Some(s) => s,
                    None => panic!("no handle to stderr"),
                };
                let stderr = std::io::stderr();
                let buffered_stderr = BufReader::new(stderr_handle);
                std::thread::spawn(move || pump(buffered_stderr, IoHandle::Stderr(stderr)));

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
