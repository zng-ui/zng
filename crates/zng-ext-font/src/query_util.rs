#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
pub use desktop::*;

#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[cfg(target_os = "android")]
pub use android::*;

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
mod desktop {
    use std::{
        borrow::Cow,
        path::{Path, PathBuf},
        sync::Arc,
    };

    use parking_lot::Mutex;
    use zng_layout::unit::ByteUnits;
    use zng_var::ResponseVar;

    use crate::{FontDataRef, FontLoadingError, FontName, FontStretch, FontStyle, FontWeight, GlyphLoadingError};

    static DATA_CACHE: Mutex<Vec<(PathBuf, std::sync::Weak<Vec<u8>>)>> = Mutex::new(vec![]);

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
            Err(font_kit::error::SelectionError::CannotAccessSource { reason }) => {
                Err(FontLoadingError::Io(Arc::new(std::io::Error::other(reason.unwrap_or_default()))))
            }
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
                    font_kit::error::SelectionError::CannotAccessSource { reason } => {
                        Err(FontLoadingError::Io(Arc::new(std::io::Error::other(reason.unwrap_or_default()))))
                    }
                };
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

                for (k, data) in DATA_CACHE.lock().iter() {
                    if *k == *path {
                        if let Some(data) = data.upgrade() {
                            return Ok((FontDataRef(data), *font_index));
                        }
                    }
                }

                let bytes = std::fs::read(&*path)?;
                tracing::debug!("read font `{}:{}`, using {}", path.display(), font_index, bytes.capacity().bytes());

                let data = Arc::new(bytes);
                let mut cache = DATA_CACHE.lock();
                cache.retain(|(_, v)| v.strong_count() > 0);
                cache.push((path.to_path_buf(), Arc::downgrade(&data)));

                Ok((FontDataRef(data), *font_index))
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
                _ => Title(font_name.txt.into()),
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
    // font-kit does not cross compile for Android because of a dependency,
    // so we reimplement/copy some of their code here.

    use std::{borrow::Cow, path::PathBuf, sync::Arc};

    use zng_var::ResponseVar;

    use crate::{FontDataRef, FontLoadingError, FontName, FontStretch, FontStyle, FontWeight};

    pub fn system_all() -> ResponseVar<Vec<FontName>> {
        zng_task::wait_respond(|| {
            let mut prev = None;
            cached_system_all()
                .iter()
                .flat_map(|(k, _)| {
                    if prev == Some(k) {
                        None
                    } else {
                        prev = Some(k);
                        Some(k)
                    }
                })
                .cloned()
                .collect()
        })
    }

    fn cached_system_all() -> parking_lot::MappedRwLockReadGuard<'static, Vec<(FontName, PathBuf)>> {
        let lock = SYSTEM_ALL.read();
        if !lock.is_empty() {
            return lock;
        }

        drop(lock);
        let mut lock = SYSTEM_ALL.write();
        if lock.is_empty() {
            for entry in ["/system/fonts/", "/system/font/", "/data/fonts/", "/system/product/fonts/"]
                .iter()
                .flat_map(std::fs::read_dir)
                .flatten()
                .flatten()
            {
                let entry = entry.path();
                let ext = entry.extension().and_then(|e| e.to_str()).unwrap_or_default().to_ascii_lowercase();
                if ["ttf", "otf"].contains(&ext.as_str()) && entry.is_file() {
                    if let Ok(bytes) = std::fs::read(&entry) {
                        match crate::FontFace::load(FontDataRef(Arc::new(bytes)), 0) {
                            Ok(f) => {
                                lock.push((f.family_name().clone(), entry));
                            }
                            Err(e) => tracing::error!("error parsing '{}', {e}", entry.display()),
                        }
                    }
                }
            }
            lock.sort_by(|a, b| a.0.cmp(&b.0));
        }
        drop(lock);
        SYSTEM_ALL.read()
    }

    pub fn best(
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Result<Option<(FontDataRef, u32)>, FontLoadingError> {
        let lock = cached_system_all();
        let lock = &*lock;

        // special names
        // source: https://android.googlesource.com/platform/frameworks/base/+/master/data/fonts/fonts.xml
        let font_name = match font_name.name() {
            "sans-serif" => Cow::Owned(FontName::new("Roboto")),
            "serif" | "fantasy" => Cow::Owned(FontName::new("Noto Serif")),
            "cursive" => Cow::Owned(FontName::new("Dancing Script")),
            "monospace" => Cow::Owned(FontName::new("Droid Sans Mono")),
            _ => Cow::Borrowed(font_name),
        };
        let font_name = &*font_name;

        let mut start_i = match lock.binary_search_by(|a| a.0.cmp(font_name)) {
            Ok(i) => i,
            Err(_) => {
                tracing::debug!(target: "font_loading", "system font not found\nquery: {:?}", (font_name, style, weight, stretch));
                return Ok(None);
            }
        };
        while start_i > 0 && &lock[start_i - 1].0 == font_name {
            start_i -= 1
        }
        let mut end_i = start_i;
        while end_i + 1 < lock.len() && &lock[end_i + 1].0 == font_name {
            end_i += 1
        }

        let family_len = end_i - start_i;
        let mut options = Vec::with_capacity(family_len);
        let mut candidates = Vec::with_capacity(family_len);

        for (_, path) in &lock[start_i..=end_i] {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(f) = ttf_parser::Face::parse(&bytes, 0) {
                    candidates.push(matching::Properties {
                        style: f.style(),
                        weight: f.weight(),
                        stretch: f.width(),
                    });
                    options.push(bytes);
                }
            }
        }

        match matching::find_best_match(
            &candidates,
            &matching::Properties {
                style: style.into(),
                weight: weight.into(),
                stretch: stretch.into(),
            },
        ) {
            Ok(i) => {
                let bytes = options.swap_remove(i);
                Ok(Some((FontDataRef(Arc::new(bytes)), 0)))
            }
            Err(FontLoadingError::NoSuchFontInCollection) => {
                tracing::debug!(target: "font_loading", "system font not found\nquery: {:?}", (font_name, style, weight, stretch));
                Ok(None)
            }
            Err(e) => Err(FontLoadingError::Io(Arc::new(std::io::Error::other(e)))),
        }
    }

    zng_app_context::app_local! {
        static SYSTEM_ALL: Vec<(FontName, PathBuf)> = vec![];
    }

    mod matching {
        // font-kit/src/matching.rs
        //
        // Copyright © 2018 The Pathfinder Project Developers.
        //
        // Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
        // http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
        // <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
        // option. This file may not be copied, modified, or distributed
        // except according to those terms.

        //! Determines the closest font matching a description per the CSS Fonts Level 3 specification.

        use ttf_parser::{Style, Weight, Width as Stretch};

        use crate::FontLoadingError;

        pub struct Properties {
            pub style: Style,
            pub weight: Weight,
            pub stretch: Stretch,
        }

        /// This follows CSS Fonts Level 3 § 5.2 [1].
        ///
        /// https://drafts.csswg.org/css-fonts-3/#font-style-matching
        pub fn find_best_match(candidates: &[Properties], query: &Properties) -> Result<usize, FontLoadingError> {
            // Step 4.
            let mut matching_set: Vec<usize> = (0..candidates.len()).collect();
            if matching_set.is_empty() {
                return Err(FontLoadingError::NoSuchFontInCollection);
            }

            // Step 4a (`font-stretch`).
            let matching_stretch = if matching_set.iter().any(|&index| candidates[index].stretch == query.stretch) {
                // Exact match.
                query.stretch
            } else if query.stretch <= Stretch::Normal {
                // Closest width, first checking narrower values and then wider values.
                match matching_set
                    .iter()
                    .filter(|&&index| candidates[index].stretch < query.stretch)
                    .min_by_key(|&&index| query.stretch.to_number() - candidates[index].stretch.to_number())
                {
                    Some(&matching_index) => candidates[matching_index].stretch,
                    None => {
                        let matching_index = *matching_set
                            .iter()
                            .min_by_key(|&&index| candidates[index].stretch.to_number() - query.stretch.to_number())
                            .unwrap();
                        candidates[matching_index].stretch
                    }
                }
            } else {
                // Closest width, first checking wider values and then narrower values.
                match matching_set
                    .iter()
                    .filter(|&&index| candidates[index].stretch > query.stretch)
                    .min_by_key(|&&index| candidates[index].stretch.to_number() - query.stretch.to_number())
                {
                    Some(&matching_index) => candidates[matching_index].stretch,
                    None => {
                        let matching_index = *matching_set
                            .iter()
                            .min_by_key(|&&index| query.stretch.to_number() - candidates[index].stretch.to_number())
                            .unwrap();
                        candidates[matching_index].stretch
                    }
                }
            };
            matching_set.retain(|&index| candidates[index].stretch == matching_stretch);

            // Step 4b (`font-style`).
            let style_preference = match query.style {
                Style::Italic => [Style::Italic, Style::Oblique, Style::Normal],
                Style::Oblique => [Style::Oblique, Style::Italic, Style::Normal],
                Style::Normal => [Style::Normal, Style::Oblique, Style::Italic],
            };
            let matching_style = *style_preference
                .iter()
                .find(|&query_style| matching_set.iter().any(|&index| candidates[index].style == *query_style))
                .unwrap();
            matching_set.retain(|&index| candidates[index].style == matching_style);

            // Step 4c (`font-weight`).
            //
            // The spec doesn't say what to do if the weight is between 400 and 500 exclusive, so we
            // just use 450 as the cutoff.
            let matching_weight = if matching_set.iter().any(|&index| candidates[index].weight == query.weight) {
                query.weight
            } else if query.weight.to_number() >= 400
                && query.weight.to_number() < 450
                && matching_set.iter().any(|&index| candidates[index].weight == Weight::from(500))
            {
                // Check 500 first.
                Weight::from(500)
            } else if query.weight.to_number() >= 450
                && query.weight.to_number() <= 500
                && matching_set.iter().any(|&index| candidates[index].weight.to_number() == 400)
            {
                // Check 400 first.
                Weight::from(400)
            } else if query.weight.to_number() <= 500 {
                // Closest weight, first checking thinner values and then fatter ones.
                match matching_set
                    .iter()
                    .filter(|&&index| candidates[index].weight.to_number() <= query.weight.to_number())
                    .min_by_key(|&&index| query.weight.to_number() - candidates[index].weight.to_number())
                {
                    Some(&matching_index) => candidates[matching_index].weight,
                    None => {
                        let matching_index = *matching_set
                            .iter()
                            .min_by_key(|&&index| candidates[index].weight.to_number() - query.weight.to_number())
                            .unwrap();
                        candidates[matching_index].weight
                    }
                }
            } else {
                // Closest weight, first checking fatter values and then thinner ones.
                match matching_set
                    .iter()
                    .filter(|&&index| candidates[index].weight.to_number() >= query.weight.to_number())
                    .min_by_key(|&&index| candidates[index].weight.to_number() - query.weight.to_number())
                {
                    Some(&matching_index) => candidates[matching_index].weight,
                    None => {
                        let matching_index = *matching_set
                            .iter()
                            .min_by_key(|&&index| query.weight.to_number() - candidates[index].weight.to_number())
                            .unwrap();
                        candidates[matching_index].weight
                    }
                }
            };
            matching_set.retain(|&index| candidates[index].weight == matching_weight);

            // Step 4d concerns `font-size`, but fonts in `font-kit` are unsized, so we ignore that.

            // Return the result.
            matching_set.into_iter().next().ok_or(FontLoadingError::NoSuchFontInCollection)
        }
    }
}
