use std::sync::Arc;

use webrender::api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat};
use zng_task::channel::IpcBytes;
use zng_txt::formatx;
use zng_unit::{Factor, PxDensity2d, PxDensityUnits as _, PxRect};
use zng_view_api::{
    Event,
    image::{ImageDataFormat, ImageId, ImageLoadedData, ImageMaskMode, ImageRequest},
    window::{FrameId, WindowId},
};

use crate::{
    AppEvent,
    image_cache::{Image, ImageData},
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
            let id = self.image_id_gen.incr();
            let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoadError {
                image: id,
                error: formatx!("no frame rendered in window `{window_id:?}`"),
            }));
            let _ = self.app_sender.send(AppEvent::Notify(Event::FrameImageReady {
                window: window_id,
                frame: frame_id,
                image: id,
                selection: rect,
            }));
            return id;
        }

        let data = self.frame_image_data(gl, rect, scale_factor, mask);

        let id = data.id;

        let _ = self.app_sender.send(AppEvent::ImageLoaded(data));
        let _ = self.app_sender.send(AppEvent::Notify(Event::FrameImageReady {
            window: window_id,
            frame: frame_id,
            image: id,
            selection: rect,
        }));

        id
    }

    /// Create frame_image for a capture request in the FrameRequest.
    pub fn frame_image_data(
        &mut self,
        gl: &dyn gleam::gl::Gl,
        rect: PxRect,
        scale_factor: Factor,
        mask: Option<ImageMaskMode>,
    ) -> ImageLoadedData {
        let data = self.frame_image_data_impl(gl, rect, scale_factor, mask);

        let flags = if data.is_opaque {
            ImageDescriptorFlags::IS_OPAQUE
        } else {
            ImageDescriptorFlags::empty()
        };

        self.images.insert(
            data.id,
            Image(Arc::new(ImageData::RawData {
                size: data.size,
                pixels: data.pixels.clone(),
                descriptor: ImageDescriptor::new(
                    data.size.width.0,
                    data.size.height.0,
                    if data.is_mask { ImageFormat::R8 } else { ImageFormat::BGRA8 },
                    flags,
                ),
                density: data.density,
            })),
        );

        data
    }

    fn frame_image_data_impl(
        &mut self,
        gl: &dyn gleam::gl::Gl,
        rect: PxRect,
        scale_factor: Factor,
        mask: Option<ImageMaskMode>,
    ) -> ImageLoadedData {
        let format = match gl.get_type() {
            gleam::gl::GlType::Gl => gleam::gl::BGRA,
            gleam::gl::GlType::Gles => gleam::gl::RGBA,
        };
        let pixels_flipped = gl.read_pixels(
            rect.origin.x.0,
            rect.origin.y.0,
            rect.size.width.0,
            rect.size.height.0,
            format,
            gleam::gl::UNSIGNED_BYTE,
        );
        let mut buf = vec![0u8; pixels_flipped.len()];
        assert_eq!(rect.size.width.0 as usize * rect.size.height.0 as usize * 4, buf.len());
        let stride = 4 * rect.size.width.0 as usize;
        for (px, buf) in pixels_flipped.chunks_exact(stride).rev().zip(buf.chunks_exact_mut(stride)) {
            buf.copy_from_slice(px);
        }

        if let Some(mask) = mask {
            if format == gleam::gl::BGRA {
                for bgra in buf.chunks_exact_mut(4) {
                    bgra.swap(0, 3);
                }
            }
            let density = 96.0 * scale_factor.0;
            let density = Some(PxDensity2d::splat(density.ppi()));

            let (pixels, size, density, is_opaque, is_mask) = Self::convert_decoded(
                image::DynamicImage::ImageRgba8(
                    image::ImageBuffer::from_raw(rect.size.width.0 as u32, rect.size.height.0 as u32, buf).unwrap(),
                ),
                Some(mask),
                density,
                None,
            )
            .unwrap(); // frame size is not large enough to trigger an memmap that can fail

            let id = self.add(ImageRequest::new(
                ImageDataFormat::A8 { size },
                pixels.clone(),
                u64::MAX,
                None,
                Some(mask),
            ));

            ImageLoadedData::new(id, size, density, is_opaque, is_mask, pixels)
        } else {
            if format == gleam::gl::RGBA {
                for rgba in buf.chunks_exact_mut(4) {
                    rgba.swap(0, 3);
                }
            }

            let is_opaque = buf.chunks_exact(4).all(|bgra| bgra[3] == 255);

            let data = IpcBytes::from_vec_blocking(buf).unwrap(); // frame size is not large enough to trigger an memmap that can fail
            let density = 96.0 * scale_factor.0;
            let density = Some(PxDensity2d::splat(density.ppi()));
            let size = rect.size;

            let id = self.add(ImageRequest::new(
                ImageDataFormat::Bgra8 { size, density },
                data.clone(),
                u64::MAX,
                None,
                mask,
            ));

            ImageLoadedData::new(id, size, density, is_opaque, false, data)
        }
    }
}
