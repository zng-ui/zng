#![cfg_attr(not(feature = "image_any"), allow(unused))]

#[cfg(feature = "image_any")]
use crate::image_cache::ImageHeader;
use crate::image_cache::ResizerCache;
use crate::image_cache::{ImageCache, RawLoadedImg};
use image::{GenericImageView as _, ImageDecoder as _};
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
        orientation: image::metadata::Orientation,
    ) -> image::ImageResult<image::DynamicImage> {
        let buf = std::io::Cursor::new(buf);

        // Some JPEG decoders can downscale to an approximation of this size
        // but that is not implemented by image crate
        let _ = downscale;

        let mut reader = image::ImageReader::new(buf);
        reader.set_format(format);
        reader.no_limits();
        let mut image = reader.decode()?;

        image.apply_orientation(orientation);

        Ok(image)
    }

    pub(super) fn convert_decoded(
        image: image::DynamicImage,
        mask: Option<ImageMaskMode>,
        density: Option<PxDensity2d>,
        icc_profile: Option<lcms2::Profile>,
        downscale: Option<zng_view_api::image::ImageDownscale>,
        resizer_cache: &ResizerCache,
    ) -> std::io::Result<RawLoadedImg> {
        use image::DynamicImage::*;

        let mut is_opaque = true;
        let size = image.dimensions();
        let pixels_len = size.0 as usize * size.1 as usize;

        let mut pixels = match image {
            ImageLuma8(img) => {
                let raw = img.into_raw();
                if mask.is_some() {
                    is_opaque = !raw.iter().any(|&a| a < 255);
                    IpcBytesMut::from_vec_blocking(raw)?
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, l) in bgra.chunks_exact_mut(4).zip(raw) {
                        p.copy_from_slice(&[l, l, l, 255])
                    }
                    bgra
                }
            }
            ImageLumaA8(img) => {
                let raw = img.into_raw();
                if let Some(mask) = mask {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    match mask {
                        ImageMaskMode::A => {
                            for (p, la) in a.iter_mut().zip(raw.chunks_exact(2)) {
                                if la[1] < 255 {
                                    is_opaque = false;
                                }
                                *p = la[1];
                            }
                        }
                        ImageMaskMode::B | ImageMaskMode::G | ImageMaskMode::R | ImageMaskMode::Luminance => {
                            for (p, la) in a.iter_mut().zip(raw.chunks_exact(2)) {
                                if la[0] < 255 {
                                    is_opaque = false;
                                }
                                *p = la[0];
                            }
                        }
                        _ => unimplemented!(),
                    }
                    a
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, la) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(2)) {
                        p.copy_from_slice(&if la[1] < 255 {
                            is_opaque = false;
                            let l = la[0] as f32 * la[1] as f32 / 255.0;
                            let l = l as u8;
                            [l, l, l, la[1]]
                        } else {
                            let l = la[0];
                            [l, l, l, la[1]]
                        });
                    }
                    bgra
                }
            }
            ImageRgb8(img) => {
                let raw = img.into_raw();
                if let Some(mask) = mask {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => {
                            for (p, rgb) in a.iter_mut().zip(raw.chunks(3)) {
                                let l = luminance(rgb);
                                if l < 255 {
                                    is_opaque = false;
                                }
                                *p = l;
                            }
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            for (p, rgb) in a.iter_mut().zip(raw.chunks(3)) {
                                let c = rgb[channel];
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                    }
                    a
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, rgb) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(3)) {
                        p.copy_from_slice(&[rgb[2], rgb[1], rgb[0], 255]);
                    }
                    bgra
                }
            }
            ImageRgba8(img) => {
                let raw = img.into_raw();
                if let Some(mask) = mask {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    match mask {
                        ImageMaskMode::Luminance => {
                            for (p, rgba) in a.iter_mut().zip(raw.chunks_exact(4)) {
                                let c = luminance(&rgba[..3]);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            for (p, rgba) in a.iter_mut().zip(raw.chunks_exact(4)) {
                                let c = rgba[channel];
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                    }
                    a
                } else {
                    let mut buf = raw;
                    buf.chunks_mut(4).for_each(|c| {
                        if c[3] < 255 {
                            is_opaque = false;
                            let a = c[3] as f32 / 255.0;
                            c[0..3].iter_mut().for_each(|c| *c = (*c as f32 * a) as u8);
                        }
                        c.swap(0, 2);
                    });
                    IpcBytesMut::from_vec_blocking(buf)?
                }
            }
            ImageLuma16(img) => {
                let raw = img.into_raw();
                if mask.is_some() {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    for (p, l) in a.iter_mut().zip(raw) {
                        let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                        if l < 255 {
                            is_opaque = false;
                        }
                        *p = l;
                    }
                    a
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, l) in bgra.chunks_exact_mut(4).zip(raw) {
                        let l = (l as f32 / u16::MAX as f32 * 255.0) as u8;
                        p.copy_from_slice(&[l, l, l, 255]);
                    }
                    bgra
                }
            }
            ImageLumaA16(img) => {
                let raw = img.into_raw();
                if let Some(mask) = mask {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    match mask {
                        ImageMaskMode::A => {
                            for (p, la) in a.iter_mut().zip(raw.chunks_exact(2)) {
                                if la[1] < u16::MAX {
                                    is_opaque = false;
                                }
                                let max = u16::MAX as f32;
                                let l = la[1] as f32 / max * 255.0;
                                *p = l as u8;
                            }
                        }
                        ImageMaskMode::B | ImageMaskMode::G | ImageMaskMode::R | ImageMaskMode::Luminance => {
                            for (p, la) in a.iter_mut().zip(raw.chunks_exact(2)) {
                                if la[0] < u16::MAX {
                                    is_opaque = false;
                                }
                                let max = u16::MAX as f32;
                                let l = la[0] as f32 / max * 255.0;
                                *p = l as u8;
                            }
                        }
                        _ => unimplemented!(),
                    }
                    a
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, la) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(2)) {
                        let max = u16::MAX as f32;
                        let l = la[0] as f32 / max;
                        let a = la[1] as f32 / max * 255.0;

                        p.copy_from_slice(&if la[1] < u16::MAX {
                            is_opaque = false;
                            let l = (l * a) as u8;
                            [l, l, l, a as u8]
                        } else {
                            let l = (l * 255.0) as u8;
                            [l, l, l, a as u8]
                        });
                    }
                    bgra
                }
            }
            ImageRgb16(img) => {
                let raw = img.into_raw();
                if let Some(mask) = mask {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => {
                            for (p, rgb) in a.iter_mut().zip(raw.chunks_exact(3)) {
                                let c = luminance_16(rgb);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            for (p, rgb) in a.iter_mut().zip(raw.chunks_exact(3)) {
                                let c = rgb[channel];
                                if c < u16::MAX {
                                    is_opaque = false;
                                }
                                *p = (c as f32 / u16::MAX as f32 * 255.0) as u8;
                            }
                        }
                    }
                    a
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, rgb) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(3)) {
                        let to_u8 = 255.0 / u16::MAX as f32;
                        p.copy_from_slice(&[
                            (rgb[2] as f32 * to_u8) as u8,
                            (rgb[1] as f32 * to_u8) as u8,
                            (rgb[0] as f32 * to_u8) as u8,
                            255,
                        ]);
                    }
                    bgra
                }
            }
            ImageRgba16(img) => {
                let raw = img.into_raw();
                if let Some(mask) = mask {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    match mask {
                        ImageMaskMode::Luminance => {
                            for (p, rgba) in a.iter_mut().zip(raw.chunks_exact(4)) {
                                let c = luminance_16(&rgba[..3]);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            for (p, rgba) in a.iter_mut().zip(raw.chunks_exact(4)) {
                                let c = rgba[channel];
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = (c as f32 / u16::MAX as f32 * 255.0) as u8;
                            }
                        }
                    }
                    a
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, rgba) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(4)) {
                        let c = if rgba[3] < u16::MAX {
                            is_opaque = false;
                            let max = u16::MAX as f32;
                            let a = rgba[3] as f32 / max * 255.0;
                            [
                                (rgba[2] as f32 / max * a) as u8,
                                (rgba[1] as f32 / max * a) as u8,
                                (rgba[0] as f32 / max * a) as u8,
                                a as u8,
                            ]
                        } else {
                            let to_u8 = 255.0 / u16::MAX as f32;
                            [
                                (rgba[2] as f32 * to_u8) as u8,
                                (rgba[1] as f32 * to_u8) as u8,
                                (rgba[0] as f32 * to_u8) as u8,
                                255,
                            ]
                        };
                        p.copy_from_slice(&c);
                    }
                    bgra
                }
            }
            ImageRgb32F(img) => {
                let raw = img.into_raw();
                if let Some(mask) = mask {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    match mask {
                        ImageMaskMode::Luminance | ImageMaskMode::A => {
                            for (p, rgb) in a.iter_mut().zip(raw.chunks_exact(3)) {
                                let c = luminance_f32(rgb);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            for (p, rgb) in a.iter_mut().zip(raw.chunks_exact(3)) {
                                let c = (rgb[channel] * 255.0) as u8;
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                    }
                    a
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, rgb) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(3)) {
                        p.copy_from_slice(&[(rgb[2] * 255.0) as u8, (rgb[1] * 255.0) as u8, (rgb[0] * 255.0) as u8, 255]);
                    }
                    bgra
                }
            }
            ImageRgba32F(img) => {
                let raw = img.into_raw();
                if let Some(mask) = mask {
                    let mut a = IpcBytes::new_mut_blocking(pixels_len)?;
                    match mask {
                        ImageMaskMode::Luminance => {
                            for (p, rgba) in a.iter_mut().zip(raw.chunks_exact(4)) {
                                let c = luminance_f32(&rgba[..3]);
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                        mask => {
                            let channel = match mask {
                                ImageMaskMode::A => 3,
                                ImageMaskMode::B => 2,
                                ImageMaskMode::G => 1,
                                ImageMaskMode::R => 0,
                                _ => unreachable!(),
                            };
                            for (p, rgba) in a.iter_mut().zip(raw.chunks_exact(4)) {
                                let c = (rgba[channel] * 255.0) as u8;
                                if c < 255 {
                                    is_opaque = false;
                                }
                                *p = c;
                            }
                        }
                    }
                    a
                } else {
                    let mut bgra = IpcBytes::new_mut_blocking(pixels_len * 4)?;
                    for (p, rgba) in bgra.chunks_exact_mut(4).zip(raw.chunks_exact(4)) {
                        let c = if rgba[3] < 1.0 {
                            is_opaque = false;
                            let a = rgba[3] * 255.0;
                            [(rgba[2] * a) as u8, (rgba[1] * a) as u8, (rgba[0] * a) as u8, a as u8]
                        } else {
                            [(rgba[2] * 255.0) as u8, (rgba[1] * 255.0) as u8, (rgba[0] * 255.0) as u8, 255]
                        };
                        p.copy_from_slice(&c);
                    }
                    bgra
                }
            }
            _ => unreachable!(),
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

        let mut size = PxSize::new(Px(size.0 as _), Px(size.1 as _));
        if let Some(s) = downscale {
            let source_size = size;
            let dest_size = s.resize_dimensions(source_size);
            if source_size.min(dest_size) != source_size {
                use fast_image_resize as fr;

                let px_type = if mask.is_none() { fr::PixelType::U8x4 } else { fr::PixelType::U8 };
                let source = fr::images::Image::from_slice_u8(size.width.0 as _, size.height.0 as _, &mut pixels, px_type).unwrap();
                let mut dest_buf = IpcBytes::new_mut_blocking(dest_size.width.0 as usize * dest_size.height.0 as usize * px_type.size())?;
                let mut dest =
                    fr::images::Image::from_slice_u8(dest_size.width.0 as _, dest_size.height.0 as _, &mut dest_buf[..], px_type).unwrap();

                let mut resize_opt = fr::ResizeOptions::new();
                // is already pre multiplied
                resize_opt.mul_div_alpha = false;
                // default, best quality
                resize_opt.algorithm = fr::ResizeAlg::Convolution(fr::FilterType::Lanczos3);
                if let zng_view_api::image::ImageDownscale::Fill(_) = s {
                    resize_opt = resize_opt.fit_into_destination(Some((0.5, 0.5)));
                }
                // try to reuse cache
                match resizer_cache.try_lock() {
                    Some(mut r) => r.resize(&source, &mut dest, Some(&resize_opt)),
                    None => fr::Resizer::new().resize(&source, &mut dest, Some(&resize_opt)),
                }
                .unwrap();
                pixels = dest_buf;
                size = dest_size;
            }
        }

        Ok((
            pixels.finish_blocking()?,
            size,
            density,
            is_opaque,
            mask.is_some(), // is_mask
        ))
    }
}

fn luminance(rgb: &[u8]) -> u8 {
    let r = rgb[0] as f32 / 255.0;
    let g = rgb[1] as f32 / 255.0;
    let b = rgb[2] as f32 / 255.0;

    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0) as u8
}

fn luminance_16(rgb: &[u16]) -> u8 {
    let max = u16::MAX as f32;
    let r = rgb[0] as f32 / max;
    let g = rgb[1] as f32 / max;
    let b = rgb[2] as f32 / max;

    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0) as u8
}

fn luminance_f32(rgb: &[f32]) -> u8 {
    let r = rgb[0];
    let g = rgb[1];
    let b = rgb[2];

    let l = r * 0.2126 + g * 0.7152 + b * 0.0722;
    (l * 255.0) as u8
}
