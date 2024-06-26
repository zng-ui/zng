# Unreleased

* **Breaking** Moved `CommandIconExt` from `zng-wgt-text` to `zng-wgt`.
* `Icon!` now auto-sizes by default.
* Add `zng::widget::ICONS`.
* Add `zng::widget::WeakWidgetFn`.
* Add `zng::widget::EDITORS`.
* Add `dyn AnyConfig::get_raw_serde_bidi`.
* Add `AnyVarValue::eq_any`.
* Add `zng::config::settings`.

# 0.9.1

* Sanitize file names in `cargo zng new`.
    - Also add `f-key-f` and `f-Key-f` for templates to interpolate sanitized file names.
* Fix `cargo zng new` values cleanup.
* Add more :case conversion functions in `.zr-rp`.
    - Add alternative longer names for all cases.
    - Add `:clean` that only applies cleanup.
    - Add `:f` or `:file` that sanitizes for file name.
* Support multiple :case functions in `.zr-rp`.
    - Pipe separator, `:T|f` applies `:Title` them `:file`.

# 0.9.0

* **Breaking** Remove `L10N.load_exe_dir`, use `zng::env::res` with `L10N.load_dir`. 
* **Breaking** `.zr-rp` now trims the values.
* **Breaking** `.zr-rp` now cleans the values for some cases.
* Fix `WINDOW.position().set` not moving the window.
* Add `ZR_LICENSE` in `cargo zng res`.
* **Breaking** Add `zng::env::About::license`.
* Implement (de)serialize for `zng::env::About`.
* **Breaking** `.zng-template` moved to `.zng-template/keys`.
* **Breaking** `.zng-template-ignore` moved to `.zng-template/ignore`.
* Add `.zng-template/post`, an optional post template generation script or crate to run.
* `.zr-rp` now can read files using `${<file}`.
* `.zr-rp` now can read stdout of bash script lines using `${!cmd}`.
* **Breaking** `.zr-rp` now requires the colon in `:?else`.
* **Breaking** `.zr-rp` `:?else` is now only used if the source cannot be read or is not set. Empty values are valid.
* Fix cargo zng res not showing tools progress prints.
* Fix crash handler attaching to other non-app processes.
* Fix `.zr-copy` not merging folders.
* **Breaking** `.zr-sh` now automatically runs with `set -e`.
* `.zr-sh` now runs in `bash` if it is present, fallbacks to `sh`.
* **Breaking** Change default `zng::env::res` for Linux to `../share/{executable-name}`.
* **Breaking** Replace dir definition files in `zng::env` 
    - From `.zng_res_dir` to `.res-dir`.
    - From `zng_config_dir` to `config-dir`.
    - From `zng_cache_dir` to `cache-dir`.
    - Relative dirs are now resolved from where the file is defined.
* **Breaking** Remove deprecated "APP.about" and related types and macro.

# 0.8.2

* Implement some system config reading for macOS.
* Add `aarch64-pc-windows-msvc` view_prebuilt.
    - Without AVIF, not tested.
* Fix view_prebuilt in `x86_64-apple-darwin`.
* Fix `cargo zng res` in non-workspace crates.
* Fix zr-glob not printing copied paths in subdirectories.

# 0.8.1

* Fix align of search box in the Inspector window.
* Fix `l10n!` interpolation of non-var values.
* Allow missing authors field in `zng_env::About::parse_manifest`.
* Fix `cargo zng res` not finding any metadata.

# 0.8.0

* **Breaking** `get_font_use` now gets font references, not just names.
* Add `ZNG_NO_CRASH_HANDLER` env var to easily disable crash handler for special runs like for a debugger.
* Add `CrashConfig::no_crash_handler` for custom crash handler disabling.
* Add `zng::app::print_tracing_filter`.
* **Breaking** `cargo zng res` defaults changed from `assets` to `res`.
* **Breaking** Remove `FullLocalContext`, a type that is from an old version of the context API.
    - The type cannot be constructed, so this has no actual impact.
