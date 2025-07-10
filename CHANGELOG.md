# Unreleased

* Remove warnings about `touch_config` not being implemented in Linux, macOS and Windows.
* Fix localization resources fallback going to different file name (take 2).

# 0.15.8

* Implement `LOW_MEMORY_EVENT` for macOS, all supported platforms covered now.
* Fix view-process config in Linux not reading default values that are only defined in schema.
* Implement `LOW_MEMORY_EVENT` for Linux.
* Fix log printing in prebuilt view-process.
* Handle incorrect localization file name normalization.
    - Assertion panic in debug builds.
    - Warning logged and normalization in release builds.
* Fix duplicate localization caching.
* Fix localization resources fallback going to different file name.
* Fix localization showing resource as loaded after load error.

# 0.15.7

* Add `"dyn_node"` to `zng` default features to avoid build issues in release builds.
  - GitHub workflow runners can't handle building with all the generics inlining that happens without this feature.
  - This is a better default for test release builds, the performance hit is negligible.
  - Production builds should disable default features and configure depending on need, see [`docs/optimize-release.md`] for details.
* Fix release builds with default features.
* Fix `zng-wgt-inspector` builds without `"live"` feature.

[`docs/optimize-release.md`]: ./docs/optimize-release.md

# 0.15.6

* Add `cargo zng trace` subcommand for recording and post processing traces.
* Add `zng::env::process_name` and name the Zng processes.
* Add `zng::app::trace_recorder` and the `"trace_recorder"` feature.
* Fix `cargo zng l10n` handling of repeated sections.
* Fix `cargo zng new` removing already existing target.

# 0.15.5

* Add support for `CLIPBOARD.file_list` in Linux and macOS.
* `LocalContext::with_context_blend` now also overrides the tracing dispatcher if it captures a dispatcher.
* Better error handling in `zng::env::migrate_config`.
* Fix `zng::env::init_cache` setting the config path instead cache.
* Fix `LayoutMetricsSnapshot` not comparing all fields.
* Fix `Transform` interpolation.
* Fix division between different kinds of `Length` not creating a Div expression.
* Fix `PxConstraints::with_more` not saturating on overflow.
* Fix `Transitionable::lerp` implementation for `Align`. 
* Fix `Align` equality not considering the `x_rtl_aware` field.
* Implement `PartialOrd` for `PreMulRgba`, `Hsla`, `Hsva`, `Rgba`, `InlineSegmentPos`, `Ppi`, `Ppm`, `AngleRadian`, `AngleGradian`, `AngleDegree`, `AngleTurn`.
* Implement `Ord` for `FontStretch`, `FontWeight`, `Factor`, `PreMulRgba`, `Hsla`, `Hsva`, `Rgba`, `InlineSegmentPos`, `Ppi`, `Ppm`, `AngleRadian`, `AngleGradian`, `AngleDegree`, `AngleTurn`.
  - *Deprecated* custom `max`, `min` and `clamp` methods of `Factor`,  `Ppi`, `Ppm`, use the Ord equivalent. 
* Implement `Eq` for `Factor`, `FactorPercent`, `Align`, `ColorMatrix`, `PreMulAlpha`, `Hsla`, `Hsva`, `FontStretch`, `FontWeight`, `Ppi`, `Ppm`, `AngleRadian`, `AngleGradian`, `AngleDegree`, `AngleTurn`, `Rgba`, `AngleGradian`.
* Implement `Hash` for `AngleRadian`, `AngleGradian`, `AngleDegree`, `AngleTurn`, `Align`, `Ppm`.
* Fix hash equality for `f32` based unit types.
  - Refactor `zng_layout::unit::about_eq` to compare finite values by *bucket granularity*.
  - Refactor `about_eq_hash` to hash the same *bucket*.
  - *Deprecated* `EQ_EPSILON` and `EQ_EPSILON_100` renamed to `EQ_GRANULARITY` and `EQ_GRANULARITY_100`.
  - These are the types that incorrectly implemented hashing and are now fixed: `ColorMatrix`, `Rgba`, `PreMulRgba`, `Hsla`, `Hsva`, `FontStretch`, `FontWeight`, `InlineSegmentPos`, `Ppi`, `Factor` and `FactorPercent`.
  - In practice direct equality comparisons between two values has no significant change, the previous *epsilon distance* based
    equality was correct, it just was not compatible with hashing.
  - Refactor `about_eq_ord` to enforce `-inf < finite < inf < NaN` using the new equality *bucket* value for the finite values.
