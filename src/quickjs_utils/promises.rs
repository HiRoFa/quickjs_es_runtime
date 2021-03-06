use crate::quickjs_utils;
use crate::quickjs_utils::functions;
use crate::quickjs_utils::objects::is_instance_of_by_name;
use crate::quickjscontext::QuickJsContext;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use hirofa_utils::js_utils::adapters::{JsPromiseAdapter, JsRealmAdapter};
use hirofa_utils::js_utils::JsError;
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

    pub fn resolve_q(&self, q_ctx: &QuickJsContext, value: JSValueRef) -> Result<(), JsError> {
        unsafe { self.resolve(q_ctx.context, value) }
    }
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn resolve(
        &self,
        context: *mut q::JSContext,
        value: JSValueRef,
    ) -> Result<(), JsError> {
        log::trace!("PromiseRef.resolve()");
        crate::quickjs_utils::functions::call_function(
            context,
            &self.resolve_function_obj_ref,
            vec![value],
            None,
        )?;
        Ok(())
    }
    pub fn reject_q(&self, q_ctx: &QuickJsContext, value: JSValueRef) -> Result<(), JsError> {
        unsafe { self.reject(q_ctx.context, value) }
    }
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn reject(
        &self,
        context: *mut q::JSContext,
        value: JSValueRef,
    ) -> Result<(), JsError> {
        log::trace!("PromiseRef.reject()");
        crate::quickjs_utils::functions::call_function(
            context,
            &self.reject_function_obj_ref,
            vec![value],
            None,
        )?;
        Ok(())
    }
}

impl Clone for PromiseRef {
    fn clone(&self) -> Self {
        Self {
            promise_obj_ref: self.promise_obj_ref.clone(),
            reject_function_obj_ref: self.reject_function_obj_ref.clone(),
            resolve_function_obj_ref: self.resolve_function_obj_ref.clone(),
        }
    }
}

impl JsPromiseAdapter for PromiseRef {
    type JsRuntimeAdapterType = QuickJsRuntime;

    fn js_promise_resolve(
        &self,
        context: &QuickJsContext,
        resolution: &JSValueRef,
    ) -> Result<(), JsError> {
        self.resolve_q(context, resolution.clone())
    }

    fn js_promise_reject(
        &self,
        context: &QuickJsContext,
        rejection: &JSValueRef,
    ) -> Result<(), JsError> {
        self.reject_q(context, rejection.clone())
    }

    fn js_promise_add_reactions<F>(
        &self,
        context: &QuickJsContext,
        then: Option<F>,
        catch: Option<F>,
        finally: Option<F>,
    ) -> Result<(), JsError>
    where
        F: Fn(&JSValueRef) -> Result<(), JsError> + 'static,
    {
        let then = then.map(|t_clos| {
            context
                .js_function_create(
                    "then",
                    move |_ctx, _this, args| {
                        t_clos(&args[0])?;
                        Ok(crate::quickjs_utils::new_null_ref())
                    },
                    1,
                )
                .ok()
                .expect("could not create function")
        });
        let catch = catch.map(|t_clos| {
            context
                .js_function_create(
                    "catch",
                    move |_ctx, _this, args| {
                        t_clos(&args[0])?;
                        Ok(crate::quickjs_utils::new_null_ref())
                    },
                    1,
                )
                .ok()
                .expect("could not create function")
        });
        let finally = finally.map(|t_clos| {
            context
                .js_function_create(
                    "finally",
                    move |_ctx, _this, args| {
                        t_clos(&args[0])?;
                        Ok(crate::quickjs_utils::new_null_ref())
                    },
                    1,
                )
                .ok()
                .expect("could not create function")
        });

        crate::quickjs_utils::promises::add_promise_reactions_q(
            context,
            &self.promise_obj_ref,
            then,
            catch,
            finally,
        )
    }

    fn js_promise_get_value(&self) -> JSValueRef {
        self.promise_obj_ref.clone()
    }
}

pub fn new_promise_q(q_ctx: &QuickJsContext) -> Result<PromiseRef, JsError> {
    unsafe { new_promise(q_ctx.context) }
}

/// create a new Promise
/// you can use this to respond asynchronously to method calls from JavaScript by returning a Promise
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_promise(context: *mut q::JSContext) -> Result<PromiseRef, JsError> {
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
) -> Result<(), JsError> {
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
) -> Result<(), JsError> {
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
    use crate::esruntime::tests::init_test_rt;
    use crate::esvalue::EsValueFacade;
    use crate::quickjs_utils::promises::{add_promise_reactions_q, is_promise_q, new_promise_q};
    use crate::quickjs_utils::{functions, new_null_ref, primitives};
    use crate::quickjsruntime::QuickJsRuntime;
    use hirofa_utils::js_utils::Script;
    use std::time::Duration;

    #[test]
    fn test_instance_of_prom() {
        log::info!("> test_instance_of_prom");

        let rt = init_test_rt();
        let io = rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let res = q_ctx.eval(Script::new(
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

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let func_ref = q_ctx
                .eval(Script::new(
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

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let func_ref = q_ctx
                .eval(Script::new(
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

        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let prom_ref = q_ctx
                .eval(Script::new(
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

    #[test]
    fn test_promise_nested() {
        log::info!("> test_promise_nested");

        let rt = init_test_rt();

        let mut esvf_res = rt.exe_task(|| {
            QuickJsRuntime::create_context("test").ok().expect("create ctx failed");
            QuickJsRuntime::do_with(|q_js_rt| {
                let q_ctx = q_js_rt.get_context("test");

                let script = "(new Promise((resolve, reject) => {resolve({a: 7});}).then((obj) => {return {b: obj.a * 5}}));";
                let esvf_res = q_ctx
                    .eval(Script::new("test_promise_nested.es", script))
                    .ok()
                    .expect("script failed");

                EsValueFacade::from_jsval(q_ctx, &esvf_res).ok().expect("poof")

            })
        });
        while esvf_res.is_promise() {
            esvf_res = esvf_res.get_promise_result_sync().ok().expect("failure");
        }
        assert!(esvf_res.is_object());
        let obj = esvf_res.get_object().ok().expect("esvf to map failed");
        let b = obj.get("b").expect("got no b");
        assert!(b.is_i32());
        let i = b.get_i32();
        assert_eq!(i, 5 * 7);

        rt.exe_task(|| {
            QuickJsRuntime::remove_context("test");
        })
    }
}
