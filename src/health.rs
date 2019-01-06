use crate::platform::CHEF_VERSION_MANIFEST;
use serde::ser::Serialize;
use std::{collections::HashMap, error::Error, io::Read, sync::RwLock};

#[derive(Debug)]
pub struct CheckError;

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "check error")
    }
}

impl std::error::Error for CheckError {
    fn cause(&self) -> Option<&std::error::Error> {
        None
    }

    fn description(&self) -> &str {
        "CheckError"
    }
}

type CheckResult<T> = std::result::Result<T, CheckError>;

trait HealthCheck {
    fn run() -> CheckResult<(String, String)>;
}

impl From<std::io::Error> for CheckError {
    fn from(_: std::io::Error) -> CheckError {
        CheckError {}
    }
}

impl From<serde_json::Error> for CheckError {
    fn from(_: serde_json::Error) -> CheckError {
        CheckError {}
    }
}

// This state represents the local health of the chef-client.
// Health is currently determined if the process can launch and return a successful
// exit code.
#[derive(Serialize)]
pub struct State<T>
where
    T: Serialize + Eq + std::hash::Hash,
{
    checks: RwLock<HashMap<T, T>>,
}

impl<T> State<T>
where
    T: Serialize + Eq + std::hash::Hash,
{
    pub fn update_checks(&self, val: HashMap<T, T>) {
        let mut current_val = self.checks.write().unwrap();

        *current_val = val;
    }
}

unsafe impl<T> Send for State<T> where T: Serialize + Eq + std::hash::Hash {}
unsafe impl<T> Sync for State<T> where T: Serialize + Eq + std::hash::Hash {}

lazy_static! {
    pub static ref HEALTH_STATE: State<String> = {
        State {
            checks: RwLock::new(HashMap::new()),
        }
    };
}

struct VersionCheck;

impl HealthCheck for VersionCheck {
    fn run() -> CheckResult<(String, String)> {
        let mut manifest = std::fs::File::open(CHEF_VERSION_MANIFEST)?;
        let mut content = String::new();
        let _ = manifest.read_to_string(&mut content);
        let manifest: serde_json::Value = serde_json::from_str(&content)?;
        let version = manifest.get("build_version").unwrap().as_str().unwrap();

        Ok(("Chef Client Version Check".to_string(), version.to_string()))
    }
}

struct ChefClientCheck;

impl HealthCheck for ChefClientCheck {
    fn run() -> CheckResult<(String, String)> {
        let mut chef_version = std::process::Command::new(crate::platform::CHEF_PATH);
        chef_version.arg("--version");
        let status = chef_version.status()?;
        let result = if status.success() { "true" } else { "false" };

        Ok((
            "Chef Client Executes Normally Check".to_string(),
            result.to_string(),
        ))
    }
}

pub fn update_health_checks() -> CheckResult<()> {
    let mut results: HashMap<String, String> = HashMap::new();
    let result = match VersionCheck::run() {
        Ok(v) => v,
        Err(e) => ("VersionCheck".to_string(), e.description().to_owned()),
    };
    results.insert(result.0, result.1);

    let result = match ChefClientCheck::run() {
        Ok(v) => v,
        Err(e) => ("ChefClientCheck".to_string(), e.description().to_owned()),
    };
    results.insert(result.0, result.1);

    HEALTH_STATE.update_checks(results);

    Ok(())
}
