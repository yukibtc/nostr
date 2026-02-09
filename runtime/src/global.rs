use std::sync::{Arc, OnceLock};

use crate::runtime::NostrRuntime;

static RUNTIME: OnceLock<Arc<dyn NostrRuntime>> = OnceLock::new();

/// Install a runtime
///
/// Returns `true` if the runtime has been successfully installed, `false` if a runtime was already installed.
#[inline]
pub fn install_runtime<T>(runtime: Arc<T>) -> bool
where
    T: NostrRuntime,
{
    RUNTIME.set(runtime).is_ok()
}

/// Try to get the installed runtime.
///
/// Returns `None` if no runtime has been installed.
#[inline]
pub fn runtime() -> Option<&'static Arc<dyn NostrRuntime>> {
    RUNTIME.get()
}
