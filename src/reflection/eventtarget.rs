//! EventTarget utils
//!

use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::objects::{
    create_object_q, get_property_q, set_property2_q, set_property_q,
};
use crate::quickjs_utils::{functions, maps, objects, parse_args, primitives};
use crate::quickjscontext::QuickJsContext;
use crate::reflection::Proxy;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

pub fn _dispatch_event(_proxy: &Proxy, _instance_id: usize, _event: JSValueRef) {
    // hmm... i'm afraid this means we need to store eventListeners in a map we can acces by id instead of the current put a map in proxy solution
    unimplemented!()
}

pub fn _set_event_bubble_target() {
    unimplemented!()
}

pub(crate) fn impl_event_target(proxy: Proxy) -> Proxy {
    // add (static)     addEventListener(), dispatchEvent(), removeEventListener()
    // a fn getEventsObj will be used to conditionally create and return an Object with Sets per eventId to store the listeners

    let mut proxy = proxy;
    if proxy.is_event_target {
        proxy = proxy
            .native_method("addEventListener", Some(ext_add_event_listener))
            .native_method("removeEventListener", Some(ext_remove_event_listener))
            .native_method("dispatchEvent", Some(ext_dispatch_event));
    }
    if proxy.is_static_event_target {
        proxy = proxy
            .static_native_method("addEventListener", Some(ext_add_event_listener))
            .static_native_method("removeEventListener", Some(ext_remove_event_listener))
            .static_native_method("dispatchEvent", Some(ext_dispatch_event));
    }

    proxy
}

fn get_events_obj_map(
    q_ctx: &QuickJsContext,
    obj: &JSValueRef,
    event_id: &str,
) -> Result<JSValueRef, EsError> {
    let opt = get_property_q(q_ctx, obj, "___eventListeners___")?;
    let events_obj = if opt.is_null_or_undefined() {
        let new_obj = create_object_q(q_ctx)?;
        set_property2_q(q_ctx, &obj, "___eventListeners___", &new_obj, 0)?;
        new_obj
    } else {
        opt
    };

    let map_opt = get_property_q(q_ctx, &events_obj, event_id)?;
    if map_opt.is_null_or_undefined() {
        let new_map = maps::new_map_q(q_ctx)?;
        set_property2_q(q_ctx, &events_obj, event_id, &new_map, 0)?;
        Ok(new_map)
    } else {
        Ok(map_opt)
    }
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

    let res = QuickJsContext::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref = JSValueRef::new(ctx, this_val, true, true, "add_event_listener_this");

        if args.len() < 2 || !args[0].is_string() || !functions::is_function_q(q_ctx, &args[1]) {
            Err(EsError::new_str("addEventListener requires at least 2 arguments (eventId: String and Listener: Function"))
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

            // get the Map
            let events_listeners_map = get_events_obj_map(q_ctx, &this_ref, event_id.as_str())?;
            // add listener and options to the map
            maps::set_q(q_ctx, &events_listeners_map, listener_func, options_obj)?;

            Ok(())
        }
    });
    match res {
        Ok(_) => quickjs_utils::new_null(),
        Err(e) => QuickJsContext::report_ex_ctx(ctx, format!("{}", e).as_str()),
    }
}

unsafe extern "C" fn ext_remove_event_listener(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let res = QuickJsContext::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref = JSValueRef::new(ctx, this_val, true, true, "remove_event_listener_this");

        if args.len() != 2 || !args[0].is_string() || !functions::is_function_q(q_ctx, &args[1]) {
            Err(EsError::new_str("removeEventListener requires at least 2 arguments (eventId: String and Listener: Function"))
        } else {
            let event_id = primitives::to_string_q(q_ctx, &args[0])?;
            let listener_func = args[1].clone();

            // get the Map
            let events_listeners_map = get_events_obj_map(q_ctx, &this_ref, event_id.as_str())?;
            // add listener and options to the map
            maps::delete_q(q_ctx, &events_listeners_map, listener_func)?;

            Ok(())
        }
    });
    match res {
        Ok(_) => quickjs_utils::new_null(),
        Err(e) => QuickJsContext::report_ex_ctx(ctx, format!("{}", e).as_str()),
    }
}

unsafe extern "C" fn ext_dispatch_event(
    ctx: *mut q::JSContext,
    this_val: q::JSValue,
    argc: ::std::os::raw::c_int,
    argv: *mut q::JSValue,
) -> q::JSValue {
    let res = QuickJsContext::with_context(ctx, |q_ctx| {
        let args = parse_args(ctx, argc, argv);

        let this_ref = JSValueRef::new(ctx, this_val, true, true, "remove_event_listener_this");

        if args.len() != 2 || !args[0].is_string() {
            Err(EsError::new_str(
                "dispatchEvent requires at least 2 arguments (eventId: String and eventObj: Object)",
            ))
        } else {
            let event_id = primitives::to_string_q(q_ctx, &args[0])?;
            let evt_obj = args[1].clone();

            // get the Map
            let events_listeners_map = get_events_obj_map(q_ctx, &this_ref, event_id.as_str())?;
            if evt_obj.is_object() {
                objects::set_property(ctx, &evt_obj, "target", &this_ref)?;
            }
            let _results = maps::entries_q(q_ctx, &events_listeners_map, |key, _val| {
                let _single_res = functions::call_function(ctx, &key, vec![evt_obj.clone()], None)?;
                // key is a function
                // val is an options obj todo: use it
                // todo check if false was returned or evt_obj.stopImmediatePropagation was called
                // todo in that case we need a way to stop the current iterate loop
                Ok(())
            })?;

            Ok(())
        }
    });
    match res {
        Ok(_) => quickjs_utils::new_null(),
        Err(e) => QuickJsContext::report_ex_ctx(ctx, format!("{}", e).as_str()),
    }
}
