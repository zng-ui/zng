# View-Process TODO

* Implement monitor changed event.
  - when monitor changes: See WindowVars::monitor()
  - actual_monitor: Computed by intersection between window and monitors? (the monitor area that contains more than half of the window?)

## API

* Remove webrender-api from view-api?
    - We can provide better documented types that map directly to webrender.
    - This change enables supporting alternative renderer backends more directly?

## Extensions

* Implement custom OpenGL texture example.
* Implement window extension.
    - Similar to `RendererExtension`, access to raw handle and window builder config.
      - Modify RendererExtension to include window stuff?

## Platforms

* OpenGL texture, in-game screen, multi-process tab apps.
* Android.
* WebAssembly.
  - Use HtmlElements to render?
  - Wait until we impl automation/screen readers.

# Clipboard

* Pasting the same image generates new images, could be the same one.