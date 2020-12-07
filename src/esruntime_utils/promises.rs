use crate::eserror::EsError;
use crate::esruntime::EsRuntime;
use crate::quickjs_utils::primitives;
use crate::quickjs_utils::promises::new_promise;
use crate::quickjs_utils::promises::PromiseRef;
use crate::quickjscontext::QuickJsContext;
use crate::valueref::JSValueRef;
use hirofa_utils::auto_id_map::AutoIdMap;
use std::cell::RefCell;
thread_local! {
    static RESOLVING_PROMISES: RefCell<AutoIdMap<PromiseRef>> = RefCell::new(AutoIdMap::new());
}

/// create a new promise with a resolver/mapper
/// the resolver will run in a helper thread and thus get a result asynchronously
/// the resulting value will then be mapped to a JSValueRef by the mapper in the EventQueue
/// the promise which was returned is then resolved with the value which is returned by the mapper
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::quickjs_utils::{functions, objects, primitives};
/// use quickjs_es_runtime::quickjs_utils;
/// use quickjs_es_runtime::esscript::EsScript;
/// use std::time::Duration;
/// use quickjs_es_runtime::esruntime_utils::promises;
/// use quickjs_es_runtime::quickjsruntime::QuickJsRuntime;
/// let rt = EsRuntimeBuilder::new().build();
/// let rt_ref = rt.clone();
/// rt.add_to_event_queue_sync(move |q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///      // create rust function, please note that using new_native_function_data will be the faster option
///      let func_ref = functions::new_function(q_ctx.context, "asyncTest", move |_this_ref, _args| {
///           let rt_ref = rt_ref.clone();
///           QuickJsRuntime::do_with(move |q_js_rt| {
///               let q_ctx = q_js_rt.get_main_context();
///               let prom = promises::new_resolving_promise(q_ctx, ||{
///                   std::thread::sleep(Duration::from_secs(1));
///                   Ok(135)
///               }, |_ctx, res|{
///                   Ok(primitives::from_i32(res))
///               }, &rt_ref);
///               prom
///           })
///      }, 1).ok().expect("could not create func");
///
///      // add func to global scope
///      let global_ref = quickjs_utils::get_global(q_ctx.context);
///      objects::set_property(q_ctx.context, &global_ref, "asyncTest", func_ref).ok()
///             .expect("could not set prop");;
///            
/// });
/// rt.eval_sync(EsScript::new("test_async.es", "console.log('async test');\n
/// let p = this.asyncTest(123); \n
/// console.log('p instanceof Promise = ' + p instanceof Promise);\n
/// p.then((res) => {\n
///     console.log('p resolved to ' + res);\n
/// }).catch((err) => {\n
///     console.log('p rejected to ' + err);\n
/// });
/// ")).ok().expect("script failed");
/// // wait so promise can fullfill
/// std::thread::sleep(Duration::from_secs(2));
/// ```
pub fn new_resolving_promise<P, R, M>(
    q_ctx: &QuickJsContext,
    producer: P,
    mapper: M,
    es_rt: &EsRuntime,
) -> Result<JSValueRef, EsError>
where
    R: Send + 'static,
    P: FnOnce() -> Result<R, String> + Send + 'static,
    M: FnOnce(&QuickJsContext, R) -> Result<JSValueRef, EsError> + Send + 'static,
{
    // create promise
    let promise_ref = new_promise(q_ctx.context)?;
    let return_ref = promise_ref.get_promise_obj_ref();

    // add to map and keep id
    let id = RESOLVING_PROMISES.with(|map_rc| {
        let map = &mut *map_rc.borrow_mut();
        map.insert(promise_ref)
    });

    let rti_ref = es_rt.inner.clone();

    let ctx_id = q_ctx.id.clone();
    // go async
    EsRuntime::add_helper_task(move || {
        // in helper thread, produce result
        let produced_result = producer();
        rti_ref.add_to_event_queue(move |q_js_rt| {
            let q_ctx = q_js_rt.get_context(ctx_id.as_str());
            // in q_js_rt worker thread, resolve promise
            // retrieve promise
            let prom_ref = RESOLVING_PROMISES.with(|map_rc| {
                let map = &mut *map_rc.borrow_mut();
                map.remove(&id)
            });

            match produced_result {
                Ok(ok_res) => {
                    // map result to JSValueRef
                    let raw_res = mapper(q_ctx, ok_res);

                    // resolve or reject promise
                    match raw_res {
                        Ok(val_ref) => {
                            prom_ref
                                .resolve(q_ctx.context, val_ref)
                                .ok()
                                .expect("prom resolution failed");
                        }
                        Err(err) => {
                            // todo use error:new_error(err.get_message)
                            let err_ref = primitives::from_string(q_ctx.context, err.get_message())
                                .ok()
                                .expect("could not create str");
                            prom_ref
                                .reject(q_ctx.context, err_ref)
                                .ok()
                                .expect("prom rejection failed");
                        }
                    }
                }
                Err(err) => {
                    // todo use error:new_error(err)
                    let err_ref = primitives::from_string(q_ctx.context, err.as_str())
                        .ok()
                        .expect("could not create str");
                    prom_ref
                        .reject(q_ctx.context, err_ref)
                        .ok()
                        .expect("prom rejection failed");
                }
            }
        })
    });

    Ok(return_ref)
}