* [Updated Webrender](https://github.com/zng-ui/zng-webrender/pull/1)
* Fix unbalanced HTML text style tags in `Markdown!` leaking outside of their block.

# 0.7.1

* Fix integrated/dedicated render mode on Ubuntu.
* Fix build of zng-view-* without `"ipc"` feature.
* Prebuilt view-process now uses the same tracing context as the app-process.
    - Note that the tracing context must be set before `run_same_process`.
* Fix "GLXBadWindow" fatal error on Ubuntu.
* Fix ComboStyle arrow icon on Ubuntu.
* Now does not capture view-process stdout/err, inherits stdio from app-process.
* Fix view-process getting killed before exit request can finish.
* Fix windows not opening maximized in X11.
* Fix bold default UI font on Ubuntu.
* Fix multiple hot-reload bugs, now is tested on Windows and Ubuntu.
* **Breaking** Remove `crash_handler` feature from defaults of `zng-app`.
    - The feature ended-up activated by the numerous crates that depend on `zng-app`.
    - This is only a breaking change for direct dependents.

# 0.7.0

* Add `zng::env::on_process_start!` to inject custom code in `zng::env::init!`.
* Add `zng::env::on_process_exit` to register a handler for process exit.
* Add `zng::env::exit` to collaboratively exit the process.
* Add `zng::app::on_app_start` to register a handler to be called when the `APP` context starts.
* Implement `From<Txt>` for `std::ffi::OsString`.
* Add `KeyLocation`.
* Fix numpad shortcuts.
* **Breaking** Remove `zng_view::init`, `zng_view_prebuilt::init`, `zng::view_process::default::init`, `zng::view_process::prebuilt::init`.
    - Only `zng::env::init!()` needs to be called to setup the view-process.
* **Breaking** Remove `zng_view::extensions::ViewExtensions::new`.
    - Now use `zng_view::view_process_extension!` to declare.
* **Breaking** `zng_view_api::Controller::start` now requires the exe path and supports optional env variables to set.
    - This only affects custom view-process implementers.
* **Breaking** New default `zng::env::res`, was `./assets` now is `./res`.
* **Breaking** Renamed `zng_view_prebuilt::ViewLib::init` to `view_process_main`.
* **Breaking** Remove `is_single_instance`, `single_instance`, `single_instance_named`.
    - Single instance now enabled automatically just by setting `feature="single_instance"`.
* **Breaking** Add name requirement for `zng::task::ipc` workers.
    - Workers are no longer limited to a single entry with an enum switch.
    - Use `zng::env::on_process_start!` to declare the worker entry anywhere.
* **Breaking** Remove `zng::app::crash_handler::init` and `CrashConfig::new`.
    - Use `zng::app::crash_handler::crash_handler_config` to config.
* **Breaking** Methods of `HeadlessAppKeyboardExt` now require key location.

# 0.6.2

* Add `log` support in hot reloaded dylib.
* Add `cargo-zng`, a Cargo extension for managing Zng projects.
    - This replaces `zng-l10n-scraper` that is now deprecated and deleted.
    - See `cargo zng l10n --help` for its replacement.
* Add `zng-env` and `zng::env` as an API to get external directories and files associated with the installed process.
* Add `zng::hot_reload::{lazy_static, lazy_static_init}`. Very useful for implementing "zng-env" like functions.
* Implement `FromStr` for `zng::l10n::Langs`.
* `Lang` now parses empty strings as `und`.
* Fix `IMAGES.from_data` never loading in headless apps.
* Fix `Img::copy_pixels`.
* Fix `view_process::default::run_same_process` exit in headless runs.
* Fix `AutoGrowMode::rows` actually enabling Columns auto grow.
* Fix "unsafe precondition(s) violated" issue ([#242](https://github.com/zng-ui/zng/issues/242)).

# 0.6.1

* Add more hot reload `BuildArgs` helpers.
* Change default hot reload rebuilder to first try env var `"ZNG_HOT_RELOAD_REBUILDER"`.
    - This feature is used in the `zng-template`, releasing soon.
* Fix `tracing` in hot reloaded dylib.

# 0.6.0

* **Breaking** Remove deprecated `NilAnimationObserver`.
* Fix release build of `zng-wgt-scroll` running out of memory. (#203)
* Implement hot reloading UI nodes.
    - Add `zng-ext-hot-reload` and `zng-ext-hot-reload-proc-macros`.
    - Add `zng::hot_reload`.
    - Add `feature="hot_reload"` in `zng` and `zng-unique-id`.
    - Add `hot_reload` example.
* Implemented VsCode snippets for common Zng macros, see [`zng.code-snippets`].
* Fix view-process cleanup when app-process panics on init.
* **Breaking** Remove all `Static{Id}` unique ID static types.
    - Use `static_id!` to declare static IDs, the new way is compatible with hot reloading.
    - Removed `StaticWindowId`, `StaticMonitorId`, `StaticPropertyId`, `StaticAppId`, `StaticWidgetId`, `StaticDeviceId`, `StaticStateId`,  `StaticCommandMetaVarId`, `StaticSpatialFrameId`.
* Implemented equality/hash for `zng::task::SignalOnce`.
* Breaking `Window::on_close` now has args type `WindowCloseArgs`.
    - Add `on_pre_window_close` and `on_window_close`.
* Fix headless windows not receiving close events on app exit request.
* Add `std::env::consts::OS` to crash error.
* **Breaking** Refactored multiple system config variables to allow app override.
  - `VARS.animations_enabled` now is read-write. Added `VARS.sys_animations_enabled` read-only variable that tracks the system config.
  - `KEYBOARD.repeat_config` now is read-write. Added `KEYBOARD.sys_repeat_config`.
  - `KEYBOARD.caret_animation_config` now is read-write. Added `KEYBOARD.sys_caret_animation_config`.
  - `MOUSE.multi_click_config` now is read-write. Added `MOUSE.sys_multi_click_config`.
  - `TOUCH.touch_config` now is read-write. Added `TOUCH.sys_touch_config`.

[`zng.code-snippets`]: .vscode/zng.code-snippets

# 0.5.1

* Add `diagnostic::on_unimplemented` notes for multiple traits.
* Add `auto_scroll` in the `Scroll!` widget, enabled by default.
* Add `CaptureFilter` helper constructors.
* Add `LocalContext::extend`.
* Add `SCROLL.context_values_set`.
* Fix `WidgetInfo::new_interaction_path` always detecting change.
* Improve iterators in `InteractionPath`, `interaction_path` and `zip` are now double ended and exact sized.
* Add `ZOOM_TO_FIT_CMD`.
    - The `CTRL+'0'` shortcut is now used for this command, not `ZOOM_RESET_CMD`.
* Deprecate `NilAnimationObserver`, use `()` now.
* Add `ForceAnimationController` to force important animations to run when animations are disabled on the system.
* Fix crash handler passing app name twice as command line arguments.
* **Breaking** Implemented new syntax for the localization scrapper to separate standalone notes per file:
    - `// l10n-file-### {note}` only adds the note to the `template/file.ftl`.
    - `// l10n-*-### {note}` adds the note to all files that match the glob pattern (`template/*.ftl`).
    - The old syntax `// l10n-### {note}` is still supported, but now it is equivalent to `// l10n--###` that
      matches the default `template.ftl` file only.
    - Note that this is only a breaking change for dependents of `zng-l10n-scraper`. Normal users (cargo install)
      must update the tool to scrap using the new syntax, comments with the new file pattern matcher are ignored
      by older scrappers.

# 0.5.0

* Add `OPEN_TITLE_BAR_CONTEXT_MENU_CMD` for windows.
* Add `DRAG_MOVE_RESIZE_CMD` for windows.
* **Breaking** View API changes:
    - Add `open_title_bar_context_menu`.
    - Rename `close_window` to `close`.
    - Rename `focus_window` to `focus`.
    - Add `set_enabled_buttons`.
    - Add `set_system_shutdown_warn`.
    - Note that this is only a breaking change for direct dependents of `zng-view-api`.
* Better "custom chrome" example in `examples/window.rs`.
* Add `OPEN_TITLE_BAR_CONTEXT_MENU_CMD` to window API.
* Fix `WIDGET.border().offsets()` not including the innermost border offset.
* Add `WindowVars::enabled_buttons` to window API.
* Add `WindowVars::system_shutdown_warn` to window API.
* **Breaking** Fix when/property assign expansion order.
    - When blocks now expand in the same declaration order, before they always expanded after all property assigns.
```rust
// code like this incorrectly builds in v0.4:
fn version_0_4() -> impl UiNode {
    let can_move = var(true);
    Container! {
        when *#{can_move} {
            mouse::cursor = mouse::CursorIcon::Move;
        }
        mouse::on_mouse_down = hn!(can_move, |_| {
            let _use = &can_move;
        });
    }
}
// now in v0.5 the value must be cloned before the last move:
fn version_0_5() -> impl UiNode {
    let can_move = var(true);
    Container! {
        when *#{can_move.clone()} {
            mouse::cursor = mouse::CursorIcon::Move;
        }
        mouse::on_mouse_down = hn!(|_| {
            let _use = &can_move;
        });
    }
}
```
* **Breaking** Rename `VarCapabilities` to `VarCapability`.
* **Breaking** Add window extension API in `zng-view`.
    - Add `ViewExtension::window`.
    - Add `OpenGlContext` and replace the `gl` fields with `context` in multiple extension API args.
    - Rename `is_config_only` to `is_init_only`.
    - Note that this is only a breaking change for direct dependents of `zng-view`.
    - Rename `ViewRenderExtensionError` to `ViewExtensionError`.
* Add window reference to args for `RendererExtension` when possible.
* Fix `zng::view_process::default::run_same_process` not propagating app panics.
* Add `WindowCloseRequestedArgs::headed/headless`.
* **Breaking** Fix tab nav when a focus scope with `FocusScopeOnFocus::LastFocused` is a child of
  another scope with `TabNav::Cycle`.
    - Breaking change has minimal impact:
        - Added input in `WidgetFocusInfo::on_focus_scope_move`.
        - Removed `FocusChangedCause::is_prev_request`.
* Add `FocusChangedCause::request_target` helper method.
* Add `WidgetPath::parent_id` helper method.
* Fix auto scroll to focused issues:
    - When the focused child does not subscribe to focus change events.
    - Scrolling when large widget is already visible.
    - Scrolling again to same widget when focus change event did not represent a widget change.
* Add `WidgetInfo::spatial_bounds`.
* Fix directional navigation cycling only inside viewport now full spatial bounds of scopes.
* Add better conversions for `CommandScope`. You can now scope on named widgets directly, `FOO_CMD.scoped("bar-wgt")`.
* Add `ContextualizedVar::new_value`.
* **Breaking** `SCROLL.scroll_*` methods now return contextual vars, not values.
* Fix panic on window move in Wayland.
* Fix minimize command removing maximized state from restore.
* Fix issue when parent widget's cursor can override child's cursor when the parent cursor var updates.
* **Breaking** Remove the `cursor_img` property and window var.
* **Breaking** The `cursor` property now has input type `CursorSource`.
    - Note that the `CursorIcon` type converts to `CursorSource`.
* Implement custom cursor images in the default view.

# 0.4.0

* Panics in `task::respond` are now resumed in the response var modify closure.
* Add `task::ipc` module, for running tasks in worker processes.
* **Breaking:** Remove `"bytemuck"` feature from `zng-unique-id`.
    - Now must use `impl_unique_id_bytemuck!` to generate the impls.
    - Note that this is only a breaking change for direct dependents of `zng-unique-id`.
* Add single app-process instance mode.
    - Adds `zng-ext-single-instance` crate re-exported in `zng::app` when non-default 
      Cargo feature `"single_instance"` is enabled.
* Implement `AsRef<std::path::Path>` for `Txt`.
* Implement `AsRef<std::ffi::OsStr>` for `Txt`.
* Add app-process crash handler.
    - Adds `zng::app::crash_handler`.
    - Can be used to easily implement crash reporting, stacktrace and minidump 
      collection, app restart on crash.
    - Call `zng::app::crash_handler::init_debug()` to quickly setup panic and minidump collection.
* Fix view-process kill by user not working after respawn.
* Fix view-process assuming any signal kill was requested by the user.
* Fix potential issue retrieving current_exe trough symbolic links.
* Fix view-process panic message.
* Add `APP.about`.
* Fix `AnsiText!` not resetting style.
* `Markdown!` widget now uses `AnsiText!` for ```console code block.
* Fix `auto_size` not using the min/max_size constraints.
* **Breaking:** Change return type of `SCROLL.vertical_offset`, `SCROLL.horizontal_offset` and `SCROLL.zoom_scale`.
    - Changed only from `ReadOnlyContextVar<Factor>` to `ContextVar<Factor>` so it has minimal impact.
* Add `vertical_offset`, `horizontal_offset` and `zoom_scale` properties in `Scroll!`.
    - Users should prefer using scroll commands over these properties, but they are useful for implementing features
      like binding two side-by-side scrolls, saving scroll state.

# 0.3.4

* Add Cargo feature documentation in each crate `README.md` and `lib.rs` docs.
* Add Screenshot function to the Inspector window.
* Fix `formatx!` causing futures to not be Send+Sync.
* `UiTask` now logs a warning if dropped while pending.
* Add `UiTask::cancel` to drop a pending task without logging a warning.
* Fix `WINDOWS.frame_image` capture with multiple windows capturing pixels from the wrong window.
* Fix `WINDOWS.frame_image` var not updating on load or error.
* Fix cursor not resetting on widget deinit.
* Add missing `zng::app::test_log`.
* **Breaking:** View API accessibility updates.
    - Added `Event::AccessDeinit`, access can now be disabled by the system.
    - Removed `WindowRequest::access_root`, no longer needed.
    - Note that this is only a breaking change for direct dependents of `zng-view-api`.
* Fix many doc broken links.

# 0.3.3

* Fix `zng-tp-licenses` build in docs.rs.
* You can now suppress license collection on build by setting `"ZNG_TP_LICENSES=false`.

# 0.3.2

* Fix docs.rs build for `zng` and `zng-wgt-material-icons`.
* Add AVIF support in prebuilt view.
* Implement prebuilt compression, prebuilt now depends on `tar`.
* Implement `PartialOrd, Ord` for `Txt`.
* Add crate `zng-tp-licenses` for collecting and bundling licenses.
* Add `third_party_licenses` on view API that provides prebuilt bundled licenses.
* Add `zng::third_party` with service and types for aggregating third party license info.
    - Includes a default impl of `OPEN_LICENSES_CMD` that shows bundled licenses.

# 0.3.0

* **Breaking:** Fix typos in public function names, struct members and enum variants.
* Fix cfg features not enabling because of typos.

# 0.2.5

* Fix docs.rs build for `zng-view-prebuilt`, `zng-app`, `zng-wgt`.
* Unlock `cc` dependency version.
* Remove crate features auto generated for optional dependencies.
* Add `zng::app::print_tracing`.
* In debug builds, prints info, warn and error tracing events if no tracing subscriber is set before the first call to `APP.defaults` or
`APP.minimal`.

# 0.2.4

* Fix `zng` README not showing in crates.io.

# 0.2.3

* Change docs website.

# 0.2.2

* Fix `"zng-ext-font"` standalone build.

# 0.2.1

* Fix build with feature `"view"`.

# 0.2.0

* Crates published, only newer changes are logged.
