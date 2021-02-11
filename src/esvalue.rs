use crate::eserror::EsError;
use crate::esruntime::EsRuntime;
use crate::quickjs_utils::promises::PromiseRef;
use crate::quickjs_utils::{arrays, dates, functions, new_null_ref, promises};
use crate::quickjscontext::QuickJsContext;
use crate::quickjsruntime::QuickJsRuntime;
use crate::reflection;
use crate::utils::auto_id_map::AutoIdMap;
use crate::valueref::*;
use futures::task::{Context, Poll};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Error, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

pub struct EsValueFacadeFuture<R, E> {
    result: Receiver<Result<R, E>>,
    sender: Sender<Result<R, E>>,
}
impl<R, E> EsValueFacadeFuture<R, E> {
    fn new() -> Self {
        let (tx, rx) = channel();

        Self {
            result: rx,
            sender: tx,
        }
    }
    fn get_sender(&self) -> Sender<Result<R, E>> {
        self.sender.clone()
    }
}
impl<R, E> Future for EsValueFacadeFuture<R, E> {
    type Output = Result<R, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Ok(res) = self.result.try_recv() {
            Poll::Ready(res)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

pub type PromiseReactionType =
    Option<Box<dyn Fn(EsValueFacade) -> Result<EsValueFacade, EsError> + Send + 'static>>;

pub trait EsValueConvertible {
    fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError>;

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
    fn invoke_function_sync(&self, _args: Vec<EsValueFacade>) -> Result<EsValueFacade, EsError> {
        panic!("i am not a function");
    }
    fn invoke_function(
        &self,
        _args: Vec<EsValueFacade>,
    ) -> EsValueFacadeFuture<EsValueFacade, EsError> {
        panic!("i am not a function");
    }
    fn invoke_function_batch_sync(
        &self,
        _args: Vec<Vec<EsValueFacade>>,
    ) -> Vec<Result<EsValueFacade, EsError>> {
        panic!("i am not a function");
    }
    fn invoke_function_batch(&self, _args: Vec<Vec<EsValueFacade>>) -> Result<(), EsError> {
        panic!("i am not a function");
    }
    fn is_promise(&self) -> bool {
        false
    }
    fn get_promise_result_sync(
        &self,
        _timeout: Duration,
    ) -> Result<Result<EsValueFacade, EsValueFacade>, RecvTimeoutError> {
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
    ) -> Result<(), EsError> {
        panic!("i am not a promise")
    }
    fn is_object(&self) -> bool {
        false
    }
    fn get_object(&self) -> &HashMap<String, EsValueFacade> {
        panic!("i am not an object");
    }
    fn get_object_mut(&mut self) -> &mut HashMap<String, EsValueFacade> {
        panic!("i am not an object");
    }
    fn is_array(&self) -> bool {
        false
    }
    fn get_array(&self) -> &Vec<EsValueFacade> {
        panic!("i am not an array");
    }
    fn get_array_mut(&mut self) -> &mut Vec<EsValueFacade> {
        panic!("i am not an array");
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
    fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        let proxy_opt = reflection::get_proxy(q_ctx, self.class_name);
        if let Some(proxy) = proxy_opt {
            reflection::new_instance3(&proxy, self.instance_id, q_ctx)
        } else {
            Err(EsError::new_string(format!(
                "no such proxy: {}",
                self.class_name
            )))
        }
    }
}

impl EsValueConvertible for EsNullValue {
    fn as_js_value(&mut self, _q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        Ok(crate::quickjs_utils::new_null_ref())
    }

    fn is_null(&self) -> bool {
        true
    }
}

impl EsValueConvertible for EsUndefinedValue {
    fn as_js_value(&mut self, _q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        Ok(crate::quickjs_utils::new_undefined_ref())
    }

    fn is_undefined(&self) -> bool {
        true
    }
}

// placeholder for promises that were passed from the script engine to rust
struct CachedJSPromise {
    cached_obj_id: i32,
    context_id: String,
    es_rt: Weak<EsRuntime>,
}

impl Drop for CachedJSPromise {
    fn drop(&mut self) {
        if let Some(rt_arc) = self.es_rt.upgrade() {
            let cached_obj_id = self.cached_obj_id;
            let context_id = self.context_id.clone();
            let _ = rt_arc.add_to_event_queue(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id.as_str());
                q_ctx.consume_cached_obj(cached_obj_id);
            });
        }
    }
}

