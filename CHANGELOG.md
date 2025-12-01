# Unreleased

* Add support for gigapixel images (decoded bgra length > i32::MAX). !!: TODO

* **Breaking** Refactor `ImageSource`.
    - `Data` now holds `IpcBytes` directly.
    - Removed `Static` because it is always converted to `IpcBytes` anyway.

* **Breaking** Refactor `zng::task::http` into a backend agnostic API.
    - Removed unmaintained `isahc` dependency.

* **Breaking** Refactor `zng::task::io::ReadLimited`.
    - No longer generic over the error function.
    - Now also implements `BufRead`, `Read` and `AsyncBufRead`.
    - Add constructor with default error.
* **Breaking** Refactor `zng::task::io::Measure`.
    - Now uses var to track progress.
    - Now also implements `BufRead`, `Read`, `Write` and `AsyncBufRead`.

* Refactor `WhiteSpace` merging to better integrate with basic paragraph spacing.
    - `Merge` now also merges multiple line breaks into a single one.
    - **Breaking** Added `MergeParagraph`. Merges spaces and trim lines. Removes single line breaks. Merge multiple line breaks.

* **Breaking** Remove `img_scale_factor` and `img_scale_density` properties.
    - Added `img_auto_scale` property and related `ImageAutoScale` enum.

* **Breaking** Remove `Animation::sleep_restart`. Add restart flag to `Animation::sleep`.

* Unify channel types.
    - **Breaking** Removed `AppChannelError`, `EventReceiver`.
    - **Breaking** Removed `bytes_channel` and related types. Use an IPC channel with `IpcBytes` messages.
    - **Breaking** Removed `AppEventSender` *extension* channels. Simply create a wrapper that sends the message and awakes the app.

* Refactor IPC and worker process API.
    - **Breaking** Remove `zng::task::ipc`.
    - Add `zng::task::channel::ipc_channel` and related types.
    - Add `zng::task::process::worker` with the same worker process types.
    - IPC types are now also available in builds without `"ipc"` feature, internally patched to use normal channels.
    - **Breaking** Remove `zng::task::channel::{UnboundSender, UnboundReceiver}`.
    - **Breaking** Unified channel error types.
    - **Breaking** Remove conversions from underlying channel types.
    - **Breaking** Remove public `duct` crate types.

* Add blocking API for `zng::task::channel` sender and receiver.
* Fix `Window!` config properties trying to use `CONFIG` in builds without `"config"` feature.

# 0.19.2

* Implement workaround deadlocks caused by the `notify` crate.
* Improve advanced animation API. 
    - Add `EasingTime::seg` helper for segmenting an animation overall time for sub-animations.
    - **Deprecated** `Animation::restart_count` renamed to `count`.
    - Add `Animation::set_count`.
    - Add `Animation::sleep_restart`.
* Change `mask_image` to apply to the borders too.
* Add `zng::process::CircularStyle`.

# 0.19.1

* Implement basic paragraph support in base `Text!` widget.
    - Added `paragraph_break` property that defines how the text is split in paragraphs.
    - Added `paragraph_spacing` property to `Text!`.
    - Added `paragraph_indent` property for inserting space on the first line or hanging other lines.

* Show layout metrics used in the Inspector.
* Fix `ProgressView!` default style not showing indicator.
* Improve `save_state` properties and node.
    - Now restores state as soon as config is loaded, ideally before window is loaded.
    - This fixes issue when window presents one frame at restored size and then maximizes.
* Image widget now ignores `img_scale_factor` when `img_scale_density` is enabled.
* Implement `FromStr` for `PxDensity`. Fixes regression, the previous density units implemented parse.

# 0.19.0

* Add `"view_hardware"` feature to make hardware rendering optional.
    - On Windows the `"view_software"` renderer uses ~20MB less RAM than Nvidia OpenGL drivers.
    - Only recommended for small apps with simple UIs.
    - If you hand pick features (recommended for release builds) you **must enable** this to retain hardware acceleration.

