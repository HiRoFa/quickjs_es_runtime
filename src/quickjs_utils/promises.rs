use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions;
use crate::quickjs_utils::objects::is_instance_of_by_name;
use crate::quickjscontext::QuickJsContext;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

pub fn is_promise_q(context: &QuickJsContext, obj_ref: &JSValueRef) -> bool {
    unsafe { is_promise(context.context, obj_ref) }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_promise(context: *mut q::JSContext, obj_ref: &JSValueRef) -> bool {
    is_instance_of_by_name(context, obj_ref, "Promise")
        .ok()
        .expect("could not check instance_of")
}

pub struct PromiseRef {
    promise_obj_ref: JSValueRef,
    reject_function_obj_ref: JSValueRef,
    resolve_function_obj_ref: JSValueRef,
}
#[allow(dead_code)]
impl PromiseRef {
    pub fn get_promise_obj_ref(&self) -> JSValueRef {
        self.promise_obj_ref.clone()
    }

    pub fn resolve_q(&self, q_ctx: &QuickJsContext, value: JSValueRef) -> Result<(), EsError> {
        unsafe { self.resolve(q_ctx.context, value) }
    }
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn resolve(
        &self,
        context: *mut q::JSContext,
        value: JSValueRef,
    ) -> Result<(), EsError> {
        log::trace!("PromiseRef.resolve()");
        crate::quickjs_utils::functions::call_function(
            context,
            &self.resolve_function_obj_ref,
            vec![value],
            None,
        )?;

        QuickJsRuntime::do_with(|q_js_rt| {
            q_js_rt.run_pending_jobs_if_any();
            Ok(())
        })
    }
    pub fn reject_q(&self, q_ctx: &QuickJsContext, value: JSValueRef) -> Result<(), EsError> {
        unsafe { self.reject(q_ctx.context, value) }
    }
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn reject(
        &self,
        context: *mut q::JSContext,
        value: JSValueRef,
    ) -> Result<(), EsError> {
        log::trace!("PromiseRef.reject()");
        crate::quickjs_utils::functions::call_function(
            context,
            &self.reject_function_obj_ref,
            vec![value],
            None,
        )?;

        QuickJsRuntime::do_with(|q_js_rt| {
            q_js_rt.run_pending_jobs_if_any();
            Ok(())
        })
    }
}

pub fn new_promise_q(q_ctx: &QuickJsContext) -> Result<PromiseRef, EsError> {
    unsafe { new_promise(q_ctx.context) }
}

/// create a new Promise
/// you can use this to respond asynchronously to method calls from JavaScript by returning a Promise
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_promise(context: *mut q::JSContext) -> Result<PromiseRef, EsError> {
    log::trace!("promises::new_promise()");

    let mut promise_resolution_functions = [quickjs_utils::new_null(), quickjs_utils::new_null()];

    let prom_val = q::JS_NewPromiseCapability(context, promise_resolution_functions.as_mut_ptr());

    let resolve_func_val = *promise_resolution_functions.get(0).unwrap();
    let reject_func_val = *promise_resolution_functions.get(1).unwrap();

    let resolve_function_obj_ref = JSValueRef::new(
        context,
        resolve_func_val,
        false,
        true,
        "promises::new_promise resolve_func_val",
    );
    let reject_function_obj_ref = JSValueRef::new(
        context,
        reject_func_val,
        false,
        true,
        "promises::new_promise reject_func_val",
    );
    debug_assert!(functions::is_function(context, &resolve_function_obj_ref));
    debug_assert!(functions::is_function(context, &reject_function_obj_ref));

    let promise_obj_ref = JSValueRef::new(
        context,
        prom_val,
        false,
        true,
        "promises::new_promise prom_val",
    );

    debug_assert_eq!(resolve_function_obj_ref.get_ref_count(), 1);
    debug_assert_eq!(reject_function_obj_ref.get_ref_count(), 1);
    debug_assert_eq!(promise_obj_ref.get_ref_count(), 3);

    Ok(PromiseRef {
        promise_obj_ref,
        reject_function_obj_ref,
        resolve_function_obj_ref,
    })
}

pub(crate) fn init_promise_rejection_tracker(q_js_rt: &QuickJsRuntime) {
    let tracker: q::JSHostPromiseRejectionTracker = Some(promise_rejection_tracker);

    unsafe {
        q::JS_SetHostPromiseRejectionTracker(q_js_rt.runtime, tracker, std::ptr::null_mut());
    }
}

pub fn add_promise_reactions_q(
    context: &QuickJsContext,
    promise_obj_ref: &JSValueRef,
    then_func_obj_ref_opt: Option<JSValueRef>,
    catch_func_obj_ref_opt: Option<JSValueRef>,
    finally_func_obj_ref_opt: Option<JSValueRef>,
) -> Result<(), EsError> {
    unsafe {
        add_promise_reactions(
            context.context,
            promise_obj_ref,
            then_func_obj_ref_opt,
            catch_func_obj_ref_opt,
            finally_func_obj_ref_opt,
        )
    }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn add_promise_reactions(
    context: *mut q::JSContext,
    promise_obj_ref: &JSValueRef,
    then_func_obj_ref_opt: Option<JSValueRef>,
    catch_func_obj_ref_opt: Option<JSValueRef>,
    finally_func_obj_ref_opt: Option<JSValueRef>,
) -> Result<(), EsError> {
    debug_assert!(is_promise(context, promise_obj_ref));

    if let Some(then_func_obj_ref) = then_func_obj_ref_opt {
        functions::invoke_member_function(
            context,
            &promise_obj_ref,
            "then",
            vec![then_func_obj_ref],
        )?;
    }
    if let Some(catch_func_obj_ref) = catch_func_obj_ref_opt {
        functions::invoke_member_function(
            context,
            &promise_obj_ref,
            "catch",
            vec![catch_func_obj_ref],
        )?;
    }
    if let Some(finally_func_obj_ref) = finally_func_obj_ref_opt {
        functions::invoke_member_function(
            context,
            &promise_obj_ref,
            "finally",
            vec![finally_func_obj_ref],
        )?;
    }

    Ok(())
}

unsafe extern "C" fn promise_rejection_tracker(
    ctx: *mut q::JSContext,
    _promise: q::JSValue,
    reason: q::JSValue,
    is_handled: ::std::os::raw::c_int,
    _opaque: *mut ::std::os::raw::c_void,
) {
    if is_handled == 0 {
        log::error!("unhandled promise rejection detected");

        let reason_ref = JSValueRef::new(
            ctx,
            reason,
            false,
            false,
            "promises::promise_rejection_tracker reason",
        );
        let reason_str_res = functions::call_to_string(ctx, &reason_ref);
        match reason_str_res {
            Ok(reason_str) => {
                log::error!("reason: {}", reason_str);
            }
            Err(e) => {
                log::error!("could not get reason: {}", e);
            }
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::promises::{add_promise_reactions_q, is_promise_q, new_promise_q};
    use crate::quickjs_utils::{functions, new_null_ref, primitives};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_instance_of_prom() {
        log::info!("> test_instance_of_prom");

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval(EsScript::new(
                "test_instance_of_prom.es",
                "(new Promise((res, rej) => {}));",
            ));
            match res {
                Ok(v) => {
                    log::info!("checking if instance_of prom");

                    is_promise_q(q_ctx, &v)
                        && is_promise_q(q_ctx, &v)
                        && is_promise_q(q_ctx, &v)
                        && is_promise_q(q_ctx, &v)
                        && is_promise_q(q_ctx, &v)
                }
                Err(e) => {
                    log::error!("err testing instance_of prom: {}", e);
                    false
                }
            }
        });
        assert!(io);

        log::info!("< test_instance_of_prom");
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn new_prom() {
        log::info!("> new_prom");

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let func_ref = q_ctx
                .eval(EsScript::new(
                    "new_prom.es",
                    "(function(p){p.then((res) => {console.log('prom resolved to ' + res);});});",
                ))
                .ok()
                .unwrap();

            let prom = new_promise_q(q_ctx).ok().unwrap();

            let res = functions::call_function_q(
                q_ctx,
                &func_ref,
                vec![prom.get_promise_obj_ref()],
                None,
            );
            if res.is_err() {
                panic!("func call failed: {}", res.err().unwrap());
            }

            unsafe {
                prom.resolve(q_ctx.context, primitives::from_i32(743))
                    .ok()
                    .expect("resolve failed");
            }
        });
        std::thread::sleep(Duration::from_secs(1));

        log::info!("< new_prom");
    }

    #[test]
    fn new_prom2() {
        log::info!("> new_prom2");

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let func_ref = q_ctx
                .eval(EsScript::new(
                    "new_prom.es",
                    "(function(p){p.catch((res) => {console.log('prom rejected to ' + res);});});",
                ))
                .ok()
                .unwrap();

            let prom = new_promise_q(q_ctx).ok().unwrap();

            let res = functions::call_function_q(
                q_ctx,
                &func_ref,
                vec![prom.get_promise_obj_ref()],
                None,
            );
            if res.is_err() {
                panic!("func call failed: {}", res.err().unwrap());
            }

            unsafe {
                prom.reject(q_ctx.context, primitives::from_i32(130))
                    .ok()
                    .expect("reject failed");
            }
        });
        std::thread::sleep(Duration::from_secs(1));

        log::info!("< new_prom2");
    }

    #[test]
    fn test_promise_reactions() {
        log::info!("> test_promise_reactions");

        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let prom_ref = q_ctx
                .eval(EsScript::new(
                    "test_promise_reactions.es",
                    "(new Promise(function(resolve, reject) {resolve(364);}));",
                ))
                .ok()
                .expect("script failed");

            let then_cb = functions::new_function_q(
                q_ctx,
                "testThen",
                |_q_ctx, _this, args| {
                    let res = primitives::to_i32(args.get(0).unwrap()).ok().unwrap();
                    log::trace!("prom resolved with: {}", res);
                    Ok(new_null_ref())
                },
                1,
            )
            .ok()
            .expect("could not create cb");
            let finally_cb = functions::new_function_q(
                q_ctx,
                "testThen",
                |_q_ctx, _this, _args| {
                    log::trace!("prom finalized");

                    Ok(new_null_ref())
                },
                1,
            )
            .ok()
            .expect("could not create cb");

            add_promise_reactions_q(q_ctx, &prom_ref, Some(then_cb), None, Some(finally_cb))
                .ok()
                .expect("could not add promise reactions");
        });
        std::thread::sleep(Duration::from_secs(1));

        log::info!("< test_promise_reactions");
    }
}
