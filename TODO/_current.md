* Widget proc-macros bug, default properties with `#[cfg(active_only_in_widget_crate)` are not set in user crate if the
  user crate does not have the same cfg flag.

* Finish smooth scrolling.
  - Chase animation, right now next scroll pos. calculated from var current value, need to record next value and compute from there.
  - Can we abstract this as a method in `Var`, seems useful.
  - Implement `smooth_scrolling` config property.

* Build Optimization, see `Optimizations.md`.
* Animation, see `Variables.md`.