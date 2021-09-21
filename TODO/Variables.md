# Var TODO

Variable API is mostly implemented, need to integrate animation and review performance.

* Review map-parse.

# Animation

When a variable is set the new value should be available *immediately* in the next app update. But we may want to implement *easing* that transitions between the previous value and the next. The idea is to extend the `Var` trait to support *get_animating* that returns the intermediary animated value between the two values.

Normal variables (the current ones) just return the new value also, because they are without *easing*, but we can have new `AnimatingVar` or something, that can have easing configuration and provides intermediary values.