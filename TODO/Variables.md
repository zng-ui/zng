# Variables TODO

* `ContextualizedVar` can get very large.
    - `FONT_PALETTE_VAR` for example, is mapped from `COLOR_SCHEME_VAR` but otherwise not set.
       In screens with many text widgets it can group to thousands of "actual" values, all for
       the same mapped var.
* Try to use sleep for `Var::steps`, right now it runs hot trying to match the step.
* Implement more oscillate animations.

## Storyboard/Sequencer

A way to coordinate multiple animations together. Starting animations inside other animations already gives then all the same animation handle, need to implement some helpers.

* Animation blending, that is when two animations overlap in time, one affects the value less the other more across the
 overlap period. Not sure if this can be done automatically, maybe we need a sequencer builder that computes this stuff.

## Other Property Attributes

* Trace?