* Optimize system fonts memory use.
    - **Breaking** Removed `FontDataRef`.
    - Added `FontBytes` that can efficiently reference bytes in various formats, including memory maps.
    - **Breaking** View API `add_font_face` now receives a `IpcFontBytes`.
    - Refactored `ColorGlyphs` and `ColorPalettes` to parse on demand.
    - Windows builds with default fonts now uses ~20MB less memory.

* Inherit `StyleMix` for `Window` to facilitate theme implementation.
    - See `zng::window` documentation for theming tips.

* Unify pixel density units.
    - **Breaking** Removed `zng::{layout::{Ppi, Dpi, ResolutionUnits}, image::ImagePpi}`.
    - Added `zng::layout::{PxDensity, PxDensity2d, PxDensityUnits}`.
    - **Breaking** Renamed all *ppi* to *density*.

* Unify `MixBlendMode` type with view API.
    - **Breaking** Remove `zng::color::RenderMixBlendMode`.
    - **Breaking** `zng::color::MixBlendMode` is now non-exhaustive and has a new variant `PlusLighter`.

* **Breaking** Task functions that capture panic now return `zng::task::TaskPanicError`.
* Fix gradient stops with midway adjustment.
* Impl of `Add` and `Sub` for `layout::Vector` is now generic over any type that converts to vector.
* Add `Var::chase_begin` to begin a deferred chase animation.
* Add `zng::app::memory_profiler` for recording DHAT heap traces.
* Add `zng::task::set_spawn_panic_handler` for apps to optionally handle panics in spawn and forget tasks.
* Refactor `toggle::{select_on_init, deselect_on_deinit}` to ignore reinit (quick deinit/init).
* Token data package for custom property assign attributes is now documented.
* `#[property]` argument `widget_impl` now accepts multiple widget targets.
* Task worker process timeout is now configurable with ZNG_TASK_WORKER_TIMEOUT env var.
* View process timeout is now configurable with ZNG_VIEW_TIMEOUT env var.
* Fix view-process recover when it stops responding.

# 0.18.2

* Fix `async_hn!` handlers in the app scope unsubscribing after first event.
* Fix layout dependent on `LayoutMetrics::screen_ppi` not updating on change.
* Add `Var::flat_map_vec` for mapping `Var<Vec<T>>` to `Var<Vec<O>>` where `T` projects a `Var<O>`.
* Default setting editor reset button is now localizable in the `zng-wgt-settings` l10n resources.
    - Also surface it in `zng::config::settings::editor::reset_button` for use in custom editors.
* Implement `FromStr` for resolution units.
* Implement more Option conversions for response vars.
    - You can now `map_into` from `ResponseVar<Option<T>>` to `Var<Option<T>>`.
* Add `flat_expr_var!`, a helper for declaring an expression var that flattens.
* Fix EXIF metadata reading to define `ppi`.

# 0.18.1

* Implement color management in `zng-view`, supports ICC profiles and PNG gamma, chromaticities.
* Update default inspector watchers. Root widget now watches some general stats. All widgets now show *actual_size*.
* Fix layout unit display print issues, precision propagation.
* Implement image `ppi` metadata in `zng-view` for JPEG, PNG and TIFF.
* Add pixels-per-centimeter support in `ImagePpi`.

# 0.18.0

* Refactor child insert properties (`child_top`, `child_start` and others).
    - **Breaking** Removed `spacing` input from each property.
    - Add `child_spacing` and `child_out_spacing` properties that now configures the spacing.

    To migrate remove the second input, and if it was not zero set it in the new spacing properties.

