# Image TODO

Image loading and rendering.

## Requirements

* All web formats (jpeg, bmp+ico, gif, png, webp).
* DPI size correcting.
* Color profile correcting.
    Use `qcms` or `lcms2`

## Nice to Have

* Progressive decoding.

## View-image TODO
* Reload functionality.
* Optional pre-multiply. (can we undo pre-multiplication?)
* Encoding API
    - Can we request frame and image pixels encoded?
* API for querying what encoders and decoders are available.
* Support creating resized image from existing image.
* Image data uploading API should support progressive upload.
* Implement limits in Images

## Questions


### Image Support

* How to support metadata reading (dpi + CC) without parsing multiple times?
* How to support webp?
* How to support progressive decoding?

The `image` crate does not support these features, no
progressive decoding, no reading of dpi and color profile metadata.

We tried implementing progressive decoders, even just the BMP decoder seems like to much work to support.

## Solutions

There is an effort to support more metadata parsing in the `image` crate, see https://github.com/image-rs/image/pull/1448
so we can wait for now and focus in other features.