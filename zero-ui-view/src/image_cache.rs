use std::sync::Arc;

use glutin::window::Icon;
use webrender::api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat};
use zero_ui_view_api::{
    units::{Px, PxRect, PxSize},
    ByteBuf, Event, ImageDataFormat, ImageId, ImagePixels, ImagePpi, IpcBytesReceiver, IpcSender,
};

use crate::{AppEvent, AppEventSender};
use rustc_hash::FxHashMap;

/// Decode and cache image resources.
pub(crate) struct ImageCache<S> {
    app_sender: S,
    images: FxHashMap<ImageId, Image>,
    image_id_gen: ImageId,
}
impl<S: AppEventSender> ImageCache<S> {
    pub fn new(app_sender: S) -> Self {
        Self {
            app_sender,
            images: FxHashMap::default(),
            image_id_gen: 0,
        }
    }

    pub fn add(&mut self, data: IpcBytesReceiver, format: ImageDataFormat) -> ImageId {
        let mut id = self.image_id_gen.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.image_id_gen = id;

        let app_sender = self.app_sender.clone();

        rayon::spawn(move || match data.recv() {
            Ok(data) => {
                let r = match format {
                    ImageDataFormat::Bgra8 { size, ppi } => {
                        let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
                        if data.len() != expected_len {
                            Err(format!(
                                "bgra8.len() is not width * height * 4, expected {}, found {}",
                                expected_len,
                                data.len()
                            ))
                        } else {
                            let opaque = data.chunks_exact(4).all(|c| c[3] == 255);
                            Ok((data, size, ppi, opaque))
                        }
                    }
                    ImageDataFormat::FileExtension(ext) => Self::load_file(data, ext),
                    ImageDataFormat::MimeType(mime) => Self::load_web(data, mime),
                    ImageDataFormat::Unknown => Self::load_unknown(data),
                };

                match r {
                    Ok((bgra8, size, ppi, opaque)) => {
                        let _ = app_sender.send(AppEvent::ImageLoaded(id, bgra8, size, ppi, opaque));
                    }
                    Err(e) => {
                        let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError(id, e)));
                    }
                }
            }
            Err(e) => {
                let _ = app_sender.send(AppEvent::Notify(Event::ImageLoadError(id, format!("{:?}", e))));
            }
        });

        id
    }

    pub fn forget(&mut self, id: ImageId) {
        self.images.remove(&id);
    }

    pub fn get(&self, id: ImageId) -> Option<&Image> {
        self.images.get(&id)
    }

    /// Called after receive and decode completes correctly.
    pub fn loaded(&mut self, id: ImageId, bgra8: Vec<u8>, size: PxSize, ppi: ImagePpi, opaque: bool) {
        let flags = if opaque {
            ImageDescriptorFlags::IS_OPAQUE
        } else {
            ImageDescriptorFlags::empty()
        };
        self.images.insert(
            id,
            Image {
                size,
                bgra8: Arc::new(bgra8),
                descriptor: ImageDescriptor::new(size.width.0, size.height.0, ImageFormat::BGRA8, flags),
                ppi,
            },
        );

        let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoaded(id, size, ppi, opaque)));
    }

    fn load_file(data: Vec<u8>, ext: String) -> Result<RawLoadedImg, String> {
        if let Some(f) = image::ImageFormat::from_extension(ext) {
            if !f.can_read() {
                return Err(format!("not supported, cannot decode `{:?}` images", f.extensions_str()));
            }
            match image::load_from_memory_with_format(&data, f) {
                Ok(img) => Ok(Self::convert_decoded(img)),
                Err(e) => Err(format!("{:?}", e)),
            }
        } else {
            Self::load_unknown(data)
        }
    }

    fn load_web(data: Vec<u8>, mime: String) -> Result<RawLoadedImg, String> {
        if let Some(format) = mime.strip_prefix("image/") {
            Self::load_file(data, format.to_owned())
        } else {
            Self::load_unknown(data)
        }
    }

    fn load_unknown(data: Vec<u8>) -> Result<RawLoadedImg, String> {
        match image::load_from_memory(&data) {
            Ok(img) => Ok(Self::convert_decoded(img)),
            Err(e) => Err(format!("{:?}", e)),
        }
    }

    fn convert_decoded(image: image::DynamicImage) -> RawLoadedImg {
        use image::DynamicImage::*;

        let mut opaque = true;
        let (size, bgra) = match image {
            ImageLuma8(img) => (img.dimensions(), img.into_raw().into_iter().flat_map(|l| [l, l, l, 255]).collect()),
            ImageLumaA8(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(2)
                    .flat_map(|la| {
                        if la[1] < 255 {
                            opaque = false;
                            let l = la[0] as f32 * la[1] as f32 / 255.0;
                            let l = l as u8;
                            [l, l, l, la[1]]
                        } else {
                            let l = la[0];
                            [l, l, l, la[1]]
                        }
                    })
                    .collect(),
            ),
            ImageRgb8(img) => (
                img.dimensions(),
                img.into_raw().chunks(3).flat_map(|c| [c[2], c[1], c[0], 255]).collect(),
            ),
            ImageRgba8(img) => (img.dimensions(), {
                let mut buf = img.into_raw();
                buf.chunks_mut(4).for_each(|c| {
                    if c[3] < 255 {
                        opaque = false;
                        let a = c[3] as f32 / 255.0;
                        c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                    }
                    c.swap(0, 2);
                });
                buf
            }),
            ImageBgr8(img) => (
                img.dimensions(),
                img.into_raw().chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
            ),
            ImageBgra8(img) => (img.dimensions(), {
                let mut buf = img.into_raw();
                buf.chunks_mut(4).for_each(|c| {
                    if c[3] < 255 {
                        opaque = false;
                        let a = c[3] as f32 / 255.0;
                        c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                    }
                });
                buf
            }),
            ImageLuma16(img) => (
                img.dimensions(),
                img.into_raw()
                    .into_iter()
                    .flat_map(|l| {
                        let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                        [l, l, l, 255]
                    })
                    .collect(),
            ),
            ImageLumaA16(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(2)
                    .flat_map(|la| {
                        let max = u16::MAX as f32;
                        let l = la[0] as f32 / max;
                        let a = la[1] as f32 / max * 255.0;

                        if la[1] < u16::MAX {
                            opaque = false;
                            let l = (l * a) as u8;
                            [l, l, l, a as u8]
                        } else {
                            let l = (l * 255.0) as u8;
                            [l, l, l, a as u8]
                        }
                    })
                    .collect(),
            ),
            ImageRgb16(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(3)
                    .flat_map(|c| {
                        let to_u8 = 255.0 / u16::MAX as f32;
                        [
                            (c[2] as f32 * to_u8) as u8,
                            (c[1] as f32 * to_u8) as u8,
                            (c[0] as f32 * to_u8) as u8,
                            255,
                        ]
                    })
                    .collect(),
            ),
            ImageRgba16(img) => (
                img.dimensions(),
                img.into_raw()
                    .chunks(4)
                    .flat_map(|c| {
                        if c[3] < u16::MAX {
                            opaque = false;
                            let max = u16::MAX as f32;
                            let a = c[3] as f32 / max * 255.0;
                            [
                                (c[2] as f32 / max * a) as u8,
                                (c[1] as f32 / max * a) as u8,
                                (c[0] as f32 / max * a) as u8,
                                a as u8,
                            ]
                        } else {
                            let to_u8 = 255.0 / u16::MAX as f32;
                            [
                                (c[2] as f32 * to_u8) as u8,
                                (c[1] as f32 * to_u8) as u8,
                                (c[0] as f32 * to_u8) as u8,
                                255,
                            ]
                        }
                    })
                    .collect(),
            ),
        };

        (bgra, PxSize::new(Px(size.0 as i32), Px(size.1 as i32)), None, opaque)
    }
}