* Refactor handlers to enable args type inference.
    - Added unified `Handler<A>` type.
    - **Breaking** Removed `WidgetHandler` and `AppHandler` trait.
    - **Breaking** Removed `FilterWidgetHandler` and `ArcWidgetHandler` struct.
    - **Breaking** Removed `app_hn!`, `app_hn_once!`, `async_app_hn!` and `async_app_hn_once!`.
    - App scoped handler are just normal handlers now, with an `APP_HANDLER` contextual service to unsubscribe from inside.

    To migrate app handlers remove `app_` prefix with normal. To migrate custom event property declarations replace `impl WidgetHandler<A>` with `Handler<A>` and execute the async task is needed. Other use cases will continue working, you can now omit the args type in most handlers. 

* Add build action properties.
    - Properties that modify the widget build, with the same access level as a widget build action.
    - `#[property]` now accepts functions with signature `fn(&mut WidgetBuilding, ...)`.
    - **Breaking** Removed `capture` from `#[property]`, migrate to an empty mixin property.
    - **Breaking** Renamed old "property build action" to "property attributes". This is a more accurate name and avoids confusion.

* Refactor how the default style is declared.
    - **Breaking** Removed `style_base_fn`.
    - **Breaking** `impl_style_fn!` macro now also requires the default style name.

* **Breaking** `Length` and `LengthExpr` are now `non_exhaustive`.

* **Breaking** Rename *logo* to *super* in `ModifiersState`. This was missed in a previous breaking refactor.

* Detect and recover from view-process not responding.
    - **Breaking** Added `Api::ping` and related items to the view-process API.
    - Only breaking for custom view-process implementers.

* Fix menu not appearing in Inspector Window.

# 0.17.4

* Fix misaligned icons in `Menu!`.

* Named styles now can also be modified in context.
    - Add `style::impl_named_style` macro and associated items.
    - Add `button::{light_style_fn, primary_style_fn, link_style_fn}`.
    - Add `dialog::{ask_style_fn, confirm_style_fn, error_style_fn, info_style_fn, warn_style_fn}`.
    - Add `menu::{context::touch_style_fn, icon_button_style_fn}`. 
    - Add `progress::simple_bar_style_fn`.
    - Add `text_input::{search_style_fn, field_style_fn}`.
    - Add `toggle::{combo_style_fn, check_style_fn, radio_style_fn, light_style_fn, switch_style_fn}`.

* Add `"deadlock_detection"` to default `"dev"` feature.
* Style more widgets inside `Menu!` root.
    - Add `menu::{TextInputStyle, ComboStyle}`.

# 0.17.3

* Fix default accent color and color scheme not using system values in Ubuntu.
* Fix default accent color and color scheme not updating on Windows settings change.
* Default `Window!` now uses `base_color` to define the background color.
* Implement `ByteUnits` for `f64` and add associated functions `ByteLength::from_*_f64`.
* Implement `FromStr` for unit types, parses the primary `Debug` and `Display` formats.
* Fix unrecoverable crash in respawned view-process not reaching the crash-handler-process.
* Fix respawn when a view-process panic happens during window creation.

# 0.17.2

* Add `zng::gesture::is_pointer_active` and related properties.
    - Allows implementing media player like controls, that vanish after a while without cursor movement.
* Fix `cargo zng fmt` for very long `when` expressions.
* Add `zng::rule_line::collapse_skip` property.
* Add `Menu::has_open` state property.
* Fix `zng::rule_line::CollapseMode::MERGE` not applying.

# 0.17.1

* Optimize read-only variables, now all var kinds are zero-cost.
* Fix `VarCapability::is_const` for contextual read-only variables.
* Fix `IMAGES.reload` panic on error.
* Add `IMAGES.watch` for auto reloading image that is modified.
    - Implemented in `zng::fs_watcher::IMAGES_Ext` and imported in the prelude.
* Add `Length::has_default` that inspects `Expr` values.
    - Method also added to all `Length` based composite units.
* Fix `length::replace_default` not considering `Expr` values.
* Fix `Length::Default` having a layout effect in exact size properties.
    - This default value now *disables* the properties, the required behavior for integration with `when` blocks in widgets.
    - Changed properties: `layout::{size, max_size, force_size}` and related properties.