* Fix `TimeUnits::ms` impl for `f32`.
* Fix `Txt::split_off` when the `Txt` is backed by `&'static str`. 

# 0.15.4

* Fix interactive carets in rich texts losing pointer capture when crossing over leaf texts.
* Fix interactive insert caret appearing in non editable text.
* Update renderer dependencies.

# 0.15.3

* Inspector properties panel is now a selectable rich text.
* Rich text selection operations that apply to lines now ignore wrap line breaks.
* Add `zng::window::inspector::INSPECTOR` and associated types for configuring the live inspector (ctrl+shift+i).
  - In this release: custom watchers and the internal data model is made public, root type `InspectedTree`.
* Fix rich selection not highlighting all text components with focused style.
* Add `EventUpdate::custom` to create a custom event delivery from an existing one.
  - Somewhat equivalent to `Event<A>::new_update_custom`, but without needing to know the event type. 
* Fix panic on explicit disabled `rich_text`.
* Enable rich text selection in default third-party licenses screen. 
* Enable rich text selection in debug crash dialog, removed "plain" stdio panels.
* Rich text copy now includes line breaks between *vertically stacked* texts in non-wrap panels.

# 0.15.2

* Fix build of version 0.14 users. See #650 and #649 for details.

# 0.15.1

* Fix release.

# 0.15.0

This release contains many small breaking changes, almost all on advanced API, the normal surface API is mostly untouched.
All changes are trivial to fix, they are either a rename or types that are now non-exhaustive. 

* **Breaking** Renamed view-process "online" to "connected".
  - `zng_view_api::Controller::online` to `is_connected`.
  - `zng_view_api::Request::must_be_online` to `must_be_connected`.
  - `VIEW_PROCESS.is_online` to ` is_connected`.
  - `zng_app::view_process::EncodeError::ViewProcessOffline` to `Disconnected`.
  - `ClipboardError::ViewProcessOffline` to `Disconnected`.
  - `ClipboardError::ViewProcessOffline` to `Disconnected`.
* **Breaking** `EventReceiver::try_recv` now returns `Result<Option<A>, AppChannelError>`.
* **Breaking** Normalized error types.
  - Replaced `AppExtSenderDisconnected` with `AppChannelError::Disconnected`.
  - Replaced `AppDisconnected` with `AppChannelError::Disconnected`.
  - Many error enums marked `non_exhaustive`.
  - Renamed `WindowNotFound` to `WindowNotFoundError`.
  - Replaced `zng_view_api::ipc::Disconnected` with `zng_view_api::ipc::ViewChannelError::Disconnected`.
  - Replaced `zng_view_api::ViewProcessOffline` with`zng_view_api::ipc::ViewChannelError::Disconnected`.
* **Breaking** Many structs marked `non_exhaustive`.
  - All are supposed to only be read or have construction associated functions.
* **Breaking** Upgrade `ron` from 0.8 to 0.10.
  - `zng_ext_config::RonValueRawError` changed.
  - `zng_ext_config::TomlValueRawError` changed.
  - `zng_ext_fs_watcher::WriteFile::write_ron` now uses new format.
  - `zng_ext_fs_watcher::WatchFile::ron` now uses new format.
* **Breaking** Upgrade `encoding` dependency to 2.
  - `zng_tp_licenses::encode_licenses` format changes. Not an issue for the recommended usage of encoding licenses at build time.
  - `zng_view_api::ApiExtensionPayload` internal format changed.
  - `zng_view_api::ApiExtensionPayload::serialize` return error type changed.
  - `zng_view_api::ApiExtensionRecvError::Deserialize` associated value changed.
  - `zng_task::RunError` changed.
* Add text `get_selection` getter property, works for local texts.
* Add text `has_selection` getter property, works for local and rich texts.
* **Breaking** Add `Command::new_update` that automatically uses the correct `CommandArgs`.
  - To create an update with custom args now use `cmd.event().new_update(args)`.
* Rich text contexts now handle scoped `SELECT_CMD` and `SELECT_ALL_CMD`.

# 0.14.4

* `Markdown!` is not a rich text context, enable `txt_selectable` to provide simple selection and copy.
* Add `zng::text::txt_selectable_alt_only` to coordinate click events with rich text selection gestures.
  - `Button!` widgets enable this by default, any button or derived widget is now clickable inside rich texts.

