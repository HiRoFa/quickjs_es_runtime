//! contains the legacy EsValueFacade which will be deprecated in favour of utils::JsValueFacade in the near future

use crate::facades::{QuickJsRuntimeFacade, QuickjsRuntimeFacadeInner};
use crate::quickjs_utils::arrays::{get_element_q, get_length_q, is_array_q};
use crate::quickjs_utils::dates::is_date_q;
use crate::quickjs_utils::errors::{error_to_js_error, is_error_q};
use crate::quickjs_utils::functions::{is_function_q, new_function_q};
use crate::quickjs_utils::json::stringify_q;
use crate::quickjs_utils::objects::{get_property_names_q, get_property_q};
use crate::quickjs_utils::primitives::to_string_q;
use crate::quickjs_utils::promises::{is_promise_q, PromiseRef};
use crate::quickjs_utils::{functions, new_null_ref, promises};
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use crate::reflection;
use crate::valueref::*;
use futures::executor::block_on;
use hirofa_utils::auto_id_map::AutoIdMap;
use hirofa_utils::debug_mutex::DebugMutex;
use hirofa_utils::js_utils::JsError;
use hirofa_utils::resolvable_future::{ResolvableFuture, ResolvableFutureResolver};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::future::Future;
use std::rc::Rc;
use std::sync::{Arc, Mutex, Weak};

pub type EsValueFacadeFuture<R, E> = ResolvableFuture<Result<R, E>>;

pub type PromiseReactionType =
    Option<Box<dyn Fn(EsValueFacade) -> Result<EsValueFacade, JsError> + Send + 'static>>;

pub trait EsValueConvertible {
    fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError>;

    #[allow(clippy::wrong_self_convention)]
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
    fn invoke_function_sync(&self, _args: Vec<EsValueFacade>) -> Result<EsValueFacade, JsError> {
        panic!("i am not a function");
    }
    fn invoke_function(
        &self,
        _args: Vec<EsValueFacade>,
    ) -> EsValueFacadeFuture<EsValueFacade, JsError> {
        panic!("i am not a function");
    }
    fn invoke_function_batch_sync(
        &self,
        _args: Vec<Vec<EsValueFacade>>,
    ) -> Vec<Result<EsValueFacade, JsError>> {
        panic!("i am not a function");
    }
    fn invoke_function_batch(&self, _args: Vec<Vec<EsValueFacade>>) -> Result<(), JsError> {
        panic!("i am not a function");
    }
    fn is_promise(&self) -> bool {
        false
    }
    fn get_promise_result_sync(&self) -> Result<EsValueFacade, EsValueFacade> {
        panic!("i am not a promise");
    }
    fn get_promise_result(&self) -> EsValueFacadeFuture<EsValueFacade, EsValueFacade> {
        panic!("i am not a promise");
    }
    fn add_promise_reactions(
        &self,
        _then: PromiseReactionType,
        _catch: PromiseReactionType,
        _finally: Option<Box<dyn Fn() + Send + 'static>>,
    ) -> Result<(), JsError> {
        panic!("i am not a promise")
    }
    fn is_object(&self) -> bool {
        false
    }
    fn get_object(&self) -> Result<HashMap<String, EsValueFacade>, JsError> {
        panic!("i am not an object");
    }
    fn is_array(&self) -> bool {
        false
    }
    fn get_array(&self) -> Result<Vec<EsValueFacade>, JsError> {
        panic!("i am not an array");
    }
    fn supports_stringify(&self) -> bool {
        false
    }
    fn stringify(&self) -> Result<String, JsError> {
        unimplemented!()
    }
    fn is_error(&self) -> bool {
        false
    }
    fn get_error(&self) -> JsError {
        unimplemented!()
    }
}

pub struct EsUndefinedValue {}
pub struct EsNullValue {}

pub const ES_NULL: EsNullValue = EsNullValue {};
pub const ES_UNDEFINED: EsUndefinedValue = EsUndefinedValue {};

pub struct EsProxyInstance {
    class_name: &'static str,
    instance_id: usize,
}

impl EsProxyInstance {
    pub fn new(class_name: &'static str, instance_id: usize) -> Self {
        Self {
            class_name,
            instance_id,
        }
    }
}

impl EsValueConvertible for EsProxyInstance {
    fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        let proxy_opt = reflection::get_proxy(q_ctx, self.class_name);
        if let Some(proxy) = proxy_opt {
            reflection::new_instance3(&proxy, self.instance_id, q_ctx)
        } else {
            Err(JsError::new_string(format!(
                "no such proxy: {}",
                self.class_name
            )))
        }
    }
}

impl EsValueConvertible for EsNullValue {
    fn as_js_value(&mut self, _q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        Ok(crate::quickjs_utils::new_null_ref())
    }

    fn is_null(&self) -> bool {
        true
    }

    fn supports_stringify(&self) -> bool {
        true
    }

    fn stringify(&self) -> Result<String, JsError> {
        Ok("null".to_string())
    }
}

impl EsValueConvertible for EsUndefinedValue {
    fn as_js_value(&mut self, _q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        Ok(crate::quickjs_utils::new_undefined_ref())
    }

    fn is_undefined(&self) -> bool {
        true
    }

    fn supports_stringify(&self) -> bool {
        true
    }

    fn stringify(&self) -> Result<String, JsError> {
        Ok("undefined".to_string())
    }
}

#[derive(PartialEq)]
enum EsType {
    Promise,
    Object,
    Date,
    Array,
    BigInt,
    Function,
    Error,
}

#[allow(clippy::upper_case_acronyms)]
struct CachedJSValueRef {
    cached_obj_id: i32,
    context_id: String,
    rti_ref: Weak<QuickjsRuntimeFacadeInner>,
    es_type: EsType,
}