// placeholder for functions that were passed from the script engine to rust
struct CachedJSFunction {
    cached_obj_id: i32,
    context_id: String,
    es_rt: Weak<EsRuntime>,
}

impl Drop for CachedJSFunction {
    fn drop(&mut self) {
        if let Some(rt_arc) = self.es_rt.upgrade() {
            let cached_obj_id = self.cached_obj_id;
            let context_id = self.context_id.clone();
            let _ = rt_arc.add_to_event_queue(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id.as_str());
                q_ctx.consume_cached_obj(cached_obj_id);
            });
        }
    }
}

fn pipe_promise_resolution_to_sender(
    q_ctx: &QuickJsContext,
    prom_obj_ref: &JSValueRef,
    tx: Sender<Result<EsValueFacade, EsValueFacade>>,
) {
    let tx2 = tx.clone();
    let then_func_ref = functions::new_function_q(
        q_ctx,
        "promise_then_result_transmitter",
        move |q_ctx, _this_ref, args| {
            // these clones are needed because create_func requires a Fn and not a FnOnce
            // in practice however the Fn is called only once
            let tx3 = tx2.clone();

            let prom_res = &args[0];
            let prom_res_esvf_res = EsValueFacade::from_jsval(q_ctx, prom_res);

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
            // these clones are needed because create_func requires a Fn and not a FnOnce
            // in practice however the Fn is called only once
            let tx3 = tx.clone();

            let prom_res = &args[0];
            let prom_res_esvf_res = EsValueFacade::from_jsval(q_ctx, prom_res);
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

impl EsValueConvertible for CachedJSPromise {
    fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        let cloned_ref = q_ctx.with_cached_obj(self.cached_obj_id, |obj_ref| obj_ref.clone());
        Ok(cloned_ref)
    }

    fn is_promise(&self) -> bool {
        true
    }

    fn get_promise_result_sync(
        &self,
        timeout: Duration,
    ) -> Result<Result<EsValueFacade, EsValueFacade>, RecvTimeoutError> {
        let (tx, rx) = channel();
        let cached_obj_id = self.cached_obj_id;
        let context_id = self.context_id.clone();
        if let Some(es_rti) = self.es_rt.upgrade() {
            es_rti.add_to_event_queue_sync(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id.as_str());

                q_ctx.with_cached_obj(cached_obj_id, |prom_obj_ref| {
                    pipe_promise_resolution_to_sender(q_ctx, prom_obj_ref, tx);
                });
            });
            Ok(rx.recv_timeout(timeout)?)
        } else {
            Ok(Err("rti dropped".to_string().to_es_value_facade()))
        }
    }

    fn get_promise_result(&self) -> EsValueFacadeFuture<EsValueFacade, EsValueFacade> {
        let fut = EsValueFacadeFuture::new();
        let tx = fut.get_sender();
        let cached_obj_id = self.cached_obj_id;
        let context_id = self.context_id.clone();
        if let Some(es_rti) = self.es_rt.upgrade() {
            let _ = es_rti.add_to_event_queue(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id.as_str());
                q_ctx.with_cached_obj(cached_obj_id, |prom_obj_ref| {
                    pipe_promise_resolution_to_sender(q_ctx, prom_obj_ref, tx);
                });
            });
        } else {
            log::error!("rt was dropped");
        }

        fut
    }

    fn add_promise_reactions(
        &self,
        then: PromiseReactionType,
        catch: PromiseReactionType,
        finally: Option<Box<dyn Fn() + Send + 'static>>,
    ) -> Result<(), EsError> {
        let cached_obj_id = self.cached_obj_id;

        if let Some(es_rt) = self.es_rt.upgrade() {
            let context_id_then = self.context_id.clone();
            let _ = es_rt.add_to_event_queue(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id_then.as_str());

                q_ctx.with_cached_obj(cached_obj_id, |prom_ref| {
                    let then_ref = if let Some(then_fn) = then {
                        let then_fn_rc = Rc::new(then_fn);

                        let then_fn_raw =
                            move |q_ctx: &QuickJsContext,
                                  _this_ref: JSValueRef,
                                  mut args_ref: Vec<JSValueRef>| {
                                let then_fn_rc = then_fn_rc.clone();
                                let val_ref = args_ref.remove(0);

                                let val_esvf = EsValueFacade::from_jsval(q_ctx, &val_ref)?;

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
                        let catch_fn_raw =
                            move |q_ctx: &QuickJsContext,
                                  _this_ref,
                                  mut args_ref: Vec<JSValueRef>| {
                                let val_ref = args_ref.remove(0);
                                let catch_fn_rc = catch_fn_rc.clone();

                                let val_esvf = EsValueFacade::from_jsval(q_ctx, &val_ref)?;

                                catch_fn_rc(val_esvf)?;
                                Ok(crate::quickjs_utils::new_null_ref())
                            };
                        let t = functions::new_function_q(q_ctx, "", catch_fn_raw, 1)
                            .ok()
                            .expect("could not create function");
                        Some(t)
                    } else {
                        None
                    };

                    let finally_ref = if let Some(finally_fn) = finally {
                        let finally_fn_raw =
                            move |_q_ctx: &QuickJsContext, _this_ref, _args_ref| {
                                finally_fn();
                                Ok(crate::quickjs_utils::new_null_ref())
                            };
                        let t = functions::new_function_q(q_ctx, "", finally_fn_raw, 0)
                            .ok()
                            .expect("could not create function");
                        Some(t)
                    } else {
                        None
                    };

                    promises::add_promise_reactions_q(
                        q_ctx,
                        prom_ref,
                        then_ref,
                        catch_ref,
                        finally_ref,
                    )
                    .ok()
                    .expect("could not add reactions")
                });
            });
            Ok(())
        } else {
            Err(EsError::new_str("runtime was dropped"))
        }
    }
}

