# Unpublished

* Fix release build of `zng-wgt-scroll` running out of memory. (#203)
* Implement hot reloading UI nodes.
    - Add `zng-ext-hot` and `zng-ext-hot-proc-macros`.
    - Add `zng::hot_reload`.
    - Add `feature="hot_reload"` in `zng`.

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
