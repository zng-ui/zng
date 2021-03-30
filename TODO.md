# TODO

<del>

## When Steps
2 - widget! must generate function that takes property args reference and returns a `impl Var<bool>`.
2.a - function generated must handle `{self|child}.{property}(.{{arg_index}|arg_name})`.
3 - widget! must pass metadata about when functions to widget_new!
4 - ?

</del>

## Core

Things that must be done first because they influence the core API design that is used to do all the rest.

<del>

* Focusable
  * Know how a focus event was generated (what type of request).
  * More focus request types.
  * Let focusable know if it is the remembered return focus of a parent scope.
  * Focus on the closest existing sibling or parent in case the focused element is deleted.
  * Customizable focus indicators in focusable, (focused/remembered?/return target?/).
    * ESC hides focus indicator? Use knowledge of how focus was attained to show indicator?
  * Initial focus closest to mouse click?

* Enabled/Disabled.

</del>

* Localization.
* Scrolling.
  * Virtualization.
* Images.
* Raw OpenGL textures.
* Theming.
* Async.
  * IO bound workers.
  * CPU bound workers.

  <del>

  * Timers.

  </del>

* Animation, transition and storyboarding.
* Automatic screen reader integration (UI Automation).
* Other DisplayListBuilder (iframe).
* Drag-drop.
* Modal Window.
* Support for external input methods (IMEs).
* Integrate thread_profiler.
* Diagnostics.
* Better Ui related macros, reduce verbosity.
* All events receive single args object.
* State-of-art Text layout.
* Widget Inspector (console).

## Basic Layout

* Grid.
* WrapPanel.
* DockPanel.

## Basic Events

* Implement all basic events.
* Implement capturing/tunneling event counterparts.

## Basic Widgets

* Progress indicator.
* Button (work button, default, primary, cancel, etc.).
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
