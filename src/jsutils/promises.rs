use crate::jsutils::helper_tasks::{add_helper_task, add_helper_task_async};
use crate::jsutils::JsError;
use crate::quickjs_utils::promises::QuickJsPromiseAdapter;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use futures::Future;

#[allow(clippy::type_complexity)]
/// create a new promise with a producer and a mapper
/// the producer will run in a helper thread(in the tokio thread pool) and thus get a result asynchronously
/// the resulting value will then be mapped to a JSValueRef by the mapper in the EventQueue thread
/// the promise which was returned is then resolved with the value which is returned by the mapper
pub fn new_resolving_promise<P, R, M>(
    realm: &QuickJsRealmAdapter,
    producer: P,
    mapper: M,
) -> Result<QuickJsValueAdapter, JsError>
where
    R: Send + 'static,
    P: FnOnce() -> Result<R, JsError> + Send + 'static,
    M: FnOnce(&QuickJsRealmAdapter, R) -> Result<QuickJsValueAdapter, JsError> + Send + 'static,
{
    // create promise
    let promise_ref = realm.create_promise()?;
    let return_ref = promise_ref.js_promise_get_value(realm);

    // add to map and keep id
    let id = realm.cache_promise(promise_ref);

    let rti_ref = realm.get_runtime_facade_inner();

    let realm_id = realm.get_realm_id().to_string();
    // go async
    add_helper_task(move || {
        // in helper thread, produce result
        let produced_result = producer();
        if let Some(rti) = rti_ref.upgrade() {
            rti.add_rt_task_to_event_loop_void(move |rt| {
                if let Some(realm) = rt.get_realm(realm_id.as_str()) {
                    // in q_js_rt worker thread, resolve promise
                    // retrieve promise
                    let prom_ref_opt: Option<QuickJsPromiseAdapter> =
                        realm.consume_cached_promise(id);
                    if let Some(prom_ref) = prom_ref_opt {
                        //let prom_ref = realm.js_promise_cache_consume(id);
                        match produced_result {
                            Ok(ok_res) => {
                                // map result to JSValueRef
                                let raw_res = mapper(realm, ok_res);

                                // resolve or reject promise
                                match raw_res {
                                    Ok(val_ref) => {
                                        if let Err(e) = prom_ref.js_promise_resolve(realm, &val_ref)
                                        {
                                            log::error!(
                                                "[{}] could not resolve promise5: {}",
                                                realm.get_realm_id(),
                                                e
                                            );
                                        }
                                    }
                                    Err(err) => {
                                        let err_ref = realm
                                            .create_error(
                                                err.get_name(),
                                                err.get_message(),
                                                err.get_stack(),
                                            )
                                            .expect("could not create error");
                                        if let Err(e) = prom_ref.js_promise_reject(realm, &err_ref)
                                        {
                                            log::error!(
                                                "[{}] could not reject promise4: {}",
                                                realm.get_realm_id(),
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                // todo use error:new_error(err)
                                let err_ref = realm
                                    .create_error(
                                        err.get_name(),
                                        err.get_message(),
                                        err.get_stack(),
                                    )
                                    .expect("could not create error");
                                if let Err(e) = prom_ref.js_promise_reject(realm, &err_ref) {
                                    log::error!(
                                        "[{}] could not reject promise3: {}",
                                        realm.get_realm_id(),
                                        e
                                    );
                                }
                            }
                        }
                    } else {
                        log::error!(
                            "async promise running for dropped realm: {} promise_id:{}",
                            realm_id,
                            id
                        );
                    }
                } else {
                    log::error!("async promise running for dropped realm: {}", realm_id);
                }
            });
        } else {
            log::error!("async promise running for dropped runtime");
        }
    });

    Ok(return_ref)
}

#[allow(clippy::type_complexity)]
/// create a new promise with an async producer and a mapper
/// the producer will be awaited asynchronously and
/// the resulting value will then be mapped to a JSValueRef by the mapper in the EventQueue thread
/// the promise which was returned is then resolved with the value which is returned by the mapper
pub(crate) fn new_resolving_promise_async<P, R, M>(
    realm: &QuickJsRealmAdapter,
    producer: P,
    mapper: M,
) -> Result<QuickJsValueAdapter, JsError>
where
    R: Send + 'static,
    P: Future<Output = Result<R, JsError>> + Send + 'static,
    M: FnOnce(&QuickJsRealmAdapter, R) -> Result<QuickJsValueAdapter, JsError> + Send + 'static,
{
    // create promise
    let promise_ref = realm.create_promise()?;
    let return_ref = promise_ref.js_promise_get_value(realm);

    // add to map and keep id
    let id = realm.cache_promise(promise_ref);

    let rti_ref = realm.get_runtime_facade_inner();

    let realm_id = realm.get_realm_id().to_string();
    // go async
    let _ignore_result = add_helper_task_async(async move {
        // in helper thread, produce result
        let produced_result = producer.await;
        if let Some(rti) = rti_ref.upgrade() {
            rti.add_rt_task_to_event_loop_void(move |rt| {
                if let Some(realm) = rt.get_realm(realm_id.as_str()) {
                    // in q_js_rt worker thread, resolve promise
                    // retrieve promise
                    let prom_ref_opt: Option<QuickJsPromiseAdapter> =
                        realm.consume_cached_promise(id);
                    if let Some(prom_ref) = prom_ref_opt {
                        //let prom_ref = realm.js_promise_cache_consume(id);
                        match produced_result {
                            Ok(ok_res) => {
                                // map result to JSValueRef
                                let raw_res = mapper(realm, ok_res);

                                // resolve or reject promise
                                match raw_res {
                                    Ok(val_ref) => {
                                        if let Err(e) = prom_ref.js_promise_resolve(realm, &val_ref)
                                        {
                                            log::error!(
                                                "[{}] could not resolve promise: {}",
                                                realm.get_realm_id(),
                                                e
                                            );
                                        }
                                    }
                                    Err(err) => {
                                        let err_ref = realm
                                            .create_error(
                                                err.get_name(),
                                                err.get_message(),
                                                err.get_stack(),
                                            )
                                            .expect("could not create err");
                                        if let Err(e) = prom_ref.js_promise_reject(realm, &err_ref)
                                        {
                                            log::error!(
                                                "[{}] could not reject promise: {}",
                                                realm.get_realm_id(),
                                                e
                                            );
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                // todo use error:new_error(err)
                                let err_ref = realm
                                    .create_error(
                                        err.get_name(),
                                        err.get_message(),
                                        err.get_stack(),
                                    )
                                    .expect("could not create str");
                                if let Err(e) = prom_ref.js_promise_reject(realm, &err_ref) {
                                    log::error!(
                                        "[{}] could not reject promise2: {}",
                                        realm.get_realm_id(),
                                        e
                                    );
                                }
                            }
                        }
                    } else {
                        log::error!(
                            "async promise running on dropped realm: {} promise_id:{}",
                            realm_id,
                            id
                        );
                    }
                } else {
                    log::error!("async promise running on dropped realm: {}", realm_id);
                }
            });
        } else {
            log::error!("async promise running on dropped runtime");
        }
    });
    Ok(return_ref)
}
