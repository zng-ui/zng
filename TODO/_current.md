# Unblock

* No extensions init before `ViewProcess::start` and that takes up to 50ms.
* WindowManager blocks in `Monitors::new` up to 14ms.
* The first update blocks in `FontFaceLoader::get_system` and `FontFace::load` up to 80ms (not a view block,`font_kit`).
* The first layout blocks in `ViewProcess.open_window` up to 180ms.
* Subsequent windows block layout in `ViewProcess.open_window` up to 130ms.

## View Start and Monitors

* Wait in background thread, send event ViewProcessLoaded that already includes the monitors info.

## Font Query/Load

* We could refactor fonts to be like the images service, async loading.
* Also gets the service ready for supporting web fonts.

## Window Open

* Implement WindowOpen event in the view API to support async context creation.
* Try to implement async context creation in default view crate.

# Other

* Try to improve image rendering performance, maybe reuse renderer?

* Animation, see `Variables.md`.
* Finish `Optimizations.md#Cache Everything`.