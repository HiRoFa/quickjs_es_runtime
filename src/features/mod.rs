use crate::eserror::EsError;
use crate::esruntime::EsRuntime;

pub mod console;
pub mod fetch;
pub mod set_timeout;
pub mod setimmediate;

pub fn init(es_rt: &EsRuntime) -> Result<(), EsError> {
    log::trace!("features::init");

    fetch::init(es_rt)?;

    es_rt.exe_rt_task_in_event_loop(move |q_js_rt| {
        console::init(q_js_rt)?;
        setimmediate::init(q_js_rt)?;
        set_timeout::init(q_js_rt)?;
        Ok(())
    })
}
