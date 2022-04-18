# Unblock

* The first update blocks in `FontFaceLoader::get_system` and `FontFace::load` up to 80ms (not a view block,`font_kit`).
* The first layout blocks in `ViewProcess.open_window` up to 180ms.
* Subsequent windows block layout in `ViewProcess.open_window` up to 130ms.

## Window Open

* Try to implement async context creation in default view crate.
* Reuse windows and surfaces.
* Start creating a window and surface as soon as possible, reuse on first request.
    - This replaces the `warmup_open_gl` with a full context that is kept awaiting a window request.

## Font Query/Load

* We could refactor fonts to be like the images service, async loading.
* Also gets the service ready for supporting web fonts.

# Other

* Try to improve image rendering performance, maybe reuse renderer?

* Animation, see `Variables.md`.
* Finish `Optimizations.md#Cache Everything`.