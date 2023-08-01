# Variables TODO

* `ContextualizedVar` can get very large.
    - `FONT_PALETTE_VAR` for example, is mapped from `COLOR_SCHEME_VAR` but otherwise not set.
       In inspector screen with many text widgets it can grow to thousands of "actual" values, all for
       the same mapped var.
    - The `DIRECTION_VAR` is mapped from `LANG_VAR` same issue.
    - Inspecting the button example generates **1321** contextual init calls for `FONT_PALETTE_VAR`.
        - The second largest is **2**.
    - Rethink `ContextInitHandle`, maybe each var can identify context dependencies?
        - No, `ContextualizedVar` is a closure, could depend on anything.
    - For now we set `font_palette` in the `Window!`.
        - This causes it to be actualized once on init by `with_context_var`.
        - This reduces the init calls to **1**.
        - Despite the large change we could not observe any performance impact.
    - Maybe we can have an special map for the context-var defaults at least?
    - Map variables could be local cached value with version checking?
        - Only the map closure cloned.
        - This might mean a map closure is applied more than once.
        - We mostly don't share map vars anyway and the `ContextInitHandle` changes so often that
          we are almost always running the map closure multiple times anyway.
        - Does not work, source var may be contextualized and map var may be used in different contexts (by a context var).
    - ContextualizedVar could be a local var cache?
        - Each clone only has one actualized backing var and `ContextInitHandle`.
        - This causes the variable to re-map every read if it is shared (by a context var).

* Try to use sleep for `Var::steps`, right now it runs hot trying to match the step.
* Implement more oscillate animations.

## Storyboard/Sequencer

A way to coordinate multiple animations together. Starting animations inside other animations already gives then all the same animation handle, need to implement some helpers.

* Animation blending, that is when two animations overlap in time, one affects the value less the other more across the
 overlap period. Not sure if this can be done automatically, maybe we need a sequencer builder that computes this stuff.

## Other Property Attributes

* Trace?