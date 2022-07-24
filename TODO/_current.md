* Integrate frame reuse with frame update, see `Optimizations.md`.
* Avoid property binding when value is not animating, webrender invalidates cache if is bound, see `prepare_interned_prim_for_render`.
* Implement virtualization, see `Optimizations.md`.
* Finish state API, see `State.md`.