type RawLoadedImg = (Vec<u8>, PxSize, ImagePpi, bool);

pub(crate) struct Image {
    pub size: PxSize,
    pub bgra8: Arc<Vec<u8>>,
    pub descriptor: ImageDescriptor,
    pub ppi: ImagePpi,
}
impl Image {
    pub fn opaque(&self) -> bool {
        self.descriptor.flags.contains(ImageDescriptorFlags::IS_OPAQUE)
    }

    pub fn read_pixels(&self, response: IpcSender<ImagePixels>) {
        let bgra8 = Arc::clone(&self.bgra8);
        let size = self.size;
        let ppi = self.ppi;
        let opaque = self.opaque();

        rayon::spawn(move || {
            let _ = response.send(ImagePixels {
                area: PxRect::from_size(size),
                bgra: ByteBuf::from((*bgra8).clone()),
                ppi,
                opaque,
            });
        });
    }

    pub fn read_pixels_rect(&self, rect: PxRect, response: IpcSender<ImagePixels>) {
        let bgra8 = Arc::clone(&self.bgra8);
        let size = self.size;
        let ppi = self.ppi;
        let opaque = self.opaque();

        rayon::spawn(move || {
            let area = PxRect::from_size(size).intersection(&rect).unwrap_or_default();
            if area.size.width.0 == 0 || area.size.height.0 == 0 {
                let _ = response.send(ImagePixels {
                    area,
                    bgra: ByteBuf::new(),
                    ppi,
                    opaque,
                });
            } else {
                let x = area.origin.x.0 as usize;
                let y = area.origin.y.0 as usize;
                let width = area.size.width.0 as usize;
                let height = area.size.height.0 as usize;
                let mut bytes = Vec::with_capacity(width * height * 4);
                for l in y..y + height {
                    let line_start = (l + x) * 4;
                    let line_end = (l + x + width) * 4;
                    let line = &bgra8[line_start..line_end];
                    bytes.extend(line);
                }

                let mut opaque = opaque;
                if !opaque && area.size != size {
                    opaque = bytes.chunks_exact(4).all(|c| c[3] == 255);
                }

                let _ = response.send(ImagePixels {
                    area,
                    bgra: ByteBuf::from(bytes),
                    ppi,
                    opaque,
                });
            }
        })
    }

