//! EventTarget utils
//!

use crate::jsutils::JsError;
use crate::quickjs_utils;
use crate::quickjs_utils::objects::{create_object_q, set_property_q};
use crate::quickjs_utils::primitives::from_bool;
use crate::quickjs_utils::{functions, objects, parse_args, primitives};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use crate::reflection::{get_proxy, get_proxy_instance_info, Proxy};
use libquickjs_sys as q;
use std::collections::HashMap;

fn with_proxy_instances_map<C, R>(
    q_ctx: &QuickJsRealmAdapter,
    proxy_class_name: &str,
    consumer: C,
) -> R
where
    C: FnOnce(
        &mut HashMap<usize, HashMap<String, HashMap<QuickJsValueAdapter, QuickJsValueAdapter>>>,
    ) -> R,
{
    let listeners = &mut *q_ctx.proxy_event_listeners.borrow_mut();
    if !listeners.contains_key(proxy_class_name) {
        listeners.insert(proxy_class_name.to_string(), HashMap::new());
    }
    let proxy_instance_map = listeners.get_mut(proxy_class_name).unwrap();

    consumer(proxy_instance_map)
}

fn with_listener_map<C, R>(
    q_ctx: &QuickJsRealmAdapter,
    proxy_class_name: &str,
    instance_id: usize,
    event_id: &str,
    consumer: C,
) -> R
where
    C: FnOnce(&mut HashMap<QuickJsValueAdapter, QuickJsValueAdapter>) -> R,
{
    with_proxy_instances_map(q_ctx, proxy_class_name, |proxy_instance_map| {
        let event_id_map = proxy_instance_map
            .entry(instance_id)
            .or_insert_with(HashMap::new);

        if !event_id_map.contains_key(event_id) {
            event_id_map.insert(event_id.to_string(), HashMap::new());
        }

        let listener_map = event_id_map.get_mut(event_id).unwrap();

        consumer(listener_map)
    })
}

fn with_static_listener_map<C, R>(
    q_ctx: &QuickJsRealmAdapter,
    proxy_class_name: &str,
    event_id: &str,
    consumer: C,
) -> R
where
    C: FnOnce(&mut HashMap<QuickJsValueAdapter, QuickJsValueAdapter>) -> R,
{
    let static_listeners = &mut *q_ctx.proxy_static_event_listeners.borrow_mut();
    if !static_listeners.contains_key(proxy_class_name) {
        static_listeners.insert(proxy_class_name.to_string(), HashMap::new());
    }
    let proxy_static_map = static_listeners.get_mut(proxy_class_name).unwrap();
    if !proxy_static_map.contains_key(event_id) {
        proxy_static_map.insert(event_id.to_string(), HashMap::new());
    }
    let event_map = proxy_static_map.get_mut(event_id).unwrap();
    consumer(event_map)
}

pub fn add_event_listener(
    q_ctx: &QuickJsRealmAdapter,
    proxy_class_name: &str,
    event_id: &str,
    instance_id: usize,
    listener_func: QuickJsValueAdapter,
    options_obj: QuickJsValueAdapter,
) {
    log::trace!(
        "eventtarget::add_listener_to_map p:{} e:{} i:{}",
        proxy_class_name,
        event_id,
        instance_id
    );
    with_listener_map(q_ctx, proxy_class_name, instance_id, event_id, |map| {
        let _ = map.insert(listener_func, options_obj);
    })
}

pub fn add_static_event_listener(
    q_ctx: &QuickJsRealmAdapter,
    proxy_class_name: &str,
    event_id: &str,
    listener_func: QuickJsValueAdapter,
    options_obj: QuickJsValueAdapter,
) {
    log::trace!(
        "eventtarget::add_static_listener_to_map p:{} e:{}",
        proxy_class_name,
        event_id
    );
    with_static_listener_map(q_ctx, proxy_class_name, event_id, |map| {
        let _ = map.insert(listener_func, options_obj);
    })
}

