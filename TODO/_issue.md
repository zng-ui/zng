# Case, many buttons like this:
button! {
    child = stack!(
        icon!(ICO),
        text!() // UI
    )
}

# Single thread:
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