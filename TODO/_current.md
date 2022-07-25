* Avoid property binding when value is not animating, webrender invalidates cache if is bound, see `prepare_interned_prim_for_render`.
    - Quote form webrender code:
    ```rust
    PropertyBinding::Binding(..) => {
        // Animated, so assume it may introduce a complex transform
        true
    }
    ```
    - Just tried preliminary profile of this, only slight gain -1ms in icon-release-lto, will profile window for comparison,
    - If we add the first full frame in the beginning of each animation we lose performance.
    - Might be worth to keep the new `enum` anyway?

* Integrate frame reuse with frame update, see `Optimizations.md`.


* Implement virtualization, see `Optimizations.md`.
* Finish state API, see `State.md`.