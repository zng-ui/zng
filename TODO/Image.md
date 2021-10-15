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
* Optional pre-multiply. (can we undo pre-multiplication?)
    Or can we use pre-multiplied images for the window icon.
* Support creating resized image from existing image.
* Partial image API implemented, need to implement use in Images and try implement partial decoding using the `images` crate.
   - Also test if `ImageMetadataLoaded` event is happening before the full image is received.
* API for choosing the format in the request for image download.
* Error View.
* Cache cleanup after memory limit.

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