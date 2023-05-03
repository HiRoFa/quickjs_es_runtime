//! contains engine features like console, setTimeout, setInterval and setImmediate

use crate::facades::QuickJsRuntimeFacade;
use crate::jsutils::JsError;
#[cfg(feature = "console")]
pub mod console;
#[cfg(any(feature = "settimeout", feature = "setinterval"))]
pub mod set_timeout;
#[cfg(feature = "setimmediate")]
pub mod setimmediate;

#[cfg(any(
    feature = "settimeout",
    feature = "setinterval",
    feature = "console",
    feature = "setimmediate"
))]
pub fn init(es_rt: &QuickJsRuntimeFacade) -> Result<(), JsError> {
    log::trace!("features::init");

    es_rt.exe_rt_task_in_event_loop(move |q_js_rt| {
        #[cfg(feature = "console")]
        console::init(q_js_rt)?;
        #[cfg(feature = "setimmediate")]
        setimmediate::init(q_js_rt)?;

        #[cfg(any(feature = "settimeout", feature = "setinterval"))]
        set_timeout::init(q_js_rt)?;
        Ok(())
    })
}
