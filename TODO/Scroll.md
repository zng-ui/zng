# Scroll TODO

* Don't invalidate render_update if no culling visibility updated.
    - In case the content is one large image for example.

* Parallax scrolling.
    - Test access to the scroll offset inside the content.

* Test scroll to end when the height changes by scrolling.
* How this ties in with virtualization? Widgets that only reserve layout space when not visible.

* Widgets may want to know what percentage of a widget is visible in the viewport, see flutter "slivers" concept.
* "Sliver" widgets may change size due to scrolling.

* Touch scrolling.
    - Need touch events first.
    - Push against end indicator.
    - Acceleration/smooth scrolling integration.