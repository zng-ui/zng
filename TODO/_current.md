# TextInput

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement automation/screen reader APIs.

# WR Items
    - Touch events.
        - Use `Spacedesk` to generate touch events.

# Extend-View

* Implement OpenGL texture image example.
    - `webrender_api::ExternalImageSource::NativeTexture`.
    - Image cache access for loading textures?
        - Optional loading bitmap.
        - Required inserting special `ImageData` for the `ExternalImageHandler`.
    - Need to expand `ExternalImageHandler` to support textures.
    - We implement the `ExternalImageId` as a raw pointer to an `Arc<ImageData>`. 