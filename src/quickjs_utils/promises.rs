use crate::jsutils::JsError;
use crate::quickjs_utils;
#[cfg(feature = "bellard")]
use crate::quickjs_utils::class_ids::JS_CLASS_PROMISE;
use crate::quickjs_utils::errors::get_stack;
use crate::quickjs_utils::functions;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use libquickjs_sys as q;
#[cfg(feature = "bellard")]
use libquickjs_sys::JS_GetClassID;
#[cfg(feature = "quickjs-ng")]
use libquickjs_sys::JS_IsPromise;

pub fn is_promise_q(context: &QuickJsRealmAdapter, obj_ref: &QuickJsValueAdapter) -> bool {
    unsafe { is_promise(context.context, obj_ref) }
}

#[allow(dead_code)]
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
#[allow(unused_variables)]
pub unsafe fn is_promise(ctx: *mut q::JSContext, obj: &QuickJsValueAdapter) -> bool {
    #[cfg(feature = "bellard")]
    {
        JS_GetClassID(*obj.borrow_value()) == JS_CLASS_PROMISE
    }
    #[cfg(feature = "quickjs-ng")]
    {
        JS_IsPromise(*obj.borrow_value())
    }
}

pub struct QuickJsPromiseAdapter {
    promise_obj_ref: QuickJsValueAdapter,
    reject_function_obj_ref: QuickJsValueAdapter,
    resolve_function_obj_ref: QuickJsValueAdapter,
}
#[allow(dead_code)]
impl QuickJsPromiseAdapter {
    pub fn get_promise_obj_ref(&self) -> QuickJsValueAdapter {
        self.promise_obj_ref.clone()
    }

    pub fn resolve_q(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        value: QuickJsValueAdapter,
    ) -> Result<(), JsError> {
        unsafe { self.resolve(q_ctx.context, value) }
    }
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn resolve(
        &self,
        context: *mut q::JSContext,
        value: QuickJsValueAdapter,
    ) -> Result<(), JsError> {
        log::trace!("PromiseRef.resolve()");
        crate::quickjs_utils::functions::call_function(
            context,
            &self.resolve_function_obj_ref,
            &[value],
            None,
        )?;
        Ok(())
    }
    pub fn reject_q(
        &self,
        q_ctx: &QuickJsRealmAdapter,
        value: QuickJsValueAdapter,
    ) -> Result<(), JsError> {
        unsafe { self.reject(q_ctx.context, value) }
    }
    /// # Safety
    /// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
    pub unsafe fn reject(
        &self,
        context: *mut q::JSContext,
        value: QuickJsValueAdapter,
    ) -> Result<(), JsError> {
        log::trace!("PromiseRef.reject()");
        crate::quickjs_utils::functions::call_function(
            context,
            &self.reject_function_obj_ref,
            &[value],
            None,
        )?;
        Ok(())
    }
}

impl Clone for QuickJsPromiseAdapter {
    fn clone(&self) -> Self {
        Self {
            promise_obj_ref: self.promise_obj_ref.clone(),
            reject_function_obj_ref: self.reject_function_obj_ref.clone(),
            resolve_function_obj_ref: self.resolve_function_obj_ref.clone(),
        }
    }
}

impl QuickJsPromiseAdapter {
    pub fn js_promise_resolve(
        &self,
        context: &QuickJsRealmAdapter,
        resolution: &QuickJsValueAdapter,
    ) -> Result<(), JsError> {
        self.resolve_q(context, resolution.clone())
    }

    pub fn js_promise_reject(
        &self,
        context: &QuickJsRealmAdapter,
        rejection: &QuickJsValueAdapter,
    ) -> Result<(), JsError> {
        self.reject_q(context, rejection.clone())
    }

    pub fn js_promise_get_value(&self, _realm: &QuickJsRealmAdapter) -> QuickJsValueAdapter {
        self.promise_obj_ref.clone()
    }
}

pub fn new_promise_q(q_ctx: &QuickJsRealmAdapter) -> Result<QuickJsPromiseAdapter, JsError> {
    unsafe { new_promise(q_ctx.context) }
}

/// create a new Promise
/// you can use this to respond asynchronously to method calls from JavaScript by returning a Promise
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_promise(context: *mut q::JSContext) -> Result<QuickJsPromiseAdapter, JsError> {
    log::trace!("promises::new_promise()");

    let mut promise_resolution_functions = [quickjs_utils::new_null(), quickjs_utils::new_null()];

    let prom_val = q::JS_NewPromiseCapability(context, promise_resolution_functions.as_mut_ptr());

    let resolve_func_val = *promise_resolution_functions.first().unwrap();
    let reject_func_val = *promise_resolution_functions.get(1).unwrap();

    let resolve_function_obj_ref = QuickJsValueAdapter::new(
        context,
        resolve_func_val,
        false,
        true,
        "promises::new_promise resolve_func_val",
    );
    let reject_function_obj_ref = QuickJsValueAdapter::new(
        context,
        reject_func_val,
        false,
        true,
        "promises::new_promise reject_func_val",
    );
    debug_assert!(functions::is_function(context, &resolve_function_obj_ref));
    debug_assert!(functions::is_function(context, &reject_function_obj_ref));

    let promise_obj_ref = QuickJsValueAdapter::new(
        context,
        prom_val,
        false,
        true,
        "promises::new_promise prom_val",
    );

    #[cfg(feature = "bellard")]
    debug_assert_eq!(resolve_function_obj_ref.get_ref_count(), 1);
    #[cfg(feature = "bellard")]
    debug_assert_eq!(reject_function_obj_ref.get_ref_count(), 1);
    #[cfg(feature = "bellard")]
    debug_assert_eq!(promise_obj_ref.get_ref_count(), 3);

    Ok(QuickJsPromiseAdapter {
        promise_obj_ref,
        reject_function_obj_ref,
        resolve_function_obj_ref,
    })
}

