# Variables TODO

* `ContextualizedVar` can get very large.
    - `FONT_PALETTE_VAR` for example, is mapped from `COLOR_SCHEME_VAR` but otherwise not set.
       In inspector screen with many text widgets it can grow to thousands of "actual" values, all for
       the same mapped var.
    - The `FONT_COLOR_VAR` maps from `COLOR_SCHEME_VAR` by default, and it is used in the same places,
      but because it is set directly for the text (in the inspector AnsiText) they end-up being cheaper
      than `FONT_PALETTE_VAR` that is not even set.
    - Inspecting the button example generates **1321** contextual init calls for `FONT_PALETTE_VAR`.
        - The second largest is **2**.
    - Rethink `ContextInitHandle`, maybe each var can identify context dependencies?
        - No, `ContextualizedVar` is a closure, could depend on anything.
    - For now we set `font_palette` in the `Window!`.
        - This causes it to be actualized once on init by `with_context_var`.
        - This reduces the init calls to **1**.
        - Despite the large change we could not observe any performance impact.
    - The `DIRECTION_VAR` is mapped from `LANG_VAR` same issue.
    - Maybe we can have an special map for the context-var defaults at least?

* Try to use sleep for `Var::steps`, right now it runs hot trying to match the step.
* Implement more oscillate animations.

## Storyboard/Sequencer

A way to coordinate multiple animations together. Starting animations inside other animations already gives then all the same animation handle, need to implement some helpers.

* Animation blending, that is when two animations overlap in time, one affects the value less the other more across the
 overlap period. Not sure if this can be done automatically, maybe we need a sequencer builder that computes this stuff.

## Other Property Attributes

* Trace?