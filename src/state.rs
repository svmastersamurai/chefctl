use std::fmt::Display;
use std::io::Error;
use std::ops::DerefMut;
use std::sync::RwLock;

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
    process_state: RwLock<String>,
    splay_countdown: RwLock<u64>,
}

impl State {
    // This will mutate global state! As such it is behind a R/W mutex.
    pub fn update_process_state(&self, ps: &mut String) -> Result<(), Error> {
        let mut old_state = self.process_state.write().unwrap();
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
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let guard = self.process_state.read().unwrap();
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
        let process = self.process_state.read().unwrap();
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
        process_state: RwLock::new(String::from("init")),
        splay_countdown: RwLock::new(0 as u64),
    };
}
