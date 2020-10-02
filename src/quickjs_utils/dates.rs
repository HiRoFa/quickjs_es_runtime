use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, objects, primitives};
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;

pub fn new_date(q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
    let constructor = quickjs_utils::get_constructor(q_js_rt, "Date")?;
    let date_ref = functions::call_constructor(q_js_rt, &constructor, &[])?;
    Ok(date_ref)
}

pub fn is_date(q_js_rt: &QuickJsRuntime, obj_ref: &JSValueRef) -> Result<bool, EsError> {
    objects::is_instance_of_by_name(q_js_rt, obj_ref, "Date")
}

pub fn set_time(
    q_js_rt: &QuickJsRuntime,
    date_ref: &JSValueRef,
    timestamp: f64,
) -> Result<(), EsError> {
    functions::invoke_member_function(
        q_js_rt,
        date_ref,
        "setTime",
        &[primitives::from_f64(timestamp)],
    )?;
    Ok(())
}

pub fn get_time(q_js_rt: &QuickJsRuntime, date_ref: &JSValueRef) -> Result<f64, EsError> {
    let time_ref = functions::invoke_member_function(q_js_rt, date_ref, "getTime", &[])?;
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
            let date_ref = dates::new_date(q_js_rt).ok().expect("new_date failed");
            assert!(is_date(q_js_rt, &date_ref)
                .ok()
                .expect("instanceof failed failed"));

            set_time(q_js_rt, &date_ref, 2147483648f64)
                .ok()
                .expect("could not set time");
            let t = get_time(q_js_rt, &date_ref)
                .ok()
                .expect("could not get time");
            assert_eq!(t, 2147483648f64);
            set_time(q_js_rt, &date_ref, 2f64)
                .ok()
                .expect("could not set time");
            let t = get_time(q_js_rt, &date_ref)
                .ok()
                .expect("could not get time");
            assert_eq!(t, 2f64);
        });
    }
}