* Fix buttons at the top-level of `Menu!` not returning focus on click.
* View-process now tries to guess image format in case of header decode error for an extension or mime defined format.
* Fix `cargo zng fmt` wrap instability inside macros.

# 0.17.0

This release contains breaking changes that affect the build and app startup.

* **Breaking** Removed deprecated crate features and deprecated items.

    To migrate, first update to latest `0.16` version and build, since `0.16.3` building with deprecated features print
    a warning message with fix suggestions. The `0.16.3` release notes also contains details about the changes.

* **Breaking** Change `APP.is_running` to only be `true` when the app is actually running.
    - Add `APP.is_started` to track app lifetime start, during the app extensions setup.
    - Services that require an app extension to function now assert this on first use.

    These changes are **runtime breaking**, trying to use app services after build starts and before run now panics.
    To migrate move all init code that is not app extension setup to inside `APP.run*`. This change affects users of `cargo zng new`
    with the default `zng-template`.

* **Breaking** Renamed `ModifiersState` LOGO -> SUPER that was missed from a previous rename.
* **Breaking** Syntax of the advanced `font_variations!` macro changed to better integrate with `cargo zng fmt`.
* **Breaking** Advanced Window API crate `zng-ext-window` now has feature `"image"` for optionally compiling image API.
* **Breaking** Renamed `UiNodeOpMethod` to `UiNodeMethod` and added list methods, to support tracing list nodes.

* Fix feature `zng/config` not enabling required `zng/fs_watcher`.

* Add `read` associated function for `zng::config::{JsonConfig, RonConfig, TomlConfig, YamlConfig}`.
    - This is a more efficient alternative to wrapping `sync` with `ReadOnlyConfig`.

# 0.16.6

* Fix focus scope `Popup!` widgets not focusing first descendant in some cases. 

* Add `ShortcutText!` widget for displaying keyboard shortcuts.
    - Add `zng-wgt-shortcut` crate and `zng::shortcut_text` module.
    - Also provides localization for key names.
    - Widget used in menu command shortcut styles and command button tooltip.

* Improve `zng::fs_watcher`, resolve config desync issues.

* Implement superscript/subscript styling in default `Markdown!`.
* Fix `Markdown!` whitespace merging.

* Add `zng::rule_line::collapse_scope` for auto *collapsing* multiple separators that are adjacent due to collapsed siblings.

* Improve `Menu!` default contextual styles.
    - `Vr!()` height is now `1.em()`, making it visible in the menu `Wrap!` panel.
    - Add `zng::menu::sub::{ButtonStyle, ToggleStyle}`, these are the same *menu item* styles, they are now applied in the `SubMenu!` widget.
    - Refactor `zng::menu::{ButtonStyle, ToggleStyle}` to apply to only widgets in the `Menu!` root, as a sort of *toolbar item* look.
    - Add `zng::menu::IconButtonStyle`, an optional style for buttons in menu root that only shows the command icon.

* Add `zng::rule_line::{hr::width, vr::height}` for contextually configuring the separator line length.

# 0.16.5

* Fix race condition in `zng::task::SignalOnce`.

* Event/command notify requested during a layout pass now all run before the next render pass.
    - After a layout pass the app does an updates pass (unchanged) and then does an app events pass (new).
    - See the `zng::app` module docs for more details.

* Fix command event properties notifying twice when set on the `Window!` widget and raised by shortcut press.

* Refactor `UiNode::trace` to work with widget and list nodes too.

* Refactor `actual_size` and related `actual_*` properties to get the size on layout.
    - Before the size was sampled on render, now it is sampled before.
    - This change means any state hooked to the actual size will now update before the new size is visible on the next frame.

* Improve scroll widget's `ZOOM_TO_FIT_CMD`.
    - Add `zng::scroll::cmd::ZoomToFitRequest` for configuring if the scale change is animated.
    - Add `Scroll::zoom_to_fit_mode` property for configuring if smaller content scales up to fit.