impl EsValueConvertible for CachedJSFunction {
    fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        let cloned_ref = q_ctx.with_cached_obj(self.cached_obj_id, |obj_ref| obj_ref.clone());
        Ok(cloned_ref)
    }

    fn is_function(&self) -> bool {
        true
    }

    fn invoke_function_sync(&self, mut args: Vec<EsValueFacade>) -> Result<EsValueFacade, EsError> {
        let cached_obj_id = self.cached_obj_id;
        let context_id = self.context_id.clone();
        if let Some(rt_arc) = self.es_rt.upgrade() {
            rt_arc.add_to_event_queue_sync(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id.as_str());
                q_ctx.with_cached_obj(cached_obj_id, move |obj_ref| {
                    let mut ref_args = vec![];
                    for arg in args.iter_mut() {
                        ref_args.push(arg.as_js_value(q_ctx)?);
                    }

                    crate::quickjs_utils::functions::call_function_q(q_ctx, obj_ref, ref_args, None)
                        .and_then(|js_val| EsValueFacade::from_jsval(q_ctx, &js_val))
                })
            })
        } else {
            Err(EsError::new_str("rt was dropped"))
        }
    }

    fn invoke_function(
        &self,
        mut args: Vec<EsValueFacade>,
    ) -> EsValueFacadeFuture<EsValueFacade, EsError> {
        let cached_obj_id = self.cached_obj_id;
        let context_id = self.context_id.clone();
        let ret = EsValueFacadeFuture::new();
        let tx = ret.get_sender();
        if let Some(rt_arc) = self.es_rt.upgrade() {
            let _ = rt_arc.add_to_event_queue(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id.as_str());
                q_ctx.with_cached_obj(cached_obj_id, move |obj_ref| {
                    let mut ref_args = vec![];
                    for arg in args.iter_mut() {
                        match arg.as_js_value(q_ctx) {
                            Ok(j_arg) => ref_args.push(j_arg),
                            Err(e) => {
                                log::error!(
                                    "arg conversion failed in esvalue::invoke_function: {}",
                                    e
                                );
                                return;
                            }
                        }
                    }

                    let res = crate::quickjs_utils::functions::call_function_q(
                        q_ctx, obj_ref, ref_args, None,
                    )
                    .and_then(|js_val| EsValueFacade::from_jsval(q_ctx, &js_val));
                    let _ = tx.send(res);
                });
            });
        } else {
            let _ = tx.send(Err(EsError::new_str("rt was dropped")));
        }
        ret
    }

    fn invoke_function_batch_sync(
        &self,
        batch_args: Vec<Vec<EsValueFacade>>,
    ) -> Vec<Result<EsValueFacade, EsError>> {
        let cached_obj_id = self.cached_obj_id;
        let context_id = self.context_id.clone();
        if let Some(rt_arc) = self.es_rt.upgrade() {
            rt_arc.add_to_event_queue_sync(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id.as_str());
                q_ctx.with_cached_obj(cached_obj_id, move |obj_ref| {
                    let mut res_vec: Vec<Result<EsValueFacade, EsError>> = vec![];
                    for mut args in batch_args {
                        let mut ref_args = vec![];
                        for arg in args.iter_mut() {
                            ref_args.push(arg.as_js_value(q_ctx).ok().expect("to_js_value failed"));
                        }

                        let res = crate::quickjs_utils::functions::call_function_q(
                            q_ctx, obj_ref, ref_args, None,
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
            })
        } else {
            vec![Err(EsError::new_str("rt was dropped"))]
        }
    }

    // todo rewrite to Future
    fn invoke_function_batch(&self, batch_args: Vec<Vec<EsValueFacade>>) -> Result<(), EsError> {
        let cached_obj_id = self.cached_obj_id;
        let context_id = self.context_id.clone();
        if let Some(rt_arc) = self.es_rt.upgrade() {
            let _ = rt_arc.add_to_event_queue(move |q_js_rt| {
                let q_ctx = q_js_rt.get_context(context_id.as_str());
                q_ctx.with_cached_obj(cached_obj_id, move |obj_ref| {
                    for mut args in batch_args {
                        let mut ref_args = vec![];
                        for arg in args.iter_mut() {
                            ref_args
                                .push(arg.as_js_value(q_ctx).ok().expect("could not convert arg"));
                        }

                        let res = crate::quickjs_utils::functions::call_function_q(
                            q_ctx, obj_ref, ref_args, None,
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
                })
            });
            Ok(())
        } else {
            Err(EsError::new_str("rt was dropped"))
        }
    }
}

impl EsValueConvertible for String {
    fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        crate::quickjs_utils::primitives::from_string_q(q_ctx, self.as_str())
    }

    fn is_str(&self) -> bool {
        true
    }

    fn get_str(&self) -> &str {
        self.as_str()
    }
}

impl EsValueConvertible for i32 {
    fn as_js_value(&mut self, _q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
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
    fn as_js_value(&mut self, _q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
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
    fn as_js_value(&mut self, _q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
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
    fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        // create the array

        let arr = crate::quickjs_utils::arrays::create_array_q(q_ctx)
            .ok()
            .unwrap();

        // add items
        for index in 0..self.len() {
            let item = self.get_mut(index).unwrap();

            let item_val_ref = item.as_js_value(q_ctx)?;

            crate::quickjs_utils::arrays::set_element_q(q_ctx, &arr, index as u32, item_val_ref)?;
        }
        Ok(arr)
    }

    fn is_array(&self) -> bool {
        true
    }

    fn get_array(&self) -> &Vec<EsValueFacade> {
        self
    }
    fn get_array_mut(&mut self) -> &mut Vec<EsValueFacade> {
        self
    }
}

impl EsValueConvertible for HashMap<String, EsValueFacade> {
    fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
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

    fn is_object(&self) -> bool {
        true
    }

    fn get_object(&self) -> &HashMap<String, EsValueFacade> {
        self
    }
    fn get_object_mut(&mut self) -> &mut HashMap<String, EsValueFacade> {
        self
    }
}

pub type EsPromiseResolver = Box<dyn FnOnce() -> Result<EsValueFacade, String> + Send + 'static>;

thread_local! {
    static ESPROMISE_REFS: RefCell<AutoIdMap<PromiseRef>> = RefCell::new(AutoIdMap::new());
}

struct EsPromiseResolvableHandleInfo {
    weak_es_rt: Weak<EsRuntime>,
    id: usize,
    context_id: String,
}

struct EsPromiseResolvableHandleInner {
    js_info: Option<EsPromiseResolvableHandleInfo>,
    resolution: Option<Result<EsValueFacade, EsValueFacade>>,
}

pub struct EsPromiseResolvableHandle {
    inner: Mutex<EsPromiseResolvableHandleInner>,
}

impl EsPromiseResolvableHandle {
    fn new() -> Self {
        Self {
            inner: Mutex::new(EsPromiseResolvableHandleInner {
                js_info: None,
                resolution: None,
            }),
        }
    }

    fn with_inner<C, R>(&self, consumer: C) -> R
    where
        C: FnOnce(&mut EsPromiseResolvableHandleInner) -> R,
    {
        let mut lck = self.inner.lock().unwrap();
        consumer(&mut *lck)
    }

    pub fn resolve(&self, mut value: EsValueFacade) {
        log::debug!("resolving handle with val: {:?}", value);
        self.with_inner(|inner| {
            if let Some(info) = &inner.js_info {
                // resolve
                let id = info.id;
                let context_id = info.context_id.clone();
                if let Some(es_rt) = info.weak_es_rt.upgrade() {
                    let _ = es_rt.add_to_event_queue(move |q_js_rt| {
                        let q_ctx = q_js_rt.get_context(context_id.as_str());
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
                    });
                }
            } else {
                // resolve later when converted to JSValue
                inner.resolution = Some(Ok(value));
            }
        })
    }
    pub fn reject(&self, mut value: EsValueFacade) {
        log::debug!("rejecting handle with val: {:?}", value);
        self.with_inner(|inner| {
            if let Some(info) = &inner.js_info {
                // resolve
                let id = info.id;
                let context_id = info.context_id.clone();
                if let Some(es_rt) = info.weak_es_rt.upgrade() {
                    let _ = es_rt.add_to_event_queue(move |q_js_rt| {
                        let q_ctx = q_js_rt.get_context(context_id.as_str());
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
                    });
                }
            } else {
                // resolve later when converted to JSValue
                inner.resolution = Some(Err(value));
            }
        })
    }
    fn set_info(&self, es_rt: &Arc<EsRuntime>, id: usize, context_id: &str) -> Result<(), EsError> {
        self.with_inner(|inner| {
            if inner.js_info.is_some() {
                Err(EsError::new_str("info was already set"))
            } else {
                // set info
                inner.js_info = Some(EsPromiseResolvableHandleInfo {
                    weak_es_rt: Arc::downgrade(es_rt),
                    id,
                    context_id: context_id.to_string(),
                });

                if let Some(resolution) = inner.resolution.take() {
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
        })
    }
}

impl Drop for EsPromiseResolvableHandleInner {
    fn drop(&mut self) {
        if let Some(info) = &self.js_info {
            let id = info.id;
            if let Some(es_rt) = info.weak_es_rt.upgrade() {
                let _ = es_rt.add_to_event_queue(move |_q_js_rt| {
                    ESPROMISE_REFS.with(move |rc| {
                        let map = &mut *rc.borrow_mut();
                        map.remove(&id);
                    });
                });
            }
        }
    }
}

/// can be used to create a new Promise which is resolved with the resolver function
/// # Example
/// ```rust
/// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
/// use quickjs_runtime::esvalue::{EsPromise, EsValueConvertible};
/// use std::time::Duration;
/// use quickjs_runtime::esscript::EsScript;
/// use log::LevelFilter;
///
/// let rt = EsRuntimeBuilder::new().build();
/// rt.set_function(vec!["my", "comp"], "create_prom", |_q_ctx, _args| {
///     Ok(EsPromise::new(|| {
///         std::thread::sleep(Duration::from_secs(1));
///         Ok(9463.to_es_value_facade())
///     }).to_es_value_facade())
/// });
/// rt.eval_sync(EsScript::new("test_prom.es", "let p765 = my.comp.create_prom(); p765.then((p_res) => {console.log('got ' + p_res)});")).ok().expect("script failed");
/// std::thread::sleep(Duration::from_secs(2));
/// ```
pub struct EsPromise {
    handle: Arc<EsPromiseResolvableHandle>,
}

impl EsPromise {
    pub fn new<R>(resolver: R) -> Self
    where
        R: FnOnce() -> Result<EsValueFacade, String> + Send + 'static,
    {
        let ret = Self::new_unresolving();

        let handle = ret.get_handle();
        EsRuntime::add_helper_task(move || {
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
    /// create a new Promise which will be resolved later
    /// this achieved by creating a Handle which is wrapped in an Arc and thus may be passed to another thread
    /// # Example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// use std::time::Duration;
    /// use quickjs_runtime::esvalue::{EsPromise, EsValueConvertible};
    /// let rt = EsRuntimeBuilder::new().build();
    /// // prep a function which reacts to a promise
    /// rt.eval_sync(EsScript::new("new_unresolving.es", "this.new_unresolving = function(prom){prom.then((res) => {console.log('promise resolved to %s', res);});};")).ok().expect("script failed");
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
    fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        log::trace!("EsPromise::to_js_value");

        let prom_ref = promises::new_promise_q(q_ctx)?;

        let ret = prom_ref.get_promise_obj_ref();
        let id = ESPROMISE_REFS.with(move |rc| {
            let map = &mut *rc.borrow_mut();
            map.insert(prom_ref)
        });
        let es_rt = QuickJsRuntime::do_with(|q_js_rt| q_js_rt.get_rt_ref().unwrap());

        self.handle.set_info(&es_rt, id, q_ctx.id.as_str())?;

        Ok(ret)
    }
}

pub struct EsValueFacade {
    convertible: Box<dyn EsValueConvertible + Send + 'static>,
}

impl EsValueFacade {
    pub fn as_js_value(&mut self, q_ctx: &QuickJsContext) -> Result<JSValueRef, EsError> {
        self.convertible.as_js_value(q_ctx)
    }

    pub fn from_jsval(q_ctx: &QuickJsContext, value_ref: &JSValueRef) -> Result<Self, EsError> {
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
            TAG_OBJECT => {
                if promises::is_promise_q(q_ctx, value_ref) {
                    let es_rt = QuickJsRuntime::do_with(|q_js_rt| q_js_rt.get_rt_ref().unwrap());
                    let cached_obj_id = q_ctx.cache_object(value_ref.clone());
                    Ok(CachedJSPromise {
                        cached_obj_id,
                        context_id: q_ctx.id.clone(),
                        es_rt: Arc::downgrade(&es_rt),
                    }
                    .to_es_value_facade())
                } else if arrays::is_array_q(q_ctx, value_ref) {
                    Self::from_jsval_array(q_ctx, value_ref)
                } else if functions::is_function_q(q_ctx, value_ref) {
                    let cached_obj_id = q_ctx.cache_object(value_ref.clone());
                    let es_rt = QuickJsRuntime::do_with(|q_js_rt| q_js_rt.get_rt_ref().unwrap());
                    let cached_func = CachedJSFunction {
                        cached_obj_id,
                        context_id: q_ctx.id.clone(),
                        es_rt: Arc::downgrade(&es_rt),
                    };
                    Ok(cached_func.to_es_value_facade())
                } else if dates::is_date_q(q_ctx, value_ref)? {
                    Err(EsError::new_str("dates are currently not supported"))
                } else {
                    Self::from_jsval_object(q_ctx, value_ref)
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
        q_ctx: &QuickJsContext,
        value_ref: &JSValueRef,
    ) -> Result<EsValueFacade, EsError> {
        assert!(value_ref.is_object());

        let len = crate::quickjs_utils::arrays::get_length_q(q_ctx, value_ref)?;

        let mut values = Vec::new();
        for index in 0..len {
            let element_ref = crate::quickjs_utils::arrays::get_element_q(q_ctx, value_ref, index)?;

            let element_value = EsValueFacade::from_jsval(q_ctx, &element_ref)?;

            values.push(element_value);
        }

        Ok(values.to_es_value_facade())
    }

    fn from_jsval_object(
        q_ctx: &QuickJsContext,
        obj_ref: &JSValueRef,
    ) -> Result<EsValueFacade, EsError> {
        assert!(obj_ref.is_object());

        let map =
            crate::quickjs_utils::objects::traverse_properties_q(q_ctx, obj_ref, |_key, val| {
                EsValueFacade::from_jsval(q_ctx, &val)
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

    pub fn get_array_mut(&mut self) -> &mut Vec<EsValueFacade> {
        self.convertible.get_array_mut()
    }

    /// get the object value
    pub fn get_object(&self) -> &HashMap<String, EsValueFacade> {
        self.convertible.get_object()
    }

    pub fn get_object_mut(&mut self) -> &mut HashMap<String, EsValueFacade> {
        self.convertible.get_object_mut()
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

    pub fn is_null(&self) -> bool {
        self.convertible.is_null()
    }
    pub fn is_undefined(&self) -> bool {
        self.convertible.is_undefined()
    }

    pub fn invoke_function_sync(
        &self,
        arguments: Vec<EsValueFacade>,
    ) -> Result<EsValueFacade, EsError> {
        self.convertible.invoke_function_sync(arguments)
    }
    pub async fn invoke_function(
        &self,
        arguments: Vec<EsValueFacade>,
    ) -> Result<EsValueFacade, EsError> {
        self.convertible.invoke_function(arguments).await
    }
    pub fn invoke_function_batch_sync(
        &self,
        arguments: Vec<Vec<EsValueFacade>>,
    ) -> Vec<Result<EsValueFacade, EsError>> {
        self.convertible.invoke_function_batch_sync(arguments)
    }
    pub fn invoke_function_batch(&self, arguments: Vec<Vec<EsValueFacade>>) -> Result<(), EsError> {
        self.convertible.invoke_function_batch(arguments)
    }
    pub fn get_promise_result_sync(
        &self,
        timeout: Duration,
    ) -> Result<Result<EsValueFacade, EsValueFacade>, RecvTimeoutError> {
        self.convertible.get_promise_result_sync(timeout)
    }

    /// wait for the result of a Promise async
    /// # Example
    /// ```rust
    /// use quickjs_runtime::esruntimebuilder::EsRuntimeBuilder;
    /// use quickjs_runtime::esscript::EsScript;
    /// use futures::executor::block_on;
    /// use quickjs_runtime::esvalue::EsValueFacade;
    /// pub async fn test_async(esvf: EsValueFacade) -> i32 {
    ///    let prom_res = esvf.get_promise_result().await;
    ///    let res_esvf = prom_res.ok().unwrap();
    ///    return res_esvf.get_i32();
    /// }
    ///
    /// let rt = EsRuntimeBuilder::new().build();
    /// let esvf = rt.eval_sync(EsScript::new("test_async_prom,es", "(new Promise((resolve, reject) => {setTimeout(() => {resolve(1360)}, 1000);}));")).ok().expect("script failed");
    /// let i = block_on(test_async(esvf));
    /// assert_eq!(i, 1360);
    ///
    /// ```
    pub async fn get_promise_result(&self) -> Result<EsValueFacade, EsValueFacade> {
        self.convertible.get_promise_result().await
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
    use crate::esruntime::tests::TEST_ESRT;
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::esvalue::EsValueFacade;
    use futures::executor::block_on;
    use std::sync::Arc;
    use std::time::Duration;

    async fn test_async_func1(esvf: EsValueFacade) -> i32 {
        let res = esvf.invoke_function(vec![]).await;
        let esvf = res.ok().expect("func failed");
        esvf.get_i32()
    }

    #[test]
    fn test_async_func() {
        let rt: Arc<EsRuntime> = TEST_ESRT.clone();
        let func_esvf = rt
            .eval_sync(EsScript::new(
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
        let rt: Arc<EsRuntime> = TEST_ESRT.clone();
        let res = rt.eval_sync(EsScript::new(
            "test_promise.es",
            "(new Promise(function(resolve, reject){resolve(537);}));",
        ));
        match res {
            Ok(esvf) => {
                assert!(esvf.is_promise());
                let res = esvf.get_promise_result_sync(Duration::from_secs(1));
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
