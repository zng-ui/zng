# View-Process TODO

* Implement monitor changed event.
  - when monitor changes: See WindowVars::monitor()
  - actual_monitor: Computed by intersection between window and monitors? (the monitor area that contains more than half of the window?)

## Extensions

* Implement custom OpenGL texture example.
* Implement window extension.
    - Similar to `RendererExtension`, access to raw handle and window builder config.
      - Modify RendererExtension to include window stuff?
    - Use case?

## Platforms

* OpenGL texture, in-game screen, multi-process tab apps.
* Apple OS.
* Android.
* WebAssembly.
  - Use HtmlElements to render?
  - Wait until we impl automation/screen readers.

# Clipboard

* Pasting the same image generates new images, could be the same one.