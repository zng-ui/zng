use winit::{
    event_loop::ActiveEventLoop,
    window::{CustomCursor, Icon},
};
use zng_task::channel::IpcBytes;
use zng_txt::{ToTxt as _, Txt, formatx};
use zng_unit::PxPoint;
use zng_view_api::{Event, image::ImageId};

use crate::{
    AppEvent,
    image_cache::{FORMATS, Image, ImageCache, ImageData},
};

impl ImageCache {
    pub fn encode(&self, id: ImageId, format: Txt) {
        let fmt = match FORMATS.iter().find(|f| f.matches(format.as_str())) {
            Some(f) => {
                if !f.can_encode {
                    let error = formatx!(
                        "cannot encode `{id:?}` to `{} ({format})`, encoding not implemented",
                        f.display_name
                    );
                    let _ = self
                        .app_sender
                        .send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
                    return;
                }
                f
            }
            None => {
                let error = formatx!("cannot encode `{id:?}` to `{format}`, unknown format");
                let _ = self
                    .app_sender
                    .send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
                return;
            }
        };

        if let Some(img) = self.get(id) {
            let fmt = image::ImageFormat::from_extension(fmt.file_extensions_iter().next().unwrap()).unwrap();
            debug_assert!(fmt.can_write());

            let img = img.clone();
            let sender = self.app_sender.clone();
            rayon::spawn(move || {
                let mut data = IpcBytes::new_writer_blocking();
                match img.encode(fmt, &mut data) {
                    Ok(_) => match data.finish() {
                        Ok(data) => {
                            let _ = sender.send(AppEvent::Notify(Event::ImageEncoded { image: id, format, data }));
                        }
                        Err(e) => {
                            let _ = sender.send(AppEvent::Notify(Event::ImageEncodeError {
                                image: id,
                                format,
                                error: e.to_txt(),
                            }));
                        }
                    },
                    Err(e) => {
                        let error = formatx!("failed to encode `{id:?}` to `{format}`, {e}");
                        let _ = sender.send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
                    }
                }
            })
        } else {
            let error = formatx!("cannot encode `{id:?}` to `{}`, image not found", fmt.display_name);
            let _ = self
                .app_sender
                .send(AppEvent::Notify(Event::ImageEncodeError { image: id, format, error }));
        }
    }
}

impl Image {
    /// Generate a window icon from the image.
    pub fn icon(&self) -> Option<Icon> {
        let (size, pixels) = match &*self.0 {
            ImageData::RawData { size, pixels, .. } => (size, pixels),
            ImageData::NativeTexture { .. } => unreachable!(),
        };

        let width = size.width.0 as u32;
        let height = size.height.0 as u32;
        if width == 0 || height == 0 || self.0.is_mask() {
            None
        } else {
            let r = if width > 255 || height > 255 {
                // resize to max 255
                let mut buf = pixels[..].to_vec();
                bgra_pre_mul_to_rgba(&mut buf, self.is_opaque());
                let img = image::ImageBuffer::from_raw(width, height, buf).unwrap();
                let img = image::DynamicImage::ImageRgba8(img);
                let img = img.resize(255, 255, image::imageops::FilterType::Lanczos3);

                use image::GenericImageView;
                let (width, height) = img.dimensions();
                let buf = img.into_rgba8().into_raw();
                winit::window::Icon::from_rgba(buf, width, height)
            } else {
                let mut buf = pixels[..].to_vec();
                bgra_pre_mul_to_rgba(&mut buf, self.is_opaque());
                winit::window::Icon::from_rgba(buf, width, height)
            };
            match r {
                Ok(i) => Some(i),
                Err(e) => {
                    tracing::error!("failed to convert image to custom icon, {e}");
                    None
                }
            }
        }
    }

    /// Generate a cursor from the image.
    pub fn cursor(&self, hotspot: PxPoint, event_loop: &ActiveEventLoop) -> Option<CustomCursor> {
        let (size, pixels) = match &*self.0 {
            ImageData::RawData { size, pixels, .. } => (size, pixels),
            ImageData::NativeTexture { .. } => unreachable!(),
        };

        let width: u16 = size.width.0.try_into().ok()?;
        let height: u16 = size.height.0.try_into().ok()?;
        let hotspot_x: u16 = hotspot.x.0.try_into().ok()?;
        let hotspot_y: u16 = hotspot.y.0.try_into().ok()?;

        if width == 0 || height == 0 || hotspot_x > width || hotspot_y > height || self.0.is_mask() {
            None
        } else {
            let mut buf = pixels[..].to_vec();
            bgra_pre_mul_to_rgba(&mut buf, self.is_opaque());
            match CustomCursor::from_rgba(buf, width, height, hotspot_x, hotspot_y) {
                Ok(c) => Some(event_loop.create_custom_cursor(c)),
                Err(e) => {
                    tracing::error!("failed to convert image to custom cursor, {e}");
                    None
                }
            }
        }
    }

