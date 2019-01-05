use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::{io::Read, sync::RwLock};

pub struct CheckError;

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
    checks: RwLock<std::collections::HashMap<T, T>>,
}

impl<T> State<T>
where
    T: Serialize + Eq + std::hash::Hash,
{
    pub fn update_checks(&self, val: std::collections::HashMap<T, T>) {
        let mut current_val = self.checks.write().unwrap();

        *current_val = val;
    }
}

unsafe impl<T> Send for State<T> where T: Serialize + Eq + std::hash::Hash {}
unsafe impl<T> Sync for State<T> where T: Serialize + Eq + std::hash::Hash {}

lazy_static! {
    pub static ref HEALTH_STATE: State<String> = {
        State {
            checks: RwLock::new(std::collections::HashMap::new()),
        }
    };
}

struct VersionCheck;

impl HealthCheck for VersionCheck {
    fn run() -> CheckResult<(String, String)> {
        let mut manifest = std::fs::File::open("/opt/chef/version-manifest.json")?;
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
        let result = match status.success() {
            true => "true",
            false => "false",
        };

        Ok((
            "Chef Client Executes Normally Check".to_string(),
            result.to_string(),
        ))
    }
}

pub fn update_health_checks() -> CheckResult<()> {
    let mut results: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let result = VersionCheck::run()?;
    results.insert(result.0, result.1);

    let result = ChefClientCheck::run()?;
    results.insert(result.0, result.1);

    HEALTH_STATE.update_checks(results);

    Ok(())
}
