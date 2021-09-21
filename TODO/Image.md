# Image TODO

Image loading and rendering.

## Requirements

* All web formats (jpeg, bmp+ico, gif, png, webp).
* DPI size correcting.
* Color profile correcting.

## Nice to Have

* Progressive decoding.

## Questions

### How to avoid duplicating memory between app-process and view-process?

We can't decode images in the view-process only because if can crash, re-downloading images seems excessive,
keeping decoded images in the app-process memory also seems excessive, we should review how Firefox does image
caching and replicate.

### Image Support

* How to support metadata reading (dpi + CC) without parsing multiple times?
* How to support webp?
* How to support progressive decoding?

The `image` crate does not support these features, no
progressive decoding, no reading of dpi and color profile metadata.

We tried implementing progressive decoders, even just the BMP decoder seems like to much work to support.