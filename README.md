# quick_es_runtime

Same goals as https://github.com/DRFos/es_runtime but with using quickjs

so 
* slower js

but
 
* smaller footprint 
* much faster compilation

For some of my projects those are a big plus!

A big thanks goes out to the people at https://github.com/theduke/quickjs-rs for making the bindings to quickjs!
I learned a lot from their code and reused some of their concepts.

# status

you can eval code, modules

there are utils for
* Editting objects
* Create and call functions
* Convert primitives
* ClassBuilder (reflection)
* Create Promises and add reactions to promises
* Modules (static and dynamic imports)
* Invoking callbacks from EsValueFacade
* get promise result blocking

# next steps
* add promise reactions
* a lot more

basicly getting it to the state where es_runtime is in

# examples

Cargo.toml

```toml
[dependencies]
quick_es_runtime = {git="https://github.com/DRFos/quick_es_runtime"}
log = "0.4.11"
simple-logging = "2.0.2"
```

main.rs

```rust

use quick_es_runtime::esruntimebuilder::EsRuntimeBuilder;
use quick_es_runtime::esscript::EsScript;
use quick_es_runtime::esvalue::EsValueFacade;

fn load_module(base: &str, name: &str) -> Option<EsScript> {
    // you should load your modules from files here
    // please note that you need to return the name as absolute_path in the returned script struct
    // return None if module is not found
    Some(EsScript::new(name, "export const foo = 12;"))
}

fn main() {
    simple_logging::log_to_stderr(LevelFilter::Info);

    let rt = EsRuntimeBuilder::new()
        .module_script_loader(load_module)
        .build();

    // eval some basic stuff

    rt.eval_sync(EsScript::new(
        "basics.es",
        "this.my_app_utils = {f: function(a, b){return a * b;}, f2: function(rf){rf(12);}};",
    ))
    .ok()
    .expect("basics.es failed");
}
```

## invoke a js method from rust

```rust
    let a = 8.to_es_value_facade();
    let b = 7.to_es_value_facade();
    let res = rt.call_function_sync(vec!["my_app_utils"], "f", vec![a, b]);
    match res {
        Ok(val) => log::info!("8*7 in JavaScript = {}", val.get_i32()),
        Err(e) => println!("script failed: {}", e),
    }
```

## add a function from rust and invoke it

```rust
    rt.set_function(vec!["nl", "my", "utils"], "methodA", |args| {
        if args.len() != 2 || !args.get(0).unwrap().is_i32() || !args.get(1).unwrap().is_i32() {
            Err(EsError::new_str(
                "i'd really like 2 args of the int32 kind please",
            ))
        } else {
            let a = args.get(0).unwrap().get_i32();
            let b = args.get(1).unwrap().get_i32();
            log::info!("rust is multiplying {} and {}", a, b);
            Ok((a * b).to_es_value_facade())
        }
    })
    .ok()
    .expect("set_function failed");

    let method_a_res = rt.eval_sync(EsScript::new(
        "test_func.es",
        "(nl.my.utils.methodA(13, 56));",
    ));

    match method_a_res {
        Ok(val) => {
            assert!(val.is_i32());
            assert_eq!(val.get_i32(), 13 * 56);
        }
        Err(e) => {
            panic!("test_func.es failed: {}", e);
        }
    }
```

## eval a module

```rust
    rt.eval_module_sync(EsScript::new(
        "my_app.mes",
        "\
    import {foo} from 'example.mes';\
    console.log('static foo is ' + foo);\
    ",
    ))
    .ok()
    .expect("module failed");
```

## eval a module with a dynamic import

```rust
    
    rt.eval_module_sync(EsScript::new(
        "my_app2.es",
        "\
    import('example.mes')\
    .then((example_module) => {\
        console.log('dynamic foo is ' + example_module.foo);\
    });\
    ",
    ))
    .ok()
    .expect("script failed");
```

##  get a function from js and invoke it in rust

```rust
    rt.set_function(vec!["nl", "my", "utils"], "methodB", |mut args| {
        if args.len() != 1 || !args.get(0).unwrap().is_function() {
            Err(EsError::new_str(
                "i'd really like 1 arg of the function kind please",
            ))
        } else {
            let consumer_func = args.remove(0);

            // invoke the func async, just because we can
            std::thread::spawn(move || {
                let a = 19.to_es_value_facade();
                let b = 17.to_es_value_facade();
                consumer_func
                    .invoke_function(vec![a, b])
                    .ok()
                    .expect("func failed");
            });

            Ok(quick_es_runtime::esvalue::EsNullValue {}.to_es_value_facade())
        }
    })
    .ok()
    .expect("set_function failed");

    rt.eval_sync(EsScript::new(
        "test_func2.es",
        "(nl.my.utils.methodB(function(a, b){console.log('consumer was called with ' +a + ', ' + b);}));",
    )).ok().expect("test_func2.es failed");

    // wait a sec for the async onvoker to run
    std::thread::sleep(Duration::from_secs(1));
```
