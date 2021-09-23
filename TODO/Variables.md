# Var TODO

Variable API is mostly implemented, need to integrate animation and review performance.

* Only notify update if the variable is shared?
    We tried to use the `Var::strong_count` but that breaks the `countdown` example because we have:
    - timer_var: Timers hold a weak reference, count_var holds the only reference (strong_count: 1)
    - count_var: held by color_var and text_var (strong_count: 2)
    - color_var: held by the background_color property (strong_count: 1)
    - text_var: held by the text_var property (strong_count: 1)

# Animation

When a variable is set the new value should be available *immediately* in the next app update. But we may want to implement *easing* that transitions between the previous value and the next. The idea is to extend the `Var` trait to support *get_animating* that returns the intermediary animated value between the two values.

Normal variables (the current ones) just return the new value also, because they are without *easing*, but we can have new `AnimatingVar` or something, that can have easing configuration and provides intermediary values.