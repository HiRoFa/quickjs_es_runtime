use crate::quickjs_utils::get_constructor;
use crate::quickjs_utils::objects::{
    construct_object, get_property_q, get_prototype_of_q, is_instance_of_by_name_q,
    is_instance_of_q,
};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::valueref::JSValueRef;
use hirofa_utils::auto_id_map::AutoIdMap;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;
use std::cell::RefCell;
use std::slice;

// relevant quickjs bindings functions
// pub type JSFreeArrayBufferDataFunc = ::std::option::Option<
//     unsafe extern "C" fn(
//         rt: *mut JSRuntime,
//         opaque: *mut ::std::os::raw::c_void,
//         ptr: *mut ::std::os::raw::c_void,
//     ),
// >;
// extern "C" {
//     pub fn JS_NewArrayBuffer(
//         ctx: *mut JSContext,
//         buf: *mut u8,
//         len: size_t,
//         free_func: JSFreeArrayBufferDataFunc,
//         opaque: *mut ::std::os::raw::c_void,
//         is_shared: ::std::os::raw::c_int,
//     ) -> JSValue;
// }
// extern "C" {
//     pub fn JS_NewArrayBufferCopy(ctx: *mut JSContext, buf: *const u8, len: size_t) -> JSValue;
// }
// extern "C" {
//     pub fn JS_DetachArrayBuffer(ctx: *mut JSContext, obj: JSValue);
// }
// extern "C" {
//     pub fn JS_GetArrayBuffer(ctx: *mut JSContext, psize: *mut size_t, obj: JSValue) -> *mut u8;
// }
// extern "C" {
//     pub fn JS_GetTypedArrayBuffer(
//         ctx: *mut JSContext,
//         obj: JSValue,
//         pbyte_offset: *mut size_t,
//         pbyte_length: *mut size_t,
//         pbytes_per_element: *mut size_t,
//     ) -> JSValue;
// }

thread_local! {
    static BUFFERS: RefCell<AutoIdMap<Vec<u8>>> = RefCell::new(AutoIdMap::new());
}

/// this method creates a new ArrayBuffer which is used as a basis ofr all typed arrays
/// the buffer vec is stored and used in js, when it is no longer needed it is destroyed
/// todo: we can obtain the buffer without it beeing freed by detacharraybuffer
///
///
pub fn new_array_buffer_q(
    q_ctx: &QuickJsRealmAdapter,
    buf: Vec<u8>,
) -> Result<JSValueRef, JsError> {
    let length = buf.len() as u64;

    let (buffer_id, buffer_ptr) = BUFFERS.with(|rc| {
        let buffers = &mut *rc.borrow_mut();
        let id = buffers.insert(buf);
        let ptr = buffers.get_mut(&id).unwrap().as_mut_ptr();
        (id, ptr)
    });

    let opaque = buffer_id;

    let is_shared = 0;

    let raw = unsafe {
        q::JS_NewArrayBuffer(
            q_ctx.context,
            buffer_ptr,
            length,
            Some(free_func),
            opaque as _,
            is_shared as _,
        )
    };
    let obj_ref = JSValueRef::new(
        q_ctx.context,
        raw,
        false,
        true,
        "typedarrays::new_array_buffer_q",
    );
    if obj_ref.is_exception() {
        return Err(JsError::new_str("Could not create array buffer"));
    }
    Ok(obj_ref)
}

pub fn is_array_buffer_q(q_ctx: &QuickJsRealmAdapter, buf: &JSValueRef) -> bool {
    if buf.is_object() {
        match is_instance_of_by_name_q(q_ctx, buf, "ArrayBuffer") {
            Ok(val) => val,
            Err(_) => false,
        }
    } else {
        false
    }
}

