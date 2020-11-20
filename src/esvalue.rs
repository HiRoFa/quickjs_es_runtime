use crate::eserror::EsError;
use crate::esruntime::EsRuntime;
use crate::esruntime_utils::promises::new_resolving_promise;
use crate::quickjs_utils;
use crate::quickjs_utils::{arrays, dates, functions, new_null_ref, promises};
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::*;
use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::rc::Rc;
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::sync::{Arc, Weak};
use std::time::Duration;

pub type PromiseReactionType =
    Option<Box<dyn Fn(EsValueFacade) -> Result<EsValueFacade, EsError> + Send + 'static>>;

pub trait EsValueConvertible {
    fn to_js_value(&mut self, q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError>;

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
    fn invoke_function_sync(&self, _args: Vec<EsValueFacade>) -> Result<EsValueFacade, EsError> {
        panic!("i am not a function");
    }
    fn invoke_function(&self, _args: Vec<EsValueFacade>) -> Result<(), EsError> {
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
    fn add_promise_reactions(
        &self,
        _es_rt: &EsRuntime,
        _then: PromiseReactionType,
        _catch: PromiseReactionType,
        _finally: Option<Box<dyn Fn() + Send + 'static>>,
    ) -> Result<(), EsError> {
        panic!("i am not a promise")
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

pub struct EsUndefinedValue {}
pub struct EsNullValue {}

impl EsValueConvertible for EsNullValue {
    fn to_js_value(&mut self, _q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
        Ok(crate::quickjs_utils::new_null_ref())
    }
}

impl EsValueConvertible for EsUndefinedValue {
    fn to_js_value(&mut self, _q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
        Ok(crate::quickjs_utils::new_undefined_ref())
    }
}

// placeholder for promises that were passed from the script engine to rust
struct CachedJSPromise {
    cached_obj_id: i32,
    es_rt: Weak<EsRuntime>,
}

impl Drop for CachedJSPromise {
    fn drop(&mut self) {
        if let Some(rt_arc) = self.es_rt.upgrade() {
            let cached_obj_id = self.cached_obj_id;

            rt_arc.add_to_event_queue(move |q_js_rt| {
                q_js_rt.consume_cached_obj(cached_obj_id);
            });
        }
    }
}

// placeholder for functions that were passed from the script engine to rust
struct CachedJSFunction {
    cached_obj_id: i32,
    es_rt: Weak<EsRuntime>,
}

impl Drop for CachedJSFunction {
    fn drop(&mut self) {
        if let Some(rt_arc) = self.es_rt.upgrade() {
            let cached_obj_id = self.cached_obj_id;

            rt_arc.add_to_event_queue(move |q_js_rt| {
                q_js_rt.consume_cached_obj(cached_obj_id);
            });
        }
    }
}

impl EsValueConvertible for CachedJSPromise {
    fn to_js_value(&mut self, q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
        let cloned_ref = q_js_rt.with_cached_obj(self.cached_obj_id, |obj_ref| obj_ref.clone());
        Ok(cloned_ref)
    }

    fn is_promise(&self) -> bool {
        true
    }

    fn await_promise_blocking(
        &self,
        timeout: Duration,
    ) -> Result<Result<EsValueFacade, EsValueFacade>, RecvTimeoutError> {
        let (tx, rx) = channel();
        let cached_obj_id = self.cached_obj_id;
        if let Some(es_rti) = self.es_rt.upgrade() {
            es_rti.add_to_event_queue_sync(move |q_js_rt| {
                q_js_rt.with_cached_obj(cached_obj_id, move |prom_obj_ref| {
                    QuickJsRuntime::do_with(move |q_js_rt| {
                        let tx2 = tx.clone();
                        let then_func_ref = functions::new_function(
                            q_js_rt,
                            "promise_then_result_transmitter",
                            move |_this_ref, args| {
                                // these clones are needed because create_func requires a Fn and not a FnOnce
                                // in practice however the Fn is called only once
                                let tx3 = tx2.clone();

                                QuickJsRuntime::do_with(move |q_js_rt| {
                                    let prom_res = &args[0];
                                    let prom_res_esvf_res =
                                        EsValueFacade::from_jsval(q_js_rt, prom_res);

                                    match prom_res_esvf_res {
                                        Ok(prom_res_esvf) => {
                                            let send_res = tx3.send(Ok(prom_res_esvf));
                                            match send_res {
                                                Ok(_) => {
                                                    log::trace!("sent prom_res_esvf ok");
                                                }
                                                Err(e) => {
                                                    log::error!("send prom_res_esvf failed: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "could not convert promise result to esvf {}",
                                                e
                                            );
                                            panic!(
                                                "could not convert promise result to esvf {}",
                                                e
                                            );
                                        }
                                    }

                                    Ok(new_null_ref())
                                })
                            },
                            1,
                        )
                        .ok()
                        .expect("could not create func");

                        let tx2 = tx.clone();
                        let catch_func_ref = functions::new_function(
                            q_js_rt,
                            "promise_catch_result_transmitter",
                            move |_this_ref, args| {
                                // these clones are needed because create_func requires a Fn and not a FnOnce
                                // in practice however the Fn is called only once
                                let tx3 = tx2.clone();

                                QuickJsRuntime::do_with(move |q_js_rt| {
                                    let prom_res = &args[0];
                                    let prom_res_esvf_res =
                                        EsValueFacade::from_jsval(q_js_rt, prom_res);
                                    match prom_res_esvf_res {
                                        Ok(prom_res_esvf) => {
                                            let send_res = tx3.send(Err(prom_res_esvf));
                                            match send_res {
                                                Ok(_) => {
                                                    log::trace!("sent prom_res_esvf ok");
                                                }
                                                Err(e) => {
                                                    log::error!("send prom_res_esvf failed: {}", e);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::error!(
                                                "could not convert promise result to esvf {}",
                                                e
                                            );
                                            panic!(
                                                "could not convert promise result to esvf {}",
                                                e
                                            );
                                        }
                                    }
                                    Ok(new_null_ref())
                                })
                            },
                            1,
                        )
                        .ok()
                        .expect("could not create func");

                        promises::add_promise_reactions(
                            q_js_rt,
                            prom_obj_ref,
                            Some(then_func_ref),
                            Some(catch_func_ref),
                            None,
                        )
                        .ok()
                        .expect("could not create promise reactions");
                    })
                });
            });
            Ok(rx.recv_timeout(timeout)?)
        } else {
            Ok(Err("rti dropped".to_string().to_es_value_facade()))
        }
    }

    fn add_promise_reactions(
        &self,
        es_rt: &EsRuntime,
        then: PromiseReactionType,
        catch: PromiseReactionType,
        finally: Option<Box<dyn Fn() + Send + 'static>>,
    ) -> Result<(), EsError> {
        let cached_obj_id = self.cached_obj_id;
        es_rt.add_to_event_queue(move |q_js_rt| {
            q_js_rt.with_cached_obj(cached_obj_id, move |prom_ref| {
                let then_ref = if let Some(then_fn) = then {
                    let then_fn_rc = Rc::new(then_fn);
                    let then_fn_raw = move |_this_ref, mut args_ref: Vec<JSValueRef>| {
                        let then_fn_rc = then_fn_rc.clone();
                        let val_ref = args_ref.remove(0);

                        QuickJsRuntime::do_with(move |q_js_rt| {
                            let val_esvf = EsValueFacade::from_jsval(q_js_rt, &val_ref)
                                .ok()
                                .expect("could not convert val to esvf");

                            then_fn_rc(val_esvf).ok().expect("then failed");
                            Ok(crate::quickjs_utils::new_null_ref())
                        })
                    };
                    let t = functions::new_function(q_js_rt, "", then_fn_raw, 1)
                        .ok()
                        .expect("could not create function");
                    Some(t)
                } else {
                    None
                };

                let catch_ref = if let Some(catch_fn) = catch {
                    let catch_fn_rc = Rc::new(catch_fn);
                    let catch_fn_raw = move |_this_ref, mut args_ref: Vec<JSValueRef>| {
                        let val_ref = args_ref.remove(0);
                        let catch_fn_rc = catch_fn_rc.clone();
                        QuickJsRuntime::do_with(move |q_js_rt| {
                            let val_esvf = EsValueFacade::from_jsval(q_js_rt, &val_ref)
                                .ok()
                                .expect("could not convert val to esvf");

                            catch_fn_rc(val_esvf).ok().expect("catch failed");
                            Ok(crate::quickjs_utils::new_null_ref())
                        })
                    };
                    let t = functions::new_function(q_js_rt, "", catch_fn_raw, 1)
                        .ok()
                        .expect("could not create function");
                    Some(t)
                } else {
                    None
                };

                let finally_ref = if let Some(finally_fn) = finally {
                    let finally_fn_raw = move |_this_ref, _args_ref| {
                        finally_fn();
                        Ok(crate::quickjs_utils::new_null_ref())
                    };
                    let t = functions::new_function(q_js_rt, "", finally_fn_raw, 0)
                        .ok()
                        .expect("could not create function");
                    Some(t)
                } else {
                    None
                };

                promises::add_promise_reactions(q_js_rt, prom_ref, then_ref, catch_ref, finally_ref)
                    .ok()
                    .expect("could not add reactions")
            });
        });
        Ok(())
    }
}

impl EsValueConvertible for CachedJSFunction {
    fn to_js_value(&mut self, q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
        let cloned_ref = q_js_rt.with_cached_obj(self.cached_obj_id, |obj_ref| obj_ref.clone());
        Ok(cloned_ref)
    }

    fn is_function(&self) -> bool {
        true
    }

    fn invoke_function_sync(&self, mut args: Vec<EsValueFacade>) -> Result<EsValueFacade, EsError> {
        let cached_obj_id = self.cached_obj_id;
        if let Some(rt_arc) = self.es_rt.upgrade() {
            rt_arc.add_to_event_queue_sync(move |q_js_rt| {
                q_js_rt.with_cached_obj(cached_obj_id, move |obj_ref| {
                    let mut ref_args = vec![];
                    for arg in args.iter_mut() {
                        ref_args.push(arg.to_js_value(q_js_rt)?);
                    }

                    let res = crate::quickjs_utils::functions::call_function(
                        q_js_rt, obj_ref, ref_args, None,
                    );
                    match res {
                        Ok(r) => EsValueFacade::from_jsval(q_js_rt, &r),
                        Err(e) => {
                            log::error!("invoke_func_sync failed: {}", e);
                            Err(e)
                        }
                    }
                })
            })
        } else {
            Err(EsError::new_str("rt was dropped"))
        }
    }

    fn invoke_function(&self, mut args: Vec<EsValueFacade>) -> Result<(), EsError> {
        let cached_obj_id = self.cached_obj_id;
        if let Some(rt_arc) = self.es_rt.upgrade() {
            rt_arc.add_to_event_queue(move |q_js_rt| {
                q_js_rt.with_cached_obj(cached_obj_id, move |obj_ref| {
                    let mut ref_args = vec![];
                    for arg in args.iter_mut() {
                        ref_args.push(
                            arg.to_js_value(q_js_rt)
                                .ok()
                                .expect("could not convert arg"),
                        );
                    }

                    let res = crate::quickjs_utils::functions::call_function(
                        q_js_rt, obj_ref, ref_args, None,
                    );
                    match res {
                        Ok(_) => {
                            log::trace!("async func ok");
                        }
                        Err(e) => {
                            log::error!("async func failed: {}", e);
                        }
                    }
                })
            });
            Ok(())
        } else {
            Err(EsError::new_str("rt was dropped"))
        }
    }
}

impl EsValueConvertible for String {
    fn to_js_value(&mut self, q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
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
    fn to_js_value(&mut self, _q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
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
    fn to_js_value(&mut self, _q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
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
    fn to_js_value(&mut self, _q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
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
    fn to_js_value(&mut self, q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
        // create the array

        let arr = crate::quickjs_utils::arrays::create_array(q_js_rt)
            .ok()
            .unwrap();

        // add items
        for index in 0..self.len() {
            let item = self.get_mut(index).unwrap();

            let item_val_ref = item.to_js_value(q_js_rt)?;

            crate::quickjs_utils::arrays::set_element(q_js_rt, &arr, index as u32, item_val_ref)?;
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
    fn to_js_value(&mut self, q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
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
                property_value_ref,
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

pub type EsPromiseResolver = Box<dyn FnOnce() -> Result<EsValueFacade, String> + Send + 'static>;

/// can be used to create a new Promise which is resolved with the resolver function
/// # Example
/// ```rust
/// use quickjs_es_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_es_runtime::esvalue::{EsPromise, EsValueConvertible};
/// use std::time::Duration;
/// use quickjs_es_runtime::esscript::EsScript;
/// use log::LevelFilter;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.set_function(vec!["my", "comp"], "create_prom", |_args| {
///     Ok(EsPromise::new(|| {
///         std::thread::sleep(Duration::from_secs(1));
///         Ok(9463.to_es_value_facade())
///     }).to_es_value_facade())
/// });
/// rt.eval_sync(EsScript::new("test_prom.es", "let p765 = my.comp.create_prom(); p765.then((p_res) => {console.log('got ' + p_res)});")).ok().expect("script failed");
/// std::thread::sleep(Duration::from_secs(2));
/// ```
pub struct EsPromise {
    // todo is box really needed?
    resolver: Option<EsPromiseResolver>,
}

impl EsPromise {
    pub fn new<R>(resolver: R) -> Self
    where
        R: FnOnce() -> Result<EsValueFacade, String> + Send + 'static,
    {
        Self {
            resolver: Some(Box::new(resolver)),
        }
    }
}

impl EsValueConvertible for EsPromise {
    fn to_js_value(&mut self, q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
        log::trace!("EsPromise::to_js_value");
        // create resolving promise

        let resolver = self
            .resolver
            .take()
            .expect("EsPromises was already converted to JSValue");

        if let Some(es_rt) = q_js_rt.get_rt_ref() {
            let producer = move || {
                // run resolver
                log::trace!("running EsPromise resolver");
                resolver()
            };
            let mapper = |mut val: EsValueFacade| {
                QuickJsRuntime::do_with(|q_js_rt| val.to_js_value(q_js_rt))
            };
            new_resolving_promise(q_js_rt, producer, mapper, &es_rt)
        } else {
            Ok(quickjs_utils::new_null_ref())
        }
    }
}

pub struct EsValueFacade {
    convertible: Box<dyn EsValueConvertible + Send + 'static>,
}

impl EsValueFacade {
    pub fn to_js_value(&mut self, q_js_rt: &QuickJsRuntime) -> Result<JSValueRef, EsError> {
        self.convertible.to_js_value(q_js_rt)
    }

    pub fn from_jsval(q_js_rt: &QuickJsRuntime, value_ref: &JSValueRef) -> Result<Self, EsError> {
        log::trace!("EsValueFacade::from_jsval: tag:{}", value_ref.get_tag());

        let r = value_ref.borrow_value();

        match r.tag {
            TAG_STRING => {
                // String.
                let s = crate::quickjs_utils::primitives::to_string(q_js_rt, value_ref)?;

                Ok(s.to_es_value_facade())
            }
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

            // Object.
            TAG_OBJECT => {
                if promises::is_promise(q_js_rt, value_ref) {
                    let cached_obj_id = q_js_rt.cache_object(value_ref.clone());
                    Ok(CachedJSPromise {
                        cached_obj_id,
                        es_rt: Arc::downgrade(&q_js_rt.get_rt_ref().unwrap()),
                    }
                    .to_es_value_facade())
                } else if arrays::is_array(q_js_rt, value_ref) {
                    Self::from_jsval_array(q_js_rt, value_ref)
                } else if functions::is_function(q_js_rt, value_ref) {
                    let cached_obj_id = q_js_rt.cache_object(value_ref.clone());
                    let cached_func = CachedJSFunction {
                        cached_obj_id,
                        es_rt: Arc::downgrade(&q_js_rt.get_rt_ref().unwrap()),
                    };
                    Ok(cached_func.to_es_value_facade())
                } else if dates::is_date(q_js_rt, value_ref)? {
                    Err(EsError::new_str("dates are currently not supported"))
                } else {
                    Self::from_jsval_object(q_js_rt, value_ref)
                }
            }
            // BigInt
            TAG_BIG_INT => Err(EsError::new_str("BigInts are currently not supported")),
            x => Err(EsError::new_string(format!(
                "Unhandled JS_TAG value: {}",
                x
            ))),
        }
    }

    fn from_jsval_array(
        q_js_rt: &QuickJsRuntime,
        value_ref: &JSValueRef,
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
        obj_ref: &JSValueRef,
    ) -> Result<EsValueFacade, EsError> {
        assert!(obj_ref.is_object());

        let map =
            crate::quickjs_utils::objects::traverse_properties(q_js_rt, obj_ref, |_key, val| {
                EsValueFacade::from_jsval(q_js_rt, &val)
            })?;
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

    /// get the array value
    pub fn get_array(&self) -> &Vec<EsValueFacade> {
        self.convertible.get_array()
    }

    /// get the object value
    pub fn get_object(&self) -> &HashMap<String, EsValueFacade> {
        self.convertible.get_object()
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

    /// check if the value is a Promise
    pub fn is_promise(&self) -> bool {
        self.convertible.is_promise()
    }

    /// check if the value is an array
    pub fn is_array(&self) -> bool {
        self.convertible.is_array()
    }

    /// check if the value is an function
    pub fn is_function(&self) -> bool {
        self.convertible.is_function()
    }

    pub fn invoke_function_sync(
        &self,
        arguments: Vec<EsValueFacade>,
    ) -> Result<EsValueFacade, EsError> {
        self.convertible.invoke_function_sync(arguments)
    }
    pub fn invoke_function(&self, arguments: Vec<EsValueFacade>) -> Result<(), EsError> {
        self.convertible.invoke_function(arguments)
    }
    pub fn await_promise_blocking(
        &self,
        timeout: Duration,
    ) -> Result<Result<EsValueFacade, EsValueFacade>, RecvTimeoutError> {
        self.convertible.await_promise_blocking(timeout)
    }
}

impl Debug for EsValueFacade {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), Error> {
        if self.is_string() {
            f.write_str(self.get_str())
        } else if self.is_i32() {
            f.write_str(format!("{}", self.get_i32()).as_str())
        } else if self.is_f64() {
            f.write_str(format!("{}", self.get_f64()).as_str())
        } else if self.is_boolean() {
            f.write_str(format!("{}", self.get_boolean()).as_str())
        } else if self.is_promise() {
            f.write_str("[Promise]")
        } else if self.is_function() {
            f.write_str("[Function]")
        } else if self.is_object() {
            f.write_str("[Object]")
        } else if self.is_array() {
            f.write_str("[Array]")
        } else {
            f.write_str("[Unknown]")
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::tests::TEST_ESRT;
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_promise() {
        let rt: Arc<EsRuntime> = TEST_ESRT.clone();
        let res = rt.eval_sync(EsScript::new(
            "test_promise.es",
            "(new Promise(function(resolve, reject){resolve(537);}));",
        ));
        match res {
            Ok(esvf) => {
                assert!(esvf.is_promise());
                let res = esvf.await_promise_blocking(Duration::from_secs(1));
                match res {
                    Ok(r) => {
                        match r {
                            Ok(v) => {
                                // promise resolved to v
                                assert!(v.is_i32());
                                assert_eq!(v.get_i32(), 537);
                            }
                            Err(e) => {
                                panic!("{}", e.get_str());
                            }
                        }
                    }
                    Err(e) => {
                        panic!("{}", e);
                    }
                }
            }
            Err(e) => panic!("{}", e),
        }
    }
}
