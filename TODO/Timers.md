# Timers TODO

There is two timers, one tied to the app thread and one for async parallel tasks, both are
implemented.

* Unify with app handlers (missing on_interval docs).
* Configurable `Instant::now` source, to advance time instantly in tests.
* Time scale, for recording?