pub fn remove_event_listener(
    q_ctx: &QuickJsRealmAdapter,
    proxy_class_name: &str,
    event_id: &str,
    instance_id: usize,
    listener_func: &QuickJsValueAdapter,
) {
    log::trace!(
        "eventtarget::remove_listener_from_map p:{} e:{} i:{}",
        proxy_class_name,
        event_id,
        instance_id
    );
    with_listener_map(q_ctx, proxy_class_name, instance_id, event_id, |map| {
        let _ = map.remove(listener_func);
    })
}

pub fn remove_static_event_listener(
    q_ctx: &QuickJsRealmAdapter,
    proxy_class_name: &str,
    event_id: &str,
    listener_func: &QuickJsValueAdapter,
) {
    log::trace!(
        "eventtarget::remove_static_listener_from_map p:{} e:{}",
        proxy_class_name,
        event_id
    );
    with_static_listener_map(q_ctx, proxy_class_name, event_id, |map| {
        let _ = map.remove(listener_func);
    })
}

fn remove_map(q_ctx: &QuickJsRealmAdapter, proxy_class_name: &str, instance_id: usize) {
    log::trace!(
        "eventtarget::remove_map p:{} i:{}",
        proxy_class_name,
        instance_id
    );
    with_proxy_instances_map(q_ctx, proxy_class_name, |map| {
        let _ = map.remove(&instance_id);
    })
}

/// dispatch an Event on an instance of a Proxy class
/// the return value is false if event is cancelable and at least one of the event listeners which received event called Event.preventDefault. Otherwise it returns true
pub fn dispatch_event(
    q_ctx: &QuickJsRealmAdapter,
    proxy: &Proxy,
    instance_id: usize,
    event_id: &str,
    event: QuickJsValueAdapter,
) -> Result<bool, JsError> {
    let proxy_class_name = proxy.get_class_name();

    with_listener_map(
        q_ctx,
        proxy_class_name.as_str(),
        instance_id,
        event_id,
        |listeners| -> Result<(), JsError> {
            let func_args = [event];
            for entry in listeners {
                let listener = entry.0;
                let _res = functions::call_function_q(q_ctx, listener, &func_args, None)?;

                // todo chekc if _res is bool, for cancel and such
                // and if event is cancelabble and preventDefault was called and such
            }
            Ok(())
        },
    )?;

    Ok(true)
}

/// dispatch an Event on a Proxy class
/// the return value is false if event is cancelable and at least one of the event listeners which received event called Event.preventDefault. Otherwise it returns true
pub fn dispatch_static_event(
    q_ctx: &QuickJsRealmAdapter,
    proxy_class_name: &str,
    event_id: &str,
    event: QuickJsValueAdapter,
) -> Result<bool, JsError> {
    with_static_listener_map(
        q_ctx,
        proxy_class_name,
        event_id,
        |listeners| -> Result<(), JsError> {
            let func_args = [event];
            for entry in listeners {
                let listener = entry.0;
                let _res = functions::call_function_q(q_ctx, listener, &func_args, None)?;

                // todo chekc if _res is bool, for cancel and such
                // and if event is cancelabble and preventDefault was called and such
            }
            Ok(())
        },
    )?;

    Ok(true)
}

pub fn _set_event_bubble_target() {
    unimplemented!()
}

fn events_instance_finalizer(q_ctx: &QuickJsRealmAdapter, proxy_class_name: &str, id: usize) {
    // drop all listeners,
    remove_map(q_ctx, proxy_class_name, id);
}

