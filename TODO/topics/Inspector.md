# Inspector TODO

* Flash widget that has property change.

* Implement expander.
    - Maybe collapse some inner widgets by default?

* Implement property search.

* Run as separate process?
    - No, difficult to plug variable observers.
    - Can add a meta flag to avoid inspecting the inspector.

* Integrate UI with tracing?
    - As in, the "performance" tab.

* Variables are not always resolved in the right context.
    - We use the info context captured in the inspector outer node, the widget may use a different context.

* Function to get source code file and line at property declaration.