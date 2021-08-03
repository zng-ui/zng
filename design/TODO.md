# TODO

## Core

Things that must be done first because they influence the core API design that is used to do all the rest.

* Improve property allowed_in_when validation for generics, generate a `new` like call for each
  argument, instead of all at once.
* Test `#[cfg(..)]` support in widget declarations.
* Update the `Mouse` service to be like the new [`Keyboard`] service with button state variables.

* Docs
  * Normalize docs using guidelines: 
  https://deterministic.space/machine-readable-inline-markdown-code-cocumentation.html
  https://github.com/rust-lang/rfcs/blob/30221dc3e025eb9f8f84ccacbc9622e3a75dff5e/text/1574-more-api-documentation-conventions.md
  https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html

* StateKey & Map
  * Invert state key maps to be held in a thread-local for each key type? avoids value boxing
  * Associate name with unique ids.

* Focusable
  * Support more then one ALT scopes?
  * Configurable shortcuts for stuff like `Tab`?
  * Custom focus request?
  * Test directional navigation.
     * Including layout transformed widgets.
  * Mnemonics.

* Widget
  * Private nodes in widget, see `widget_private_nodes_design.rs`.
  * let properties reference other properties.
  * Support doc(cfg).
  * Support cfg in captures.

* Timers.
  * Unify with app handlers (missing on_interval docs).
  * Configurable `Instant::now` source, to advance time instantly in tests.
  * Time scale, for recording?

* Var.
  * Bind-into.
  * Bind-text.
  * Map-split.
    * Result.
    * Map/bind parse.
  * Strong-count.
  * Set but don't notify in case of single count.

* Task.
  * Finish network tasks.
  * Test, check if WriteTask can be used without blocking after each write.
  * Implement panic handling for all tasks.

* Text Rendering, enable per-font config, https://docs.rs/webrender_api/0.61.0/x86_64-pc-windows-msvc/webrender_api/struct.FontInstanceOptions.html, integrate this with Renderer level config.

* Window
  * Finish window vars.
  * Drag regions.
  * Custom resize borders.
  * Modal.
  * Review `redraw` event.
  * Video rendering.
  * Finish Screens, needs the window target + update all layouts on change.

* Widget Inspector (console).

* Localization.
* App state serialization.
* Scrolling.
  * Virtualization.
* Images.
* Sound.
* Video.
* Raw OpenGL textures.
* Theming.
* Async.
  * Parallel async tasks, including timers, large file write, and network ops.
* Animation, transition and storyboarding.
* Automatic screen reader integration (UI Automation).
* Other DisplayListBuilder (iframe).
* Drag-drop.
* Support for external input methods (IMEs).
* Integrate thread_profiler.
* Diagnostics.
* State-of-art Text layout.

## Basic Layout

* Grid.
* WrapPanel.
* DockPanel.

## Basic Events

* Implement all basic events.
* Implement capturing/tunneling event counterparts.

## Basic Widgets

* Progress indicator.
* Button (work button, default, primary, cancel, progress disabled).
* Validation.
* TextInput.
* CheckInput.
* ToggleInput.
* Slider.
* ContextMenu.
* ToolTip.
* Resize parent.
* MainMenu.
  * Auto-merge separators depending on the visibility of items in between.
* Basic markdown view.

## Virtualizing Widgets

Widgets that contain many elements that must be loaded on demand.

* ListView.
* TreeView.
* GridView.
* TabView.
* Infinite Canvas.

## Dialogs

* Message dialogs (themeable?).
* File dialogs (not themeable?).

## Commands

* Add common commands with localized name+info.
* Implement *check-box* commands.

## Window Widgets

Widgets that stay at the root of the Window and define type of app interaction that is used in the window.

* Custom decoration.
* Wizard.
* Ribbon.
* Docking editor.

## Advanced

Hard to-do but does not mess with the core API.

* Plugins (run as a separate process that is hosted in an iframe like widget).
* Hosting Ui in a custom OpenGL window.
* Custom installers.
* App Settings that auto-generate some UI.
* Widget Inspector (UI).
* Full static HTML support for full markdown and e-book viewers.
* Use one renderer for all windows.

## Themes

* Dark/Light default.
* High contrast.
* OS imitation?

## OS Integration

* MainMenu.
* TaskbarItemInfo.

## More Widgets

* Charts.
* Media (audio & video).
* Image effects.
* Markdown.
* Massive images (deep zoom).
* SVG images.
* Html (CSS, no Js).
* Diagram editor.
* Rich text editor.
* Browser hosting.
* Parallax.
* Morphing.
* PropertyGrid.
* Review widgets available in other frameworks?

## Docs

* Widget image/videos rendering from doc-tests.
* Link property declaration to their source-code point.
* Make a tool that can be used replace JS hacks with generated HTML.