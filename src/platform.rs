// Windows specific file paths.
#[cfg(target_os = "windows")]
pub const CHEF_PATH: &str = "C:\\opscode\\chef\\bin\\chef-client.cmd";
#[cfg(target_os = "windows")]
pub const CONFIG_FILE_PATH: &str = "C:\\chef\\chefctl.yml";
#[cfg(target_os = "windows")]
pub const LOCK_FILE_PATH: &str = "C:\\chef\\chefctl.lock";

// Non-Windows file paths.
#[cfg(not(target_os = "windows"))]
pub const CHEF_PATH: &str = "/opt/chef/embedded/bin/chef-client";
#[cfg(not(target_os = "windows"))]
pub const CONFIG_FILE_PATH: &str = "/etc/chefctl.yml";
#[cfg(not(target_os = "windows"))]
pub const LOCK_FILE_PATH: &str = "/var/lock/subsys/chefctl";
