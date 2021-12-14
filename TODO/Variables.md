# Var TODO

Variable API is mostly implemented, need to integrate animation and review performance.

# ContextVar Write

Currently `ContextVar` variables are always read-only, it would be useful to allow `modify` to have children widgets configure the parent,
it would allow us to split the `scrollable!` context into read-write context variables instead of transporting RcVars inside a "context" value.

```rust
impl Vars {
    pub fn with_context_var_write<C, R, F>(&self, context_var: C, data: ContextVarData<C::Type>, f: F) -> R
    where
        C: ContextVar,
        F: FnOnce() -> R,
    {
        todo!()
    }
}
```

# Animation

When a variable is set the new value should be available *immediately* in the next app update. But we may want to implement *easing* that transitions between the previous value and the next. The idea is to extend the `Var` trait to support *get_animating* that returns the intermediary animated value between the two values.

Normal variables (the current ones) just return the new value also, because they are without *easing*, but we can have new `AnimatingVar` or something, that can have easing configuration and provides intermediary values.