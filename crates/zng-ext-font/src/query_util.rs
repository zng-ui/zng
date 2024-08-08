#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub use desktop::*;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(target_os = "android")]
pub use android::*;

#[cfg(target_arch = "wasm32")]
mod wasm {
    use zng_var::ResponseVar;

    use crate::{FontDataRef, FontLoadingError, FontName, FontStretch, FontStyle, FontWeight};

    pub fn system_all() -> ResponseVar<Vec<FontName>> {
        zng_var::response_done_var(vec![])
    }

    pub fn best(
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Result<Option<(FontDataRef, u32)>, FontLoadingError> {
        let _ = (font_name, style, weight, stretch);
        Err(FontLoadingError::NoFilesystem)
    }
}

#[cfg(target_os = "android")]
mod android {
    use zng_var::ResponseVar;

    use crate::{FontDataRef, FontLoadingError, FontName, FontStretch, FontStyle, FontWeight};

    #[cfg(feature = "android_api_level_29")]
    pub fn system_all() -> ResponseVar<Vec<FontName>> {
        zng_task::wait_respond(|| {
            ndk::font::SystemFontIterator::new()
                .into_iter()
                .flatten()
                .filter_map(|f| {
                    let bytes = std::fs::read(f.path()).ok()?;
                    let bytes = FontDataRef(std::sync::Arc::new(bytes));
                    let face_index = f.collection_index();
                    let f = crate::FontFace::load(bytes, face_index as _).ok()?;
                    Some(f.family_name().clone())
                })
                .collect()
        })
    }

    #[cfg(not(feature = "android_api_level_29"))]
    pub fn system_all() -> ResponseVar<Vec<FontName>> {
        zng_var::response_done_var(vec![])
    }

