// Windows specific file paths.
#[cfg(target_os = "windows")]
pub const CHEF_PATH: &str = "C:\\opscode\\chef\\bin\\chef-client.bat";
#[cfg(target_os = "windows")]
pub const CHEF_VERSION_MANIFEST: &str = "C:\\opscode\\chef\\version-manifest.json";
#[cfg(target_os = "windows")]
pub const CONFIG_FILE_PATH: &str = "C:\\chef\\chefctl.yml";
#[cfg(target_os = "windows")]
pub const LOCK_FILE_PATH: &str = "C:\\chef\\chefctl.lock";
#[cfg(target_os = "windows")]
pub const CHEF_RUN_CURRENT_PATH: &str = "C:\\chef\\outputs\\chef.cur.out";
#[cfg(target_os = "windows")]
pub const CHEF_RUN_LAST_PATH: &str = "C:\\chef\\outputs\\chef.last.out";
#[cfg(target_os = "windows")]
pub const FD_NULL: &str = "NUL";

// Non-Windows file paths.
#[cfg(not(target_os = "windows"))]
pub const CHEF_PATH: &str = "/opt/chef/embedded/bin/chef-client";
#[cfg(not(target_os = "windows"))]
pub const CHEF_VERSION_MANIFEST: &str = "/opt/chef/version-manifest.json";
#[cfg(not(target_os = "windows"))]
pub const CONFIG_FILE_PATH: &str = "/etc/chefctl.yml";
#[cfg(not(target_os = "windows"))]
pub const LOCK_FILE_PATH: &str = "/var/lock/subsys/chefctl";
#[cfg(not(target_os = "windows"))]
pub const CHEF_RUN_CURRENT_PATH: &str = "/tmp/chef.cur.out";
#[cfg(not(target_os = "windows"))]
pub const CHEF_RUN_LAST_PATH: &str = "/tmp/chef.last.out";
#[cfg(not(target_os = "windows"))]
pub const FD_NULL: &str = "/dev/null";
// #[cfg(not(target_os = "windows"))]
// pub const CHEF_RUN_LAST_PATH: &str = "/var/chef/outputs/chef.last.out";
