use crate::quickjs_utils::get_constructor;
use crate::quickjs_utils::objects::construct_object;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::valueref::JSValueRef;
use hirofa_utils::auto_id_map::AutoIdMap;
use hirofa_utils::js_utils::JsError;
use libquickjs_sys as q;
use std::cell::RefCell;
use std::sync::Arc;

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

pub fn new_array_buffer_arc_q(
    _q_ctx: &QuickJsRealmAdapter,
    _buf: Arc<Vec<u8>>,
) -> Result<JSValueRef, JsError> {
    //unsafe { q::JS_NewArrayBuffer(q_ctx.context, buf, length, free_func, opaque, is_shared) }
    todo!();
}

pub fn borrow_array_buffer_buffer_q<'a>(
    _q_ctx: &QuickJsRealmAdapter,
    _buffer: &'a JSValueRef,
) -> &'a Vec<u8> {
    //let mut len: u64 = 0;
    //let ptr = q::JS_GetArrayBuffer(q_ctx.context, &mut len, buffer.borrow_value());
    //unsafe {Vec::from_raw_parts(ptr, len as usize, len as usize)}
    todo!();
}

pub fn detach_array_buffer_buffer_q(_q_ctx: &QuickJsRealmAdapter, _buffer: &JSValueRef) -> Vec<u8> {
    //JS_DetachArrayBuffer
    todo!();
}

pub fn get_array_buffer_q(
    _q_ctx: &QuickJsRealmAdapter,
    _typed_array: &JSValueRef,
) -> Result<JSValueRef, JsError> {
    // let raw = q::JS_GetTypedArrayBuffer()
    todo!();
}

pub fn new_uint8_array_q(q_ctx: &QuickJsRealmAdapter, buf: Vec<u8>) -> Result<JSValueRef, JsError> {
    let array_buffer = new_array_buffer_q(q_ctx, buf)?;
    let constructor = unsafe { get_constructor(q_ctx.context, "Uint8Array") }?;
    unsafe { construct_object(q_ctx.context, &constructor, &[&array_buffer]) }
}

pub fn new_uint8_array_copy_q(
    _q_ctx: &QuickJsRealmAdapter,
    _buf: &Vec<u8>,
) -> Result<JSValueRef, JsError> {
    //unsafe { q::JS_NewArrayBuffer(q_ctx.context, buf, length, free_func, opaque, is_shared) }
    todo!();
}

unsafe extern "C" fn free_func(
    _rt: *mut q::JSRuntime,
    opaque: *mut ::std::os::raw::c_void,
    _ptr: *mut ::std::os::raw::c_void,
) {
    let id = opaque as usize;
    println!("free buffer {}", id);
    BUFFERS.with(|rc| {
        let buffers = &mut *rc.borrow_mut();
        let _buf = buffers.remove(&id);
    });
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::quickjs_utils::typedarrays::{new_array_buffer_q, new_uint8_array_q};
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

        println!("{}", res);

        let _res = rt.js_loop_realm_sync(None, |_rt, realm| {
            realm
                .eval(Script::new(
                    "testu8",
                    "globalThis.testTyped = function(typedArray) {console.log('t=%s len=%s 0=%s', typedArray.constructor.name, typedArray.length, typedArray[0])};",
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
                    println!("buffer created, dropping");
                    drop(ab);
                    println!("buffer created, dropped");
                }
                Err(e) => {
                    println!("err: {}", e);
                }
            }

            let arr_res = new_uint8_array_q(realm, buf2);
            match arr_res {
                Ok(arr) => {
                    println!("arr created, dropping");

                    realm
                        .js_function_invoke_by_name(&[], "testTyped", &[arr.clone()])
                        .ok()
                        .expect("testTyped failed");

                    drop(arr);
                    println!("arr created, dropped");
                }
                Err(e) => {
                    println!("err2: {}", e);
                }
            }
        });
    }
}
