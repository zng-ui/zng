#![cfg_attr(not(feature = "image_any"), allow(unused))]

#[cfg(feature = "image_any")]
use crate::image_cache::ImageHeader;
use crate::image_cache::ResizerCache;
#[cfg(feature = "image_any")]
use crate::image_cache::ipc_dyn_image::IpcDynamicImage;
use crate::image_cache::{ImageCache, RawLoadedImg};
use image::ImageDecoder as _;
use zng_task::channel::{IpcBytes, IpcBytesMut};
use zng_txt::ToTxt as _;
use zng_txt::Txt;
use zng_unit::PxDensityUnits as _;
use zng_unit::{Px, PxDensity2d, PxSize};
use zng_view_api::image::ImageDataFormat;
use zng_view_api::image::ImageMaskMode;

#[cfg(not(feature = "image_any"))]
use crate::image_cache::lcms2;

impl ImageCache {
    #[cfg(feature = "image_any")]
    pub(super) fn header_decode(fmt: &ImageDataFormat, data: &[u8]) -> Result<ImageHeader, Txt> {
        let maybe_fmt = match fmt {
            ImageDataFormat::FileExtension(ext) => image::ImageFormat::from_extension(ext.as_str()),
            ImageDataFormat::MimeType(t) => t.strip_prefix("image/").and_then(image::ImageFormat::from_extension),
            ImageDataFormat::Unknown => None,
            ImageDataFormat::Bgra8 { .. } => unreachable!(),
            ImageDataFormat::A8 { .. } => unreachable!(),
            _ => None,
        };

        let reader = match maybe_fmt {
            Some(fmt) => image::ImageReader::with_format(std::io::Cursor::new(data), fmt),
            None => image::ImageReader::new(std::io::Cursor::new(data))
                .with_guessed_format()
                .map_err(|e| e.to_txt())?,
        };

        match reader.format() {
            Some(fmt) => {
                use image::metadata::Orientation::*;

                let mut decoder = match reader.into_decoder() {
                    Ok(d) => d,
                    Err(e) => {
                        // decoder error, try fallback to Unknown
                        if let image::ImageError::Decoding(_) = &e
                            && maybe_fmt.is_some()
                            && let Ok(r) = Self::header_decode(&ImageDataFormat::Unknown, data)
                        {
                            return Ok(r);
                        }
                        return Err(e.to_txt());
                    }
                };
                let (mut w, mut h) = decoder.dimensions();
                let orientation = decoder.orientation().unwrap_or(NoTransforms);
                if matches!(orientation, Rotate90 | Rotate270 | Rotate90FlipH | Rotate270FlipH) {
                    std::mem::swap(&mut w, &mut h)
                }

                let mut density = None;
                #[cfg(feature = "image_png")]
                let mut png_gamma = None;
                #[cfg(feature = "image_png")]
                let mut png_chromaticities = None;

                match fmt {
                    #[cfg(feature = "image_jpeg")]
                    image::ImageFormat::Jpeg => {
                        // `image` uses `zune-jpeg`, that decoder does not parse density correctly,
                        // so we do it manually here
                        fn parse_density(data: &[u8]) -> Option<(u8, u16, u16)> {
                            let mut i = 0;
                            while i + 4 < data.len() {
                                // APP0
                                if data[i] == 0xFF && data[i + 1] == 0xE0 {
                                    let len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
                                    if i + 2 + len > data.len() {
                                        break;
                                    }

                                    // APP0 payload starts at i+4, identifier is 5 bytes: "JFIF\0"
                                    let p = i + 4;
                                    if &data[p..p + 5] == b"JFIF\0" && p + 14 <= data.len() {
                                        let unit = data[p + 7];
                                        let x = u16::from_be_bytes([data[p + 8], data[p + 9]]);
                                        let y = u16::from_be_bytes([data[p + 10], data[p + 11]]);
                                        return Some((unit, x, y));
                                    }

                                    i += 2 + len;
                                } else if data[i] == 0xFF && data[i + 1] == 0xDA {
                                    // Start of Scan
                                    break;
                                } else {
                                    i += 1;
                                }
                            }
                            None
                        }
                        if let Some((unit, x, y)) = parse_density(data) {
                            match unit {
                                // inches
                                1 => {
                                    density = Some(PxDensity2d::new((x as f32).ppi(), (y as f32).ppi()));
                                }
                                // centimeters
                                2 => {
                                    density = Some(PxDensity2d::new((x as f32).ppcm(), (y as f32).ppcm()));
                                }
                                _ => {}
                            }
                        }
                    }
                    #[cfg(feature = "image_png")]
                    image::ImageFormat::Png => {
                        let d = png::Decoder::new_with_limits(std::io::Cursor::new(data), png::Limits { bytes: usize::MAX });
                        let d = d.read_info().map_err(|e| e.to_txt())?;
                        let info = d.info();
                        if let Some(d) = info.pixel_dims {
                            match d.unit {
                                png::Unit::Unspecified => {}
                                png::Unit::Meter => {
                                    use zng_unit::PxDensity;

                                    density = Some(PxDensity2d::new(
                                        PxDensity::new_ppm(d.xppu as f32),
                                        PxDensity::new_ppm(d.yppu as f32),
                                    ));
                                }
                            }
                        }
                        png_gamma = info.gama_chunk;
                        png_chromaticities = info.chrm_chunk;
                    }
                    #[cfg(feature = "image_tiff")]
                    image::ImageFormat::Tiff => {
                        use tiff::{decoder::ifd::Value, tags::Tag};
                        let mut d = tiff::decoder::Decoder::new(std::io::Cursor::new(data))
                            .map_err(|e| e.to_txt())?
                            .with_limits(tiff::decoder::Limits::unlimited());
                        let res_unit = d.get_tag(Tag::ResolutionUnit).ok().and_then(|t| t.into_u16().ok()).unwrap_or(2);
                        if let Ok(Value::Rational(x_num, x_denom)) = d.get_tag(Tag::XResolution)
                            && let Ok(Value::Rational(y_num, y_denom)) = d.get_tag(Tag::YResolution)
                        {
                            let x = x_num as f32 / x_denom as f32;
                            let y = y_num as f32 / y_denom as f32;
                            match res_unit {
                                // inches
                                2 => {
                                    density = Some(PxDensity2d::new(x.ppi(), y.ppi()));
                                }
                                // centimeters
                                3 => {
                                    density = Some(PxDensity2d::new(x.ppcm(), y.ppcm()));
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }

                if density.is_none()
                    && let Ok(Some(exif)) = decoder.exif_metadata()
                    && let Ok(exif) = exif::Reader::new().read_raw(exif)
                {
                    use exif::Tag;
                    if let Some(unit) = exif.get_field(Tag::ResolutionUnit, exif::In::PRIMARY)
                        && let Some(x) = exif.get_field(Tag::XResolution, exif::In::PRIMARY)
                        && let Some(y) = exif.get_field(Tag::YResolution, exif::In::PRIMARY)
                        && let exif::Value::Rational(x) = &x.value
                        && let exif::Value::Rational(y) = &y.value
                    {
                        let x = x[0].to_f32();
                        let y = y[0].to_f32();
                        match unit.value.get_uint(0) {
                            // inches
                            Some(2) => density = Some(PxDensity2d::new(x.ppi(), y.ppi())),
                            // centimeters
                            Some(3) => density = Some(PxDensity2d::new(x.ppcm(), y.ppcm())),
                            _ => {}
                        }
                    }
                }

                let mut icc_profile = None;
                if let Ok(Some(icc)) = decoder.icc_profile() {
                    match lcms2::Profile::new_icc(&icc) {
                        Ok(p) => icc_profile = Some(p),
                        Err(e) => tracing::error!("error parsing ICC profile, {e}"),
                    }
                }
                #[cfg(feature = "image_png")]
                if icc_profile.is_none() {
                    // PNG has some color management metadata, convert to standard
                    icc_profile = crate::util::png_color_metadata_to_icc(png_gamma, png_chromaticities);
                }

                Ok(ImageHeader {
                    format: fmt,
                    size: PxSize::new(Px(w as i32), Px(h as i32)),
                    orientation,
                    density,
                    icc_profile,
                })
            }
            None => Err(Txt::from_static("unknown format")),
        }
    }

    #[cfg(feature = "image_any")]
    pub(super) fn image_decode(
        buf: &[u8],
        format: image::ImageFormat,
        downscale: Option<zng_view_api::image::ImageDownscale>,
    ) -> image::ImageResult<IpcDynamicImage> {
        let buf = std::io::Cursor::new(buf);

        // Some JPEG decoders can downscale to an approximation of this size
        // but that is not implemented by image crate
        let _ = downscale;

        let mut reader = image::ImageReader::new(buf);
        reader.set_format(format);
        reader.no_limits();

        IpcDynamicImage::decode(reader)
    }

    pub(super) fn convert_decoded(
        image: IpcDynamicImage,
        mask: Option<ImageMaskMode>,
        density: Option<PxDensity2d>,
        icc_profile: Option<lcms2::Profile>,
        downscale: Option<zng_view_api::image::ImageDownscale>,
        orientation: image::metadata::Orientation,
        resizer_cache: &ResizerCache,
    ) -> std::io::Result<RawLoadedImg> {
        use IpcDynamicImage::*;

        let mut is_opaque = true;
        let size = image.dimensions();
        let pixels_len = size.0 as usize * size.1 as usize;

        let mut pixels = match image {
            ImageLuma8(img) => {
                let raw = img.into_raw();
                if mask.is_some() {
                    is_opaque = !raw.iter().any(|&a| a < 255);
                    raw
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, l) in bgra.chunks_exact_mut(4).zip(raw.iter().copied()) {
                        p.copy_from_slice(&[l, l, l, 255])
                    }
                    bgra
                }
            }
            ImageLumaA8(img) => {
                let mut raw = img.into_raw();
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::A => {
                            raw.reduce_in_place(|[_, a]| {
                                is_opaque &= a == 255;
                                [a]
                            });
                        }
                        ImageMaskMode::B | ImageMaskMode::G | ImageMaskMode::R | ImageMaskMode::Luminance => {
                            raw.reduce_in_place(|[l, _]| {
                                is_opaque &= l == 255;
                                [l]
                            });
                        }
                        _ => unimplemented!(),
                    }
                    raw
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, la) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(2)) {
                        let a = la[1];
                        is_opaque &= a == 255;

                        let l = la[0] as f32 * a as f32 / 255.0;
                        let l = l as u8;

                        p.copy_from_slice(&[l, l, l, a]);
                    }
                    bgra
                }
            }
            ImageRgb8(img) => {
                let mut raw = img.into_raw();
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => {
                            raw.reduce_in_place(|[r, g, b]| {
                                let l = luminance(r, g, b);
                                is_opaque &= l == 255;
                                [l]
                            });
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            raw.reduce_in_place(|rgb: [u8; 3]| {
                                let c = rgb[channel];
                                is_opaque &= c == 255;
                                [c]
                            });
                        }
                    }
                    raw
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, rgb) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(3)) {
                        p.copy_from_slice(&[rgb[2], rgb[1], rgb[0], 255]);
                    }
                    bgra
                }
            }
            ImageRgba8(img) => {
                let mut raw = img.into_raw();
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance => {
                            raw.reduce_in_place(|[r, g, b, _]| {
                                let l = luminance(r, g, b);
                                is_opaque &= l == 255;
                                [l]
                            });
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            raw.reduce_in_place(|rgba: [u8; 4]| {
                                let c = rgba[channel];
                                is_opaque &= c == 255;
                                [c]
                            });
                        }
                    }
                    raw
                } else {
                    raw.chunks_mut(4).for_each(|c| {
                        let a = c[3];
                        is_opaque &= a == 255;

                        // pre multiply
                        let a = a as f32 / 255.0;
                        c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);

                        // to bgra
                        c.swap(0, 2);
                    });
                    raw
                }
            }
            ImageLuma16(img) => {
                let raw = img.into_raw();
                if mask.is_some() {
                    let mut raw = raw.into_inner();
                    raw.cast_reduce_in_place(|[l]: [u16; 1]| {
                        let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                        is_opaque &= l == 255;
                        [l]
                    });
                    raw
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, l) in bgra.chunks_exact_mut(4).zip(raw.iter().copied()) {
                        let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                        p.copy_from_slice(&[l, l, l, 255]);
                    }
                    bgra
                }
            }
            ImageLumaA16(img) => {
                let mut raw = img.into_raw().into_inner();
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::A => {
                            raw.cast_reduce_in_place(|[_, a]: [u16; 2]| {
                                is_opaque &= a == u16::MAX;
                                let max = u16::MAX as f32;
                                let l = a as f32 / max * 255.0;
                                [l as u8]
                            });
                        }
                        ImageMaskMode::B | ImageMaskMode::G | ImageMaskMode::R | ImageMaskMode::Luminance => {
                            raw.cast_reduce_in_place(|[l, _]: [u16; 2]| {
                                is_opaque &= l == u16::MAX;
                                let max = u16::MAX as f32;
                                let l = l as f32 / max * 255.0;
                                [l as u8]
                            });
                        }
                        _ => unimplemented!(),
                    }
                } else {
                    raw.cast_reduce_in_place(|[l, a]: [u16; 2]| {
                        let max = u16::MAX as f32;
                        let l = l as f32 / max;
                        let a = a as f32 / max * 255.0;
                        let l = (l * a) as u8;
                        let a = a as u8;
                        is_opaque &= a == 255;
                        [l, l, l, a]
                    });
                }
                raw
            }
            ImageRgb16(img) => {
                let mut raw = img.into_raw().into_inner();
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => {
                            raw.cast_reduce_in_place(|[r, g, b]: [u16; 3]| {
                                let c = luminance_16(r, g, b);
                                is_opaque &= c == 255;
                                [c]
                            });
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            raw.cast_reduce_in_place(|rgb: [u16; 3]| {
                                let c = rgb[channel];
                                let c = (c as f32 / u16::MAX as f32 * 255.0) as u8;
                                is_opaque &= c == 255;
                                [c]
                            });
                        }
                    }
                } else {
                    let to_u8 = 255.0 / u16::MAX as f32;
                    raw.cast_reduce_in_place(|[r, g, b]: [u16; 3]| {
                        [(b as f32 * to_u8) as u8, (g as f32 * to_u8) as u8, (r as f32 * to_u8) as u8, 255]
                    });
                }
                raw
            }
            ImageRgba16(img) => {
                let mut raw = img.into_raw().into_inner();
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance => {
                            raw.cast_reduce_in_place(|[r, g, b, _]: [u16; 4]| {
                                let c = luminance_16(r, g, b);
                                is_opaque &= c == 255;
                                [c]
                            });
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            raw.cast_reduce_in_place(|rgb: [u16; 3]| {
                                let c = rgb[channel];
                                let c = (c as f32 / u16::MAX as f32 * 255.0) as u8;
                                is_opaque &= c == 255;
                                [c]
                            });
                        }
                    }
                } else {
                    raw.cast_reduce_in_place(|[r, g, b, a]: [u16; 4]| {
                        let max = u16::MAX as f32;
                        let af = a as f32 / max * 255.0;
                        let a = af as u8;
                        is_opaque &= a == 255;
                        [
                            (b as f32 / max * af) as u8,
                            (g as f32 / max * af) as u8,
                            (r as f32 / max * af) as u8,
                            a,
                        ]
                    });
                }
                raw
            }
            ImageRgb32F(img) => {
                let mut raw = img.into_raw().into_inner();
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => {
                            raw.cast_reduce_in_place(|[r, g, b]: [f32; 3]| {
                                let c = luminance_f32(r, g, b);
                                is_opaque &= c == 255;
                                [c]
                            });
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            raw.cast_reduce_in_place(|rgb: [f32; 3]| {
                                let c = (rgb[channel] * 255.0).clamp(0.0, 255.0) as u8;
                                is_opaque &= c == 255;
                                [c]
                            })
                        }
                    }
                } else {
                    raw.cast_reduce_in_place(|[r, g, b]: [f32; 3]| {
                        [
                            (b * 255.0).clamp(0.0, 255.0) as u8,
                            (g * 255.0).clamp(0.0, 255.0) as u8,
                            (r * 255.0).clamp(0.0, 255.0) as u8,
                        ]
                    });
                }
                raw
            }
            ImageRgba32F(img) => {
                let mut raw = img.into_raw().into_inner();
                if let Some(mask) = mask {
                    match mask {
                        ImageMaskMode::Luminance => raw.cast_reduce_in_place(|[r, g, b, _]: [f32; 4]| {
                            let c = luminance_f32(r, g, b);
                            is_opaque &= c == 255;
                            [c]
                        }),
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            raw.cast_reduce_in_place(|rgba: [f32; 4]| {
                                let c = (rgba[channel] * 255.0).clamp(0.0, 255.0) as u8;
                                is_opaque &= c == 255;
                                [c]
                            })
                        }
                    }
                } else {
                    raw.cast_reduce_in_place(|[r, g, b, a]: [f32; 4]| {
                        let af = a * 255.0;
                        let a = a.clamp(0.0, 255.0) as u8;
                        is_opaque &= a == 255;
                        [
                            (b * af).clamp(0.0, 255.0) as u8,
                            (g * af).clamp(0.0, 255.0) as u8,
                            (r * af).clamp(0.0, 255.0) as u8,
                            a,
                        ]
                    });
                }
                raw
            }
        };

        #[cfg(feature = "image_any")]
        if let Some(p) = icc_profile {
            use lcms2::*;
            let srgb = Profile::new_srgb();
            let t = Transform::new(&p, PixelFormat::BGRA_8, &srgb, PixelFormat::BGRA_8, Intent::Perceptual).unwrap();
            t.transform_in_place(&mut pixels);
        }
        #[cfg(not(feature = "image_any"))]
        let _ = (icc_profile, &mut pixels);

        let (mut size, mut pixels) = Self::apply_orientation(orientation, size, mask.is_some(), pixels)?;

        if let Some((s, px)) = Self::downscale_decoded(mask, downscale, resizer_cache, size, &pixels)? {
            size = s;
            pixels = px;
        }

        Ok((
            pixels.finish_blocking()?,
            size,
            density,
            is_opaque,
            mask.is_some(), // is_mask
        ))
    }

    fn apply_orientation(
        orientation: image::metadata::Orientation,
        size: (u32, u32),
        is_mask: bool,
        mut pixels: IpcBytesMut,
    ) -> std::io::Result<(PxSize, IpcBytesMut)> {
        use image::metadata::Orientation::*;
        let size = PxSize::new(Px(size.0 as _), Px(size.1 as _));

        match orientation {
            NoTransforms => Ok((size, pixels)),
            Rotate180 => {
                if is_mask {
                    pixels.reverse();
                } else {
                    pixels.reverse_chunks::<4>();
                }

                Ok((size, pixels))
            }
            FlipHorizontal => {
                if is_mask {
                    let row_len = size.width.0 as usize;
                    for row in pixels.chunks_exact_mut(row_len) {
                        row.reverse();
                    }
                } else {
                    let row_len = size.width.0 as usize * 4;
                    for row in pixels.chunks_exact_mut(row_len) {
                        row.as_chunks_mut::<4>().0.reverse();
                    }
                }
                Ok((size, pixels))
            }
            FlipVertical => {
                let row_len = if is_mask {
                    size.width.0 as usize
                } else {
                    size.width.0 as usize * 4
                };
                pixels.reverse_chunks_dyn(row_len);
                Ok((size, pixels))
            }
            alloc_needed => {
                let mut out = IpcBytes::new_mut_blocking(pixels.len())?;
                let out_slice = &mut out[..];

                // iterate using loop tiling for better CPU cache perf
                // map_coords is (x, y) -> (out_x, out_y).

                let width = size.width.0 as usize;
                let height = size.height.0 as usize;
                let out_w = height;
                let out_h = width;
                let bpp = if is_mask { 1 } else { 4 };

                const TILE: usize = 32;
                macro_rules! tiled_rotation {
                    (|$x:ident, $y:ident| $map_coords:expr) => {
                        for y_base in (0..height).step_by(TILE) {
                            for x_base in (0..width).step_by(TILE) {
                                let y_max = (y_base + TILE).min(height);
                                let x_max = (x_base + TILE).min(width);

                                for y in y_base..y_max {
                                    let src_row_start = y * width * bpp;
                                    let src_row = &pixels[src_row_start..src_row_start + width * bpp];

                                    for x in x_base..x_max {
                                        let (out_x, out_y) = {
                                            let $x = x;
                                            let $y = y;
                                            $map_coords
                                        };

                                        let src_offset = x * bpp;
                                        let dst_offset = (out_y * out_w + out_x) * bpp;

                                        out_slice[dst_offset..dst_offset + bpp].copy_from_slice(&src_row[src_offset..src_offset + bpp]);
                                    }
                                }
                            }
                        }
                    };
                }
                match alloc_needed {
                    Rotate90 => tiled_rotation!(|x, y| (out_w - 1 - y, x)),
                    Rotate270 => tiled_rotation!(|x, y| (y, out_h - 1 - x)),
                    Rotate90FlipH => tiled_rotation!(|x, y| (y, x)),
                    Rotate270FlipH => tiled_rotation!(|x, y| (out_w - 1 - y, out_h - 1 - x)),
                    _ => unreachable!(),
                }

                Ok((PxSize::new(size.height, size.width), out))
            }
        }
    }

    pub(super) fn convert_bgra8_to_mask(
        size: PxSize,
        bgra8: &[u8],
        mask: ImageMaskMode,
        density: Option<PxDensity2d>,
        downscale: Option<zng_view_api::image::ImageDownscale>,
        resizer_cache: &ResizerCache,
    ) -> std::io::Result<RawLoadedImg> {
        let mut a = IpcBytes::new_mut_blocking(bgra8.len() / 4)?;
        let mut is_opaque = true;
        match mask {
            ImageMaskMode::Luminance => {
                for (p, bgra) in a.iter_mut().zip(bgra8.chunks_exact(4)) {
                    let c = luminance(bgra[2], bgra[1], bgra[0]);
                    is_opaque &= c == 255;
                    *p = c;
                }
            }
            mask => {
                let channel = match mask {
                    ImageMaskMode::A => 3,
                    ImageMaskMode::B => 0,
                    ImageMaskMode::G => 1,
                    ImageMaskMode::R => 2,
                    _ => unreachable!(),
                };
                for (p, bgra) in a.iter_mut().zip(bgra8.chunks_exact(4)) {
                    let c = bgra[channel];
                    is_opaque &= c == 255;
                    *p = c;
                }
            }
        }

        let mut size = size;
        if let Some((s, px)) = Self::downscale_decoded(Some(mask), downscale, resizer_cache, size, &a)? {
            size = s;
            a = px;
        }

        Ok((
            a.finish_blocking()?,
            size,
            density,
            is_opaque,
            true, // is_mask
        ))
    }

    pub(super) fn convert_a8_to_bgra8(
        size: PxSize,
        a8: &[u8],
        density: Option<PxDensity2d>,
        downscale: Option<zng_view_api::image::ImageDownscale>,
        resizer_cache: &ResizerCache,
    ) -> std::io::Result<RawLoadedImg> {
        let mut bgra = IpcBytes::new_mut_blocking(a8.len() * 4)?;
        for (p, &l) in bgra.chunks_exact_mut(4).zip(a8) {
            p.copy_from_slice(&[l, l, l, 255])
        }

        let mut size = size;
        if let Some((s, px)) = Self::downscale_decoded(None, downscale, resizer_cache, size, &bgra)? {
            size = s;
            bgra = px;
        }

        Ok((
            bgra.finish_blocking()?,
            size,
            density,
            true,  // is_opaque
            false, // is_mask
        ))
    }

    pub(super) fn downscale_decoded(
        mask: Option<ImageMaskMode>,
        downscale: Option<zng_view_api::image::ImageDownscale>,
        resizer_cache: &ResizerCache,
        source_size: PxSize,
        pixels: &[u8],
    ) -> std::io::Result<Option<(PxSize, IpcBytesMut)>> {
        if let Some(downscale) = downscale {
            let dest_size = downscale.resize_dimensions(source_size);
            if source_size.min(dest_size) != source_size {
                use fast_image_resize as fr;

                let px_type = if mask.is_none() { fr::PixelType::U8x4 } else { fr::PixelType::U8 };
                let source = fr::images::ImageRef::new(source_size.width.0 as _, source_size.height.0 as _, pixels, px_type).unwrap();
                let mut dest_buf = IpcBytes::new_mut_blocking(dest_size.width.0 as usize * dest_size.height.0 as usize * px_type.size())?;
                let mut dest =
                    fr::images::Image::from_slice_u8(dest_size.width.0 as _, dest_size.height.0 as _, &mut dest_buf[..], px_type).unwrap();

                let mut resize_opt = fr::ResizeOptions::new();
                // is already pre multiplied
                resize_opt.mul_div_alpha = false;
                // default, best quality
                resize_opt.algorithm = fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3);
                // try to reuse cache
                match resizer_cache.try_lock() {
                    Some(mut r) => r.resize(&source, &mut dest, Some(&resize_opt)),
                    None => fr::Resizer::new().resize(&source, &mut dest, Some(&resize_opt)),
                }
                .unwrap();

                return Ok(Some((dest_size, dest_buf)));
            }
        }

        Ok(None)
    }
}

fn luminance(r: u8, g: u8, b: u8) -> u8 {
    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0) as u8
}

fn luminance_16(r: u16, g: u16, b: u16) -> u8 {
    let max = u16::MAX as f32;
    let r = r as f32 / max;
    let g = g as f32 / max;
    let b = b as f32 / max;

    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0) as u8
}

fn luminance_f32(r: f32, g: f32, b: f32) -> u8 {
    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0).clamp(0.0, 255.0) as u8
}