#[cfg(test)]

pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esruntime_utils::promises;
    use crate::esruntime_utils::promises::RESOLVING_PROMISES;
    use crate::esscript::EsScript;
    use crate::quickjs_utils;
    use crate::quickjs_utils::{functions, objects, primitives};
    use crate::quickjsruntime::QuickJsRuntime;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_resolving_prom() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let rt_ref = rt.clone();
        rt.add_to_event_queue_sync(move |q_js_rt| {
            // create rust function, please note that using new_native_function_data will be the faster option
            let q_ctx = q_js_rt.get_main_context();
            let func_ref = functions::new_function(
                q_ctx.context,
                "asyncTest",
                move |_this_ref, _args| {
                    let rt_ref = rt_ref.clone();
                    QuickJsRuntime::do_with(move |q_js_rt| {
                        let q_ctx = q_js_rt.get_main_context();
                        let prom = promises::new_resolving_promise(
                            q_ctx,
                            || {
                                std::thread::sleep(Duration::from_secs(1));
                                Ok(135)
                            },
                            |_q_ctx, res| Ok(primitives::from_i32(res)),
                            &rt_ref,
                        );
                        prom
                    })
                },
                1,
            )
            .ok()
            .expect("could not create func");

            assert_eq!(1, func_ref.get_ref_count());

            // add func to global scope
            let global_ref = quickjs_utils::get_global(q_ctx.context);
            let i = global_ref.get_ref_count();
            objects::set_property(q_ctx.context, &global_ref, "asyncTest", func_ref.clone())
                .ok()
                .expect("could not set prop");
            assert_eq!(i, global_ref.get_ref_count());
            assert_eq!(2, func_ref.get_ref_count());
        });
        log::trace!("running gc after resolving promise init");
        //rt.gc_sync();

        rt.eval_sync(EsScript::new(
            "test_async.es",
            "console.log('async test');\n
         let p = this.asyncTest(123); \n
         console.log('p instanceof Promise = ' + p instanceof Promise);\n
         p.then((res) => {\n
             console.log('p resolved to ' + res);\n
         }).catch((err) => {\n
             console.log('p rejected to ' + err);\n
         });
         ",
        ))
        .ok()
        .expect("script failed");
        rt.gc_sync();
        // wait so promise can fullfill
        std::thread::sleep(Duration::from_secs(2));
        assert!(RESOLVING_PROMISES.with(|rc| { (*rc.borrow()).is_empty() }))
    }
}