impl CachedJSValueRef {
    fn new(q_ctx: &QuickJsRealmAdapter, value_ref: &JSValueRef) -> Self {
        log::trace!("> CachedJSValueRef::new");
        let el_ref = QuickJsRuntimeAdapter::do_with(|q_js_rt| q_js_rt.get_rti_ref().unwrap());
        let cached_obj_id = q_ctx.cache_object(value_ref.clone());

        let es_type = if value_ref.is_big_int() {
            EsType::BigInt
        } else if is_array_q(q_ctx, value_ref) {
            EsType::Array
        } else if is_date_q(q_ctx, value_ref) {
            EsType::Date
        } else if is_function_q(q_ctx, value_ref) {
            EsType::Function
        } else if is_promise_q(q_ctx, value_ref) {
            EsType::Promise
        } else if is_error_q(q_ctx, value_ref) {
            EsType::Error
        } else {
            EsType::Object
        };

        let ret = Self {
            cached_obj_id,
            context_id: q_ctx.id.to_string(),
            rti_ref: Arc::downgrade(&el_ref),
            es_type,
        };
        log::trace!("< CachedJSValueRef::new");
        ret
    }

    fn do_with_sync<C, R: Send + 'static>(&self, consumer: C) -> R
    where
        C: FnOnce(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter, JSValueRef) -> R + Send + 'static,
    {
        let cached_obj_id = self.cached_obj_id;

        if let Some(el_ref) = self.rti_ref.upgrade() {
            let context_id_then = self.context_id.clone();
            el_ref.exe_rt_task_in_event_loop(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id_then.as_str());

                q_ctx.with_cached_obj(cached_obj_id, |cached_obj_ref| {
                    consumer(q_js_rt, q_ctx, cached_obj_ref)
                })
            })
        } else {
            panic!("rt was dropped");
        }
    }

    fn do_with_async<C>(&self, consumer: C)
    where
        C: FnOnce(&QuickJsRuntimeAdapter, &QuickJsRealmAdapter, JSValueRef) + Send + 'static,
    {
        let cached_obj_id = self.cached_obj_id;

        if let Some(rti_ref) = self.rti_ref.upgrade() {
            let context_id_then = self.context_id.clone();
            rti_ref.add_rt_task_to_event_loop_void(move |q_js_rt| {
                if let Some(q_ctx) = q_js_rt.opt_context(context_id_then.as_str()) {
                    q_ctx.with_cached_obj(cached_obj_id, |cached_obj_ref| {
                        consumer(q_js_rt, q_ctx, cached_obj_ref);
                    });
                } else {
                    log::error!("do_with_async failed, no such context: {}", context_id_then)
                }
            });
        } else {
            panic!("rt was dropped");
        }
    }
}

impl Drop for CachedJSValueRef {
    fn drop(&mut self) {
        if let Some(rti_ref) = self.rti_ref.upgrade() {
            let cached_obj_id = self.cached_obj_id;
            let context_id = self.context_id.clone();
            rti_ref.add_rt_task_to_event_loop_void(move |q_js_rt| {
                if q_js_rt.has_context(context_id.as_str()) {
                    if let Some(q_ctx) = q_js_rt.opt_context(context_id.as_str()) {
                        q_ctx.remove_cached_obj_if_present(cached_obj_id);
                    }
                }
            });
        }
    }
}

fn pipe_promise_resolution_to_sender(
    q_ctx: &QuickJsRealmAdapter,
    prom_obj_ref: &JSValueRef,
    tx: Arc<ResolvableFutureResolver<Result<EsValueFacade, EsValueFacade>>>,
) {
    log::trace!("pipe_promise_resolution_to_sender");
    let tx2 = tx.clone();
    let then_func_ref = functions::new_function_q(
        q_ctx,
        "promise_then_result_transmitter",
        move |q_ctx, _this_ref, args| {
            log::trace!("pipe_promise_resolution_to_sender.then_func");
            // these clones are needed because create_func requires a Fn and not a FnOnce
            // in practice however the Fn is called only once
            let tx3 = tx2.clone();

            let prom_res = &args[0];
            let prom_res_esvf_res = EsValueFacade::from_jsval(q_ctx, prom_res);

            match prom_res_esvf_res {
                Ok(prom_res_esvf) => {
                    let send_res = tx3.resolve(Ok(prom_res_esvf));
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
                    log::error!("could not convert promise result to esvf {}", e);
                    panic!("could not convert promise result to esvf {}", e);
                }
            }

            Ok(new_null_ref())
        },
        1,
    )
    .ok()
    .expect("could not create func");

    let catch_func_ref = functions::new_function_q(
        q_ctx,
        "promise_catch_result_transmitter",
        move |q_ctx, _this_ref, args| {
            log::trace!("pipe_promise_resolution_to_sender.catch_func");
            // these clones are needed because create_func requires a Fn and not a FnOnce
            // in practice however the Fn is called only once
            let tx3 = tx.clone();

            let prom_res = &args[0];
            let prom_res_esvf_res = EsValueFacade::from_jsval(q_ctx, prom_res);
            match prom_res_esvf_res {
                Ok(prom_res_esvf) => {
                    let send_res = tx3.resolve(Err(prom_res_esvf));
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
                    log::error!("could not convert promise result to esvf {}", e);
                    panic!("could not convert promise result to esvf {}", e);
                }
            }
            Ok(new_null_ref())
        },
        1,
    )
    .ok()
    .expect("could not create func");

    promises::add_promise_reactions_q(
        q_ctx,
        prom_obj_ref,
        Some(then_func_ref),
        Some(catch_func_ref),
        None,
    )
    .ok()
    .expect("could not create promise reactions");
}