    /// Generate a window icon from the image.
    pub fn icon(&self) -> Option<Icon> {
        let width = self.size.width.0 as u32;
        let height = self.size.height.0 as u32;
        if width == 0 || height == 0 {
            None
        } else if width > 255 || height > 255 {
            // resize to max 255
            let img = image::ImageBuffer::from_raw(width, height, (*self.bgra8).clone()).unwrap();
            let img = image::DynamicImage::ImageBgra8(img);
            img.resize(255, 255, image::imageops::FilterType::Triangle);

            use image::GenericImageView;
            let (width, height) = img.dimensions();
            let buf = img.to_rgba8().into_raw();
            glutin::window::Icon::from_rgba(buf, width, height).ok()
        } else {
            let mut buf = (*self.bgra8).clone();
            // BGRA to RGBA
            buf.chunks_exact_mut(4).for_each(|c| c.swap(0, 2));
            glutin::window::Icon::from_rgba(buf, width, height).ok()
        }
    }

    pub fn encode(&self, format: image::ImageFormat, buffer: &mut Vec<u8>) -> image::ImageResult<()> {
        if self.size.width <= Px(0) || self.size.height <= Px(0) {
            return Err(image::ImageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "cannot encode zero sized image",
            )));
        }

        use image::*;

        // invert rows, `image` only supports top-to-bottom buffers.
        let bgra: Vec<_> = self
            .bgra8
            .rchunks_exact(self.size.width.0 as usize * 4)
            .flatten()
            .copied()
            .collect();

        let width = self.size.width.0 as u32;
        let height = self.size.height.0 as u32;
        let opaque = self.opaque();

        match format {
            ImageFormat::Jpeg => {
                let mut jpg = codecs::jpeg::JpegEncoder::new(buffer);
                if let Some((ppi_x, ppi_y)) = self.ppi {
                    jpg.set_pixel_density(codecs::jpeg::PixelDensity {
                        density: (ppi_x as u16, ppi_y as u16),
                        unit: codecs::jpeg::PixelDensityUnit::Inches,
                    });
                }
                jpg.encode(&bgra, width, height, ColorType::Bgra8)?;
            }
            ImageFormat::Farbfeld => {
                let mut pixels = Vec::with_capacity(bgra.len() * 2);
                for bgra in bgra.chunks(4) {
                    fn c(c: u8) -> [u8; 2] {
                        let c = (c as f32 / 255.0) * u16::MAX as f32;
                        (c as u16).to_ne_bytes()
                    }
                    pixels.extend(c(bgra[2]));
                    pixels.extend(c(bgra[1]));
                    pixels.extend(c(bgra[0]));
                    pixels.extend(c(bgra[3]));
                }

                let ff = codecs::farbfeld::FarbfeldEncoder::new(buffer);
                ff.encode(&pixels, width, height)?;
            }
            ImageFormat::Tga => {
                let tga = codecs::tga::TgaEncoder::new(buffer);
                tga.encode(&bgra, width, height, ColorType::Bgra8)?;
            }
            rgb_only => {
                let mut pixels;
                let color_type;
                if opaque {
                    color_type = ColorType::Rgb8;
                    pixels = Vec::with_capacity(width as usize * height as usize * 3);
                    for bgra in bgra.chunks(4) {
                        pixels.push(bgra[2]);
                        pixels.push(bgra[1]);
                        pixels.push(bgra[0]);
                    }
                } else {
                    color_type = ColorType::Rgba8;
                    pixels = bgra;
                    for pixel in pixels.chunks_mut(4) {
                        pixel.swap(0, 2);
                    }
                }

                match rgb_only {
                    ImageFormat::Png => {
                        if let Some((ppi_x, ppi_y)) = self.ppi {
                            let mut png_bytes = vec![];
                            let png = codecs::png::PngEncoder::new(&mut png_bytes);
                            png.encode(&pixels, width, height, color_type)?;

                            let mut png = img_parts::png::Png::from_bytes(png_bytes.into()).unwrap();

                            let chunk_kind = *b"pHYs";
                            debug_assert!(png.chunk_by_type(chunk_kind).is_none());

                            use byteorder::*;
                            let mut chunk = Vec::with_capacity(4 * 2 + 1);

                            // ppi / inch_to_metric
                            let ppm_x = (ppi_x / 0.0254) as u32;
                            let ppm_y = (ppi_y / 0.0254) as u32;

                            chunk.write_u32::<BigEndian>(ppm_x).unwrap();
                            chunk.write_u32::<BigEndian>(ppm_y).unwrap();
                            chunk.write_u8(1).unwrap(); // metric

                            let chunk = img_parts::png::PngChunk::new(chunk_kind, chunk.into());
                            png.chunks_mut().insert(1, chunk);

                            png.encoder().write_to(buffer)?;
                        } else {
                            let png = codecs::png::PngEncoder::new(buffer);
                            png.encode(&pixels, width, height, color_type)?;
                        }
                    }
                    ImageFormat::Tiff => {
                        // TODO set ResolutionUnit to 2 (inch) and set both XResolution and YResolution
                        let mut seek_buf = std::io::Cursor::new(vec![]);
                        let tiff = codecs::tiff::TiffEncoder::new(&mut seek_buf);

                        tiff.encode(&pixels, width, height, color_type)?;

                        *buffer = seek_buf.into_inner();
                    }
                    ImageFormat::Gif => {
                        let mut gif = codecs::gif::GifEncoder::new(buffer);

                        gif.encode(&pixels, width, height, color_type)?;
                    }
                    ImageFormat::Bmp => {
                        // TODO set biXPelsPerMeter and biYPelsPerMeter
                        let mut bmp = codecs::bmp::BmpEncoder::new(buffer);

                        bmp.encode(&pixels, width, height, color_type)?;
                    }
                    ImageFormat::Ico => {
                        // TODO set density in the inner PNG?
                        let ico = codecs::ico::IcoEncoder::new(buffer);

                        ico.encode(&pixels, width, height, color_type)?;
                    }
                    unsuported => {
                        use image::error::*;
                        let hint = ImageFormatHint::Exact(unsuported);
                        return Err(ImageError::Unsupported(UnsupportedError::from_format_and_kind(
                            hint.clone(),
                            UnsupportedErrorKind::Format(hint),
                        )));
                    }
                }
            }
        }

        Ok(())
    }
}
