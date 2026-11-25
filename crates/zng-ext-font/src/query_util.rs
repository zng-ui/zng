use zng_app_context::app_local;
use zng_task::{channel::IpcBytes, parking_lot::MappedRecursiveRwLockWriteGuard};
use zng_txt::Txt;
use zng_var::ResponseVar;

use crate::{FontBytes, FontLoadingError, FontName, FontStretch, FontStyle, FontWeight, WeakFontBytes};

struct FontiqueCache {
    collection: fontique::Collection,
    source_cache: fontique::SourceCache,
    ipc_cache: Vec<(fontique::FamilyId, usize, WeakFontBytes)>,
}

app_local! {
    static FONTIQUE_CACHE: Option<FontiqueCache> = const { None };
}

pub fn invalidate_fontique_cache() {
    *FONTIQUE_CACHE.write() = None;
}

fn fontique_cache() -> MappedRecursiveRwLockWriteGuard<'static, FontiqueCache> {
    let mut c = FONTIQUE_CACHE.write();
    if c.is_none() {
        let collection = fontique::Collection::new(fontique::CollectionOptions {
            shared: false,
            system_fonts: true,
        });
        let source_cache = fontique::SourceCache::new(fontique::SourceCacheOptions { shared: false });
        *c = Some(FontiqueCache {
            collection,
            source_cache,
            ipc_cache: vec![],
        });
    }
    MappedRecursiveRwLockWriteGuard::map(c, |c| c.as_mut().unwrap())
}

pub fn system_all() -> ResponseVar<Vec<FontName>> {
    zng_task::wait_respond(|| {
        fontique_cache()
            .collection
            .family_names()
            .map(|n| Txt::from_str(n).into())
            .collect()
    })
}

pub fn best(
    font_name: &FontName,
    style: FontStyle,
    weight: FontWeight,
    stretch: FontStretch,
) -> Result<Option<(FontBytes, u32)>, FontLoadingError> {
    use fontique::QueryFamily;

    // fontique does basic replacement for QueryFamily::Generic,
    // but generic names only appear here if they where not resolved by FONTS
    // where we already define replacements
    let family = if cfg!(target_os = "macos") && font_name == "system-ui" {
        // macOS has a special function to get the UI font
        QueryFamily::Generic(fontique::GenericFamily::SystemUi)
    } else {
        // on Linux fontconfig handles generic names so we can just pass the name
        // other systems will just not match
        QueryFamily::Named(font_name.name())
    };

    let attributes = fontique::Attributes {
        width: fontique::FontWidth::from_ratio(stretch.0),
        weight: fontique::FontWeight::new(weight.0),
        style: match style {
            FontStyle::Normal => fontique::FontStyle::Normal,
            FontStyle::Italic => fontique::FontStyle::Italic,
            FontStyle::Oblique => fontique::FontStyle::Oblique(None),
        },
    };

    let mut c = fontique_cache();
    let c = &mut *c;

    let mut matched = None;
    {
        let mut q = c.collection.query(&mut c.source_cache);
        q.set_families([family]);
        q.set_attributes(attributes);
        q.matches_with(|f| {
            matched = Some((f.family, f.index));
            fontique::QueryStatus::Stop
        });
    }
    if matched.is_none() {
        tracing::debug!(target: "font_loading", "system font not found\nquery: {:?}", (font_name, style, weight, stretch));
        return Ok(None);
    }
    let ((family_id, font_file_idx), font_collection_idx) = matched.unwrap();

    let mut active_ipc = None;
    c.ipc_cache.retain(|(f, i, b)| {
        if f == &family_id && *i == font_file_idx {
            if let Some(b) = b.upgrade() {
                active_ipc = Some(b);
                true
            } else {
                false
            }
        } else {
            b.strong_count() > 0
        }
    });
    if let Some((_, _, b)) = c.ipc_cache.iter().find(|(f, i, _)| f == &family_id && *i == font_file_idx)
        && let Some(b) = b.upgrade()
    {
        return Ok(Some((b, font_collection_idx)));
    }

    let family_info = c.collection.family(family_id).unwrap();
    let bytes = match &family_info.fonts()[font_file_idx].source().kind {
        fontique::SourceKind::Memory(b) => FontBytes::from_ipc(IpcBytes::from_slice_blocking(b.as_ref())?),
        fontique::SourceKind::Path(p) => FontBytes::from_file(p.to_path_buf())?,
    };
    c.ipc_cache.push((family_id, font_file_idx, bytes.downgrade()));

    Ok(Some((bytes, font_collection_idx)))
}
