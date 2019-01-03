use std::fmt::Display;
use std::ops::DerefMut;
use std::sync::RwLock;

pub enum Error {
    CannotUpdateProcess,
}

#[derive(Debug)]
pub enum DisplayState {
    PreRun,
    Waiting,
    Running,
    PostRun,
}

impl Display for DisplayState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let my_type = match self {
            DisplayState::PreRun => "PreRun",
            DisplayState::Waiting => "Waiting",
            DisplayState::Running => "Running",
            DisplayState::PostRun => "PostRun",
        };

        write!(f, "DisplayState({})", my_type)
    }
}

// Struct used to represent the global state of the application. This has to
// "implement" the Send + Sync marker traits since it will be crossing thread
// boundaries into the REST API.
//
// Different states can be added to the struct and you can twiddle how they will
// be represented to the REST API here.
//
// Currently things are kept very simple and when you `HTTP GET` against `localhost`
// it will return a string of what state the `chef-client` process is in.
pub struct State {
    process: RwLock<crate::process::State>,
    splay_countdown: RwLock<u64>,
}

impl State {
    // This will mutate global state! As such it is behind a R/W mutex.
    pub fn update_process_state(&self, ps: &mut crate::process::State) -> Result<(), Error> {
        let mut old_state = self.process.write().unwrap();
        let old_state_ref = old_state.deref_mut();

        std::mem::swap(old_state_ref, ps);
        std::mem::drop(old_state);

        Ok(())
    }

    pub fn update_splay_countdown(&self, mut v: u64) {
        let mut s = self.splay_countdown.write().unwrap();
        // Don't think we need to call drop explicitly for primitives?
        std::mem::swap(&mut v, &mut s);
    }

    // This will read global state! As such it is behind a R/W mutex.
    pub fn peek(&self) -> DisplayState {
        let guard = self.process.read().unwrap();

        match *guard {
            crate::process::State::PreRun(_) => DisplayState::PreRun,
            crate::process::State::Waiting(..) => DisplayState::Waiting,
            crate::process::State::Running(_) => DisplayState::Running,
            crate::process::State::PostRun(_) => DisplayState::PostRun,
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let guard = self.process.read().unwrap();
        let countdown = self.splay_countdown.read().unwrap();

        write!(
            f,
            "Chef Process State: {}, Splay Countdown: {}",
            guard, countdown
        )
    }
}

impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let process = self.process.read().unwrap();
        let countdown = self.splay_countdown.read().unwrap();

        write!(f, "ApplicationState({}, {})", process, countdown)
    }
}

// Bad idea, young padawan. But fortunately I have some books to read on how to
// do this safely!
unsafe impl Send for State {}
unsafe impl Sync for State {}

lazy_static! {
    pub static ref APP_STATE: State = State {
        process: RwLock::new(crate::process::State::PreRun(
            crate::process::ChefProcess::new("echo"),
        )),
        splay_countdown: RwLock::new(0 as u64),
    };
}
