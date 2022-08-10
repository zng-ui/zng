* Text Editable
    - Caret.
    - Selection.
* `text_input!`.
    - Inherit from `text!`.
    - Appearance of a text-box.
* IME.
* `LineBreakVar`.
    - When char is `\n` or `\r` read this var and insert it instead. 
    - Review https://en.wikipedia.org/wiki/Newline


* Review, can we remove outer-transform/bounds?
    - It is updated in `push_inner` anyway.
    - It halves the computation of bounds if removed.
    - Can layout be done without `outer_offset`?
        - If not we can still remove the outer transform, outer offset remains just a vector for the layout pass only.
        - Outer transform is used to update descendants during reuse.
        - Can store relative transform from `push_widget` to `push_inner`, private just for the update.
            - Almost same cost, just loses bounds?
    - Can the transform be lazy, save a ref to the parent info in each child, walk up the tree to compute transform.
        - Still needs to iterate over all descendants to invalidate cache.
        - Not worth-it.