pub fn is_typed_array_q(q_ctx: &QuickJsRealmAdapter, arr: &JSValueRef) -> bool {
    if arr.is_object() {
        unsafe {
            match get_constructor(q_ctx.context, "Uint16Array") {
                Ok(u_int_some_array) => match get_prototype_of_q(q_ctx, &u_int_some_array) {
                    Ok(typed_array) => is_instance_of_q(q_ctx, arr, &typed_array),
                    Err(_) => false,
                },
                Err(_) => false,
            }
        }
    } else {
        false
    }
}

pub fn new_array_buffer_copy_q(
    q_ctx: &QuickJsRealmAdapter,
    buf: &Vec<u8>,
) -> Result<JSValueRef, JsError> {
    let length = buf.len() as u64;
    let raw = unsafe { q::JS_NewArrayBufferCopy(q_ctx.context, buf.as_ptr(), length) };
    let obj_ref = JSValueRef::new(
        q_ctx.context,
        raw,
        false,
        true,
        "typedarrays::new_array_buffer_copy_q",
    );
    if obj_ref.is_exception() {
        return Err(JsError::new_str("Could not create array buffer"));
    }
    Ok(obj_ref)
}
/*
pub fn borrow_array_buffer_buffer_q<'a>(
    _q_ctx: &QuickJsRealmAdapter,
    _buffer: &'a JSValueRef,
) -> &'a Vec<u8> {
    //let mut len: u64 = 0;
    //let ptr = q::JS_GetArrayBuffer(q_ctx.context, &mut len, buffer.borrow_value());
    //unsafe {Vec::from_raw_parts(ptr, len as usize, len as usize)}
    todo!();
}

 */

pub fn get_array_buffer_buffer_q(
    q_ctx: &QuickJsRealmAdapter,
    array_buffer: &JSValueRef,
) -> Vec<u8> {
    assert!(is_array_buffer_q(q_ctx, &array_buffer));

    let mut len: u64 = 0;
    let ptr =
        unsafe { q::JS_GetArrayBuffer(q_ctx.context, &mut len, *array_buffer.borrow_value()) };

    let v = unsafe { slice::from_raw_parts(ptr, len as usize).to_vec() };

    unsafe { q::JS_DetachArrayBuffer(q_ctx.context, *array_buffer.borrow_value()) };

    v
}

pub fn get_array_buffer_q(
    q_ctx: &QuickJsRealmAdapter,
    typed_array: &JSValueRef,
) -> Result<JSValueRef, JsError> {
    //let raw = q::JS_GetTypedArrayBuffer()

    // todo!();

    get_property_q(q_ctx, typed_array, "buffer")
}

pub fn new_uint8_array_q(q_ctx: &QuickJsRealmAdapter, buf: Vec<u8>) -> Result<JSValueRef, JsError> {
    let array_buffer = new_array_buffer_q(q_ctx, buf)?;
    let constructor = unsafe { get_constructor(q_ctx.context, "Uint8Array") }?;
    unsafe { construct_object(q_ctx.context, &constructor, &[&array_buffer]) }
}

pub fn new_uint8_array_copy_q(
    q_ctx: &QuickJsRealmAdapter,
    buf: &Vec<u8>,
) -> Result<JSValueRef, JsError> {
    let array_buffer = new_array_buffer_copy_q(q_ctx, buf)?;
    let constructor = unsafe { get_constructor(q_ctx.context, "Uint8Array") }?;
    unsafe { construct_object(q_ctx.context, &constructor, &[&array_buffer]) }
}

