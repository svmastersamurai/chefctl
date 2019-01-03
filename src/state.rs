use std::fmt::Display;
use std::ops::DerefMut;
use std::sync::RwLock;

#[derive(Debug)]
pub struct State {
    process: RwLock<crate::process::State>,
}

pub enum Error {
    CannotUpdateProcess,
}

#[derive(Debug)]
pub enum DisplayState {
    PreRun,
    Running,
    PostRun,
}

impl State {
    pub fn update_process_state(&self, ps: &mut crate::process::State) -> Result<(), Error> {
        let mut old_state = self.process.write().unwrap();
        let old_state_ref = old_state.deref_mut();

        std::mem::swap(old_state_ref, ps);
        std::mem::drop(old_state);

        Ok(())
    }

    pub fn peek(&self) -> DisplayState {
        let guard = self.process.read().unwrap();

        match *guard {
            crate::process::State::PreRun(_) => DisplayState::PreRun,
            crate::process::State::Running(_) => DisplayState::Running,
            crate::process::State::PostRun(_) => DisplayState::PostRun,
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let guard = self.process.read().unwrap();

        write!(f, "ApplicationState({})", guard)
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
    };
}
