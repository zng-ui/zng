* Finish smooth scrolling.
  - Chase animation, right now next scroll pos. calculated from var current value, need to record next value and compute from there.
    - See `Variables.md`.
  - Can we abstract this as a method in `Var`, seems useful.
  - Implement `smooth_scrolling` config property.

* Animation, see `Variables.md`.
* Simplify layout, see `Optimizations.md#Single Pass Layout`.