pub(crate) fn init_promise_rejection_tracker(q_js_rt: &QuickJsRuntimeAdapter) {
    let tracker: q::JSHostPromiseRejectionTracker = Some(promise_rejection_tracker);

    unsafe {
        q::JS_SetHostPromiseRejectionTracker(q_js_rt.runtime, tracker, std::ptr::null_mut());
    }
}

pub fn add_promise_reactions_q(
    context: &QuickJsRealmAdapter,
    promise_obj_ref: &QuickJsValueAdapter,
    then_func_obj_ref_opt: Option<QuickJsValueAdapter>,
    catch_func_obj_ref_opt: Option<QuickJsValueAdapter>,
    finally_func_obj_ref_opt: Option<QuickJsValueAdapter>,
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
    promise_obj_ref: &QuickJsValueAdapter,
    then_func_obj_ref_opt: Option<QuickJsValueAdapter>,
    catch_func_obj_ref_opt: Option<QuickJsValueAdapter>,
    finally_func_obj_ref_opt: Option<QuickJsValueAdapter>,
) -> Result<(), JsError> {
    debug_assert!(is_promise(context, promise_obj_ref));

    if let Some(then_func_obj_ref) = then_func_obj_ref_opt {
        functions::invoke_member_function(context, promise_obj_ref, "then", &[then_func_obj_ref])?;
    }
    if let Some(catch_func_obj_ref) = catch_func_obj_ref_opt {
        functions::invoke_member_function(
            context,
            promise_obj_ref,
            "catch",
            &[catch_func_obj_ref],
        )?;
    }
    if let Some(finally_func_obj_ref) = finally_func_obj_ref_opt {
        functions::invoke_member_function(
            context,
            promise_obj_ref,
            "finally",
            &[finally_func_obj_ref],
        )?;
    }

    Ok(())
}