# 0.14.3

* Enable basic text selection and copy for `Markdown!` and `AnsiText!`.
* Add rich text context, `TEXT.rich` and associated API.
  - Implemented most text gestures for rich text.
  - Missing: proper line selection, touch carets across text runs.
* Add `Command::new_update_param` helper method. 
* Add `WidgetBoundsInfo::inner_rects` and associated methods.
* Add `WidgetInfo::nearest_rect` and associated methods.
* Add `WidgetInfo::cmp_sibling_in` method for fast ordering between widget infos.
* Text now clears selection on `Key::Escape`.
* Implement all `FromIterator` and `Extend` that `String` implements for `Txt`.
* Fix Alt focus scope navigation from inside nested focus scopes.
* Implement `From<WidgetFocusInfo> for WidgetInfo`.
* Surface `zng::text::txt_selectable` property.
* Fix incorrect `Wrap!` debug validation with negative spacing.
* Fix shift+arrow key gestures not starting text selection until next arrow key press.
* Update dependencies.

# 0.14.2

* Fix build error in Rust 1.86. [See details](https://github.com/zng-ui/zng/pull/633#issuecomment-2777515702)

# 0.14.1

* Add functions `all`, `all_ok`, `all_some`, `any`, `any_ok`, `any_some` in `zng::task`. These functions are dynamic versions of the
* Task macros `all!`, `all_ok!`, `all_some!`, `any!`, `any_ok!` and `any_some` now accept `IntoFuture` inputs.

# 0.14.0

* Upgrade all crates to Rust 2024 edition.
* **Breaking** Many return impl Trait changed lifetimes slightly, some replaced with doc hidden types. In practice all code should still just compile, but these are breaking changes.
* **Breaking** Some public `hashbrown::{HashMap, HashSet}` API replaced with the `std::collections` equivalent.
* **Breaking** Remove deprecated `zng::text::justify` that was replaced with `justify_mode` in 0.13.11.

# 0.13.12

* Implemented text justify for *rich text* composed of `Wrap!` and `Text!`.

# 0.13.11

* Implement text fill/justify.
* Deprecate `justify`, add `justify_mode`.
* Fix `#[deprecated]` usage in `#[widget]` structs.
* Fix `#[deprecated]` usage in `#[property]` functions.
* Fix text overflow truncate not applying in most texts.
* Fix misaligned hyphenation split points.
* Fix missing lang region in hyphenation query.

# 0.13.10

* Fix regression, some `Text!` in `Wrap!` not rendering.

# 0.13.9

* Fix `Text!` measure incorrect size after a previous layout.
* Add `scroll::zoom_size_only` property that disables scaling in `Scroll!` descendants and resizes them instead.
* Fix small `Text!` in a `Wrap!` not starting a new line.

# 0.13.8

* Fix `cargo zng fmt` for widgets with more than one `when` block.
* Fix `Wrap!` ignoring child `Text!` that is only a line-break.
* Fix soft/hard breaks in `Markdown!`.
* Improve cache of font data to use less memory.

# 0.13.7

* More feature optimization.

# 0.13.6

* Fix compilation of `zng-var` without features in debug mode.
* Add features for each sub-module of the `zng`. This enabled compile size optimization.

# 0.13.5

* Various optimizations to reduce code bloat.

# 0.13.4

* Fix crash handler creating a temp file in the executable directory.

# 0.13.3

* Fix hang rendering some text with image emojis.

# 0.13.2

* Fix deadlock in release builds (introduced in 0.13.1).

# 0.13.1

* Add `L10N.load_tar` to support embedded localization resources.
* Changed `ByteLength` to display unit symbols.
* Ignore not found error on `cargo zng l10n` cleanup.

# 0.13.0

* Add `zng::drag_drop`, with limited support drag&drop gestures.
* Add missing `zng::var::OnVarArgs`.
* **Breaking** Implemented drag&drop on the view-process API.
* **Breaking** `Event::visit_subscribers` and `Command::visit_scopes` visitor closures now must return `ControlFlow`.
* **Breaking** Refactored drag&drop in the view API to be general purpose.
* The view API `ReferenceFrameId` type now reserves some IDs for the view process.
* Add `border_img` and other related properties and types.
* **Breaking** Moved `zng::render::RepeatMode` to `zng::widget`.
* **Breaking** Add `zng_app::render::Img::size` for the renderer image interface trait.
* **Breaking** Changed default image filter to only allow images in `zng::env::res`, replacing the current exe dir filter.
* **Breaking** Add missing inputs in 9-patch border rendering methods in `FrameBuilder` and `DisplayListBuilder`.

# 0.12.10

* Fix `zng_tp_licenses::collect_cargo_about` call in Powershell.

# 0.12.9

* Fix EXIF orientation not applying to images.

# 0.12.8

* Properties `size` and related now have a default value allowing conditional assign.
* Add `zng::slider` with `Slider` widget.
* Fix `force_size` returning the parent's constraint min size.
* Fix hit-test in rounded rectangles with too large corner radius.
* Fix headless rendering in Wayland. Property `needs_fallback_chrome` now is `false` for headless windows.

# 0.12.7

* Revert `fs4` dependency upgrade to fix build. It was yanked.

# 0.12.6

* Add debug validation of capture only property use in widgets that don't capture it.
* Fix Wayland custom chrome breaking window padding.
* Fix window text properties affecting the Wayland fallback chrome text.
* Fix `swgl` build error.
* Changed how relative lengths are computed in `offset`, `x` and `y`, now uses the maximum bounded length from constraint, the same as `size` properties.
* Fix display print of `FactorPercent` not rounding.
* `ChildInsert::{Over, Under}` now allows insert to affect layout size, like other inserts.
    - Use `background` and `foreground` properties as *layout passive* alternatives for `child_under` and `child_over`.
* Add `zng::container::child`.
* **Breaking** `zng_wgt_container::child` can now be used as a standalone property that behaves the same as `child_under`.
    - Note that this is only a breaking change for direct dependents of `zng-wgt-container`.
* Fix warning on `flood` with 0 area.
* Add `zng::task::Progress` value type for reporting a task progress status.
* Add `zng::progress` with `ProgressView` widget for displaying a task progress status.

# 0.12.5

* Fix `cargo zng fmt` and `cargo zng l10n` on files that start with `#!`.
* Fix layers anchored to the root widget never rendering in some windows.

# 0.12.4

* Export `LOW_MEMORY_EVENT` on the surface API in `zng::app`.
* Fix `LOW_MEMORY_EVENT` not notifying in Android.
* Implement `LOW_MEMORY_EVENT` in Windows.
* Fix window not updating state after restore in Wayland.
* Add `FrameBuilder::render_widgets` and `render_update_widgets` to `FrameBuilder` and `FrameUpdate` to inspect external render requests.
* Implement support for raster and svg emojis.
* Add `FontFace::ttf` to quickly access the full parsed TTF data.
* Add `has_raster_images` and `has_svg_images` method to `FontFace` and `ShapedText`.
* Add software render in macOS.
* Fix software render in Wayland.

# 0.12.3

* Fix close button icon in Wayland instances without any close icon available
* Add `IMAGES.image_task` to load async image sources.
* Implement support for SVG images.
    - Add `zng-ext-svg`.
    - Add non default `"svg"` feature on the `zng` crate.
* Fix `view_process_extension!` not running in same-process mode.
* **Breaking** `WindowExtension` now also instantiated for headless surfaces.
    - Note that this is only a breaking change for direct dependents of `zng-view` extensions API.
* **Breaking** Add `as_any` casting method for `RendererExtension` and `WindowExtension`.
    - Note that this is only a breaking change for direct dependents of `zng-view` extensions API.
* Add `"zng-view.prefer_angle"` window extension to support enabling ANGLE EGL over WGL on Windows.

# 0.12.2

* Add `widget_impl:` directive for `command_property!`.
* Allow missing trailing comma in `event_property!`.
* Fix visibility and transform events losing track of a widget after info rebuild.
* Add visibility changed event properties, `on_visibility_changed`, `on_show`, `on_collapse` and others.
* Add `VisibilityChangedArgs` helper methods for tracking changes for a specific widget.
* Fix doc links in inherited properties fetched from redirected original pages.
* Fix `cargo zng res` not getting explicit metadata from lib crates.
* Implement `--verbose` for `cargo zng res`.
* Localize settings search box placeholder text.

# 0.12.1

* Fix panic trying to use font index on macOS.
* Fix default UI font in Apple systems.
* **Breaking** Update webrender dependency.
    - Note that this is only a breaking change for direct dependents of `zng-view` extensions API.
* Fix `--clean-deps` in `cargo zng l10n`.
* Implement `--verbose` for `cargo zng l10n`.

# 0.12.0

* Log warning when property is not used because it has no default value.
* Define default `max_size`, `max_width` and `max_height` so these properties can now be only set by when conditions.
* Fix `sticky_size`, `sticky_width` and `sticky_height` properties when dynamically disabled and re-enabled.
* Fix `sticky_height` using *x* constraint.
* Fix fill align in `Scroll!` dimensions that do not scroll.
* Implement alternate `SettingsEditor!` layout for narrow width (mobile).
* Implement equality and comparison for `Dip` and `Px` to `i32` (and `f32` for `Dip`).
* **Breaking** Move `IS_MOBILE_VAR` to `zng-wgt` and `zng::widget`.
* **Breaking** Move `is_mobile` and `force_mobile` to `zng-wgt` and `zng::widget`.
    - These properties are no longer strongly associated with `Window`.
* Add `SettingsEditor::panel_fn` for customizing the full editor layout.
* **Breaking** Remove previous deprecated `UiNodeVec`.
* **Breaking** Remove unused renderer param in `FrameUpdate::new`.

# 0.11.8

* Add `cargo zng l10n --clean`.
    - Add `cargo zng l10n --clean-deps` to remove previously copied localization before new copy.
    - Add `cargo zng l10n --clean-template` to remove previously scraped files.
* Add `cargo zng l10n --no-pkg` to skip scraping the target package, only copy localization from dependencies.
* Don't show keyboard shortcuts in mobile menus.
* Fix incorrect `TouchInputArgs::position` in nested windows.

# 0.11.7

* Fix OpenGL version check.
* Fix window receiving a cursor move event while cursor is not over (on x11).
* Remove end punctuation from command `info` fields
* Add better custom chrome for GNOME+Wayland.
* Add `zng::widget::node::bind_state_init` helper.
* Add `Window::needs_fallback_chrome` and `Window::prefer_custom_chrome` property.
* **Breaking** Add `ChromeConfig` and related events to the view API.
    - Note that this is only a breaking change for direct dependents of `zng-view-api` and `zng-app`.

# 0.11.6

* Fix breaking change in 0.11.5, `UiNodeVec` is only deprecated, but was removed from preludes and re-exports.

# 0.11.5

* Monitor query now falls back to largest screen when there is no primary monitor.
* Fix monitor query not updating for new window before first layout. Fixes window size in Ubuntu without GPU.
* Fix touch event targeting in nested windows.
* Fix `MOUSE.position` not tracking nested windows.
* Fix context menu not opening in nested windows.
* Fix focus not clearing from nested window when parent window loses focus on the system.
* Fix context menus of child and parent opening at the same time.
* Add missing `zng::event::AppCommandArgs`, command app level event handling is part of the surface API.
* Fix nested window render update flickering.
* *Deprecated* Renamed `UiNodeVec` to `UiVec`, old name is now a deprecated type alias.
* Fix focus not returning to main window after nested window closes.
* **Breaking** View API focus now returns a new `FocusResult`.
    - Note that this is only a breaking change for direct dependents of `zng-view-api` and `zng-app`.
* Fix app context in nested windows.
* Add `CaptureFilter::app_only` and `ContextValueSet::insert_app`.

# 0.11.4

* Add `zng::container::{child_out_*, child_under, child_over}` properties.
* Implement window nesting, primarily as an adapter for mobile platforms.
    - Add `WINDOWS.register_open_nested_handler`.
    - Add `WindowVars::is_nesting`.
    - Add `nested_window` and `nested_window_tree` helper methods for `WidgetInfo` and focus info.
    - Add default nesting handler on platforms that only support one window (Android).
* Add `LAYERS_INSERT_CMD` for inserting layer widgets from outside the window context.
* Add `LAYERS_REMOVE_CMD` for removing layer widgets from outside the window context.
* Fix hang opening a popup from another closing popup.
* Define oldest supported macOS prebuilt. Only supported >=11, now this is documented.
* Fix `"view_prebuilt"` linking on macOS.
* Fix panic on old macOS (<11). Color scheme and accent is only supported >=11.
* Fix `cargo zng fmt` of widgets with multi value property assigns.

# 0.11.3

* Add `IS_MOBILE_VAR`, `is_mobile` and `force_mobile` context var and properties.
* Add `WindowVars::safe_padding` and implement it for Android.
* **Breaking** Add `WindowOpenData::safe_padding` and `WindowChanged::safe_padding` to the view API.
    - Note that this is only a breaking change for direct dependents of `zng-view-api` and `zng-app`.
* Fix text input moving caret on focus.
* Fix interactive caret touch causing loss of focus on the text input.
* Implemented keyboard support for Android (no IME).
* Add `--cfg=zng_view_image_has_avif` for `zng-view` to support building AVIF.
    - See [docs/avif-setup.md] for more details.
* `Markdown!` now supports definition lists.

# 0.11.2

* Implement initial `ColorScheme` for Android.
* Support `RUSTFLAGS` "deny warnings" in cargo zng.
* Warn when `.zr-copy` does not find the directory or file.
* Refactor `.zr-apk` to not require to be inside the staging dir.
* Refactor `Impl Future` parameters into `impl IntoFuture`. 
* Implement `IntoFuture for ResponseVar<T>`.
* Remove `.zr-apk` requirement of extension on the folder name.

# 0.11.1

* Add `zng::env::android_install_res` helper.
* Add `zng::env::android_external`.
* Add `zng_env::android_internal`.
* Add `zng::view_process::default::android`.
* Implement Android suspend/resume cycle using the existing "respawn" API.
* Add `APP.is_suspended` var.
* Add `VIEW_PROCESS_SUSPENDED_EVENT`.
* `VIEW_PROCESS_INITED_EVENT` now notifies a "respawn" on resume after suspension.
* **Breaking** Add `Event::Suspended`.
    - Note that this is only a breaking change for direct dependents of `zng-view-api`.
* Add `ViewExtension::suspended/resumed`.
* Implement system fonts query for Android.
* Implement conversions from `FontStyle`, `FontWeight` and  `FontStretch` to the `ttf-parser` equivalent types.
* Implement `PartialOrd, Ord` for `FontName`.
* Add `zng_view::platform`.
* Implemented Android `run_same_process` entry point.
* Fixed Android build errors.
* Fix gradient stops that mix positional stops with offset stops. 
* Fix build in platforms without `AtomicU64`.
* Fix `zng::env::bin` in Wasm builds.

# 0.11.0

* **Breaking** Remove `OutlineHintingOptions` and change `Font::outline` signature.
* **Breaking** Remove `FontFace::font_kit`, `Font::advance`, `Font::origin` and `Font::typographic_bounds`.
* Fix crash window summary tab when there are no localization resources.
* Replace `breakpad-handler` with `minidumper` + `crash-handler`.
    - This removes dependency on native breakpad, a common cause of compilation issues.
* Fix large rendered window icon resize.
* Fix Emoji color palette panic (Windows 11 Emoji).
* **Breaking** Remove `0.10.5` deprecated items.
* Fix `FONTS` matching obsolete Type1 fonts when there is an OpenType alternative.
* **Breaking** `FONTS.system_fonts` now returns a `ResponseVar`.
* **Breaking** Replaced harfbuzz backend, `font::Face::harfbuzz` and `font::Font::harfbuzz` are the new accessors.
* The `"wasm-unknown-unknown"` target now compiles without error.
    - `zng::time` works.
    - `zng::env::on_process_start!` and `init!` works with a small JS setup requirement.
    - `zng::app::print_tracing` and panics log to browser console.
    - View-process is **not implemented**, only headless without renderer apps can run on this release.
    - Unfortunately many dependencies compile without actually supporting Wasm and panic during runtime, these will be fixed gradually.

# 0.10.5

* Add `cargo zng fmt` that formats normal code with `cargo fmt` + Zng and other macros.
    - See [`cargo-zng/README.md`](./crates/cargo-zng/README.md#fmt) for details on IDE integration.
* Fix named `Align` deserialization from human readable formats.
* Fix `SelectableText!` shorthand syntax.
* Fix layer `AnchorSize::Window` not filling the window space by default.
* Fix `ContextCapture::NoCapture` excluding popup config.
* Add `ResponseVar::map_response`.
* Add `Dialog!` widget, `DIALOG` service and related types.
* *Deprecated* `http::get_text`, ` http::Client::get_text` and `Var::get_text`.
    - Renamed to `get_txt`.
* *Deprecated* `zng::window::native_dialog` module.
    - The new `zng::dialog` is the new surface API for all dialogs.
    - The underlying native dialogs will not be removed, just the surface API.

# 0.10.4

* `DInstant` addition now also saturates like subtraction.
    - Fixes crash in systems without caret blink animation.
* Add `return_focus_on_deinit`.
* Add a copy button to the markdown links popup.
* Add warning for slow event handlers in debug builds.
* Add in memory "clipboard" for headless tests that use `CLIPBOARD`.
* Fix `tooltip` showing instead of `disabled_tooltip` in contexts that disable the widget after a slight delay.
* Fix tooltip opened by `ACCESS.show_tooltip` closing immediately on mouse leave.

# 0.10.3

* Fix view-process sometimes never connecting in slow machines.
* Fix `#.#.#-local` localization not matching the app resources.

# 0.10.2

* Localization now scraps `#.#.#-local` workspace dependencies.
* Add `Var::hold`, `AnyVar::hold_any`.
* Add `AnyVar::perm`, `VARS::perm`.

# 0.10.1

* Fix race condition in command metadata init when parallel widgets read the same metadata.
* Fix `cargo zng l10n` not generating a .gitignore file for deps.
* Implement serialization for l10n types.

# 0.10.0

* **Breaking** Removed support for `{lang}.ftl` localization files, now is named `{lang}/_.ftl`.
* **Breaking** Removed `L10N.localized_message`, use `L10N.message(..).build_for(lang)`.
* **Breaking** `cargo zng l10n` CLI refactor.
    - Now requires arg name for input and output.
    - Pseudo arg values now define a dir and lang to generate pseudo from.
* Add `cargo zng l10n --package/--manifest-path` for scrapping from lib crates for publishing.
* Fix localization scrapper only adding section comments to main file.
* Add localization helper for commands.
    - Set `l10n!: true` in `command!` declarations to localize metadata.
    - Use `cargo zng l10n` to scrap metadata.
* **Breaking** Use HashSet for `EVENTS.commands`.
* Impl `std::hash::Hash` for `AppLocal<T>`, `AnyEvent` and `Command`.
* Add `zng::button::PrimaryStyle`.
* **Breaking** View API now groups color scheme with a new accent color config.
* **Breaking** Refactored "color pair".
    - Now named `LightDark`.
    - Removed helper methods for declaring vars, now `IntoVar<Rgba> for LightDark` is contextual.
    - Add `light_dark` helper function and `LightDarkVarExt` helper methods.
* **Breaking** Removed `BASE_COLORS_CAR` form specific widgets, now use the unified `zng::color::BASE_COLOR_VAR`.
* Add `TextEditOp::clear`.
* Add `button::LightStyle!()` and `toggle::LightStyle!()`.
* Fix when expr not recognized.
* Fix `WINDOWS.is_loading`.
* Add `WINDOWS.wait_loaded`.
* **Breaking** Refactored `Button::cmd_param` to accept any type var.
* Fix `SelectionBy::Mouse` never being set on mouse selection.
* Add auto-selection on click, when the action does not disrupt the user.
* **Breaking** Refactored `AutoSelection` into bitflags that implement more features.
* Add  `CONFIG.insert`.
* **BReaking** `Config::get` and `AnyConfig::get_raw` now also receives an `insert` boolean.
* **Breaking** `CONFIG.get` and `Config::get` now receive a default value, not a closure.
    - The closure was immediately evaluated by most config backends.
* **Breaking** Refactored `zng_wgt_window::SaveState`.
    - Is now in `zng::config::SaveState`.
    - Does not define window load wait time.
        - A new `Window::config_block_window_load` property handles blocking for all configs on window.
    - Add `zng::config::save_state_node` helper for declaring state persistency properties for other widgets.
    - Automatic config keys now require an ID name.
* Fix "child_insert" layouts when the children end-up having a single full widget and other non-widget nodes.
* Add `TextInput::placeholder`.
    - Add `TextInput::placeholder_txt`.
* Add `Container::child_under/over`.
    - Add `ChildInsert::Under/Over`.
* Add `zng::text_input::SearchStyle`.
* **Breaking** Refactored `zng::icon::material*` modules and `zng-wgt-material-icons`.
    - Removed consts for each icon.
    - Modules renamed to `zng::icon::material::*`.
    - Now uses a `phf` map from string names.
        - To convert an old const name, lowercase + replace '_' with '-' and strip 'N' prefix if the next char is a number.
          Example: `N1K_PLUS` -> `"1k-plus"`.
    - Now registers `ICONS` for each name.
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
