# Profile Stress Test

This crate implements various UI stress tests harnessed for profiling.

# Usage

* Edit `src/main.rs` if needed.
    You may want to implement a custom filter for one of the tests for example.

* Run `do profile --stress <stress-test> --release`.
    Note that you can pass cargo flags, just like `do run`.

* Interact with the UI a bit, then close, a profile file is created.
    The file is created in the current directory with a name `profile-stress-{test-name}`.

* View the profile using `chrome://tracing`.