    pub fn encode(&self, format: image::ImageFormat, buffer: &mut (impl std::io::Write + std::io::Seek)) -> image::ImageResult<()> {
        let (size, pixels, density) = match &*self.0 {
            ImageData::RawData { size, pixels, density, .. } => (size, pixels, density),
            ImageData::NativeTexture { .. } => unreachable!(),
        };

        if size.width <= 0 || size.height <= 0 {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot encode zero sized image",
            )));
        }

        if self.0.is_mask() {
            let width = size.width.0 as u32;
            let height = size.height.0 as u32;
            let is_opaque = self.0.is_opaque();
            let r8 = pixels[..].to_vec();

            let mut img = image::DynamicImage::ImageLuma8(image::ImageBuffer::from_raw(width, height, r8).unwrap());
            if is_opaque {
                img = image::DynamicImage::ImageRgb8(img.to_rgb8());
            }
            img.write_to(buffer, format)?;

            return Ok(());
        }

        let mut buf = pixels[..].to_vec();
        bgra_pre_mul_to_rgba(&mut buf, self.0.is_opaque());
        let rgba = buf;

        let width = size.width.0 as u32;
        let height = size.height.0 as u32;
        let is_opaque = self.0.is_opaque();

        match format {
            #[cfg(feature = "image_jpeg")]
            image::ImageFormat::Jpeg => {
                let mut jpg = image::codecs::jpeg::JpegEncoder::new(buffer);
                if let Some(density) = density {
                    jpg.set_pixel_density(image::codecs::jpeg::PixelDensity {
                        density: (density.height.ppi() as u16, density.height.ppi() as u16),
                        unit: image::codecs::jpeg::PixelDensityUnit::Inches,
                    });
                }
                jpg.encode(&rgba, width, height, image::ColorType::Rgba8.into())?;
            }
            #[cfg(feature = "image_png")]
            image::ImageFormat::Png => {
                let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(width, height, rgba).unwrap());
                if is_opaque {
                    img = image::DynamicImage::ImageRgb8(img.to_rgb8());
                }
                if let Some(density) = density {
                    let mut png_bytes = vec![];

                    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)?;

                    let mut png = img_parts::png::Png::from_bytes(png_bytes.into()).unwrap();

                    let chunk_kind = *b"pHYs";
                    debug_assert!(png.chunk_by_type(chunk_kind).is_none());

                    use byteorder::*;
                    let mut chunk = Vec::with_capacity(4 * 2 + 1);

                    // density / inch_to_metric
                    let ppm_x = density.width.ppm() as u32;
                    let ppm_y = density.height.ppm() as u32;

                    chunk.write_u32::<BigEndian>(ppm_x).unwrap();
                    chunk.write_u32::<BigEndian>(ppm_y).unwrap();
                    chunk.write_u8(1).unwrap(); // metric

                    let chunk = img_parts::png::PngChunk::new(chunk_kind, chunk.into());
                    png.chunks_mut().insert(1, chunk);

                    png.encoder().write_to(buffer)?;
                } else {
                    img.write_to(buffer, image::ImageFormat::Png)?;
                }
            }
            _ => {
                // other formats that we don't with custom PPI meta.

                let _ = density; // suppress warning when both png an jpeg are not enabled

                let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::from_raw(width, height, rgba).unwrap());
                if is_opaque {
                    img = image::DynamicImage::ImageRgb8(img.to_rgb8());
                }
                img.write_to(buffer, format)?;
            }
        }

        Ok(())
    }
}

fn bgra_pre_mul_to_rgba(buf: &mut [u8], is_opaque: bool) {
    if is_opaque {
        buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
    } else {
        buf.chunks_exact_mut(4).for_each(|c| {
            let alpha = c[3];

            // idea here is to avoid div by zero, without introducing an if branch
            let is_not_zero = (alpha > 0) as u8 as f32;
            let divisor = (alpha as f32) + (1.0 - is_not_zero);
            let scale = (255.0 / divisor) * is_not_zero;

            let b = c[0] as f32 * scale;
            let g = c[1] as f32 * scale;
            let r = c[2] as f32 * scale;

            c[0] = r.min(255.0).round() as u8;
            c[1] = g.min(255.0).round() as u8;
            c[2] = b.min(255.0).round() as u8;
        });
    }
}
