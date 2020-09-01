use crate::eserror::EsError;
use crate::quickjsruntime::{
    OwnedValueRef, QuickJsRuntime, TAG_BOOL, TAG_FLOAT64, TAG_INT, TAG_NULL, TAG_OBJECT,
    TAG_UNDEFINED,
};
use std::collections::HashMap;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

pub trait EsValueConvertible {
    fn to_js_value(&self, q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError>;

    fn to_es_value_facade(self) -> EsValueFacade
    where
        Self: Sized + Send + 'static,
    {
        EsValueFacade {
            convertible: Box::new(self),
        }
    }

    fn is_null(&self) -> bool {
        false
    }

    fn is_undefined(&self) -> bool {
        false
    }

    fn is_bool(&self) -> bool {
        false
    }
    fn get_bool(&self) -> bool {
        panic!("i am not a boolean");
    }
    fn is_str(&self) -> bool {
        false
    }
    fn get_str(&self) -> &str {
        panic!("i am not a string");
    }
    fn is_i32(&self) -> bool {
        false
    }
    fn get_i32(&self) -> i32 {
        panic!("i am not an i32");
    }
    fn is_f64(&self) -> bool {
        false
    }
    fn get_f64(&self) -> f64 {
        panic!("i am not an f64");
    }
    fn is_function(&self) -> bool {
        false
    }
    fn invoke_function(&self, _args: Vec<EsValueFacade>) -> Result<EsValueFacade, EsError> {
        panic!("i am not a function");
    }
    fn is_promise(&self) -> bool {
        false
    }
    fn await_promise_blocking(
        &self,
        _timeout: Duration,
    ) -> Result<Result<EsValueFacade, EsValueFacade>, RecvTimeoutError> {
        panic!("i am not a promise");
    }
    fn is_object(&self) -> bool {
        false
    }
    fn get_object(&self) -> &HashMap<String, EsValueFacade> {
        panic!("i am not an object");
    }
    fn is_array(&self) -> bool {
        false
    }
    fn get_array(&self) -> &Vec<EsValueFacade> {
        panic!("i am not an array");
    }
}

struct EsUndefinedValue {}
struct EsNullValue {}

impl EsValueConvertible for EsNullValue {
    fn to_js_value(&self, _q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        Ok(crate::quickjs_utils::new_null())
    }
}

impl EsValueConvertible for EsUndefinedValue {
    fn to_js_value(&self, _q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        Ok(crate::quickjs_utils::new_undefined())
    }
}

impl EsValueConvertible for String {
    fn to_js_value(&self, q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        crate::quickjs_utils::primitives::from_string(q_js_rt, self.as_str())
    }

    fn is_str(&self) -> bool {
        true
    }

    fn get_str(&self) -> &str {
        self.as_str()
    }
}

impl EsValueConvertible for i32 {
    fn to_js_value(&self, _q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        Ok(crate::quickjs_utils::primitives::from_i32(*self))
    }

    fn is_i32(&self) -> bool {
        true
    }

    fn get_i32(&self) -> i32 {
        *self
    }
}

impl EsValueConvertible for bool {
    fn to_js_value(&self, _q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        Ok(crate::quickjs_utils::primitives::from_bool(*self))
    }

    fn is_bool(&self) -> bool {
        true
    }

    fn get_bool(&self) -> bool {
        *self
    }
}

impl EsValueConvertible for f64 {
    fn to_js_value(&self, _q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        Ok(crate::quickjs_utils::primitives::from_f64(*self))
    }
    fn is_f64(&self) -> bool {
        true
    }

    fn get_f64(&self) -> f64 {
        *self
    }
}

impl EsValueConvertible for Vec<EsValueFacade> {
    fn to_js_value(&self, q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        // create the array

        let arr = crate::quickjs_utils::arrays::create_array(q_js_rt)
            .ok()
            .unwrap();

        // add items
        for index in 0..self.len() {
            let item = self.get(index).unwrap();

            let item_val_ref = item.to_js_value(q_js_rt)?;

            crate::quickjs_utils::arrays::set_element(q_js_rt, &arr, index as u32, &item_val_ref)?;
        }
        Ok(arr)
    }

