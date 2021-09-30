use std::sync::Arc;

use glutin::window::Icon;
use webrender::api::{ImageDescriptor, ImageDescriptorFlags, ImageFormat};
use zero_ui_view_api::{
    units::{Px, PxRect, PxSize},
    ByteBuf, Event, ImageDataFormat, ImageId, ImagePixels, IpcBytesReceiver, IpcSender,
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

    pub fn cache(&mut self, data: IpcBytesReceiver, format: ImageDataFormat) -> ImageId {
        let mut id = self.image_id_gen.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.image_id_gen = id;

        let app_sender = self.app_sender.clone();

        rayon::spawn(move || match data.recv() {
            Ok(data) => {
                let r = match format {
                    ImageDataFormat::Bgra8 { size, dpi } => {
                        let expected_len = size.width.0 as usize * size.height.0 as usize * 4;
                        if data.len() != expected_len {
                            Err(format!(
                                "bgra8.len() is not width * height * 4, expected {}, found {}",
                                expected_len,
                                data.len()
                            ))
                        } else {
                            let opaque = data.chunks_exact(4).all(|c| c[3] == 255);
                            Ok((data, size, dpi, opaque))
                        }
                    }
                    ImageDataFormat::FileExt(ext) => Self::load_file(data, ext),
                    ImageDataFormat::Mime(mime) => Self::load_web(data, mime),
                    ImageDataFormat::Unknown => Self::load_unknown(data),
                };

                match r {
                    Ok((bgra8, size, dpi, opaque)) => {
                        let _ = app_sender.send(AppEvent::ImageLoaded(id, bgra8, size, dpi, opaque));
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

    pub fn uncache(&mut self, id: ImageId) {
        self.images.remove(&id);
    }

    pub fn get(&self, id: ImageId) -> Option<&Image> {
        self.images.get(&id)
    }

    /// Called after receive and decode completes correctly.
    pub fn loaded(&mut self, id: ImageId, bgra8: Vec<u8>, size: PxSize, dpi: (f32, f32), opaque: bool) {
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
                dpi,
            },
        );

        let _ = self.app_sender.send(AppEvent::Notify(Event::ImageLoaded(id, size, dpi, opaque)));
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

        (bgra, PxSize::new(Px(size.0 as i32), Px(size.1 as i32)), (96.0, 96.0), opaque)
    }
}

type RawLoadedImg = (Vec<u8>, PxSize, (f32, f32), bool);

pub(crate) struct Image {
    pub size: PxSize,
    pub bgra8: Arc<Vec<u8>>,
    pub descriptor: ImageDescriptor,
    pub dpi: (f32, f32),
}
impl Image {
    pub fn opaque(&self) -> bool {
        self.descriptor.flags.contains(ImageDescriptorFlags::IS_OPAQUE)
    }

    pub fn read_pixels(&self, response: IpcSender<ImagePixels>) {
        let bgra8 = Arc::clone(&self.bgra8);
        let size = self.size;
        let dpi = self.dpi;
        let opaque = self.opaque();

        rayon::spawn(move || {
            let _ = response.send(ImagePixels {
                area: PxRect::from_size(size),
                bgra: ByteBuf::from((*bgra8).clone()),
                dpi,
                opaque,
            });
        });
    }

    pub fn read_pixels_rect(&self, rect: PxRect, response: IpcSender<ImagePixels>) {
        let bgra8 = Arc::clone(&self.bgra8);
        let size = self.size;
        let dpi = self.dpi;
        let opaque = self.opaque();

        rayon::spawn(move || {
            let area = PxRect::from_size(size).intersection(&rect).unwrap_or_default();
            if area.size.width.0 == 0 || area.size.height.0 == 0 {
                let _ = response.send(ImagePixels {
                    area,
                    bgra: ByteBuf::new(),
                    dpi,
                    opaque,
                });
            } else {
                let x = area.origin.x.0 as usize;
                let y = area.origin.y.0 as usize;
                let width = area.size.width.0 as usize;
                let height = area.size.height.0 as usize;
                let mut bytes = Vec::with_capacity(width * height * 4);
                for l in y..y + height {
                    let line = &bgra8[l + x..l + x + width];
                    bytes.extend(line);
                }

                let mut opaque = opaque;
                if !opaque && area.size != size {
                    opaque = bytes.chunks_exact(4).all(|c| c[3] == 255);
                }

                let _ = response.send(ImagePixels {
                    area,
                    bgra: ByteBuf::from(bytes),
                    dpi,
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
}