unsafe extern "C" fn free_func(
    _rt: *mut q::JSRuntime,
    opaque: *mut ::std::os::raw::c_void,
    _ptr: *mut ::std::os::raw::c_void,
) {
    let id = opaque as usize;
    log::trace!("free buffer {}", id);
    BUFFERS.with(|rc| {
        let buffers = &mut *rc.borrow_mut();
        if buffers.contains_key(&id) {
            let _buf = buffers.remove(&id);
            log::trace!("dropped buffer");
        } else {
            log::trace!("buffer was already dropped");
        }
    });
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::quickjs_utils::typedarrays::{
        get_array_buffer_buffer_q, get_array_buffer_q, is_array_buffer_q, is_typed_array_q,
        new_array_buffer_q, new_uint8_array_q,
    };
    use hirofa_utils::js_utils::adapters::JsRealmAdapter;
    use hirofa_utils::js_utils::facades::{JsRuntimeBuilder, JsRuntimeFacade};
    use hirofa_utils::js_utils::Script;

    #[test]
    fn test_typed() {
        std::panic::set_hook(Box::new(|panic_info| {
            let backtrace = backtrace::Backtrace::new();
            println!(
                "thread panic occurred: {}\nbacktrace: {:?}",
                panic_info, backtrace
            );
            log::error!(
                "thread panic occurred: {}\nbacktrace: {:?}",
                panic_info,
                backtrace
            );
        }));

        // simple_logging::log_to_stderr(log::LevelFilter::max());

        let rt = QuickJsRuntimeBuilder::new().js_build();

        let res = rt.js_loop_realm_sync(None, |_rt, realm| {
            let obj = realm
                .eval(Script::new(
                    "testu8",
                    "const arr = new Uint8Array(10); arr;",
                ))
                .ok()
                .expect("script failed");

            obj.get_tag()
        });

        log::debug!("tag res {}", res);

        let _res = rt.js_loop_realm_sync(None, |_rt, realm| {
            realm
                .eval(Script::new(
                    "testu8",
                    "globalThis.testTyped = function(typedArray) {console.log('t=%s len=%s 0=%s 1=%s 2=%s', typedArray.constructor.name, typedArray.length, typedArray[0], typedArray[1], typedArray[2]); typedArray[0] = 34;};",
                ))
                .ok()
                .expect("script failed");
        });

        let _blah = rt.js_loop_realm_sync(None, |_rt, realm| {
            let buf = vec![1, 2, 3];
            let buf2 = vec![1, 2, 3];

            let ab_res = new_array_buffer_q(realm, buf);

            match ab_res {
                Ok(ab) => {
                    log::debug!("buffer created, dropping");
                    assert!(is_array_buffer_q(realm, &ab));
                    drop(ab);
                    log::debug!("buffer created, dropped");
                }
                Err(e) => {
                    log::debug!("err: {}", e);
                }
            }

            let arr_res = new_uint8_array_q(realm, buf2);
            match arr_res {
                Ok(mut arr) => {
                    arr.label("arr");
                    log::debug!("arr created");

                    assert!(is_typed_array_q(realm, &arr));

                    realm
                        .js_function_invoke_by_name(&[], "testTyped", &[arr.clone()])
                        .ok()
                        .expect("testTyped failed");

                    let ab = get_array_buffer_q(realm, &arr)
                        .ok()
                        .expect("did not get buffer");

                    log::trace!("reclaiming");
                    let buf2_reclaimed = get_array_buffer_buffer_q(realm, &ab);

                    //unsafe { q::JS_DetachArrayBuffer(realm.context, *arr.borrow_value()) };

                    log::trace!("reclaimed");

                    //realm
                    //    .js_function_invoke_by_name(&[], "testTyped", &[arr.clone()])
                    //    .err()
                    //    .expect("testTyped should have failed");

                    log::trace!("ab dropped");
                    // atm this causes another call to free_buffer, also code above just works ... is detach not working?
                    drop(arr);
                    log::trace!("arr dropped");

                    log::debug!("buf2.len={}", buf2_reclaimed.len());

                    log::debug!(
                        "0={} 1={}, 2={}",
                        buf2_reclaimed.get(0).unwrap(),
                        buf2_reclaimed.get(1).unwrap(),
                        buf2_reclaimed.get(2).unwrap()
                    );
                }
                Err(e) => {
                    log::debug!("err2: {}", e);
                }
            }
        });
    }
}
