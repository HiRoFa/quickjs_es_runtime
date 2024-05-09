//! this module is a work in progress and is currently used by me to pass Vec<u8>'s from rust to js and back again
//!
//!
//!
use crate::jsutils::JsError;
use crate::quickjs_utils::get_constructor;
use crate::quickjs_utils::objects::{
    construct_object, get_property, get_prototype_of, is_instance_of, is_instance_of_by_name,
    set_property2,
};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsvalueadapter::QuickJsValueAdapter;
use hirofa_utils::auto_id_map::AutoIdMap;
use libquickjs_sys as q;
use std::cell::RefCell;

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
    // max size is 32.max because we store id as prop
    pub static BUFFERS: RefCell<AutoIdMap<Vec<u8>>> = RefCell::new(AutoIdMap::new_with_max_size(i32::MAX as usize));
}

/// this method creates a new ArrayBuffer which is used as a basis for all typed arrays
/// the buffer vec is stored and used in js, when it is no longer needed it is dropped
pub fn new_array_buffer_q(
    q_ctx: &QuickJsRealmAdapter,
    buf: Vec<u8>,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { new_array_buffer(q_ctx.context, buf) }
}

/// this method creates a new ArrayBuffer which is used as a basis for all typed arrays
/// the buffer vec is stored and used in js, when it is no longer needed it is dropped
/// # Safety
/// QuickJsRealmAdapter should not be dropped before using this, e.g. the context should still be valid
pub unsafe fn new_array_buffer(
    ctx: *mut q::JSContext,
    buf: Vec<u8>,
) -> Result<QuickJsValueAdapter, JsError> {
    log::trace!("new_array_buffer");

    let length: usize = buf.len();

    let (buffer_id, buffer_ptr) = BUFFERS.with(|rc| {
        let buffers = &mut *rc.borrow_mut();
        let id = buffers.insert(buf);

        let ptr = buffers.get_mut(&id).unwrap().as_mut_ptr();
        (id, ptr)
    });

    let opaque = buffer_id;

    let is_shared = 0;

    let raw = q::JS_NewArrayBuffer(
        ctx,
        buffer_ptr,
        length,
        Some(free_func),
        opaque as _,
        is_shared as _,
    );
    let obj_ref =
        QuickJsValueAdapter::new(ctx, raw, false, true, "typedarrays::new_array_buffer_q");
    if obj_ref.is_exception() {
        return Err(JsError::new_str("Could not create array buffer"));
    }
    let prop_ref = crate::quickjs_utils::primitives::from_i32(buffer_id as i32);
    set_property2(ctx, &obj_ref, "__buffer_id", &prop_ref, 0)?;

    Ok(obj_ref)
}

pub fn is_array_buffer_q(q_ctx: &QuickJsRealmAdapter, buf: &QuickJsValueAdapter) -> bool {
    unsafe { is_array_buffer(q_ctx.context, buf) }
}

/// check if a ref is an ArrayBuffer
/// # Safety
/// please ensure that the relevant QuickjsRealmAdapter is not dropped while using this function or a result of this function
pub unsafe fn is_array_buffer(ctx: *mut q::JSContext, buf: &QuickJsValueAdapter) -> bool {
    buf.is_object() && is_instance_of_by_name(ctx, buf, "ArrayBuffer").unwrap_or(false)
}

pub fn is_typed_array_q(q_ctx: &QuickJsRealmAdapter, arr: &QuickJsValueAdapter) -> bool {
    unsafe { is_typed_array(q_ctx.context, arr) }
}

/// check if a ref is a TypedArray
/// # Safety
/// please ensure that the relevant QuickjsRealmAdapter is not dropped while using this function or a result of this function
pub unsafe fn is_typed_array(ctx: *mut q::JSContext, arr: &QuickJsValueAdapter) -> bool {
    if arr.is_object() {
        match get_constructor(ctx, "Uint16Array") {
            Ok(u_int_some_array) => match get_prototype_of(ctx, &u_int_some_array) {
                Ok(typed_array) => is_instance_of(ctx, arr, &typed_array),
                Err(_) => false,
            },
            Err(_) => false,
        }
    } else {
        false
    }
}

/// create an array buffer with a copy of the data in a Vec
pub fn new_array_buffer_copy_q(
    q_ctx: &QuickJsRealmAdapter,
    buf: &[u8],
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { new_array_buffer_copy(q_ctx.context, buf) }
}

