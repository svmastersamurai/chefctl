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
#[derive(Serialize, Deserialize)]
pub struct State {
    process_state: RwLock<String>,
    splay_countdown: RwLock<u64>,
}

impl State {
    pub fn update_process_state(&self, ps: String) {
        let mut val = self.process_state.write().unwrap();

        *val = ps;
    }

    pub fn update_splay_countdown(&self, v: u64) {
        let mut val = self.splay_countdown.write().unwrap();

        *val = v;
    }
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

lazy_static! {
    pub static ref APP_STATE: State = State {
        process_state: RwLock::new(String::from("init")),
        splay_countdown: RwLock::new(0 as u64),
    };
}
