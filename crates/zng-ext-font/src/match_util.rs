use std::path::Path;

use crate::{FontName, FontStretch, FontStyle, FontWeight};

pub fn best(font_name: &FontName, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<font_kit::handle::Handle> {
    if font_name == "Ubuntu" {
        if let Some(h) = workaround_ubuntu(style, weight, stretch) {
            return Some(h);
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
        Ok(mut handle) => {
            if let font_kit::handle::Handle::Path { path, .. } = &mut handle {
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
                                    *path = rep;
                                }
                            }
                        }
                    }
                }
            }

            Some(handle)
        }
        Err(font_kit::error::SelectionError::NotFound) => {
            tracing::debug!(target: "font_loading", "system font not found\nquery: {:?}", (font_name, style, weight, stretch));
            None
        }
        Err(e) => {
            tracing::error!(target: "font_loading", "failed to select system font, {e}\nquery: {:?}", (font_name, style, weight, stretch));
            None
        }
    }
}

// see https://github.com/servo/font-kit/issues/245
fn workaround_ubuntu(style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<font_kit::handle::Handle> {
    let source = font_kit::source::SystemSource::new();
    let ubuntu = source.select_family_by_name("Ubuntu").ok()?;
    for handle in ubuntu.fonts() {
        let font = handle.load().ok()?;
        let name = font.postscript_name()?;

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

        return Some(handle.clone());
    }
    None
}
