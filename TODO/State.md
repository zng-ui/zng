# State TODO

Contextual state TODOs.

* Invert state key maps to be held in a thread-local for each key type? avoids value boxing
* Generate accessor traits like those generated for services.
    - If implemented `ctx.window_state.req(WindowVarsKey)` becomes `ctx.window_state.window_vars()`.
* Strong types for each `StateMap`, we should only be able to query for `WindowVars` in window state map.
* State serialization.
    - Support mixing serializable and not.