/// create an array buffer with a copy of the data in a Vec
/// # Safety
/// please ensure that the relevant QuickjsRealmAdapter is not dropped while using this function or a result of this function
pub unsafe fn new_array_buffer_copy(
    ctx: *mut q::JSContext,
    buf: &[u8],
) -> Result<QuickJsValueAdapter, JsError> {
    log::trace!("new_array_buffer_copy");

    let length: usize = buf.len();

    let raw = q::JS_NewArrayBufferCopy(ctx, buf.as_ptr(), length);
    let obj_ref = QuickJsValueAdapter::new(
        ctx,
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

/// detach the array buffer and return it, after this the TypedArray is no longer usable in JS (or at least all items will return undefined)
pub fn detach_array_buffer_buffer_q(
    q_ctx: &QuickJsRealmAdapter,
    array_buffer: &QuickJsValueAdapter,
) -> Result<Vec<u8>, JsError> {
    unsafe { detach_array_buffer_buffer(q_ctx.context, array_buffer) }
}

/// detach the array buffer and return it, after this the TypedArray is no longer usable in JS (or at least all items will return undefined)
/// # Safety
/// please ensure that the relevant QuickjsRealmAdapter is not dropped while using this function or a result of this function
pub unsafe fn detach_array_buffer_buffer(
    ctx: *mut q::JSContext,
    array_buffer: &QuickJsValueAdapter,
) -> Result<Vec<u8>, JsError> {
    log::trace!("detach_array_buffer_buffer");

    debug_assert!(is_array_buffer(ctx, array_buffer));

    // check if vec is one we buffered, if not we create a new one from the slice we got from quickjs
    // abuf->opaque seems impossible to get at, so we store the id ourselves as well
    let id_prop = get_property(ctx, array_buffer, "__buffer_id")?;
    let id_opt = if id_prop.is_i32() {
        Some(id_prop.to_i32() as usize)
    } else {
        None
    };

    let v = if let Some(id) = id_opt {
        BUFFERS.with(|rc| {
            let buffers = &mut *rc.borrow_mut();
            buffers.remove(&id)
        })
    } else {
        let mut len: usize = 0;

        let ptr = q::JS_GetArrayBuffer(ctx, &mut len, *array_buffer.borrow_value());

        Vec::from_raw_parts(ptr, len as _, len as _)
    };

    q::JS_DetachArrayBuffer(ctx, *array_buffer.borrow_value());

    Ok(v)
}

/// Get a copy of the underlying array buffer and return it
/// unlike when using detach_array_buffer_buffer_q the TypedArray is still intact after using this
/// the operation is just more expensive because the Vec is cloned
pub fn get_array_buffer_buffer_copy_q(
    q_ctx: &QuickJsRealmAdapter,
    array_buffer: &QuickJsValueAdapter,
) -> Result<Vec<u8>, JsError> {
    unsafe { get_array_buffer_buffer_copy(q_ctx.context, array_buffer) }
}

/// Get a copy of the underlying array buffer and return it
/// unlike when using detach_array_buffer_buffer_q the TypedArray is still intact after using this
/// the operation is just more expensive because the Vec is cloned
/// # Safety
/// please ensure that the relevant QuickjsRealmAdapter is not dropped while using this function or a result of this function
pub unsafe fn get_array_buffer_buffer_copy(
    ctx: *mut q::JSContext,
    array_buffer: &QuickJsValueAdapter,
) -> Result<Vec<u8>, JsError> {
    debug_assert!(is_array_buffer(ctx, array_buffer));

    log::trace!("get_array_buffer_buffer_copy");

    let id_prop = get_property(ctx, array_buffer, "__buffer_id")?;
    let id_opt = if id_prop.is_i32() {
        Some(id_prop.to_i32() as usize)
    } else {
        None
    };

    if let Some(id) = id_opt {
        let b = BUFFERS.with(|rc| {
            let buffers = &mut *rc.borrow_mut();
            buffers.get(&id).expect("invalid buffer state").clone()
        });
        Ok(b)
    } else {
        let mut len: usize = 0;

        let ptr = q::JS_GetArrayBuffer(ctx, &mut len, *array_buffer.borrow_value());

        let slice = std::slice::from_raw_parts(ptr, len as _);

        Ok(slice.to_vec())
    }
}

/// get the underlying ArrayBuffer of a TypedArray
pub fn get_array_buffer_q(
    q_ctx: &QuickJsRealmAdapter,
    typed_array: &QuickJsValueAdapter,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { get_array_buffer(q_ctx.context, typed_array) }
}

/// get the underlying ArrayBuffer of a TypedArray
/// # Safety
/// please ensure that the relevant QuickjsRealmAdapter is not dropped while using this function or a result of this function
pub unsafe fn get_array_buffer(
    ctx: *mut q::JSContext,
    typed_array: &QuickJsValueAdapter,
) -> Result<QuickJsValueAdapter, JsError> {
    debug_assert!(is_typed_array(ctx, typed_array));
    // this is probably needed later for different typed arrays
    //let raw = q::JS_GetTypedArrayBuffer()

    // todo!();
    // for our Uint8Array uses cases this works fine
    log::trace!("get_array_buffer");
    get_property(ctx, typed_array, "buffer")
}

/// create a new TypedArray with a buffer, the buffer is consumed and can be reclaimed later by calling detach_array_buffer_buffer_q
pub fn new_uint8_array_q(
    q_ctx: &QuickJsRealmAdapter,
    buf: Vec<u8>,
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { new_uint8_array(q_ctx.context, buf) }
}

/// create a new TypedArray with a buffer, the buffer is consumed and can be reclaimed later by calling detach_array_buffer_buffer_q
/// # Safety
/// please ensure that the relevant QuickjsRealmAdapter is not dropped while using this function or a result of this function
pub unsafe fn new_uint8_array(
    ctx: *mut q::JSContext,
    buf: Vec<u8>,
) -> Result<QuickJsValueAdapter, JsError> {
    let array_buffer = new_array_buffer(ctx, buf)?;
    let constructor = get_constructor(ctx, "Uint8Array")?;
    construct_object(ctx, &constructor, &[&array_buffer])
}

/// create a new TypedArray with a buffer, the buffer is copied and that copy can be reclaimed later by calling detach_array_buffer_buffer_q
pub fn new_uint8_array_copy_q(
    q_ctx: &QuickJsRealmAdapter,
    buf: &[u8],
) -> Result<QuickJsValueAdapter, JsError> {
    unsafe { new_uint8_array_copy(q_ctx.context, buf) }
}

/// create a new TypedArray with a buffer, the buffer is copied and that copy can be reclaimed later by calling detach_array_buffer_buffer_q
/// # Safety
/// please ensure that the relevant QuickjsRealmAdapter is not dropped while using this function or a result of this function
pub unsafe fn new_uint8_array_copy(
    ctx: *mut q::JSContext,
    buf: &[u8],
) -> Result<QuickJsValueAdapter, JsError> {
    let array_buffer = new_array_buffer_copy(ctx, buf)?;
    let constructor = get_constructor(ctx, "Uint8Array")?;
    construct_object(ctx, &constructor, &[&array_buffer])
}

unsafe extern "C" fn free_func(
    _rt: *mut q::JSRuntime,
    opaque: *mut ::std::os::raw::c_void,
    ptr: *mut ::std::os::raw::c_void,
) {
    let id = opaque as usize;
    log::trace!("typedarrays::free_func {}", id);

    if ptr.is_null() {
        return;
    }

    BUFFERS.with(|rc| {
        let buffers = &mut *rc.borrow_mut();
        if buffers.contains_key(&id) {
            let _buf = buffers.remove(&id);
        }
    });
}

#[cfg(test)]
pub mod tests {
    use crate::jsutils::{JsError, Script};
    use crate::quickjs_utils::typedarrays::{
        detach_array_buffer_buffer_q, get_array_buffer_buffer_copy_q, get_array_buffer_q,
        is_array_buffer_q, is_typed_array_q, new_array_buffer_q, new_uint8_array_copy_q,
        new_uint8_array_q,
    };
    use crate::values::{JsValueFacade, TypedArrayType};

    use crate::facades::tests::init_test_rt;
    use crate::quickjs_utils::objects::set_property_q;
    use crate::quickjs_utils::{get_global_q, new_undefined_ref};
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_typed() {
        let rt = init_test_rt();

        let res: Result<(), JsError> = rt.exe_rt_task_in_event_loop(|rt| {
            let mut buffer: Vec<u8> = vec![];
            for y in 0..(10 * 1024 * 1024) {
                buffer.push(y as u8);
            }

            let realm = rt.get_main_realm();
            let adapter = new_uint8_array_copy_q(realm, &buffer)?;
            drop(buffer);
            let global = get_global_q(realm);
            set_property_q(realm, &global, "buf", &adapter)?;

            drop(adapter);

            realm.eval(Script::new(
                "l.js",
                "console.log(`buf l=%s 1=%s`, globalThis.buf.length, globalThis.buf[1]);",
            ))?;

            set_property_q(realm, &global, "buf", &new_undefined_ref())?;

            rt.gc();

            Ok(())
        });
        match res {
            Ok(_) => {
                log::info!("done ok");
            }
            Err(err) => {
                panic!("{err}");
            }
        }
    }

    #[test]
    fn test_typed2() {
        let rt = init_test_rt();

        let res = rt.loop_realm_sync(None, |_rt, realm| {
            let obj = realm
                .eval(Script::new(
                    "testu8",
                    "const arr = new Uint8Array(10); arr;",
                ))
                .expect("script failed");

            obj.get_tag()
        });

        log::debug!("tag res {}", res);

        rt.loop_realm_sync(None, |_rt, realm| {
            realm
                .eval(Script::new(
                    "testu8",
                    "globalThis.testTyped = function(typedArray) {console.log('t=%s len=%s 0=%s 1=%s 2=%s', typedArray.constructor.name, typedArray.length, typedArray[0], typedArray[1], typedArray[2]); typedArray[0] = 34; const ret = []; for (let x = 0; x < 1024; x++) {ret.push(x);}; return new Uint8Array(ret);};",
                ))
                .expect("script failed");
        });

        rt.loop_realm_sync(None, |_rt, realm| {
            let buf = vec![1, 2, 3];

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

            let mut buf2 = vec![];
            for x in 0..(1024 * 1024) {
                buf2.push(x as u8);
            }

            let arr_res = new_uint8_array_q(realm, buf2);
            match arr_res {
                Ok(mut arr) => {
                    arr.label("arr");
                    log::debug!("arr created, refcount={}", arr.get_ref_count());

                    assert!(is_typed_array_q(realm, &arr));

                    realm
                        .invoke_function_by_name(&[], "testTyped", &[arr.clone()])
                        .expect("testTyped failed");

                    let ab = get_array_buffer_q(realm, &arr).expect("did not get buffer");

                    let copy = get_array_buffer_buffer_copy_q(realm, &ab).expect("could not copy");
                    log::info!("copy.len = {}", copy.len());

                    log::trace!("reclaiming");
                    let buf2_reclaimed =
                        detach_array_buffer_buffer_q(realm, &ab).expect("detach failed");

                    //unsafe { q::JS_DetachArrayBuffer(realm.context, *arr.borrow_value()) };

                    log::trace!("reclaimed");

                    // this still works but all values should be undefined..
                    realm
                        .invoke_function_by_name(&[], "testTyped", &[arr.clone()])
                        .expect("script failed");

                    log::trace!("ab dropped");
                    // atm this causes another call to free_buffer, also code above just works ... is detach not working?
                    drop(arr);
                    log::trace!("arr dropped");

                    log::debug!("buf2.len={}", buf2_reclaimed.len());

                    log::debug!(
                        "0={} 1={}, 2={}",
                        buf2_reclaimed.first().unwrap(),
                        buf2_reclaimed.get(1).unwrap(),
                        buf2_reclaimed.get(2).unwrap()
                    );
                }
                Err(e) => {
                    log::debug!("err2: {}", e);
                }
            }
        });

        thread::sleep(Duration::from_secs(1));

        for x in 0..10 {
            let mut buffer2: Vec<u8> = vec![];
            for y in 0..(10 * 1024 * 1024) {
                buffer2.push(y as u8);
            }
            let ta = JsValueFacade::TypedArray {
                buffer: buffer2,
                array_type: TypedArrayType::Uint8,
            };
            let res = rt.invoke_function_sync(None, &[], "testTyped", vec![ta]);
            match res {
                Ok(r) => {
                    log::info!("from jsvf got {:?}", r);
                }
                Err(e) => {
                    panic!("{}", e);
                }
            }
            log::info!("x={x}");
        }

        drop(rt);

        crate::quickjs_utils::typedarrays::BUFFERS.with(|rc| {
            let buffers = &mut *rc.borrow_mut();
            log::info!("BUFFERS.len = {}", buffers.len());
        });

        thread::sleep(Duration::from_secs(1));
    }
}
