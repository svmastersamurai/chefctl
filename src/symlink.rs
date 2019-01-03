extern crate chrono;

use std::path::PathBuf;

#[cfg(target_os = "windows")]
use std::os::windows::fs::symlink_file as std_symlink;

#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::symlink as std_symlink;

pub fn create_symlink<T>(link: &T, target: &T) -> std::io::Result<()>
where
    T: ToString + AsRef<std::ffi::OsStr> + AsRef<std::path::Path> + std::fmt::Debug + Sized,
{
    ensure_path(link);
    ensure_path(target);
    println!("create symlink {:?} -> {:?}", link, target);
    ensure_symlink(link.to_string())?;
    ensure_symlink(target.to_string())?;

    match std_symlink(target, link) {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    }
}

// Validates that the directory structure needed for the file about to be written
// exists.
fn ensure_path<P>(p: P)
where
    PathBuf: From<P>,
{
    let path = PathBuf::from(p);

    if !path.parent().unwrap().exists() {
        match std::fs::create_dir_all(path.parent().unwrap()) {
            Ok(_) => {}
            Err(e) => panic!("could not create_dir_all: {}", e),
        }
    }
}

fn ensure_symlink(p: String) -> std::io::Result<()> {
    let path = PathBuf::from(p);

    if path.exists() {
        // Only remove symlinks.
        if let Ok(m) = path.symlink_metadata() {
            if m.file_type().is_symlink() {
                std::fs::remove_file(&path)?
            }
        }
    }

    Ok(())
}
