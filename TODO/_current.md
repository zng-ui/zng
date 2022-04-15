* Fix easing functions and modifiers.
* Fix image rendering blocking the UI.
    - Changing backend to Software improves some, we still need to make context and renderer
      creation asynchronous.
    - Reuse renderer?

* Animation, see `Variables.md`.
* Finish `Optimizations.md#Cache Everything`.