# quick_es_runtime

Same goals as es_runtime but with using quickjs

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
* editting objects
* create and call functions
* convert primitives
* ClassBuilder (reflection)
* create Promises and add reactions to promises
* Modules (static and dynamic imports)

# next steps

* invoking callbacks from EsValueFacade
* get promise result blocking
* a lot more

basicly getting it to the state where es_runtime is in