    fn is_array(&self) -> bool {
        true
    }

    fn get_array(&self) -> &Vec<EsValueFacade> {
        self
    }
}

impl EsValueConvertible for HashMap<String, EsValueFacade> {
    fn to_js_value(&self, q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        // create new obj
        let obj_ref = crate::quickjs_utils::objects::create_object(q_js_rt)
            .ok()
            .unwrap();

        for prop in self {
            let prop_name = prop.0;
            let prop_esvf = prop.1;

            // set prop in obj

            let property_value_ref = prop_esvf.to_js_value(q_js_rt)?;

            crate::quickjs_utils::objects::set_property(
                q_js_rt,
                &obj_ref,
                prop_name.as_str(),
                &property_value_ref,
            )?;
        }

        Ok(obj_ref)
    }

    fn is_object(&self) -> bool {
        true
    }

    fn get_object(&self) -> &HashMap<String, EsValueFacade> {
        self
    }
}

pub struct EsValueFacade {
    convertible: Box<dyn EsValueConvertible + Send + 'static>,
}

impl EsValueFacade {
    fn to_js_value(&self, q_js_rt: &QuickJsRuntime) -> Result<OwnedValueRef, EsError> {
        self.convertible.to_js_value(q_js_rt)
    }

    pub(crate) fn from_jsval(
        q_js_rt: &QuickJsRuntime,
        value_ref: &OwnedValueRef,
    ) -> Result<Self, EsError> {
        let r = &value_ref.value;

        let res = match r.tag {
            // Int.
            TAG_INT => {
                let val: i32 = crate::quickjs_utils::primitives::to_i32(value_ref)
                    .ok()
                    .expect("could not convert to i32");
                Ok(val.to_es_value_facade())
            }
            // Bool.
            TAG_BOOL => {
                let val: bool = crate::quickjs_utils::primitives::to_bool(value_ref)
                    .ok()
                    .expect("could not convert to bool");
                Ok(val.to_es_value_facade())
            }
            // Null.
            TAG_NULL => Ok(EsNullValue {}.to_es_value_facade()),
            // Undefined.
            TAG_UNDEFINED => Ok(EsUndefinedValue {}.to_es_value_facade()),
            // Float.
            TAG_FLOAT64 => {
                let val: f64 = crate::quickjs_utils::primitives::to_f64(value_ref)
                    .ok()
                    .expect("could not convert to f64");
                Ok(val.to_es_value_facade())
            }
            // String.
            TAG_STRING => {
                let s = crate::quickjs_utils::primitives::to_string(q_js_rt, value_ref)
                    .ok()
                    .expect("failed to convert to string");

                Ok(s.to_es_value_facade())
            }
            // Object.
            TAG_OBJECT => {
                let is_array = crate::quickjs_utils::arrays::is_array(q_js_rt, value_ref);
                if is_array {
                    Self::from_jsval_array(q_js_rt, value_ref)
                } else {
                    #[cfg(feature = "chrono")]
                    {
                        use chrono::offset::TimeZone;

                        let date_constructor = js_date_constructor(context);
                        let is_date = unsafe {
                            q::JS_IsInstanceOf(q_js_rt.context, *r, date_constructor) > 0
                        };

                        if is_date {
                            let getter = unsafe {
                                q::JS_GetPropertyStr(
                                    q_js_rt.context,
                                    *r,
                                    std::ffi::CStr::from_bytes_with_nul(b"getTime\0")
                                        .unwrap()
                                        .as_ptr(),
                                )
                            };
                            assert_eq!(getter.tag, TAG_OBJECT);

                            let timestamp_raw =
                                unsafe { q::JS_Call(context, getter, *r, 0, std::ptr::null_mut()) };
                            unsafe {
                                free_value(q_js_rt.context, getter);
                                free_value(q_js_rt.context, date_constructor);
                            };

                            let res = if timestamp_raw.tag != TAG_FLOAT64 {
                                Err(ValueError::Internal(
                                    "Could not convert 'Date' instance to timestamp".into(),
                                ))
                            } else {
                                let f = unsafe { timestamp_raw.u.float64 } as i64;
                                let datetime = chrono::Utc.timestamp_millis(f);
                                Ok(JsValue::Date(datetime))
                            };
                            return res;
                        } else {
                            unsafe { free_value(q_js_rt.context, date_constructor) };
                        }
                    }

                    Self::from_jsval_object(q_js_rt, value_ref)
                }
            }
            // BigInt
            #[cfg(feature = "bigint")]
            TAG_BIG_INT => {
                let mut int: i64 = 0;
                let ret = unsafe { q::JS_ToBigInt64(context, &mut int, *r) };
                if ret == 0 {
                    Ok(JsValue::BigInt(BigInt {
                        inner: BigIntOrI64::Int(int),
                    }))
                } else {
                    let ptr = unsafe { q::JS_ToCStringLen2(context, std::ptr::null_mut(), *r, 0) };

                    if ptr.is_null() {
                        return Err(ValueError::Internal(
                            "Could not convert BigInt to string: got a null pointer".into(),
                        ));
                    }

                    let cstr = unsafe { std::ffi::CStr::from_ptr(ptr) };
                    let bigint = num_bigint::BigInt::parse_bytes(cstr.to_bytes(), 10).unwrap();

                    // Free the c string.
                    unsafe { q::JS_FreeCString(context, ptr) };

                    Ok(JsValue::BigInt(BigInt {
                        inner: BigIntOrI64::BigInt(bigint),
                    }))
                }
            }
            x => Err(EsError::new_string(format!(
                "Unhandled JS_TAG value: {}",
                x
            ))),
        };

        res
    }

