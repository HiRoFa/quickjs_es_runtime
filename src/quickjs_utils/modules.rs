#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use std::sync::Arc;

    #[test]
    fn test_module_sandbox() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let _io = rt.add_to_event_queue_sync(|q_js_rt| {
            let m = q_js_rt
                .eval_module(EsScript::new(
                    "test1.mes",
                    "export const name = 'foobar';\nconsole.log('evalling module'); this;",
                ))
                .ok()
                .expect("parse mod failed");

            log::trace!("tag={}", m.borrow_value().tag);
        });
    }
}