impl EsValueConvertible for CachedJSValueRef {
    fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        Ok(q_ctx.with_cached_obj(self.cached_obj_id, |obj_ref| obj_ref))
    }

    fn is_function(&self) -> bool {
        self.es_type == EsType::Function
    }

    fn invoke_function_sync(&self, mut args: Vec<EsValueFacade>) -> Result<EsValueFacade, JsError> {
        assert!(self.is_function());

        self.do_with_sync(move |_q_js_rt, q_ctx, obj_ref| {
            let mut ref_args = vec![];
            for arg in args.iter_mut() {
                ref_args.push(arg.as_js_value(q_ctx)?);
            }

            crate::quickjs_utils::functions::call_function_q(q_ctx, &obj_ref, ref_args, None)
                .and_then(|js_val| EsValueFacade::from_jsval(q_ctx, &js_val))
        })
    }

    fn invoke_function(
        &self,
        mut args: Vec<EsValueFacade>,
    ) -> EsValueFacadeFuture<EsValueFacade, JsError> {
        assert!(self.is_function());
        let ret = EsValueFacadeFuture::new();
        let tx = ret.get_resolver();
        self.do_with_sync(move |_q_js_rt, q_ctx, obj_ref| {
            let mut ref_args = vec![];
            for arg in args.iter_mut() {
                match arg.as_js_value(q_ctx) {
                    Ok(j_arg) => ref_args.push(j_arg),
                    Err(e) => {
                        log::error!("arg conversion failed in esvalue::invoke_function: {}", e);
                        return;
                    }
                }
            }

            let res =
                crate::quickjs_utils::functions::call_function_q(q_ctx, &obj_ref, ref_args, None)
                    .and_then(|js_val| EsValueFacade::from_jsval(q_ctx, &js_val));
            let _ = tx.resolve(res);
        });

        ret
    }

    fn invoke_function_batch_sync(
        &self,
        batch_args: Vec<Vec<EsValueFacade>>,
    ) -> Vec<Result<EsValueFacade, JsError>> {
        assert!(self.is_function());
        self.do_with_sync(move |_q_js_rt, q_ctx, obj_ref| {
            let mut res_vec: Vec<Result<EsValueFacade, JsError>> = vec![];
            for mut args in batch_args {
                let mut ref_args = vec![];
                for arg in args.iter_mut() {
                    ref_args.push(arg.as_js_value(q_ctx).ok().expect("to_js_value failed"));
                }

                let res = crate::quickjs_utils::functions::call_function_q(
                    q_ctx, &obj_ref, ref_args, None,
                );
                match res {
                    Ok(r) => {
                        res_vec.push(EsValueFacade::from_jsval(q_ctx, &r));
                    }
                    Err(e) => {
                        log::error!("invoke_func_sync failed: {}", e);
                        res_vec.push(Err(e));
                    }
                }
            }
            res_vec
        })
    }

    // todo rewrite to Future
    fn invoke_function_batch(&self, batch_args: Vec<Vec<EsValueFacade>>) -> Result<(), JsError> {
        assert!(self.is_function());
        self.do_with_sync(move |_q_js_rt, q_ctx, obj_ref| {
            for mut args in batch_args {
                let mut ref_args = vec![];
                for arg in args.iter_mut() {
                    ref_args.push(arg.as_js_value(q_ctx).ok().expect("could not convert arg"));
                }

                let res = crate::quickjs_utils::functions::call_function_q(
                    q_ctx, &obj_ref, ref_args, None,
                );
                match res {
                    Ok(_) => {
                        log::trace!("async func ok");
                    }
                    Err(e) => {
                        log::error!("async func failed: {}", e);
                    }
                }
            }
        });
        Ok(())
    }

    fn is_promise(&self) -> bool {
        self.es_type == EsType::Promise
    }

    fn get_promise_result_sync(&self) -> Result<EsValueFacade, EsValueFacade> {
        assert!(self.is_promise());
        let fut = self.get_promise_result();
        log::trace!("block_on get_promise_result_sync");
        let res = block_on(fut);
        log::trace!("block_on get_promise_result_sync -> done");
        res
    }

    fn get_promise_result(&self) -> EsValueFacadeFuture<EsValueFacade, EsValueFacade> {
        assert!(self.is_promise());
        let fut = EsValueFacadeFuture::new();
        let tx = fut.get_resolver();
        self.do_with_async(move |_q_js_rt, q_ctx, prom_obj_ref| {
            pipe_promise_resolution_to_sender(q_ctx, &prom_obj_ref, tx);
        });
        fut
    }

    fn add_promise_reactions(
        &self,
        then: PromiseReactionType,
        catch: PromiseReactionType,
        finally: Option<Box<dyn Fn() + Send + 'static>>,
    ) -> Result<(), JsError> {
        assert!(self.is_promise());
        self.do_with_sync(move |_q_js_rt, q_ctx, prom_ref| {
            let then_ref = if let Some(then_fn) = then {
                let then_fn_rc = Rc::new(then_fn);

                let then_fn_raw = move |q_ctx: &QuickJsRealmAdapter,
                                        _this_ref: &JSValueRef,
                                        args_ref: &[JSValueRef]| {
                    let then_fn_rc = then_fn_rc.clone();
                    let val_ref = &args_ref[0];

                    let val_esvf = EsValueFacade::from_jsval(q_ctx, val_ref)?;

                    then_fn_rc(val_esvf)?;
                    Ok(crate::quickjs_utils::new_null_ref())
                };
                let t = functions::new_function_q(q_ctx, "", then_fn_raw, 1)
                    .ok()
                    .expect("could not create function");
                Some(t)
            } else {
                None
            };

            let catch_ref = if let Some(catch_fn) = catch {
                let catch_fn_rc = Rc::new(catch_fn);

                let t = functions::new_function_q(
                    q_ctx,
                    "",
                    move |q_ctx: &QuickJsRealmAdapter, _this_ref, args_ref: &[JSValueRef]| {
                        let val_ref = &args_ref[0];
                        let catch_fn_rc = catch_fn_rc.clone();

                        let val_esvf = EsValueFacade::from_jsval(q_ctx, val_ref)?;

                        catch_fn_rc(val_esvf)?;
                        Ok(crate::quickjs_utils::new_null_ref())
                    },
                    1,
                )
                .ok()
                .expect("could not create function");
                Some(t)
            } else {
                None
            };

            let finally_ref = if let Some(finally_fn) = finally {
                let t = functions::new_function_q(
                    q_ctx,
                    "",
                    move |_q_ctx: &QuickJsRealmAdapter, _this_ref, _args_ref| {
                        finally_fn();
                        Ok(crate::quickjs_utils::new_null_ref())
                    },
                    0,
                )
                .ok()
                .expect("could not create function");
                Some(t)
            } else {
                None
            };

            promises::add_promise_reactions_q(q_ctx, &prom_ref, then_ref, catch_ref, finally_ref)
                .ok()
                .expect("could not add reactions")
        });

        Ok(())
    }

    fn is_object(&self) -> bool {
        self.es_type == EsType::Object
    }

    fn get_object(&self) -> Result<HashMap<String, EsValueFacade>, JsError> {
        assert!(self.is_object());
        self.do_with_sync(|_q_js_rt, q_ctx, obj_ref| {
            let mut ret = HashMap::new();

            for prop_name in get_property_names_q(q_ctx, &obj_ref)? {
                log::trace!("EsValueConvertible::get_object > prop: {}", prop_name);
                let prop_ref = get_property_q(q_ctx, &obj_ref, prop_name.as_str())?;
                let prop_esvf = EsValueFacade::from_jsval(q_ctx, &prop_ref)?;
                ret.insert(prop_name, prop_esvf);
            }
            log::trace!("EsValueConvertible::get_object > return");
            Ok(ret)
        })
    }

    fn is_array(&self) -> bool {
        self.es_type == EsType::Array
    }

    fn get_array(&self) -> Result<Vec<EsValueFacade>, JsError> {
        assert!(self.is_array());

        self.do_with_sync(|_q_js_rt, q_ctx, obj_ref| {
            let mut ret = vec![];

            for x in 0..get_length_q(q_ctx, &obj_ref)? {
                let item_ref = get_element_q(q_ctx, &obj_ref, x)?;
                let esvf = EsValueFacade::from_jsval(q_ctx, &item_ref)?;
                ret.push(esvf)
            }

            Ok(ret)
        })
    }

    fn supports_stringify(&self) -> bool {
        true
    }

    fn stringify(&self) -> Result<String, JsError> {
        assert!(self.supports_stringify());
        self.do_with_sync(|_q_js_rt, q_ctx, obj_ref| {
            let res = stringify_q(q_ctx, &obj_ref, None)?;
            to_string_q(q_ctx, &res)
        })
    }

    fn is_error(&self) -> bool {
        self.es_type == EsType::Error
    }

    fn get_error(&self) -> JsError {
        assert!(self.is_error());
        self.do_with_sync(|_q_js_rt, q_ctx, obj_ref| unsafe {
            error_to_js_error(q_ctx.context, &obj_ref)
        })
    }
}

