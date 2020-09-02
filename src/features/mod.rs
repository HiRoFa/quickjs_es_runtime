use crate::eserror::EsError;
use crate::quickjsruntime::QuickJsRuntime;

mod console;

pub fn init(q_js_rt: &QuickJsRuntime) -> Result<(), EsError> {
    log::trace!("features::init");

    console::init(q_js_rt)?;

    Ok(())
}
