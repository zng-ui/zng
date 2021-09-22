# Proc-Macros TODO

Proc-macros are mostly implemented, there are some improvements we can make:

* Profile build time.
* Improve property `allowed_in_when` validation for generics, generate a `new` like call for each
  argument, instead of all at once.
* Study viability of `widget_bind_self.rs`.
* Support doc(cfg).
* Support cfg in captures.

# Difficult

* Figure out a way to enable auto-complete and hover tooltip inside macro code?
* Pre-build to wasm: 
    Need $crate support, or to be able to read cargo.toml from wasm,
    both aren't natively supported with [`watt`](https://crates.io/crates/watt).