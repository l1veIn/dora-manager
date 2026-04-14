#[cfg(all(test, not(target_os = "windows")))]
use std::ffi::OsString;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard, OnceLock};

#[cfg(test)]
pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|err| err.into_inner())
}

#[cfg(all(test, not(target_os = "windows")))]
pub(crate) struct PathGuard(Option<OsString>);

#[cfg(all(test, not(target_os = "windows")))]
impl Drop for PathGuard {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            std::env::set_var("PATH", path);
        } else {
            std::env::remove_var("PATH");
        }
    }
}

#[cfg(all(test, not(target_os = "windows")))]
pub(crate) fn set_path(value: impl Into<OsString>) -> PathGuard {
    let original = std::env::var_os("PATH");
    std::env::set_var("PATH", value.into());
    PathGuard(original)
}

#[cfg(all(test, not(target_os = "windows")))]
pub(crate) fn clear_path() -> PathGuard {
    let original = std::env::var_os("PATH");
    std::env::set_var("PATH", "");
    PathGuard(original)
}
