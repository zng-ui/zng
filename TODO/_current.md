* Merge.
* Update webrender to Firefox-109 version.

* Text direction can't always be derived from language.
    - See https://www.w3.org/International/questions/qa-direction-from-language
    - Html requires explicit direction to work.

* Review bidi text across inlined widgets.
    - Test how HTML does it, with spans of 

* Implement vertical text.
    - Need to expand `LayoutDirection` to define chars and lines direction.
    - Or a different layout property for the "lines" direction.
    - See `./Layout.md#Direction`.

* Continue "#Parallel UI" in `./Performance.md`.
    - Refactor services into `app_local!` backed structs, with associated functions.
        - Remove `Services`.

* Review all docs.
    - Mentions of threads in particular.