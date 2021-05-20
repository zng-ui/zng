# TODO

## Core

Things that must be done first because they influence the core API design that is used to do all the rest.

* Clone-move closure macro.
* Improve property allowed_in_when validation for generics, generate a `new` like call for each
  argument, instead of all at once.
* Test `#[cfg(..)]` support in widget declarations.
* Use something like this [https://docs.rs/crate/xss-probe/0.2.0/source/build.rs] to patch docs.
* Link property declaration to their source-code point.
* FIX: Reexported properties in widgets become the target of intra-doc links in some cases.
  - This is a bug in `rustdoc`, opened an issue: [https://github.com/rust-lang/rust/issues/83976]

* Focusable
  * Support more then one ALT scopes?
  * Configurable shortcuts for stuff like `Tab`?
  * Custom focus request?
  * Test directional navigation.
     * Including layout transformed widgets.

* Commands

* Window
  * Finish window vars.
  * Drag regions.
  * Custom resize borders.
  * Modal.

* Widget Inspector (console).
  * Debug print of some values is too verbose.

* Localization.
* Scrolling.
  * Virtualization.
* Images.
* Sound.
* Video.
* Raw OpenGL textures.
* Theming.
* Async.
  * IO bound workers.
  * CPU bound workers.

* Animation, transition and storyboarding.
* Automatic screen reader integration (UI Automation).
* Other DisplayListBuilder (iframe).
* Drag-drop.
* Support for external input methods (IMEs).
* Integrate thread_profiler.
* Diagnostics.
* State-of-art Text layout.

* Let properties detect and warn when they are used in weird places (like margin on a window).

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