use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, objects, primitives};
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

pub fn new_date(context: *mut q::JSContext) -> Result<JSValueRef, EsError> {
    let constructor = quickjs_utils::get_constructor(context, "Date")?;
    let date_ref = functions::call_constructor(context, &constructor, &[])?;
    Ok(date_ref)
}

pub fn is_date(context: *mut q::JSContext, obj_ref: &JSValueRef) -> Result<bool, EsError> {
    objects::is_instance_of_by_name(context, obj_ref, "Date")
}

pub fn set_time(
    context: *mut q::JSContext,
    date_ref: &JSValueRef,
    timestamp: f64,
) -> Result<(), EsError> {
    functions::invoke_member_function(
        context,
        date_ref,
        "setTime",
        vec![primitives::from_f64(timestamp)],
    )?;
    Ok(())
}

pub fn get_time(context: *mut q::JSContext, date_ref: &JSValueRef) -> Result<f64, EsError> {
    let time_ref = functions::invoke_member_function(context, date_ref, "getTime", vec![])?;
    primitives::to_f64(&time_ref)
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::quickjs_utils::dates;
    use crate::quickjs_utils::dates::{get_time, is_date, set_time};
    use std::sync::Arc;

    #[test]
    fn test_date() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let date_ref = dates::new_date(q_ctx.context)
                .ok()
                .expect("new_date failed");
            assert!(is_date(q_ctx.context, &date_ref)
                .ok()
                .expect("instanceof failed failed"));

            set_time(q_ctx.context, &date_ref, 2147483648f64)
                .ok()
                .expect("could not set time");
            let t = get_time(q_ctx.context, &date_ref)
                .ok()
                .expect("could not get time");
            assert_eq!(t, 2147483648f64);
            set_time(q_ctx.context, &date_ref, 2f64)
                .ok()
                .expect("could not set time");
            let t = get_time(q_ctx.context, &date_ref)
                .ok()
                .expect("could not get time");
            assert_eq!(t, 2f64);
        });
    }
}
