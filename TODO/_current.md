* Implement inline layout.
    - Just flow LTR for now.
    - `text!`.
        - Text shape wrap.
    - Properties
        - Border can impl as a polygonal outline.
            - Need to impl path rendering first?
                - Not sure if all styles are supported.
        - Review other visual properties. `fill_node` implemented, so background+foreground already done.

* Implement inline info in bounds info.
* Implement `LayoutDirection` for `flow!`.

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?