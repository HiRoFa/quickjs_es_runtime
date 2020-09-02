use crate::eserror::EsError;
use crate::quickjs_utils::objects::is_instance_of_by_name;
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};

#[allow(dead_code)]
pub fn is_promise(q_js_rt: &QuickJsRuntime, obj_ref: &OwnedValueRef) -> Result<bool, EsError> {
    is_instance_of_by_name(q_js_rt, obj_ref, "Promise")
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::promises::is_promise;
    use std::sync::Arc;

    #[test]
    fn test_instance_of_prom() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let res = q_js_rt.eval(EsScript::new(
                "".to_string(),
                "(new Promise((res, rej) => {}));".to_string(),
            ));
            match res {
                Ok(v) => is_promise(q_js_rt, &v)
                    .ok()
                    .expect("could not get instanceof"),
                Err(e) => {
                    panic!("err: {}", e);
                }
            }
        });
        assert!(io);
    }
}
