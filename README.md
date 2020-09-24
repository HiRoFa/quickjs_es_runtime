# quick_es_runtime

Same goals as https://github/com/DRFos/es_runtime but with using quickjs

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

# next steps
* get promise result blocking
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
    let rt = EsRuntimeBuilder::new().module_script_loader(load_module).build();

    // eval some basic stuff

    rt.eval_sync(EsScript::new("basics.es", "this.my_app_utils = {f: function(a, b){return a * b;}};"));

    // invoke a js method from rust

    let a = 8.to_es_value_facade();
    let b = 7.to_es_value_facade();
    let res = rt.call_function_sync(vec!["my_app_utils"], "f", vec![a, b]);
    match res {
        Ok(val) => {
            println!("got {}", val.get_i32())
        },
        Err(e) => println!("script failed: {}", e)
    }

}

```