* Add `zng::mouse::ctrl_scroll` contextual property.
    - Also strongly associated with `Scroll!` widget as `Scroll::ctrl_scroll`.
    - When enabled inverts priority of mouse wheel gesture so that it zooms when no modifier is pressed and scrolls when `CTRL` is pressed.

# 0.16.4

* Fix bitflags serialization error in RON configs.
* Add `Window::parallel` and `WINDOW.vars().parallel` that has the same effect as the standalone property plus it also applies
  to window *root extensions*.
  - Fixes window with disabled parallel still running in another thread when built with the inspector extension.
* Changed `zng::widget::parallel` property to apply for all nodes in an widget, not just context nodes and inner.
* Fix `UiVec::render_update` applying twice to some children when parallel is enabled.

* Add `is_inner` in the layout constraints API.
    - **deprecated** `PxConstraints::fill` field and `fill_pref` method.
    - Added `PxConstraints::is_fill`, `is_inner` and `with_inner` methods.
    - This is an advanced API for custom layout implementers only. The normal align/fill API remains the same.

* Fix `Grid!` layout.
    - Exact size columns/rows now are sized correctly.
    - Columns `min/max_width` and rows `min/max_height` are now respected in auto sized or leftover sized columns/rows.
    - Default sized columns are now proportionally downsized in case of overflow.

# 0.16.3

* Improve UI parallelization, now can also parallelize smaller lists if the child nodes are *heavy*.
    - Custom list nodes now should use `dyn UiNodeImpl::parallelize_hint` together with `PARALLEL_VAR` to enable parallelization.

* Add `UiNode::try_for_each_child`.
    - Custom list nodes must implement `UiNodeImpl::try_for_each_child`.

