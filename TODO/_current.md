* Avoid property binding when value is not animating, webrender invalidates cache if is bound, see `prepare_interned_prim_for_render`.
* Integrate frame reuse with frame update, see `Optimizations.md`.
    - Can we have our own "binding" in view-api?
    ```rust
    enum FrameBinding {
        /// Bound value, if updated causes a new webrender frame, but view-api is only an update request.
        Binding(Id, Value),
        /// Like binding, but is not cached in webrender, (actual webrender frame update).
        Animating(Id, Value),
        /// Not bound value.
        Value(Value),
    }
    ```
    - Quote form webrender code:
    ```rust
    PropertyBinding::Binding(..) => {
        // Animated, so assume it may introduce a complex transform
        true
    }
    ```


* Implement virtualization, see `Optimizations.md`.
* Finish state API, see `State.md`.