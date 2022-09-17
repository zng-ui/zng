# Widgets TODO

Widget that need to be implemented.

* `view` as a full widget.
  - Need to support generics in widget constructors?

* Progress indicator.
* Button (work button, default, primary, cancel, progress disabled).
* Thumb, draggable.
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

## Basic Layout

Layout widgets.

* Refactor `v_stack/h_stack/z_stack` to `column/row/stack`.
* Refactor `wrap` to `row_wrap` and implement `column_wrap`.
* Implement properties that work with any panel to draw lines in between items.
  - Also backgrounds and foregrounds for each item.
  - Cell borders that don't affect size.
  - Selection borders, backgrounds.
* Implement `grid`, that works like overlapping row/columns.
* Implement reverse `UiNodeList` and other ops in the lists directly.
  - Like sorting and the z-index has worked well, other auto-updating "iterator inspired" helpers are probably pretty cool.

## Virtualizing Widgets

Widgets that contain many elements that must be loaded on demand.

* ListView.
* TreeView.
* GridView.
* TabView.
* Infinite Canvas.

## Window Widgets

Widgets that stay at the root of the Window and define type of app interaction that is used in the window.

* Custom decoration.
* Wizard.
* Ribbon.
* Docking editor.

## Themes

* Is a style collection.
* High contrast.
* OS imitation?

## OS Integration

* MainMenu.
* TaskbarItemInfo.

## Dialogs

* Message dialogs (styleable?).
* File dialogs (not styleable?).

## Inspector

Widget inspector.

* Console, a print-out of the window state.
* UI, an interactive window like the browsers inspectors.