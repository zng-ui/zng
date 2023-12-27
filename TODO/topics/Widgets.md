# Widgets TODO

Widget that need to be implemented.

* Implement visibility change event.
  - on_show, on_hide, on_collapse?
  - on_show, should also fire once after first render?
* Progress indicator.
* Button (work button, default, primary, cancel, progress disabled).
* Thumb, draggable.
* Slider.
* Resize parent.
* Charts.
* Media (audio & video).
* Image effects.
* Massive images (deep zoom).
* SVG images.
* HTML (CSS, no Js).
* Full Markdown (HTML+CSS).
* Diagram editor.
* Rich text editor.
* Browser hosting.
* Parallax.
* Morphing.
* PropertyGrid.
* Review widgets available in other frameworks?

## Basic Layout

Layout widgets.

* Implement `column_wrap`, wrap mode?
* Implement properties that work with any panel to draw lines in between items.
  - Also backgrounds and foregrounds for each item.
  - Cell borders that don't affect size.
  - Selection borders, backgrounds.
* Implement reverse `UiNodeList` and other ops in the lists directly.
  - Sorting and the z-index has worked well, implement other auto-updating "iterator inspired" helpers.

## Virtualizing Widgets

Widgets that contain many elements that must be loaded on demand.

* ListView.
* TreeView.
* GridView.
* TabView.
* Infinite Canvas.

## Window Widgets

Widgets that derive from Window and define type of app interaction.

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
