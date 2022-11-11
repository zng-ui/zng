# Image TODO

## Requirements

* All web formats.
    - AVIF still a pain.
* DPI size correcting.
* Color profile correcting.
    Use `qcms` or `lcms2`
* Vector images (see Canvas.md).

## Nice to Have

* Progressive decoding.

## View-image TODO
* Support creating resized image from existing image.
* Partial image API implemented, need to implement use in Images and try implement partial decoding using the `images` crate.
   - Also test if `ImageMetadataLoaded` event is happening before the full image is received.

## images
* Cache cleanup after memory limit.
* Download/file blocking.
* Per-request limits.
* Improve limits error message.
* Optional hold window open (first layout) until an image is loaded.

## Image Support

* How to support metadata reading (dpi + CC) without parsing multiple times?
* How to support progressive decoding?

The `image` crate does not support these features, no
progressive decoding, no reading of dpi and color profile metadata.

We tried implementing progressive decoders, even just the BMP decoder seems like to much work to support.

## Solutions

There is an effort to support more metadata parsing in the `image` crate, see https://github.com/image-rs/image/pull/1448
so we can wait for now and focus in other features.


## Large Image

* Need to generate mipmaps, and virtual tiles for zoomed in.
    How to provide access to these? Can use channel index?
* Allow keeping image only at a size, to save memory for images that don't resize.
* Image widget need to allow defining min/max scale and offset.
    Different widget?