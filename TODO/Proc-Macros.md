# Proc-Macros TODO

Proc-macros are mostly implemented, there are some improvements we can make:

* Review setting inherited child property not in `child { }` block, got confused trying to set `padding` in the border example.
* Improve property `allowed_in_when` validation for generics, generate a `new` like call for each
  argument, instead of all at once.
* Study viability of `widget_bind_self.rs`.
* Support doc(cfg).
* Support cfg in captures.
* Allow "property as new_name" syntax in widget_new? Can be used for things like double fancy borders.
* Use `get_` prefix for properties that only return a value, can teach as inverse of normal accessor methods where setting uses the `set_` prefix
and getting only uses the name directly.

# Difficult

* Figure out a way to enable auto-complete and hover tooltip inside macro code?
* Pre-build to wasm: 
    Need $crate support, or to be able to read cargo.toml from wasm,
    both aren't natively supported with [`watt`](https://crates.io/crates/watt).