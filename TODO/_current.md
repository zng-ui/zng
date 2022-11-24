* Animation lag when text cursor is blinking.
* Alt scope not working in focus example.
    - Disable window in overlay also failed once (stop responding? or did not render?)
* Transitions from light theme to dark on init.
    - Disable animations when changing theme?
* Easing in `line_spacing` ambiguous import.

* Continue "#Parallel UI" in `./Performance.md`.
* Review all docs.
    - Mentions of threads in particular.

# Other

* Implement window `modal`.
    - Mark the parent window as not interactive.
    - Focus modal child when the parent gets focused.
    - This is not the full Windows modal experience, the user can still interact with the parent chrome?
        - Can we disable resize, minimize, maximize?