    fn from_jsval_array(
        q_js_rt: &QuickJsRuntime,
        value_ref: &OwnedValueRef,
    ) -> Result<EsValueFacade, EsError> {
        assert!(value_ref.is_object());

        let len = crate::quickjs_utils::arrays::get_length(q_js_rt, value_ref)?;

        let mut values = Vec::new();
        for index in 0..len {
            let element_ref = crate::quickjs_utils::arrays::get_element(q_js_rt, value_ref, index)?;

            let element_value = EsValueFacade::from_jsval(q_js_rt, &element_ref)?;

            values.push(element_value);
        }

        Ok(values.to_es_value_facade())
    }

    fn from_jsval_object(
        q_js_rt: &QuickJsRuntime,
        obj_ref: &OwnedValueRef,
    ) -> Result<EsValueFacade, EsError> {
        assert!(obj_ref.is_object());

        let mut map: HashMap<String, EsValueFacade> = HashMap::new();
        crate::quickjs_utils::objects::traverse_properties(q_js_rt, obj_ref, |key, val| {
            map.insert(key.to_string(), EsValueFacade::from_jsval(q_js_rt, &val)?);
            Ok(())
        });
        Ok(map.to_es_value_facade())
    }
    /// get the String value
    pub fn get_str(&self) -> &str {
        self.convertible.get_str()
    }

    /// get the i32 value
    pub fn get_i32(&self) -> i32 {
        self.convertible.get_i32()
    }

    /// get the f64 value
    pub fn get_f64(&self) -> f64 {
        self.convertible.get_f64()
    }

    /// get the boolean value
    pub fn get_boolean(&self) -> bool {
        self.convertible.get_bool()
    }

    /// check if the value is a String
    pub fn is_string(&self) -> bool {
        self.convertible.is_str()
    }

    /// check if the value is a i32
    pub fn is_i32(&self) -> bool {
        self.convertible.is_i32()
    }

    /// check if the value is a f64
    pub fn is_f64(&self) -> bool {
        self.convertible.is_f64()
    }

    /// check if the value is a bool
    pub fn is_boolean(&self) -> bool {
        self.convertible.is_bool()
    }

    /// check if the value is an object
    pub fn is_object(&self) -> bool {
        self.convertible.is_object()
    }

    /// check if the value is an array
    pub fn is_array(&self) -> bool {
        self.convertible.is_array()
    }

    /// check if the value is an function
    pub fn is_function(&self) -> bool {
        self.convertible.is_function()
    }
}
