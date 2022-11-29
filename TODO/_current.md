* Implement inline layout.
    - Just flow LTR for now.
    - `wrap!`.
        - Optional clip inline items?
    - `text!`.
        - Text shape wrap.
    - Properties like border, margin?
        - Test other frameworks.
            - WPF has an special base widget for "run" items.
            - CSS generate a multiple clipped borders effect, does not look good.
        - As a first pass we can use `disable_inline` in all properties that affect layout?
            - This also marks properties for inline impl later.
        - Properties that render need to be aware too?
            - Or we need to impl `wrap::clip_inline` first.
        - We can impl inline background using two clips.
            - This sets the general tone of the widget being a single contiguous unit, not like CSS where the widget gets split into multiple lines.
        - Border can impl as a polygonal outline.
            - Need to impl path rendering first?
                - Not sure if all styles are supported.
        - Margin can add to the inline_advance?
            - CSS ignores the vertical, but we could have it?
        - What about min-max-size.
            - Lets disable for now.

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