pub(crate) fn impl_event_target(proxy: Proxy) -> Proxy {
    // add (static)     addEventListener(), dispatchEvent(), removeEventListener()
    // a fn getEventsObj will be used to conditionally create and return an Object with Sets per eventId to store the listeners

    let proxy_class_name = proxy.get_class_name();

    let mut proxy = proxy;
    if proxy.is_event_target {
        proxy = proxy
            .native_method("addEventListener", Some(ext_add_event_listener))
            .native_method("removeEventListener", Some(ext_remove_event_listener))
            .native_method("dispatchEvent", Some(ext_dispatch_event))
            .finalizer(move |_rt, q_ctx, id| {
                let n = proxy_class_name.as_str();
                events_instance_finalizer(q_ctx, n, id);
            });
    }
    if proxy.is_static_event_target {
        // todo, these should be finalized before context is destroyed, we really need a hook in QuickJsContext for that
        proxy = proxy
            .static_native_method("addEventListener", Some(ext_add_static_event_listener))
            .static_native_method(
                "removeEventListener",
                Some(ext_remove_static_event_listener),
            )
            .static_native_method("dispatchEvent", Some(ext_dispatch_static_event));
    }

    proxy
}

unsafe extern "C" fn ext_add_event_listener(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    // require 2 or 3 args, string, function, object
    // if third is boolean it is option {capture: true}

    // events_obj will be structured like this
    // ___eventListeners___: {eventId<String>: Map<Function, Object>} // the key of the map is the function, the value are the options

    let res = QuickJsRealmAdapter::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref =
            QuickJsValueAdapter::new(ctx, this_val, true, true, "add_event_listener_this");

        let proxy_info = get_proxy_instance_info(this_ref.borrow_value());

        if args.len() < 2 || !args[0].is_string() || !functions::is_function_q(q_ctx, &args[1]) {
            Err(JsError::new_str("addEventListener requires at least 2 arguments (eventId: String and Listener: Function"))
        } else {
            let event_id = primitives::to_string_q(q_ctx, &args[0])?;
            let listener_func = args[1].clone();

            // use the passed options arg or create a new obj
            let options_obj = if args.len() == 3 && args[2].is_object() {
                args[2].clone()
            } else {
                create_object_q(q_ctx)?
            };
            // if the third args was a boolean then set that bool as the capture option
            if args.len() == 3 && args[2].is_bool() {
                set_property_q(q_ctx, &options_obj, "capture", &args[2])?;
            }

            add_event_listener(
                q_ctx,
                proxy_info.class_name.as_str(),
                event_id.as_str(),
                proxy_info.id,
                listener_func,
                options_obj,
            );

            Ok(())
        }
    });
    match res {
        Ok(_) => quickjs_utils::new_null(),
        Err(e) => QuickJsRealmAdapter::report_ex_ctx(ctx, format!("{e}").as_str()),
    }
}

unsafe extern "C" fn ext_remove_event_listener(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let res = QuickJsRealmAdapter::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref =
            QuickJsValueAdapter::new(ctx, this_val, true, true, "remove_event_listener_this");

        let proxy_info = get_proxy_instance_info(this_ref.borrow_value());

        if args.len() != 2 || !args[0].is_string() || !functions::is_function_q(q_ctx, &args[1]) {
            Err(JsError::new_str("removeEventListener requires at least 2 arguments (eventId: String and Listener: Function"))
        } else {
            let event_id = primitives::to_string_q(q_ctx, &args[0])?;
            let listener_func = args[1].clone();

            remove_event_listener(
                q_ctx,
                proxy_info.class_name.as_str(),
                event_id.as_str(),
                proxy_info.id,
                &listener_func,
            );

            Ok(())
        }
    });
    match res {
        Ok(_) => quickjs_utils::new_null(),
        Err(e) => QuickJsRealmAdapter::report_ex_ctx(ctx, format!("{e}").as_str()),
    }
}

unsafe extern "C" fn ext_dispatch_event(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let res = QuickJsRealmAdapter::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref =
            QuickJsValueAdapter::new(ctx, this_val, true, true, "remove_event_listener_this");

        let proxy_info = get_proxy_instance_info(this_ref.borrow_value());

        if args.len() != 2 || !args[0].is_string() {
            Err(JsError::new_str(
                "dispatchEvent requires at least 2 arguments (eventId: String and eventObj: Object)",
            ))
        } else {
            let event_id = primitives::to_string_q(q_ctx, &args[0])?;
            let evt_obj = args[1].clone();

            let proxy = get_proxy(q_ctx, proxy_info.class_name.as_str()).unwrap();

            let res = dispatch_event(q_ctx, &proxy, proxy_info.id, event_id.as_str(), evt_obj)?;

            Ok(res)
        }
    });
    match res {
        Ok(res) => {
            let b_ref = from_bool(res);
            b_ref.clone_value_incr_rc()
        }
        Err(e) => QuickJsRealmAdapter::report_ex_ctx(ctx, format!("{e}").as_str()),
    }
}