impl EsValueConvertible for String {
    fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        crate::quickjs_utils::primitives::from_string_q(q_ctx, self.as_str())
    }

    fn is_str(&self) -> bool {
        true
    }

    fn get_str(&self) -> &str {
        self.as_str()
    }

    fn supports_stringify(&self) -> bool {
        true
    }

    fn stringify(&self) -> Result<String, JsError> {
        Ok(format!("\"{}\"", self.as_str().replace('"', "\\\"")))
    }
}

impl EsValueConvertible for i32 {
    fn as_js_value(&mut self, _q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        Ok(crate::quickjs_utils::primitives::from_i32(*self))
    }

    fn is_i32(&self) -> bool {
        true
    }

    fn get_i32(&self) -> i32 {
        *self
    }

    fn supports_stringify(&self) -> bool {
        true
    }

    fn stringify(&self) -> Result<String, JsError> {
        Ok(format!("{}", self))
    }
}

impl EsValueConvertible for bool {
    fn as_js_value(&mut self, _q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        Ok(crate::quickjs_utils::primitives::from_bool(*self))
    }

    fn is_bool(&self) -> bool {
        true
    }

    fn get_bool(&self) -> bool {
        *self
    }

    fn supports_stringify(&self) -> bool {
        true
    }

    fn stringify(&self) -> Result<String, JsError> {
        Ok(format!("{}", self))
    }
}

impl EsValueConvertible for f64 {
    fn as_js_value(&mut self, _q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        Ok(crate::quickjs_utils::primitives::from_f64(*self))
    }
    fn is_f64(&self) -> bool {
        true
    }

    fn get_f64(&self) -> f64 {
        *self
    }

    fn supports_stringify(&self) -> bool {
        true
    }

    fn stringify(&self) -> Result<String, JsError> {
        Ok(format!("{}", self))
    }
}

impl EsValueConvertible for Vec<EsValueFacade> {
    fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        // create the array

        let arr = crate::quickjs_utils::arrays::create_array_q(q_ctx)
            .ok()
            .unwrap();

        // add items
        for index in 0..self.len() {
            let item = self.get_mut(index).unwrap();

            let item_val_ref = item.as_js_value(q_ctx)?;

            crate::quickjs_utils::arrays::set_element_q(q_ctx, &arr, index as u32, &item_val_ref)?;
        }
        Ok(arr)
    }
}

impl EsValueConvertible for HashMap<String, EsValueFacade> {
    fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        // create new obj
        let obj_ref = crate::quickjs_utils::objects::create_object_q(q_ctx)
            .ok()
            .unwrap();

        for prop in self {
            let prop_name = prop.0;
            let prop_esvf = prop.1;

            // set prop in obj

            let property_value_ref = prop_esvf.as_js_value(q_ctx)?;

            crate::quickjs_utils::objects::set_property_q(
                q_ctx,
                &obj_ref,
                prop_name.as_str(),
                &property_value_ref,
            )?;
        }

        Ok(obj_ref)
    }
}

pub type EsPromiseResolver = Box<dyn FnOnce() -> Result<EsValueFacade, String> + Send + 'static>;

thread_local! {
    static ESPROMISE_REFS: RefCell<AutoIdMap<PromiseRef>> = RefCell::new(AutoIdMap::new());
}

struct EsPromiseResolvableHandleInfo {
    weak_rti_ref: Weak<QuickjsRuntimeFacadeInner>,
    id: usize,
    context_id: String,
}

struct EsPromiseResolvableHandleInner {
    js_info: Option<EsPromiseResolvableHandleInfo>,
    resolution: Option<Result<EsValueFacade, EsValueFacade>>,
}

pub struct EsPromiseResolvableHandle {
    inner: DebugMutex<EsPromiseResolvableHandleInner>,
}

impl EsPromiseResolvableHandle {
    fn new() -> Self {
        Self {
            inner: DebugMutex::new(
                EsPromiseResolvableHandleInner {
                    js_info: None,
                    resolution: None,
                },
                "EsPromiseResolvableHandle::inner_mtx",
            ),
        }
    }

