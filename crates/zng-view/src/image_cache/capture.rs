use std::sync::Arc;

use zng_task::{channel::IpcBytes, parking_lot::Mutex};
use zng_txt::formatx;
use zng_unit::{Factor, PxDensity2d, PxDensityUnits as _, PxRect};
use zng_view_api::{
    Event,
    image::{ColorType, ImageDecoded, ImageId, ImageMaskMode, ImageMetadata},
    window::{FrameId, WindowId},
};

use crate::{
    AppEvent,
    image_cache::{Image, ImageData, dyn_image::IpcDynamicImage},
};

use super::ImageCache;

impl ImageCache {
    /// Create frame_image for an `Api::frame_image` request.
    pub fn frame_image(
        &mut self,
        gl: &dyn gleam::gl::Gl,
        rect: PxRect,
        window_id: WindowId,
        frame_id: FrameId,
        scale_factor: Factor,
        mask: Option<ImageMaskMode>,
    ) -> ImageId {
        if frame_id == FrameId::INVALID {
            let id = self.image_id_gen.lock().incr();
            let _ = self.app_sender.send(AppEvent::Notify(Event::ImageDecodeError {
                image: id,
                error: formatx!("no frame rendered in window `{window_id:?}`"),
            }));
            return id;
        }

        match self.frame_image_data(gl, rect, scale_factor, mask) {
            Ok(data) => {
                let id = data.meta.id;
                let _ = self.app_sender.send(AppEvent::Notify(Event::ImageDecoded(data)));
                id
            }
            Err(e) => {
                let id = self.image_id_gen.lock().incr();
                let _ = self.app_sender.send(AppEvent::Notify(Event::ImageDecodeError {
                    image: id,
                    error: formatx!("cannot create frame image for window `{window_id:?}`, {e}"),
                }));
                id
            }
        }
    }

    /// Create frame_image for a capture request in the FrameRequest.
    pub fn frame_image_data(
        &mut self,
        gl: &dyn gleam::gl::Gl,
        rect: PxRect,
        scale_factor: Factor,
        mask: Option<ImageMaskMode>,
    ) -> std::io::Result<ImageDecoded> {
        let (format, og_color_type) = match gl.get_type() {
            gleam::gl::GlType::Gl => (gleam::gl::BGRA, ColorType::BGRA8),
            gleam::gl::GlType::Gles => (gleam::gl::RGBA, ColorType::RGBA8),
        };
        let pixels_flipped = gl.read_pixels(
            rect.origin.x.0,
            rect.origin.y.0,
            rect.size.width.0,
            rect.size.height.0,
            format,
            gleam::gl::UNSIGNED_BYTE,
        );
        let mut buf = IpcBytes::new_mut_blocking(pixels_flipped.len())?;
        assert_eq!(rect.size.width.0 as usize * rect.size.height.0 as usize * 4, buf.len());
        let stride = 4 * rect.size.width.0 as usize;
        for (px, buf) in pixels_flipped.chunks_exact(stride).rev().zip(buf.chunks_exact_mut(stride)) {
            buf.copy_from_slice(px);
        }

        if let Some(mask) = mask {
            let density = 96.0 * scale_factor.0;
            let density = Some(PxDensity2d::splat(density.ppi()));

            let r = if format == gleam::gl::BGRA {
                Self::convert_bgra8_to_mask_in_place(rect.size, buf, mask, density, None, &self.resizer)
            } else {
                Self::convert_decoded(
                    IpcDynamicImage::ImageRgba8(
                        image::ImageBuffer::from_raw(rect.size.width.0 as u32, rect.size.height.0 as u32, buf).unwrap(),
                    ),
                    Some(mask),
                    density,
                    None,
                    None,
                    image::metadata::Orientation::NoTransforms,
                    &self.resizer,
                )
            };

            // frame size is not large enough to trigger a memmap that can fail;
            let (pixels, size, density, is_opaque, is_mask) = r.unwrap();

            let id = self.image_id_gen.lock().incr();
            self.images.insert(
                id,
                Image(Arc::new(ImageData::RawData {
                    size,
                    range: 0..pixels.len(),
                    pixels: pixels.clone(),
                    is_opaque,
                    density,
                    stripes: Mutex::new(Box::new([])),
                })),
            );

            let mut meta = ImageMetadata::new(id, size, is_mask, og_color_type);
            meta.density = density;
            Ok(ImageDecoded::new(meta, pixels, is_opaque))
        } else {
            if format == gleam::gl::RGBA {
                for rgba in buf.chunks_exact_mut(4) {
                    rgba.swap(0, 3);
                }
            }

            let is_opaque = buf.chunks_exact(4).all(|bgra| bgra[3] == 255);

            let data = buf.finish_blocking()?;
            let density = 96.0 * scale_factor.0;
            let density = Some(PxDensity2d::splat(density.ppi()));
            let size = rect.size;

            let id = self.image_id_gen.lock().incr();
            self.images.insert(
                id,
                Image(Arc::new(ImageData::RawData {
                    size,
                    range: 0..data.len(),
                    pixels: data.clone(),
                    is_opaque,
                    density,
                    stripes: Mutex::new(Box::new([])),
                })),
            );
            let mut meta = ImageMetadata::new(id, size, false, og_color_type);
            meta.density = density;
            Ok(ImageDecoded::new(meta, data, is_opaque))
        }
    }
}