unsafe fn get_static_proxy_class_name(
    q_ctx: &QuickJsRealmAdapter,
    obj: &QuickJsValueAdapter,
) -> String {
    let proxy_name_ref = objects::get_property(q_ctx.context, obj, "name")
        .ok()
        .unwrap();
    primitives::to_string(q_ctx.context, &proxy_name_ref)
        .ok()
        .unwrap()
}

unsafe extern "C" fn ext_add_static_event_listener(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let res = QuickJsRealmAdapter::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref =
            QuickJsValueAdapter::new(ctx, this_val, true, true, "add_event_listener_this");

        let proxy_name = get_static_proxy_class_name(q_ctx, &this_ref);

        if args.len() < 2 || !args[0].is_string() || !functions::is_function_q(q_ctx, &args[1]) {
            Err(JsError::new_str("addEventListener requires at least 2 arguments (eventId: String and Listener: Function"))
        } else {
            let event_id = primitives::to_string_q(q_ctx, &args[0])?;
            let listener_func = args[1].clone();

            // use the passed options arg or create a new obj
            let options_obj = if args.len() == 3 && args[2].is_object() {
                args[2].clone()
            } else {
                create_object_q(q_ctx)?
            };
            // if the third args was a boolean then set that bool as the capture option
            if args.len() == 3 && args[2].is_bool() {
                set_property_q(q_ctx, &options_obj, "capture", &args[2])?;
            }

            add_static_event_listener(
                q_ctx,
                proxy_name.as_str(),
                event_id.as_str(),
                listener_func,
                options_obj,
            );

            Ok(())
        }
    });
    match res {
        Ok(_) => quickjs_utils::new_null(),
        Err(e) => QuickJsRealmAdapter::report_ex_ctx(ctx, format!("{e}").as_str()),
    }
}

unsafe extern "C" fn ext_remove_static_event_listener(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let res = QuickJsRealmAdapter::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref =
            QuickJsValueAdapter::new(ctx, this_val, true, true, "remove_event_listener_this");

        let proxy_name = get_static_proxy_class_name(q_ctx, &this_ref);

        if args.len() != 2 || !args[0].is_string() || !functions::is_function_q(q_ctx, &args[1]) {
            Err(JsError::new_str("removeEventListener requires at least 2 arguments (eventId: String and Listener: Function"))
        } else {
            let event_id = primitives::to_string_q(q_ctx, &args[0])?;
            let listener_func = args[1].clone();

            remove_static_event_listener(
                q_ctx,
                proxy_name.as_str(),
                event_id.as_str(),
                &listener_func,
            );

            Ok(())
        }
    });
    match res {
        Ok(_) => quickjs_utils::new_null(),
        Err(e) => QuickJsRealmAdapter::report_ex_ctx(ctx, format!("{e}").as_str()),
    }
}

unsafe extern "C" fn ext_dispatch_static_event(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let res = QuickJsRealmAdapter::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref =
            QuickJsValueAdapter::new(ctx, this_val, true, true, "remove_event_listener_this");

        let proxy_name = get_static_proxy_class_name(q_ctx, &this_ref);

        if args.len() != 2 || !args[0].is_string() {
            Err(JsError::new_str(
                "dispatchEvent requires at least 2 arguments (eventId: String and eventObj: Object)",
            ))
        } else {
            let event_id = primitives::to_string_q(q_ctx, &args[0])?;
            let evt_obj = args[1].clone();

            let res =
                dispatch_static_event(q_ctx, proxy_name.as_str(), event_id.as_str(), evt_obj)?;

            Ok(res)
        }
    });
    match res {
        Ok(res) => {
            let b_ref = from_bool(res);
            b_ref.clone_value_incr_rc()
        }
        Err(e) => QuickJsRealmAdapter::report_ex_ctx(ctx, format!("{e}").as_str()),
    }
}