    pub fn best(
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Result<Option<(FontDataRef, u32)>, FontLoadingError> {
        let _ = (font_name, style, weight, stretch);
        Err(FontLoadingError::NoFilesystem)
    }
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod desktop {
    use std::{borrow::Cow, path::Path, sync::Arc};

    use zng_var::ResponseVar;

    use crate::{FontDataRef, FontLoadingError, FontName, FontStretch, FontStyle, FontWeight, GlyphLoadingError};

    pub fn system_all() -> ResponseVar<Vec<FontName>> {
        zng_task::wait_respond(|| {
            font_kit::source::SystemSource::new()
                .all_families()
                .unwrap_or_default()
                .into_iter()
                .map(FontName::from)
                .collect()
        })
    }

    pub fn best(
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Result<Option<(FontDataRef, u32)>, FontLoadingError> {
        if font_name == "Ubuntu" {
            if let Ok(Some(h)) = workaround_ubuntu(style, weight, stretch) {
                return Ok(Some(h));
            }
        }

        let family_name = font_kit::family_name::FamilyName::from(font_name.clone());
        match font_kit::source::SystemSource::new().select_best_match(
            &[family_name],
            &font_kit::properties::Properties {
                style: style.into(),
                weight: weight.into(),
                stretch: stretch.into(),
            },
        ) {
            Ok(handle) => {
                let r = load_handle(&handle)?;
                Ok(Some(r))
            }
            Err(font_kit::error::SelectionError::NotFound) => {
                tracing::debug!(target: "font_loading", "system font not found\nquery: {:?}", (font_name, style, weight, stretch));
                Ok(None)
            }
            Err(font_kit::error::SelectionError::CannotAccessSource { reason }) => Err(FontLoadingError::Io(Arc::new(
                std::io::Error::new(std::io::ErrorKind::Other, reason.unwrap_or_default()),
            ))),
        }
    }

    // see https://github.com/servo/font-kit/issues/245
    fn workaround_ubuntu(
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Result<Option<(FontDataRef, u32)>, FontLoadingError> {
        let source = font_kit::source::SystemSource::new();
        let ubuntu = match source.select_family_by_name("Ubuntu") {
            Ok(u) => u,
            Err(e) => {
                return match e {
                    font_kit::error::SelectionError::NotFound => Ok(None),
                    font_kit::error::SelectionError::CannotAccessSource { reason } => Err(FontLoadingError::Io(Arc::new(
                        std::io::Error::new(std::io::ErrorKind::Other, reason.unwrap_or_default()),
                    ))),
                }
            }
        };
        for handle in ubuntu.fonts() {
            let font = handle.load()?;
            let name = match font.postscript_name() {
                Some(n) => n,
                None => continue,
            };

            // Ubuntu-ExtraBold
            // Ubuntu-Condensed
            // Ubuntu-CondensedLight
            // Ubuntu-CondensedBold
            // Ubuntu-CondensedMedium
            // Ubuntu-CondensedExtraBold
            // UbuntuItalic-CondensedLightItalic
            // UbuntuItalic-CondensedItalic
            // UbuntuItalic-CondensedMediumItalic
            // UbuntuItalic-CondensedBoldItalic
            // UbuntuItalic-CondensedExtraBoldItalic
            // Ubuntu-Italic
            // UbuntuItalic-ThinItalic
            // UbuntuItalic-LightItalic
            // UbuntuItalic-Italic
            // UbuntuItalic-MediumItalic
            // UbuntuItalic-BoldItalic
            // UbuntuItalic-ExtraBoldItalic
            // UbuntuItalic-CondensedThinItalic
            // Ubuntu-Thin
            // Ubuntu-Regular
            // Ubuntu-Light
            // Ubuntu-Bold
            // Ubuntu-Medium
            // Ubuntu-CondensedThin

            if (style == FontStyle::Italic) != name.contains("Italic") {
                continue;
            }

            if (FontWeight::MEDIUM..FontWeight::SEMIBOLD).contains(&weight) != name.contains("Medium") {
                continue;
            }
            if (weight >= FontWeight::EXTRA_BOLD) != name.contains("ExtraBold") {
                continue;
            }
            if (FontWeight::SEMIBOLD..FontWeight::EXTRA_BOLD).contains(&weight) != name.contains("Bold") {
                continue;
            }

            if (FontWeight::EXTRA_LIGHT..FontWeight::LIGHT).contains(&weight) != name.contains("Light") {
                continue;
            }
            if (weight < FontWeight::EXTRA_LIGHT) != name.contains("Thin") {
                continue;
            }

            if (stretch <= FontStretch::CONDENSED) != name.contains("Condensed") {
                continue;
            }

            return Ok(Some(load_handle(handle)?));
        }
        Ok(None)
    }

    fn load_handle(handle: &font_kit::handle::Handle) -> Result<(FontDataRef, u32), FontLoadingError> {
        match handle {
            font_kit::handle::Handle::Path { path, font_index } => {
                let mut path = Cow::Borrowed(path);
                // try replacing type1 fonts with OpenType
                // RustyBuzz does not support type1 (neither does Harfbuzz, it is obsolete)
                //
                // Example case from default Ubuntu fonts:
                // /usr/share/fonts/type1/urw-base35/Z003-MediumItalic.t1
                // /usr/share/fonts/opentype/urw-base35/Z003-MediumItalic.otf
                if let Ok(base) = path.strip_prefix("/usr/share/fonts/type1/") {
                    if let Some(name) = base.file_name() {
                        if let Some(name) = name.to_str() {
                            if name.ends_with(".t1") {
                                let rep = Path::new("/usr/share/fonts/opentype/").join(base.with_extension("otf"));
                                if rep.exists() {
                                    tracing::debug!("replaced `{name}` with .otf of same name");
                                    path = Cow::Owned(rep);
                                }
                            }
                        }
                    }
                }

                let bytes = std::fs::read(&*path)?;

                Ok((FontDataRef(Arc::new(bytes)), *font_index))
            }
            font_kit::handle::Handle::Memory { bytes, font_index } => Ok((FontDataRef(bytes.clone()), *font_index)),
        }
    }

    impl From<font_kit::error::FontLoadingError> for FontLoadingError {
        fn from(ve: font_kit::error::FontLoadingError) -> Self {
            match ve {
                font_kit::error::FontLoadingError::UnknownFormat => Self::UnknownFormat,
                font_kit::error::FontLoadingError::NoSuchFontInCollection => Self::NoSuchFontInCollection,
                font_kit::error::FontLoadingError::Parse => Self::Parse(ttf_parser::FaceParsingError::MalformedFont),
                font_kit::error::FontLoadingError::NoFilesystem => Self::NoFilesystem,
                font_kit::error::FontLoadingError::Io(e) => Self::Io(Arc::new(e)),
            }
        }
    }

    impl From<FontName> for font_kit::family_name::FamilyName {
        fn from(font_name: FontName) -> Self {
            use font_kit::family_name::FamilyName::*;
            match font_name.name() {
                "serif" => Serif,
                "sans-serif" => SansSerif,
                "monospace" => Monospace,
                "cursive" => Cursive,
                "fantasy" => Fantasy,
                _ => Title(font_name.text.into()),
            }
        }
    }

    impl From<FontStretch> for font_kit::properties::Stretch {
        fn from(value: FontStretch) -> Self {
            font_kit::properties::Stretch(value.0)
        }
    }

    impl From<FontStyle> for font_kit::properties::Style {
        fn from(value: FontStyle) -> Self {
            use font_kit::properties::Style::*;
            match value {
                FontStyle::Normal => Normal,
                FontStyle::Italic => Italic,
                FontStyle::Oblique => Oblique,
            }
        }
    }

    impl From<FontWeight> for font_kit::properties::Weight {
        fn from(value: FontWeight) -> Self {
            font_kit::properties::Weight(value.0)
        }
    }

    impl From<font_kit::error::GlyphLoadingError> for GlyphLoadingError {
        fn from(value: font_kit::error::GlyphLoadingError) -> Self {
            use GlyphLoadingError::*;
            match value {
                font_kit::error::GlyphLoadingError::NoSuchGlyph => NoSuchGlyph,
                font_kit::error::GlyphLoadingError::PlatformError => PlatformError,
            }
        }
    }
}