    fn with_inner<C, R>(&self, consumer: C) -> R
    where
        C: FnOnce(&mut EsPromiseResolvableHandleInner) -> R,
    {
        let mut lck = self.inner.lock("with_inner").unwrap();
        consumer(&mut *lck)
    }

    pub fn resolve(&self, value: EsValueFacade) {
        log::debug!("resolving handle with val: {:?}", value);
        // this is done in two stages so we unlock the inner mutex while resolving
        let rt_opt = self.with_inner(|inner| {
            if let Some(info) = &inner.js_info {
                // resolve
                let id = info.id;
                let context_id = info.context_id.clone();
                if let Some(rt) = info.weak_rti_ref.upgrade() {
                    Some((rt, id, context_id, value))
                } else {
                    log::error!("rt was dropped while resolving");
                    None
                }
            } else {
                // resolve later when converted to JSValue
                inner.resolution = Some(Ok(value));
                None
            }
        });
        if let Some((rti_ref, id, context_id, mut value)) = rt_opt {
            rti_ref.add_rt_task_to_event_loop_void(move |q_js_rt| {
                log::trace!("resolving handle with val, stage 2: {:?}", value);
                if let Some(q_ctx) = q_js_rt.opt_context(context_id.as_str()) {
                    ESPROMISE_REFS.with(move |rc| {
                        let map = &*rc.borrow();
                        let p_ref = map.get(&id).expect("no such promise");

                        let js_val = value
                            .as_js_value(q_ctx)
                            .ok()
                            .expect("could not convert to JSValue");
                        let resolve_res = unsafe { p_ref.resolve(q_ctx.context, js_val) };
                        if resolve_res.is_err() {
                            log::error!("resolve failed: {}", resolve_res.err().unwrap());
                        }
                    });
                } else {
                    log::error!("resolve failed: no such context {}", context_id);
                }
            });
        }
    }
    pub fn reject(&self, value: EsValueFacade) {
        log::debug!("rejecting handle with val: {:?}", value);
        let rt_opt = self.with_inner(|inner| {
            if let Some(info) = &inner.js_info {
                // resolve
                let id = info.id;
                let context_id = info.context_id.clone();

                if let Some(rt) = info.weak_rti_ref.upgrade() {
                    Some((rt, id, context_id, value))
                } else {
                    log::error!("rt was dropped while rejecting");
                    None
                }
            } else {
                // resolve later when converted to JSValue
                inner.resolution = Some(Err(value));
                None
            }
        });
        if let Some((rti_ref, id, context_id, mut value)) = rt_opt {
            rti_ref.add_rt_task_to_event_loop_void(move |q_js_rt| {
                log::trace!("rejecting handle with val, stage 2: {:?}", value);
                if let Some(q_ctx) = q_js_rt.opt_context(context_id.as_str()) {
                    ESPROMISE_REFS.with(move |rc| {
                        let map = &*rc.borrow();
                        let p_ref = map.get(&id).expect("no such promise");
                        let js_val = value
                            .as_js_value(q_ctx)
                            .ok()
                            .expect("could not convert to JSValue");
                        let reject_res = unsafe { p_ref.reject(q_ctx.context, js_val) };
                        if reject_res.is_err() {
                            log::error!("reject failed: {}", reject_res.err().unwrap());
                        }
                    });
                } else {
                    log::error!("reject failed: no such context {}", context_id);
                }
            });
        }
    }
    fn set_info(
        &self,
        rti_ref: Weak<QuickjsRuntimeFacadeInner>,
        id: usize,
        context_id: &str,
    ) -> Result<(), JsError> {
        let resolution_opt: Option<Result<EsValueFacade, EsValueFacade>> =
            self.with_inner(|inner| {
                if inner.js_info.is_some() {
                    Err(JsError::new_str("info was already set"))
                } else {
                    // set info
                    inner.js_info = Some(EsPromiseResolvableHandleInfo {
                        weak_rti_ref: rti_ref,
                        id,
                        context_id: context_id.to_string(),
                    });

                    // todo, take this outside with_inner
                    if let Some(resolution) = inner.resolution.take() {
                        Ok(Some(resolution))
                    } else {
                        Ok(None)
                    }
                }
            })?;
        if let Some(resolution) = resolution_opt {
            match resolution {
                Ok(val) => {
                    self.resolve(val);
                }
                Err(val) => {
                    self.reject(val);
                }
            }
        }
        Ok(())
    }
}

impl Drop for EsPromiseResolvableHandleInner {
    fn drop(&mut self) {
        if let Some(info) = &self.js_info {
            let id = info.id;
            if let Some(rti_ref) = info.weak_rti_ref.upgrade() {
                rti_ref.add_task_to_event_loop_void(move || {
                    ESPROMISE_REFS.with(move |rc| {
                        let map = &mut *rc.borrow_mut();
                        map.remove(&id);
                    });
                });
            }
        }
    }
}

/// used to create a new Function
/// # Example
/// ```rust
///    
/// ```
#[allow(clippy::type_complexity)]
pub struct EsFunction {
    // todo rebuild this to an Option which we'll move into the worker thread on to_js_val
    // thus killing off the Mutex and such
    method:
        Arc<Mutex<dyn Fn(Vec<EsValueFacade>) -> Result<EsValueFacade, String> + Send + 'static>>,
    name: &'static str,
}

