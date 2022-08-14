# Themes TODO

* Themable mix-in?
    - Can use doc-hidden properties for each priority.
    - Context priority can be the visible `theme` property.
    * Advantages:
        - Can inherit any widget that is not themable and make it themable.
        - We don't want `text!` to be themable, but we want `text_input!`, and that inherits from `text!`.
            - Worth implementing just for this.
    * Disadvantages:
        - Order or theme property and widget properties of the same priority is not as clearly defined as in a parent.

* Implement a `button_theme` that inherits from `theme`.