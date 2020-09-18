use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions;
use crate::quickjs_utils::objects::is_instance_of_by_name;
use crate::quickjsruntime::{OwnedValueRef, QuickJsRuntime};
use libquickjs_sys as q;

#[allow(dead_code)]
pub fn is_promise(q_js_rt: &QuickJsRuntime, obj_ref: &OwnedValueRef) -> Result<bool, EsError> {
    is_instance_of_by_name(q_js_rt, obj_ref, "Promise")
}

pub struct PromiseRef {
    promise_obj_ref: OwnedValueRef,
    reject_function_obj_ref: OwnedValueRef,
    resolve_function_obj_ref: OwnedValueRef,
}
#[allow(dead_code)]
impl PromiseRef {
    fn get_promise_obj_ref(&self) -> OwnedValueRef {
        OwnedValueRef::new_no_free(*self.promise_obj_ref.borrow_value())
    }

    fn resolve(&self, q_js_rt: &QuickJsRuntime, value: OwnedValueRef) -> Result<(), EsError> {
        crate::quickjs_utils::functions::call_function(
            q_js_rt,
            &self.resolve_function_obj_ref,
            &vec![value],
        )?;
        Ok(())
    }
    fn reject(&self, q_js_rt: &QuickJsRuntime, value: OwnedValueRef) -> Result<(), EsError> {
        crate::quickjs_utils::functions::call_function(
            q_js_rt,
            &self.reject_function_obj_ref,
            &vec![value],
        )?;
        Ok(())
    }
}

#[allow(dead_code)]
pub fn new_promise(q_js_rt: &QuickJsRuntime) -> Result<PromiseRef, EsError> {
    let mut promise_resolution_functions = [quickjs_utils::new_null(), quickjs_utils::new_null()];

    let prom_val = unsafe {
        q::JS_NewPromiseCapability(q_js_rt.context, promise_resolution_functions.as_mut_ptr())
    };

    let resolve_func_val = *promise_resolution_functions.get(0).unwrap();
    let reject_func_val = *promise_resolution_functions.get(1).unwrap();

    let resolve_function_obj_ref = OwnedValueRef::new(resolve_func_val);
    let reject_function_obj_ref = OwnedValueRef::new(reject_func_val);
    assert!(functions::is_function(q_js_rt, &resolve_function_obj_ref));
    assert!(functions::is_function(q_js_rt, &reject_function_obj_ref));

    let promise_obj_ref = OwnedValueRef::new(prom_val);

    Ok(PromiseRef {
        promise_obj_ref,
        reject_function_obj_ref,
        resolve_function_obj_ref,
    })
}

#[allow(dead_code)]
pub fn add_promise_reactions(
    _q_js_rt: &QuickJsRuntime,
    _promise_obj_ref: &OwnedValueRef,
    _then_func_obj_ref: Option<OwnedValueRef>,
    _catch_func_obj_ref: Option<OwnedValueRef>,
    _finally_func_obj_ref: Option<OwnedValueRef>,
) -> Result<(), EsError> {
    // todo, before getting into this i want to get callbacks working decently
    unimplemented!()
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::promises::{is_promise, new_promise};
    use crate::quickjs_utils::{functions, primitives};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_instance_of_prom() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let res = q_js_rt.eval(EsScript::new(
                "test_instance_of_prom.es",
                "(new Promise((res, rej) => {}));",
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
    fn new_prom() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let func_ref = q_js_rt
                .eval(EsScript::new(
                    "new_prom.es",
                    "(function(p){p.then((res) => {console.log('prom resolved to ' + res);});});",
                ))
                .ok()
                .unwrap();

            let prom = new_promise(q_js_rt).ok().unwrap();

            functions::call_function(q_js_rt, &func_ref, &vec![prom.get_promise_obj_ref()])
                .ok()
                .unwrap();

            prom.resolve(q_js_rt, primitives::from_i32(743))
                .ok()
                .expect("resolve failed");
        });
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn new_prom2() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let func_ref = q_js_rt
                .eval(EsScript::new(
                    "new_prom.es",
                    "(function(p){p.catch((res) => {console.log('prom rejected to ' + res);});});",
                ))
                .ok()
                .unwrap();

            let prom = new_promise(q_js_rt).ok().unwrap();

            functions::call_function(q_js_rt, &func_ref, &vec![prom.get_promise_obj_ref()])
                .ok()
                .unwrap();

            prom.reject(q_js_rt, primitives::from_i32(130))
                .ok()
                .expect("reject failed");
        });
        std::thread::sleep(Duration::from_secs(1));
    }
}