unsafe extern "C" fn promise_rejection_tracker(
    ctx: *mut q::JSContext,
    _promise: q::JSValue,
    reason: q::JSValue,
    #[cfg(feature = "bellard")] is_handled: ::std::os::raw::c_int,
    #[cfg(feature = "quickjs-ng")] is_handled: bool,

    _opaque: *mut ::std::os::raw::c_void,
) {
    #[cfg(feature = "bellard")]
    let handled = is_handled != 0;
    #[cfg(feature = "quickjs-ng")]
    let handled = is_handled;

    if !handled {
        let reason_ref = QuickJsValueAdapter::new(
            ctx,
            reason,
            false,
            false,
            "promises::promise_rejection_tracker reason",
        );
        let reason_str_res = functions::call_to_string(ctx, &reason_ref);
        QuickJsRuntimeAdapter::do_with(|rt| {
            let realm = rt.get_quickjs_context(ctx);
            let realm_id = realm.get_realm_id();
            let stack = match get_stack(realm) {
                Ok(s) => match s.to_string() {
                    Ok(s) => s,
                    Err(_) => "".to_string(),
                },
                Err(_) => "".to_string(),
            };
            match reason_str_res {
                Ok(reason_str) => {
                    log::error!(
                        "[{}] unhandled promise rejection, reason: {}{}",
                        realm_id,
                        reason_str,
                        stack
                    );
                }
                Err(e) => {
                    log::error!(
                        "[{}] unhandled promise rejection, could not get reason: {}{}",
                        realm_id,
                        e,
                        stack
                    );
                }
            }
        });
    }
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::Script;
    use crate::quickjs_utils::promises::{add_promise_reactions_q, is_promise_q, new_promise_q};
    use crate::quickjs_utils::{functions, new_null_ref, primitives};
    use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
    use crate::values::JsValueFacade;
    use futures::executor::block_on;
    use std::time::Duration;

    #[test]
    fn test_instance_of_prom() {
        log::info!("> test_instance_of_prom");

        let rt = init_test_rt();
        let io = rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_realm();
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
            let q_ctx = q_js_rt.get_main_realm();
            let func_ref = q_ctx
                .eval(Script::new(
                    "new_prom.es",
                    "(function(p){p.then((res) => {console.log('prom resolved to ' + res);});});",
                ))
                .ok()
                .unwrap();

            let prom = new_promise_q(q_ctx).ok().unwrap();

            let res =
                functions::call_function_q(q_ctx, &func_ref, &[prom.get_promise_obj_ref()], None);
            if res.is_err() {
                panic!("func call failed: {}", res.err().unwrap());
            }

            unsafe {
                prom.resolve(q_ctx.context, primitives::from_i32(743))
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
            let q_ctx = q_js_rt.get_main_realm();
            let func_ref = q_ctx
                .eval(Script::new(
                    "new_prom.es",
                    "(function(p){p.catch((res) => {console.log('prom rejected to ' + res);});});",
                ))
                .ok()
                .unwrap();

            let prom = new_promise_q(q_ctx).ok().unwrap();

            let res =
                functions::call_function_q(q_ctx, &func_ref, &[prom.get_promise_obj_ref()], None);
            if res.is_err() {
                panic!("func call failed: {}", res.err().unwrap());
            }

            unsafe {
                prom.reject(q_ctx.context, primitives::from_i32(130))
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
            let q_ctx = q_js_rt.get_main_realm();
            let prom_ref = q_ctx
                .eval(Script::new(
                    "test_promise_reactions.es",
                    "(new Promise(function(resolve, reject) {resolve(364);}));",
                ))
                .expect("script failed");

            let then_cb = functions::new_function_q(
                q_ctx,
                "testThen",
                |_q_ctx, _this, args| {
                    let res = primitives::to_i32(args.first().unwrap()).ok().unwrap();
                    log::trace!("prom resolved with: {}", res);
                    Ok(new_null_ref())
                },
                1,
            )
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
            .expect("could not create cb");

            add_promise_reactions_q(q_ctx, &prom_ref, Some(then_cb), None, Some(finally_cb))
                .expect("could not add promise reactions");
        });
        std::thread::sleep(Duration::from_secs(1));

        log::info!("< test_promise_reactions");
    }

    #[tokio::test]
    async fn test_promise_async() {
        let rt = init_test_rt();
        let jsvf = rt
            .eval(
                None,
                Script::new("test_prom_async.js", "Promise.resolve(123)"),
            )
            .await
            .expect("script failed");
        if let JsValueFacade::JsPromise { cached_promise } = jsvf {
            let res = cached_promise
                .get_promise_result()
                .await
                .expect("promise resolve send code stuf exploded");
            match res {
                Ok(prom_res) => {
                    if prom_res.is_i32() {
                        assert_eq!(prom_res.get_i32(), 123);
                    } else {
                        panic!("promise did not resolve to an i32.. well that was unexpected!");
                    }
                }
                Err(e) => {
                    panic!("prom was rejected: {}", e.stringify())
                }
            }
        }
    }
    #[test]
    fn test_promise_nested() {
        log::info!("> test_promise_nested");

        let rt = init_test_rt();

        let mut jsvf_res = rt.exe_task_in_event_loop(|| {
            QuickJsRuntimeAdapter::create_context("test").expect("create ctx failed");
            QuickJsRuntimeAdapter::do_with(|q_js_rt| {
                let q_ctx = q_js_rt.get_context("test");

                let script = "(new Promise((resolve, reject) => {resolve({a: 7});}).then((obj) => {return {b: obj.a * 5}}));";
                let esvf_res = q_ctx
                    .eval(Script::new("test_promise_nested.es", script))
                    .expect("script failed");

                q_ctx.to_js_value_facade(&esvf_res).expect("poof")

            })
        });

        while jsvf_res.is_js_promise() {
            match jsvf_res {
                JsValueFacade::JsPromise { cached_promise } => {
                    jsvf_res = cached_promise
                        .get_promise_result_sync()
                        .expect("prom timed out")
                        .expect("prom was rejected");
                }
                _ => {}
            }
        }

        assert!(jsvf_res.is_js_object());

        match jsvf_res {
            JsValueFacade::JsObject { cached_object } => {
                let obj = cached_object.get_object_sync().expect("esvf to map failed");
                let b = obj.get("b").expect("got no b");
                assert!(b.is_i32());
                let i = b.get_i32();
                assert_eq!(i, 5 * 7);
            }
            _ => {}
        }

        rt.exe_task_in_event_loop(|| {
            QuickJsRuntimeAdapter::remove_context("test");
        })
    }

    #[test]
    fn test_to_string_err() {
        let rt = QuickJsRuntimeBuilder::new().build();

        let res = block_on(rt.eval(
            None,
            Script::new(
                "test_test_to_string_err.js",
                r#"
            (async () => {
                throw Error("poof");
            })();
        "#,
            ),
        ));
        match res {
            Ok(val) => {
                if let JsValueFacade::JsPromise { cached_promise } = val {
                    let prom_res =
                        block_on(cached_promise.get_promise_result()).expect("promise timed out");
                    match prom_res {
                        Ok(v) => {
                            panic!("promise unexpectedly resolved to val: {:?}", v);
                        }
                        Err(ev) => {
                            println!("prom resolved to error: {ev:?}");
                        }
                    }
                } else {
                    panic!("func did not return a promise");
                }
            }
            Err(e) => {
                panic!("scrtip failed {}", e)
            }
        }
    }
}
