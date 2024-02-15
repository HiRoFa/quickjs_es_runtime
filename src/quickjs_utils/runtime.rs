use libquickjs_sys as q;

/// create new class id
/// # Safety
/// make sure the runtime param is from a live JsRuntimeAdapter instance
pub unsafe fn new_class_id(_runtime: *mut q::JSRuntime) -> u32 {
    let mut c_id: u32 = 0;

    #[cfg(feature = "bellard")]
    let class_id: u32 = q::JS_NewClassID(&mut c_id);

    #[cfg(feature = "quickjs-ng")]
    let class_id: u32 = q::JS_NewClassID(_runtime, &mut c_id);

    log::trace!("got class id {}", class_id);

    class_id
}
