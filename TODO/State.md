# State TODO

Contextual state TODOs.

* Invert state key maps to be held in a thread-local for each key type? avoids value boxing
* Strong types for each `StateMap`, we should only be able to query for `WindowVars` in window state map.
* State serialization.
    - Support mixing serializable and not.