# quick_es_runtime

Same goals as es_runtime but with using quickjs

so 
* slower js 
* smaller footprint 
* much faster compilation

For some of my projects those are a big plus!

A big thanks goes out to the people at https://github.com/theduke/quickjs-rs for making the bindings to quickjs!

I learned a lot from their code and reused some of their concepts.

# status

you can eval code,

there are utils to
* edit objects
* call and create functions
* convert primitives
* a lot more

Promises resolve async

# next steps

* Modules
* ClassBuilder (reflection)
* getting promise result sync
* invoking callbacks from EsValueFacade
* a lot more

basicly getting it to the state where es_runtime is in