impl EsFunction {
    /// create a new Function based on a method
    pub fn new<R>(name: &'static str, method: R, async_function: bool) -> Self
    where
        R: Fn(Vec<EsValueFacade>) -> Result<EsValueFacade, String> + Send + 'static,
    {
        if async_function {
            // this is suboptimal because of the mutex but i have to make the method Sync
            // todo: rebuild to move method to worker thread first instead of working with EsPromise (which requires method to be Send causing the Sync requirement)
            let method_rc = Arc::new(Mutex::new(method));
            let wrapper = move |args| {
                // create new promise
                let method_rc = method_rc.clone();

                Ok(EsPromise::new(move || {
                    //
                    let method = &*method_rc.lock().unwrap();
                    method(args)
                })
                .to_es_value_facade())
            };
            Self {
                method: Arc::new(Mutex::new(wrapper)),
                name,
            }
        } else {
            Self {
                method: Arc::new(Mutex::new(method)),
                name,
            }
        }
    }
    /// create a new Function based on a method
    /// # Example
    /// ```rust
    /// use quickjs_runtime::esvalue::{EsFunction, EsValueConvertible, ES_NULL, EsValueFacade};
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use hirofa_utils::js_utils::Script;
    /// async fn do_something(args: Vec<EsValueFacade>) -> Result<EsValueFacade, String> {
    ///     Ok(123.to_es_value_facade())
    /// }
    /// let func_esvf = EsFunction::new_async("my_callback", do_something).to_es_value_facade();
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// rt.eval_sync(Script::new("new_async.es", "this.test_func = function(cb){return cb();};")).ok().expect("func invo failed");
    /// let func_res = rt.call_function_sync(vec![], "test_func", vec![func_esvf]).ok().expect("func invo failed2");
    /// let ret = func_res.get_promise_result_sync().ok().expect("do_something returned err");
    /// assert_eq!(ret.get_i32(), 123);
    /// ```
    pub fn new_async<R, F>(name: &'static str, method: R) -> Self
    where
        F: Future<Output = Result<EsValueFacade, String>> + Send + 'static,
        R: Fn(Vec<EsValueFacade>) -> F + Send + 'static,
    {
        let wrapper = move |args| {
            // create new promise
            let fut = method(args);
            Ok(EsPromise::new_async(fut).to_es_value_facade())
        };

        Self {
            method: Arc::new(Mutex::new(wrapper)),
            name,
        }
    }
}

impl EsValueConvertible for EsFunction {
    fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        let func_arc_mtx = self.method.clone();

        new_function_q(
            q_ctx,
            self.name,
            move |q_ctx, _this_ref, args| {
                let mut args_facades = vec![];

                for arg_ref in args {
                    args_facades.push(EsValueFacade::from_jsval(q_ctx, arg_ref)?);
                }

                let func = &*func_arc_mtx.lock().unwrap();

                let mut res: EsValueFacade = func(args_facades).map_err(JsError::new_string)?;
                res.as_js_value(q_ctx)
            },
            1,
        )
    }
}

/// can be used to create a new Promise which is resolved with the resolver function
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::esvalue::{EsPromise, EsValueConvertible};
/// use std::time::Duration;
/// use hirofa_utils::js_utils::Script;
/// use log::LevelFilter;
///
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.set_function(vec!["my", "comp"], "create_prom", |_q_ctx, _args| {
///     Ok(EsPromise::new(|| {
///         std::thread::sleep(Duration::from_secs(1));
///         Ok(9463.to_es_value_facade())
///     }).to_es_value_facade())
/// });
/// rt.eval_sync(Script::new("test_prom.es", "let p765 = my.comp.create_prom(); p765.then((p_res) => {console.log('got ' + p_res)});")).ok().expect("script failed");
/// std::thread::sleep(Duration::from_secs(2));
/// ```
pub struct EsPromise {
    handle: Arc<EsPromiseResolvableHandle>,
}

impl EsPromise {
    /// create a new Promise based on a resolver
    pub fn new<R>(resolver: R) -> Self
    where
        R: FnOnce() -> Result<EsValueFacade, String> + Send + 'static,
    {
        let ret = Self::new_unresolving();

        let handle = ret.get_handle();
        QuickJsRuntimeFacade::add_helper_task(move || {
            let val = resolver();
            match val {
                Ok(v) => {
                    handle.resolve(v);
                }
                Err(e) => {
                    handle.reject(e.to_es_value_facade());
                }
            }
        });

        ret
    }

    /// create a new Promise based on an async resolver
    /// this can be used to implement a resolver which in turn used .await to get results of other async functions
    /// # Example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use quickjs_runtime::esvalue::{EsPromise, EsValueConvertible};
    /// use hirofa_utils::js_utils::Script;
    /// use std::time::Duration;
    ///
    /// async fn a(i: i32) -> i32 {
    ///     std::thread::sleep(Duration::from_secs(2));
    ///     i * 3
    /// }
    ///
    /// let rt = QuickJsRuntimeBuilder::new().build();
    ///
    /// rt.set_function(vec!["com", "my"], "testasyncfunc", |_q_ctx, args| {
    ///     let input = args[0].get_i32();
    ///     let prom = EsPromise::new_async(async move {
    ///         let i = a(input).await;
    ///         Ok(i.to_es_value_facade())
    ///     });
    ///     Ok(prom.to_es_value_facade())
    /// })
    /// .ok()
    /// .expect("setfunction failed");
    ///
    /// let res_prom = rt
    ///     .eval_sync(Script::new("testasync2.es", "(com.my.testasyncfunc(7))"))
    ///     .ok()
    ///     .expect("script failed");
    /// let res_i32 = res_prom.get_promise_result_sync().expect("prom failed");
    /// assert_eq!(res_i32.get_i32(), 21);
    /// ```
    pub fn new_async<R>(resolver: R) -> Self
    where
        R: Future<Output = Result<EsValueFacade, String>> + Send + 'static,
    {
        let ret = Self::new_unresolving();

        let handle = ret.get_handle();

        let _ = QuickJsRuntimeFacade::add_helper_task_async(async move {
            let val = resolver.await;
            match val {
                Ok(v) => {
                    handle.resolve(v);
                }
                Err(e) => {
                    handle.reject(e.to_es_value_facade());
                }
            }
        });

        ret
    }
    /// create a new Promise which will be resolved later
    /// this achieved by creating a Handle which is wrapped in an Arc and thus may be passed to another thread
    /// # Example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use hirofa_utils::js_utils::Script;
    /// use std::time::Duration;
    /// use quickjs_runtime::esvalue::{EsPromise, EsValueConvertible};
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// // prep a function which reacts to a promise
    /// rt.eval_sync(Script::new("new_unresolving.es", "this.new_unresolving = function(prom){prom.then((res) => {console.log('promise resolved to %s', res);});};")).ok().expect("script failed");
    /// // prep a EsPromise object
    /// let prom = EsPromise::new_unresolving();
    /// // get the handle
    /// let prom_handle = prom.get_handle();
    /// // call the function with the promise as arg
    /// rt.call_function(vec![], "new_unresolving".to_string(), vec![prom.to_es_value_facade()]);
    /// // start a new thread which resolves the handler after x seconds
    /// std::thread::spawn(move || {
    ///     std::thread::sleep(Duration::from_secs(3));
    ///     prom_handle.resolve("hello there".to_string().to_es_value_facade());
    /// });
    /// // wait a few secs to see the log output
    /// std::thread::sleep(Duration::from_secs(5));
    /// ```
    ///
    pub fn new_unresolving() -> Self {
        Self {
            handle: Arc::new(EsPromiseResolvableHandle::new()),
        }
    }
    /// get the handle which can be used to resolve a promise
    pub fn get_handle(&self) -> Arc<EsPromiseResolvableHandle> {
        self.handle.clone()
    }
}

impl EsValueConvertible for EsPromise {
    fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        log::trace!("EsPromise::to_js_value");

        let prom_ref = promises::new_promise_q(q_ctx)?;

        let ret = prom_ref.get_promise_obj_ref();
        let id = ESPROMISE_REFS.with(move |rc| {
            let map = &mut *rc.borrow_mut();
            map.insert(prom_ref)
        });
        let el_ref = QuickJsRuntimeAdapter::do_with(|q_js_rt| q_js_rt.get_rti_ref().unwrap());

        self.handle
            .set_info(Arc::downgrade(&el_ref), id, q_ctx.id.as_str())?;

        Ok(ret)
    }
}

pub struct EsValueFacade {
    convertible: Box<dyn EsValueConvertible + Send + 'static>,
}

impl EsValueFacade {
    /// stringify the value
    pub fn stringify(&self) -> Result<String, JsError> {
        self.convertible.stringify()
    }

    /// convert the value to a JSValueRef
    pub fn as_js_value(&mut self, q_ctx: &QuickJsRealmAdapter) -> Result<JSValueRef, JsError> {
        self.convertible.as_js_value(q_ctx)
    }

    /// convert a JSValueRef to an EsValueFacade
    pub fn from_jsval(
        q_ctx: &QuickJsRealmAdapter,
        value_ref: &JSValueRef,
    ) -> Result<Self, JsError> {
        log::trace!("EsValueFacade::from_jsval: tag:{}", value_ref.get_tag());

        let r = value_ref.borrow_value();

        match r.tag {
            TAG_STRING => {
                // String.
                let s = crate::quickjs_utils::primitives::to_string_q(q_ctx, value_ref)?;

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
            TAG_NULL => Ok(ES_NULL.to_es_value_facade()),
            // Undefined.
            TAG_UNDEFINED => Ok(ES_UNDEFINED.to_es_value_facade()),

            // Float.
            TAG_FLOAT64 => {
                let val: f64 = crate::quickjs_utils::primitives::to_f64(value_ref)
                    .ok()
                    .expect("could not convert to f64");
                Ok(val.to_es_value_facade())
            }

            // Object.
            TAG_OBJECT => Ok(CachedJSValueRef::new(q_ctx, value_ref).to_es_value_facade())
            ,
            // BigIn
            TAG_BIG_INT => Ok(CachedJSValueRef::new(q_ctx, value_ref).to_es_value_facade()),
            x => Err(JsError::new_string(format!(
                "Unhandled JS_TAG value: {}",
                x
            ))),
        }
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
    pub fn get_array(&self) -> Result<Vec<EsValueFacade>, JsError> {
        self.convertible.get_array()
    }

    /// get the object value
    pub fn get_object(&self) -> Result<HashMap<String, EsValueFacade>, JsError> {
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

    /// check if the value is null
    pub fn is_null(&self) -> bool {
        self.convertible.is_null()
    }

    /// check if the value is undefined
    pub fn is_undefined(&self) -> bool {
        self.convertible.is_undefined()
    }

    /// invoke the Function represented by this EsValueFacade
    pub fn invoke_function_sync(
        &self,
        arguments: Vec<EsValueFacade>,
    ) -> Result<EsValueFacade, JsError> {
        self.convertible.invoke_function_sync(arguments)
    }
    /// invoke the Function represented by this EsValueFacade
    pub async fn invoke_function(
        &self,
        arguments: Vec<EsValueFacade>,
    ) -> Result<EsValueFacade, JsError> {
        self.convertible.invoke_function(arguments).await
    }

    /// invoke a function multiple times with a different set of arguments
    pub fn invoke_function_batch_sync(
        &self,
        arguments: Vec<Vec<EsValueFacade>>,
    ) -> Vec<Result<EsValueFacade, JsError>> {
        self.convertible.invoke_function_batch_sync(arguments)
    }

    /// invoke a function multiple times with a different set of arguments
    pub fn invoke_function_batch(&self, arguments: Vec<Vec<EsValueFacade>>) -> Result<(), JsError> {
        self.convertible.invoke_function_batch(arguments)
    }

    /// get the result of a Promise, this method blocks until the Promise is fulfilled.
    /// The Result will be an Ok if the Promise was resolved or an Err if the Promise was rejected
    pub fn get_promise_result_sync(&self) -> Result<EsValueFacade, EsValueFacade> {
        self.convertible.get_promise_result_sync()
    }

    /// wait for the result of a Promise async.
    /// The Result will be an Ok if the Promise was resolved or an Err if the Promise was rejected
    /// # Example
    /// ```rust
    /// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
    /// use hirofa_utils::js_utils::Script;
    /// use futures::executor::block_on;
    /// use quickjs_runtime::esvalue::EsValueFacade;
    /// pub async fn test_async(esvf: EsValueFacade) -> i32 {
    ///    let prom_res = esvf.get_promise_result().await;
    ///    let res_esvf = prom_res.ok().unwrap();
    ///    return res_esvf.get_i32();
    /// }
    ///
    /// let rt = QuickJsRuntimeBuilder::new().build();
    /// let esvf = rt.eval_sync(Script::new("test_async_prom,es", "(new Promise((resolve, reject) => {setTimeout(() => {resolve(1360)}, 1000);}));")).ok().expect("script failed");
    /// let i = block_on(test_async(esvf));
    /// assert_eq!(i, 1360);
    ///
    /// ```
    pub fn get_promise_result(&self) -> impl Future<Output = Result<EsValueFacade, EsValueFacade>> {
        self.convertible.get_promise_result()
    }

    pub fn is_error(&self) -> bool {
        self.convertible.is_error()
    }

    pub fn get_error(&self) -> JsError {
        self.convertible.get_error()
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
        } else if self.is_null() {
            f.write_str("[null]")
        } else if self.is_undefined() {
            f.write_str("[undefined]")
        } else {
            f.write_str("[Unknown]")
        }
    }
}

#[cfg(test)]
pub mod tests {
    use crate::builder::QuickJsRuntimeBuilder;
    use crate::esvalue::{EsPromise, EsValueConvertible, EsValueFacade};
    use crate::facades::tests::init_test_rt;
    use crate::facades::QuickJsRuntimeFacade;
    use futures::executor::block_on;
    use hirofa_utils::js_utils::Script;
    use std::sync::{Arc, Weak};
    use std::time::Duration;

    async fn test_async_func1(esvf: EsValueFacade) -> i32 {
        let res = esvf.invoke_function(vec![]).await;
        let esvf = res.ok().expect("func failed");
        esvf.get_i32()
    }

    #[test]
    fn test_async_func() {
        let rt = init_test_rt();
        let func_esvf = rt
            .eval_sync(Script::new(
                "test_async_func.es",
                "(function someFunc(){return 147;});",
            ))
            .ok()
            .expect("script failed");
        let fut = block_on(test_async_func1(func_esvf));
        assert_eq!(fut, 147);
    }

    #[test]
    fn test_promise() {
        let rt = init_test_rt();
        let res = rt.eval_sync(Script::new(
            "test_promise.es",
            "(new Promise(function(resolve, reject){resolve(537);}));",
        ));
        match res {
            Ok(esvf) => {
                assert!(esvf.is_promise());
                let res = esvf.get_promise_result_sync();
                match res {
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
            Err(e) => panic!("{}", e),
        }
    }

    #[test]
    fn test_promise_async() {
        //simple_logging::log_to_stderr(LevelFilter::max());
        pub async fn test_async(esvf: EsValueFacade) -> i32 {
            let prom_res = esvf.get_promise_result().await;
            let res_esvf = prom_res.ok().unwrap();
            return res_esvf.get_i32();
        }

        let rt = QuickJsRuntimeBuilder::new().build();
        let esvf = rt
            .eval_sync(Script::new(
                "test_async_prom,es",
                "(new Promise((resolve, reject) => {setTimeout(() => {resolve(1360)}, 1000);}));",
            ))
            .ok()
            .expect("script failed");
        let i = block_on(test_async(esvf));
        assert_eq!(i, 1360);
    }

    async fn a(i: i32, rt_ref: Weak<QuickJsRuntimeFacade>) -> i32 {
        let rt = rt_ref.upgrade().unwrap();
        let second_prom = rt
            .eval(Script::new(
                "o.es",
                "(new Promise((resolve) => {resolve(321);}))",
            ))
            .await
            .ok()
            .expect("script failed");

        let second_prom_res = second_prom.get_promise_result();
        let res = second_prom_res.await.ok().expect("prom was rejected");

        assert_eq!(res.get_i32(), 321);

        std::thread::sleep(Duration::from_secs(1));
        i * 3
    }

    #[test]
    fn test_promise_async2() {
        let rt = Arc::new(QuickJsRuntimeBuilder::new().build());
        let rt_ref = Arc::downgrade(&rt);
        rt.set_function(vec!["com", "my"], "testasyncfunc", move |_q_ctx, args| {
            let rt_ref = rt_ref.clone();
            let input = args[0].get_i32();
            let prom = EsPromise::new_async(async move {
                let i = a(input, rt_ref).await;
                Ok(i.to_es_value_facade())
            });
            Ok(prom.to_es_value_facade())
        })
        .ok()
        .expect("setfunction failed");

        let res_prom = rt
            .eval_sync(Script::new("testasync2.es", "(com.my.testasyncfunc(7))"))
            .ok()
            .expect("script failed");
        let res_i32 = res_prom.get_promise_result_sync().expect("prom failed");
        assert_eq!(res_i32.get_i32(), 21);
    }

    #[test]
    fn test_stringify() {
        let rt = init_test_rt();
        let esvf = rt
            .eval_sync(Script::new(
                "test_stringify.es",
                "({a: 1, b: 'abc', c: true, d: {a: 1, b: 2}});",
            ))
            .ok()
            .expect("script failed");

        log::info!("test_stringify: before create map");
        let map = match esvf.get_object() {
            Ok(mp) => mp,
            Err(e) => {
                log::error!("could not create map due to {}", e);
                panic!("could not create map due to {}", e);
            }
        };
        log::info!("test_stringify: after create map");
        let a = map.get("a").unwrap();
        let b = map.get("b").unwrap();
        let c = map.get("c").unwrap();
        let d = map.get("d").unwrap();

        assert_eq!(a.stringify().ok().unwrap(), "1");
        assert_eq!(b.stringify().ok().unwrap(), "\"abc\"");
        assert_eq!(c.stringify().ok().unwrap(), "true");
        assert_eq!(d.stringify().ok().unwrap(), "{\"a\":1,\"b\":2}");
        log::info!("test_stringify: drop rt");
        drop(rt);
        log::info!("test_stringify: after drop rt");
    }

    #[test]
    fn test_error() {
        let rt = init_test_rt();
        let esvf = match rt.eval_sync(Script::new(
            "test_err.es",
            "((async function() {throw Error('poof')})());",
        )) {
            Ok(e) => e,
            Err(ex) => panic!("script failed: {}", ex),
        };
        let res = esvf.get_promise_result_sync();
        assert!(res.is_err());
        let err_esvf = res.err().unwrap();
        assert!(err_esvf.is_error());
        let es_error = err_esvf.get_error();
        assert!(es_error.get_message().contains("poof"));
    }
}
