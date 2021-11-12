# TODO

General TODO, items in this file can also be be a `../` file.

## Core

Things that must be done first because they influence the core API design that is used to do all the rest.

* Finish `./Docs.md`.

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

* Task.
  * Finish network tasks.
  * Test, check if WriteTask can be used without blocking after each write.
  * Implement panic handling for all tasks.

* Text Rendering, enable per-font config, https://docs.rs/webrender_api/0.61.0/x86_64-pc-windows-msvc/webrender_api/struct.FontInstanceOptions.html, integrate this with Renderer level config.

* Localization.
* App state serialization.
* Scrolling.
  * Virtualization.
* Vector Images, see https://github.com/servo/pathfinder.
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
* Diagnostics.

## Basic Events

* Implement all basic events.
* Implement capturing/tunneling event counterparts.

## Advanced

Hard to-do but does not mess with the core API.

* Plugins (run as a separate process that is hosted in an iframe like widget).
* Hosting Ui in a custom OpenGL window.
* Custom installers.
* App Settings that auto-generate some UI.
* Widget Inspector (UI).
* Full static HTML support for full markdown and e-book viewers.
* Use one renderer for all windows.
