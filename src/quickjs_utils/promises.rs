use crate::eserror::EsError;
use crate::quickjs_utils::objects::is_instance_of_by_name;
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};

#[allow(dead_code)]
pub fn is_promise(q_js_rt: &QuickJsRuntime, obj_ref: &OwnedValueRef) -> Result<bool, EsError> {
    is_instance_of_by_name(q_js_rt, obj_ref, "Promise")
}

pub struct PromiseRef {
    promise_obj: OwnedValueRef,
    reject_function_obj: OwnedValueRef,
    resolve_function_obj: OwnedValueRef,
}

impl PromiseRef {
    fn get_promise_obj(&self) -> &OwnedValueRef {
        &self.promise_obj
    }
    fn resolve(&self, value: OwnedValueRef) -> Result<(), EsError> {
        unimplemented!()
    }
    fn reject(&self, value: OwnedValueRef) -> Result<(), EsError> {
        unimplemented!()
    }
}

pub fn new_promise(q_js_rt: &QuickJsRuntime) -> Result<PromiseRefs, EsError> {}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils;
    use crate::quickjs_utils::promises::is_promise;
    use crate::quickjs_utils::{functions, primitives};
    use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime, TAG_OBJECT};
    use libquickjs_sys as q;
    use log::trace;
    use std::sync::Arc;
    use std::time::Duration;

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

    #[test]
    fn prom_sandbox() {
        // pub fn JS_NewPromiseCapability(ctx: *mut JSContext, resolving_funcs: *mut JSValue) -> JSValue;
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {

            let mut promise_resolution_functions = [
                quickjs_utils::new_null(),
                quickjs_utils::new_null()
            ];

            let prom_val = unsafe {
                q::JS_NewPromiseCapability(q_js_rt.context, promise_resolution_functions.as_mut_ptr())
            };

            let resolve_func_val = *promise_resolution_functions.get(0).unwrap();
            let reject_func_val = *promise_resolution_functions.get(1).unwrap();

            let resolve_func_ref = OwnedValueRef::new(resolve_func_val);
            let reject_func_ref = OwnedValueRef::new(reject_func_val);
            trace!("resolve_func_val.is_func = {}", functions::is_function(q_js_rt, &resolve_func_ref));
            trace!("reject_func_val.is_func = {}", functions::is_function(q_js_rt, &reject_func_ref));

            let prom_ref = OwnedValueRef::new(prom_val);

            let func_ref2= q_js_rt
                .eval(EsScript::new(
                    "prom_sandbox.es".to_string(),
                    "(function(p){console.log('adding then');p.then((r) => {console.log('thenned with ' + r);}).catch((er) => {console.log('cought ' + er);});console.log('after then added');});".to_string(),
                ))
                .ok()
                .unwrap();

            crate::quickjs_utils::functions::call_function(q_js_rt, &func_ref2, &vec![prom_ref]).ok().expect("calling func2 failed");

            // resolve
            //crate::quickjs_utils::functions::call_function(q_js_rt, &resolve_func_ref, &vec![primitives::from_i32(9864)]).ok().expect("calling func failed");
            crate::quickjs_utils::functions::call_function(q_js_rt, &reject_func_ref, &vec![primitives::from_i32(345)]).ok().expect("calling func failed");

            while q_js_rt.has_pending_jobs() {
                trace!("running pending job in sandbox");
                q_js_rt.run_pending_job();
            }

            std::thread::sleep(Duration::from_secs(1));
            trace!("done");
        });
    }
}