* In main crate add `"dev"` feature, replaces `"debug_default"`.
    - Feature is enabled by default, recommended setup of dev/release feature pair, see [docs](https://github.com/zng-ui/zng/tree/main/crates/zng#dev).
    - `"debug_default"` is now deprecated, it was an attempt to auto enable debug features in debug builds that causes issues in downstream release builds.
* In all component crates the `"debug_default"` now has all the default features and is deprecated.
    - Advanced users targeting component crates directly must select each feature actually needed.
    - Next breaking release will remove all deprecated features, making component crates have no features by default.

* Deprecated feature `"dyn_closure"`, no longer needed.
* Deprecated feature `"dyn_node"`, no longer needed.
* Add `Var<VarEq<T>>::flatten` method.

# 0.16.2

* Fix image request made before view-process init never loading.
* Improve view-process crash detection for respawn.
* x86_64-apple-darwin prebuilt view-process is now cross-compiled in the GitHub ARM runner.
    - Removed support for AVIF images in the prebuilt for this target.
    - Follow the `docs/avif-setup.md` guide to build view-process with AVIF support.

# 0.16.1

* Multiple improvements for `cargo zng fmt`.
    - Now only reformats modified files when running in crates/workspaces.
    - Add `--edition` option, 2024 is the default. This fixes inconsistency between workspace run and single file run.
    - Add support for `static ref` style macro syntax, like `lazy_static!`.
    - Add support for `command!`, `event_property!` syntaxes.
    - Add support for struct declaration, init style macro syntax, like `cfg_aliases!`.
    - Add support for `bitflags!` syntax.
    - Add support for simple ident list syntax.
    - Add support for `widget_impl!` syntax.
    - Reimplemented support for widget macros, now covers full syntax.
    - Add support for `when_var!` syntax.
    - Add support for Rust code blocks in Markdown files.
    - Add support for doctest code blocks.

* Fix missing mouse move events when cursor is captured and leaves the window in Windows.
* Implement `IntoUiNode` for `std::iter` iterators of `UiNode` items.
    - You can now omit `.collect::<UiVec>()` in code that generates widget lists from iterators.

# 0.16.0

This release contains breaking changes that affect the normal surface API. All changes are trivial to fix, its mostly a job for find & replace.

These changes where necessary to fix the rampant code bloat issue. Release builds of the example projects are now 55% smaller on average. 
Optimized release builds (following the `./docs/optimized-release.md` guide) are now 30% smaller.

* **Breaking** Refactor `zng::widget::node` API.

    Unified UI node (and list) types into a new `UiNode` struct, most node types are implemented using `match_node` and not affected,
    custom node types now must implement `UiNodeImpl`, it is a simplified version of the previous API. The main motivation
    for this refactor is reduction of generics code bloat, the usage of `-> impl UiNode` scales extremely bad as the anonymous
    type is monomorphised for each generic input, this combined with node nesting causes an explosion of code copies.

    To migrate UiNode:

    - Replace output `-> impl UiNode` with just `UiNode`.
    - Replace input `_: impl UiNode` with `_: impl IntoUiNode`.
    - Replace `NilUiNode` with `UiNode::nil()`.
    - Custom nodes now must implement `UiNodeImpl`.
    - Replace `#[ui_node]` impls with manual impl of `UiNodeImpl`. The proc-macro attribute was removed, the new 
      `UiNodeImpl` provides default impls for methods.

    To migrate UiNodeList:

    - Replace output `-> impl UiNodeList` with just `UiNode`.
    - Replace input `_: impl UiNodeList` with `_: impl IntoUiNode`.
    - Replace `EditableUiNodeList` with `EditableUiVec`.

    UI nodes and lists are the same thing now, panel widgets use `UiNode::is_list` to distinguish, normal nodes are layout and rendered
    as a list with a single item. You can also set lists directly on single child widgets, the multiple nodes will be Z-stacked.

* Fix `accepts_enter` and `accepts_tag` in text editor widgets. 
* Fix zero sized gradients causing render panic.

* **Breaking** Refactor `zng::var` API.
    
    Unified var types to new `Var<T>` and `AnyVar` structs. Variables still behave the same 
    and everything that could be done before can still be done with the new API. The main motivation
    for this refactor is reduction of generics code bloat, and since a breaking change is already happening
    some poorly named methods and functions where also renamed.

    To migrate:

    - Replace `impl Var<T>` and other var structs with `Var<T>`.
    - Replace `impl AnyVar` with `AnyVar`.
    - Replace `LocalVar(_)` with `const_var(_)`.
    - Replace `ContextualizedVar::new(_)` with `contextual_var(_)`.
    - Replace `Var::wait_value` with `Var::wait_match`.
    - Replace `Var::map_ref` with `Var::map`, `map_ref_bidi` with `map_bidi` or new `map_bidi_modify` in cases where
      the mapped value is a subset of the source value.
    - Now always use `Var::capabilities` to inspect *kind* of var.
    - Modify methods `Var::{set, update, modify}` now simply DEBUG log if the variable is read-only, 
      use `try_set, try_update, try_modify` to get the error.

* **Breaking** `zng::command_property::command_property!` now also generates contextual property and var that enable/disable the handlers.
    - Adds `zng::clipboard::{can_cut, can_copy, can_paste}`.
    - Adds `zng::config::settings::can_settings`.
    - Adds `zng::app::{can_new, can_open, can_save, can_save_as}`.
    - Will not generate this if `enabled:` is set.

* **Breaking** Refactor `MONITORS` state reporting to use variables.

* Fix deserialization of `PxConstraints` failing when the `max` field is not set and format is "human readable".

* **Breaking** Refactor `zng::config::SyncConfig` to use a map of `RawConfigValue` directly.
    - Removed `ConfigMap` trait, now use `SyncConfigBackend` to implement custom formats.
    - All provided formats work the same on the surface, this is only breaking for custom format implementers.

* **Breaking** Refactor `zng::config::RawConfigValue` to represent the full serde data model.
   - Removed default JSON support, use the new `"config_json"` feature to enable JSON config files.
   - Remove conversion implementations and related error types, can now (de)serialize directly to `RawConfigValue`.

* **Breaking** Refactor how view-process config events notify.
    - Initial non default config state now reported as events on init.
    - All config and monitors info removed from `ViewProcessInitedArgs` and related API.
* Fix `VARS.animations_enabled` not updating when it is not set and the `sys_animations_enabled` changes. 

* **Breaking** Refactor how raw device events are enabled on the view-process.
    - Now can dynamically enable/disable and with more precision of what kind of events.
    - Removed `enable_input_device_events` and all related API from view-process controller.
    - Added `APP.device_events_filter` variable that can be set to enable/disable device events.

* **Breaking** Remove all deprecated items.

* **Breaking** Refactor `zng::slider` API.
    - Removed direct support to std range type, use `Selector::many` with two values.
    - Selector `value_with` and `many_with` now expects `Sync` closures.
    - Thumb args now uses a variable to track the position.
* Fix `Slider!` not reacting to value changes.
* Fix inherited widget properties not showing in documentation.

* **Breaking** Add unimplemented audio decoding and playback to the view-process API in preparation of a future release.

# 0.15.11

* Fix `DIALOG.confirm` always cancelling.
* Fix auto scroll on text caret move, only apply if the `Text!` or rich text context has focus.
* Fix auto scroll on focus change, ignore focus change due to entire `Scroll!` disabling.
* Fix `Button!` inside `SubMenu!` not filling horizontal space when the sub-menu header is wider them the button.
* Change `Scroll!` child layout to act the same as a `Container!` in the dimensions scrolling is not enabled. 
* Add `InteractionPath` methods for checking if the path contains an widget with the given interactivity.
    - **Deprecated** multiple event args methods that reimplemented this feature.
* Fix disabled `SubMenu!` opening.

# 0.15.10

* **Deprecated** Zng features "ron", "toml" and "yaml" renamed to "config_ron", "config_toml" and "config_yaml".
* **Deprecated** the view-process API "raw devices", it is replaced by "raw input devices", distinct from audio or any other devices.
    - This change mostly renames types and adds `InputDeviceInfo` with device metadata.
    - This change is mostly advanced API only, only some renamed types and events surface in `zng`.
    - The normal processed window input events are not affected.
* Add audio devices to the view-process API in preparation for a future release.
* Add extension methods for generating node lists from vars, `present_list` and `present_list_from_iter`.
    - Implemented by `zng_wgt::node::VarPresentList` and `VarPresentListFromIter` traits that are reexported as `_` in the preludes.
* Add extension methods for generating nodes from vars, `present`, `present_opt` and `present_data`.
    - Implemented by `zng_wgt::node::VarPresent`, `VarPresentOpt` and `VarPresentData` traits that are reexported as `_` in the preludes.
* Add `zng::widget::node::list_presenter_from_iter`.
    - Reexported by the `prelude_wgt`.

# 0.15.9

* Add `ImageSource::linear_vertical` and `linear_horizontal` for generating fast gradient masks.
* Implement `Eq` and `Hash` for `Length`, `LengthExpr`, `Size`, `Line`, `Point`, `Factor2d`, `GridSpacing`, `FactorSideOffsets`, `Rect`, `SideOffsets`, `Vector`, `LinearGradientAxis`, `ColorStop`, `GradientStop`, `GradientStops`.
* Add background and foreground image properties.
    - An alternative to using the `Image!` widget as `background` or `foreground`.
    - Properties implemented in `zng-wgt-image` and surfaced in `zng::widget`.
    - `background_img`, `background_img_align`, `background_img_crop`, `background_img_fit`, `background_img_offset`, `background_img_opacity`, `background_img_repeat`, `background_img_repeat_spacing`.
    - `foreground_img`, `foreground_img_align`, `foreground_img_crop`, `foreground_img_fit`, `foreground_img_offset`, `foreground_img_opacity`, `foreground_img_repeat`, `foreground_img_repeat_spacing`.
* Fix `"http"` feature including the image widget crate in `zng`.
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
