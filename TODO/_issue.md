# Case, many buttons like this:
button! {
    child = stack!(
        icon!(ICO),
        text!() // UI
    )
}

# Single-thread:
var.base="UI"
    var.with="ICO" { // icon!(ICO) usage
        var.read() => "ICO"
    }
    var.read()="UI" // text!() usage

    // next button
    var.with="ICO {
        var.read() => "ICO
    }
    var.read()=>"UI

# Multi-thread:
var.base="UI"
    #thread-1
    var.with="ICO" { // icon!(ICO) usage
        var.read() => "ICO"
    }
    var.read()="UI" // text!() usage

    // next button, in parallel
    #thread-2
    var.with="ICO" { // icon!(ICO) usage
        var.read() => "ICO"
    }
    var.read()="UI", sometimes "ICO"!! // text!() usage

# Code
ContextVar::with_context("ICO")
    ContextLocal<BoxedVar<T>>::with_context
        1 - WRITE LOCK
        2 - search for thread_id
            2.a   - if found thread_id >>
            2.a.1 - replace value, hold previous in stack
            2.a.2 - DROP WRITE LOCK
            2.a.3 - <INNER>
            2.a.4 - WRITE LOCK
            2.a.5 - replace value with previous

            2.b   - if not found thread_id >>
            2.b.1 - insert value
            2.b.2 - DROP WRITE LOCK
            2.b.3 - <INNER>
            2.b.4 - WRITE LOCK
            2.b.5  - remove value

## <INNER>
ContextVar::get()
    ContextLocal::read()
        1 - READ LOCK
        2 - Get `ThreadContext`.
            2.a  - if found any thread id in local map lock to it.
            2.b  - if not found any thread return default.

# Observations

* We read a value for a `ThreadContext` that is not our logical context.
* All error reads where at least 5 threads deep in the `ThreadContext`.
    - All first 4 the same thread context "T1/T9/T14/T8/0to2 more", pure or with more one or two inside.

* Write lock is dropped between "push" and "pop".
    - Not a problem, only affects current thread ID:
```
== linear
* T1 LOCK
    - insert T1
    - UNLOCK
    - <INNER>
    - LOCK
    - remove T1
* T2 LOCK
    - insert T2
    - UNLOCK
    - <INNER>
    - LOCK
    - remove T2

== parallel
* T1 LOCK
    - insert T1
    - UNLOCK
* T2 LOCK
    - insert T2
    - UNLOCK
    - <INNER>
    - LOCK
    - remove T1
    - <INNER>
    - LOCK
    - remove T2
```

* Is the issue in `ThreadContext`?

* Thread context can "cycle" `ThreadContext:AppId(1)//ThreadId(1)/ThreadId(10)/ThreadId(12)/ThreadId(9)/ThreadId(7)/ThreadId(12)`
    - Rayon work stealing setup causes this often, `ThreadId(12)` here was free when `ThreadId(7)` was busy.
    - Does this causes the problem?

Thread(1): write, <------------------yield------------------->, write
Thread(2):        write, <-----------yield----------->, write
Thread(1):               write, <----yield---->, write

0,          5
  1,     4,
    2, 3,

0 - t(1)write: with_context -> push_value (first in t(1))
1 - t(2)write: with_context -> push_value (first in t(2))
2 - t(1)write: with_context -> replace_value (second in t(1))
3 - t(1)write: with_context -> replace_value (drop)
4 - t(2)write: with_context -> pop_value  (last in t(2))
5 - t(1)write: with_context -> pop_value  (last in (t1))

## Yield Shuffle

Thread(1): write, <-yield->, write
Thread(2):        write, <-----------yield----------->, write
Thread(1):               write, <----yield---->, write

Not possible because Rayon will not return until Thread(2) joins.

# Rayon Nested

```rust
wgt0! { // par_each
    wgt1!();
    wgt2! { // par_each
        wgt_a!();
        wgt_b!();
        wgt_c!();
    };
    wgt3!();
}
```

In the tree above Rayon can init `wgt_a!` in a thread that has `wgt3!` in context?

Because this is the printout of the bug:

```log
with_context ThreadId(11) { // enter with_context, all prints for `font_family` setting to "Material Icons Outlined"
with_context ThreadId(7) {
with_context ThreadId(9) {
} // ThreadId(11) // exit with_context, looks out of sync but ok, others in different threads (there is no nested font assign in `icon` example).
} // ThreadId(7)
} // ThreadId(9)
with_context ThreadId(14) {
} // ThreadId(14)
with_context ThreadId(8) {
with_context ThreadId(10) {
with_context ThreadId(13) {
with_context ThreadId(12) {
with_context ThreadId(11) {
} // ThreadId(8)
} // ThreadId(10)
} // ThreadId(13)
with_context ThreadId(9) {
                                                                // should not have got "Material Icons Outlined" here for text "East".
"East" used "Material Icons Outlined"
ThreadContext:AppId(1)//ThreadId(1)/ThreadId(7)/ThreadId(8)/ThreadId(11)/ThreadId(9)/ThreadId(14)
[(ThreadId(11), "Material Icons Outlined"), (ThreadId(12), "Material Icons Outlined"), (ThreadId(9), "Material Icons Outlined")]

} // ThreadId(12)
} // ThreadId(11)
with_context ThreadId(7) {
"Aspect Ratio" used "Material Icons Outlined"
ThreadContext:AppId(1)//ThreadId(1)/ThreadId(7)/ThreadId(8)/ThreadId(11)/ThreadId(10)
[(ThreadId(9), "Material Icons Outlined"), (ThreadId(7), "Material Icons Outlined")]

"Handyman" used "Material Icons Outlined"
ThreadContext:AppId(1)//ThreadId(1)/ThreadId(7)/ThreadId(8)/ThreadId(11)/ThreadId(8)
[(ThreadId(9), "Material Icons Outlined"), (ThreadId(7), "Material Icons Outlined")]

} // ThreadId(9)
"Fastfood" used "Material Icons Outlined"
ThreadContext:AppId(1)//ThreadId(1)/ThreadId(7)/ThreadId(8)/ThreadId(11)/ThreadId(13)
[(ThreadId(7), "Material Icons Outlined")]

"Error" used "Material Icons Outlined"
ThreadContext:AppId(1)//ThreadId(1)/ThreadId(7)/ThreadId(8)/ThreadId(11)/ThreadId(9)/ThreadId(12)
[(ThreadId(7), "Material Icons Outlined")]

"10mp" used "Material Icons Outlined"
ThreadContext:AppId(1)//ThreadId(1)/ThreadId(7)/ThreadId(8)/ThreadId(11)
[(ThreadId(7), "Material Icons Outlined")]

} // ThreadId(7)
with_context ThreadId(14) {
with_context ThreadId(8) {
with_context ThreadId(10) {
with_context ThreadId(13) {
} // ThreadId(14)
with_context ThreadId(11) {
with_context ThreadId(12) {
} // ThreadId(8)
} // ThreadId(13)
"Eco" used "Material Icons Outlined"
ThreadContext:AppId(1)//ThreadId(1)/ThreadId(7)/ThreadId(8)/ThreadId(11)/ThreadId(9)/ThreadId(14)
[(ThreadId(11), "Material Icons Outlined"), (ThreadId(12), "Material Icons Outlined"), (ThreadId(10), "Material Icons Outlined")]

} // ThreadId(10)
} // ThreadId(11)
```