#[cfg(test)]
pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::jsutils::Script;
    use crate::quickjs_utils::get_global_q;
    use crate::quickjs_utils::objects::{create_object_q, get_property_q};
    use crate::quickjs_utils::primitives::to_i32;
    use crate::reflection::eventtarget::dispatch_event;
    use crate::reflection::{get_proxy, Proxy};
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_proxy_eh() {
        let instance_ids: Arc<Mutex<Vec<usize>>> = Arc::new(Mutex::new(vec![]));

        let instance_ids2 = instance_ids.clone();

        let rt = init_test_rt();
        let ct = rt.exe_rt_task_in_event_loop(move |q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            Proxy::new()
                .namespace(&[])
                .constructor(move |_rt, _q, id, _args| {
                    log::debug!("construct id={}", id);
                    let vec = &mut *instance_ids2.lock().unwrap();
                    vec.push(id);
                    Ok(())
                })
                .finalizer(|_rt, _q_ctx, id| {
                    log::debug!("finalize id={}", id);
                })
                .name("MyThing")
                .event_target()
                .install(q_ctx, true)
                .expect("proxy failed");

            match q_ctx.eval(Script::new(
                "test_proxy_eh.es",
                "\
            this.called = false;\
            let test_proxy_eh_instance = new MyThing();\
            this.ct = 0;\
            let listener1 = (evt) => {this.ct++;};\
            let listener2 = (evt) => {this.ct++;};\
            test_proxy_eh_instance.addEventListener('someEvent', listener1);\
            test_proxy_eh_instance.addEventListener('someEvent', listener2);\
            test_proxy_eh_instance.removeEventListener('someEvent', listener2);\
            ",
            )) {
                Ok(_) => {}
                Err(e) => {
                    log::error!("script failed: {}", e);
                    panic!("script failed: {}", e);
                }
            };
            let global = get_global_q(q_ctx);

            let proxy = get_proxy(q_ctx, "MyThing").unwrap();
            let vec = &mut *instance_ids.lock().unwrap();
            let id = vec[0];
            let evt = create_object_q(q_ctx).ok().unwrap();
            let _ = dispatch_event(q_ctx, &proxy, id, "someEvent", evt).expect("dispatch failed");

            let ct_ref = get_property_q(q_ctx, &global, "ct").ok().unwrap();

            to_i32(&ct_ref).ok().unwrap()
        });
        log::info!("ok was {}", ct);
        assert_eq!(ct, 1);
    }

    #[test]
    fn test_proxy_eh_rcs() {
        let rt = init_test_rt();
        rt.exe_rt_task_in_event_loop(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            Proxy::new()
                .namespace(&[])
                .constructor(move |_rt, _q, id, _args| {
                    log::debug!("construct id={}", id);
                    Ok(())
                })
                .finalizer(|_rt, _q_ctx, id| {
                    log::debug!("finalize id={}", id);
                })
                .name("MyThing")
                .event_target()
                .install(q_ctx, true)
                .expect("proxy failed");

            q_ctx
                .eval(Script::new("e.es", "let target = new MyThing();"))
                .expect("constr failed");
            let target_ref = q_ctx
                .eval(Script::new("t.es", "(target);"))
                .expect("could not get target");
            assert_eq!(target_ref.get_ref_count(), 2); // one for me one for global

            q_ctx
                .eval(Script::new(
                    "r.es",
                    "target.addEventListener('someEvent', (evt) => {console.log('got event');});",
                ))
                .expect("addlistnrfailed");

            assert_eq!(target_ref.get_ref_count(), 2); // one for me one for global
        });
    }
}
