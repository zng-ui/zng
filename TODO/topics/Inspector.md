# Inspector TODO

* Flash widget that has property change.

* Implement expander.
    - Maybe collapse some inner widgets by default?

* Visually differentiate between user or widget set properties.

* Implement property search.

* Run as separate process?
    - No, difficult to plug variable observers.
    - Can add a meta flag to avoid inspecting the inspector.

* Integrate UI with tracing?
    - As in, the "performance" tab.

* Function to get source code file and line at property declaration.

* Make text selectable.

* Let widgets set custom info watchers.
    - Like `interactivity` and `visibility` in the info section of the properties panel.
    - Custom watcher is a name, category and `BoxedVar<Txt>` that shows the value. 