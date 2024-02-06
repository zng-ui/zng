# View-Process TODO

* Implement direct-composition to support effects like semi-transparent blur the pixels "behind" the window.
        See: https://github.com/servo/webrender/blob/master/example-compositor/compositor/src/main.rs

## API

* Remove webrender-api from view-api?
    - We can provide better documented types that map directly to webrender.
    - This change enables supporting alternative renderer backends more directly?

## Extensions

* Implement better custom OpenGL texture example.
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