# Inspector TODO

* Implement inspector UI.
* Run as separate process?
    - No, difficult to plug variable observers.
    - Can add a meta flag to avoid inspecting the inspector.
* Integrate UI with tracing?
    - As in, the "performance" tab.
* Variables are not always resolved in the right context.
    - We use the info context captured in the inspector outer node, the widget may use a different context.