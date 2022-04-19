# Unblock

The app-process blocks waiting for the view-process in some cases, need to implement better async communication
and unblock the view-process.

## Window Open

* Try to implement async context creation in default view crate.
    - Problem, glutin needs the event-loop window target to build a context (it is not send and must be in main).
    - Can use `build_raw_context` that only requires a window handle, so we create the winit window blocking then offload
    everything to a thread.

* Reuse windows and surfaces.
* Start creating a window and surface as soon as possible, reuse on first request.
    - This replaces the `gl::warmup` with a full context that is kept awaiting a window request.

## Font Query/Load

* We could refactor fonts to be like the images service, async loading.
* Also gets the service ready for supporting web fonts.

# Other

* Try to improve image rendering performance, maybe reuse renderer?

* Animation, see `Variables.md`.
* Finish `Optimizations.md#Cache Everything`.