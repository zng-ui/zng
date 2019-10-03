# TODO

## Core

Things that must be done first because they influence the core API design that is used to do all the rest.

* Focusable, keyboard events only when focused, automatic TAB navigation, focus events, user code focus.
  * Focus scope, can nest, ESC returns to parent scope.
  * Use FocusKey?
* Enabled/Disabled.
* Images.
* Raw OpenGL textures.
* Automatic screen reader integration.
* Animation, transition and storyboarding.
* Localization.
* Theming.
* Scrolling.
* Other DisplayListBuilder (iframe).
* Drag-drop.
* Modal Window.
* Workers.
* Diagnostics.
* Better Ui related macros, reduce verbosity.

## Basic Layout

* Align.
* Min/Max size.
* Grid.
* WrapPanel.
* DockPanel.

## Basic Events

* Implement all basic events.
* Implement capturing/tunneling event counterparts.

## Basic Widgets

* Progress indicator.
* Button (work button, default, primary, cancel, etc).
* Validation.
* TextInput.
* CheckInput.
* ToggleInput.
* Slider.
* ContextMenu.
* ToolTip.
* Resize parent.
* MainMenu.

## Virtualizing Widgets

Widgets that contain many elements that must be loaded on demand.

* ListView.
* TreeView.
* GridView.
* TabView.
* Infinite Canvas.

## Dialogs

* Message dialogs (themable?).
* File dialogs (not themable?).

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

## Themes

* Dark/Light default.
* High contrast.
* Fluent Design (Windows 10).
* Material Design (Google).

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
* [List of Widgets](https://www.telerik.com/products/wpf/overview.aspx)
