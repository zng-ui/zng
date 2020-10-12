use super::{FontInstanceKey, FontMetrics, FontName, FontSizePt, FontStretch, FontStyle, FontWeight};
use crate::core::{app::AppExtension, context::AppInitContext, context::WindowService, var::ContextVar};
use crate::properties::text_theme::FontFamilyVar;
use fnv::FnvHashMap;
use font_kit::properties::Properties as FontProperties;
use std::collections::hash_map::Entry as HEntry;
use std::{cell::RefCell, collections::HashMap, sync::Arc};
use webrender::api::{units::Au, FontKey, RenderApi, Transaction};

/// Application extension that provides the [`Fonts`] window service.
#[derive(Default)]
pub struct FontManager;
impl AppExtension for FontManager {
    fn init(&mut self, r: &mut AppInitContext) {
        r.window_services.register(|ctx| Fonts {
            api: Arc::clone(ctx.render_api),
            fonts: HashMap::default(),
        })
    }
}

/// Fonts cache service.
pub struct Fonts {
    api: Arc<RenderApi>,
    fonts: HashMap<FontQueryKey, Font>,
}
impl Fonts {
    /// Gets a cached font or loads a font.
    #[inline]
    pub fn get(&mut self, font_names: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<Font> {
        self.get_font(font_names, FontProperties { style, weight, stretch })
    }

    /// Gets a font using [`get`](Self::get) or fallback to the any of the default fonts.
    #[inline]
    pub fn get_or_default(&mut self, font_names: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Font {
        self.get_font(font_names, FontProperties { style, weight, stretch })
            .or_else(|| {
                warn_println!("did not found font: {:?}", font_names);
                self.get_font(FontFamilyVar::default_value(), FontProperties::default())
            })
            .expect("did not find any default font")
    }

    /// Removes unused font instances and fonts from the cache.
    pub fn drop_unused(&mut self) {
        let mut txn = Transaction::new();
        self.fonts.retain(|_, v| v.retain(&mut txn));

        if !txn.is_empty() {
            self.api.update_resources(txn.resource_updates);
        }
    }

    fn get_font(&mut self, font_names: &[FontName], properties: FontProperties) -> Option<Font> {
        let query_key = (font_names.to_vec().into_boxed_slice(), FontPropertiesKey::new(properties));

        match self.fonts.entry(query_key) {
            HEntry::Occupied(e) => Some(e.get().clone()),
            HEntry::Vacant(e) => {
                if let Some(font) = Self::load_font(self.api.clone(), font_names, properties) {
                    Some(e.insert(font).clone())
                } else {
                    None
                }
            }
        }
    }

    fn load_font(api: Arc<RenderApi>, font_names: &[FontName], properties: FontProperties) -> Option<Font> {
        let family_names: Vec<font_kit::family_name::FamilyName> = font_names.iter().map(|n| n.clone().into()).collect();
        match font_kit::source::SystemSource::new().select_best_match(&family_names, &properties) {
            Ok(handle) => {
                let mut txn = Transaction::new();
                let font_key = api.generate_font_key();

                let font_kit_font = handle.load().expect("cannot load font [font_kit]");

                let harfbuzz_face = match handle {
                    font_kit::handle::Handle::Path { path, font_index } => {
                        let r = harfbuzz_rs::Face::from_file(&path, font_index).expect("cannot load font [harfbuzz]");
                        txn.add_native_font(font_key, webrender::api::NativeFontHandle { path, index: font_index });
                        r
                    }
                    font_kit::handle::Handle::Memory { bytes, font_index } => {
                        let blob = harfbuzz_rs::Blob::with_bytes_owned(Arc::clone(&bytes), |a| &*a);
                        let r = harfbuzz_rs::Face::new(blob, font_index);
                        txn.add_raw_font(font_key, (&*bytes).clone(), font_index);
                        r
                    }
                };

                api.update_resources(txn.resource_updates);

                Some(Font::new(api, font_key, font_kit_font, harfbuzz_face.into()))
            }
            Err(font_kit::error::SelectionError::NotFound) => None,
            Err(font_kit::error::SelectionError::CannotAccessSource) => panic!("cannot access system font source"),
        }
    }
}
impl WindowService for Fonts {}

struct FontInner {
    api: Arc<RenderApi>,
    font_key: FontKey,
    properties: FontProperties,
    metrics: font_kit::metrics::Metrics,
    display_name: FontName,
    family_name: FontName,
    postscript_name: Option<String>,
    font_kit_font: font_kit::font::Font,
    harfbuzz_face: HarfbuzzFace,
    instances: RefCell<FnvHashMap<FontSizePt, FontInstance>>,
}

/// Reference to a specific font (family + style, weight and stretch).
#[derive(Clone)]
pub struct Font {
    inner: Arc<FontInner>,
}
impl Font {
    fn new(api: Arc<RenderApi>, font_key: FontKey, font_kit_font: font_kit::font::Font, harfbuzz_face: HarfbuzzFace) -> Self {
        Font {
            inner: Arc::new(FontInner {
                api,
                font_key,
                metrics: font_kit_font.metrics(),
                display_name: FontName::new(font_kit_font.full_name()),
                family_name: FontName::new(font_kit_font.family_name()),
                postscript_name: font_kit_font.postscript_name(),
                properties: font_kit_font.properties(),
                font_kit_font,
                harfbuzz_face,
                instances: RefCell::default(),
            }),
        }
    }

    /// Gets a cached instance of instantiate the font at the size.
    pub fn instance(&self, font_size: FontSizePt) -> FontInstance {
        if let Some(instance) = self.inner.instances.borrow().get(&font_size) {
            return instance.clone();
        }

        let api = &self.inner.api;
        let mut txn = Transaction::new();
        let instance_key = api.generate_font_instance_key();

        let size_px = font_size as f32 * 96.0 / 72.0;
        txn.add_font_instance(instance_key, self.inner.font_key, Au::from_f32_px(size_px), None, None, Vec::new());
        api.update_resources(txn.resource_updates);

        let mut harfbuzz_font = harfbuzz_rs::Font::new(harfbuzz_rs::Shared::clone(&self.inner.harfbuzz_face));

        harfbuzz_font.set_ppem(font_size, font_size);
        harfbuzz_font.set_scale(font_size as i32 * 64, font_size as i32 * 64);

        let metrics = FontMetrics::new(size_px, &self.inner.metrics);

        let instance = FontInstance::new(self.clone(), instance_key, font_size, metrics, harfbuzz_font.to_shared());
        self.inner.instances.borrow_mut().insert(font_size, instance.clone());

        instance
    }

    /// Font full name.
    #[inline]
    pub fn display_name(&self) -> &FontName {
        &self.inner.display_name
    }

    /// Font family name.
    #[inline]
    pub fn family_name(&self) -> &FontName {
        &self.inner.family_name
    }

    /// Font globally unique name.
    #[inline]
    pub fn postscript_name(&self) -> Option<&str> {
        self.inner.postscript_name.as_deref()
    }

    /// Index of the font in the font file.
    #[inline]
    pub fn index(&self) -> u32 {
        self.inner.harfbuzz_face.index()
    }

    /// Number of glyphs in the font.
    #[inline]
    pub fn glyph_count(&self) -> u32 {
        self.inner.harfbuzz_face.glyph_count()
    }

    /// Font weight.
    #[inline]
    pub fn weight(&self) -> FontWeight {
        self.inner.properties.weight
    }

    /// Font style.
    #[inline]
    pub fn style(&self) -> FontStyle {
        self.inner.properties.style
    }

    /// If the font is fixed-width.
    #[inline]
    pub fn is_monospace(&self) -> bool {
        self.inner.font_kit_font.is_monospace()
    }

    /// The WebRender font key.
    ///
    /// # Careful
    ///
    /// The WebRender font resource is managed by this struct, don't manually request a font delete with this key.
    ///
    /// Keep a clone of the [`Font`] reference alive if you want to manually create font instances, otherwise the
    /// font may be cleaned-up.
    #[inline]
    pub fn font_key(&self) -> webrender::api::FontKey {
        self.inner.font_key
    }

    /// Reference the underlying [`font-kit`](font_kit) font handle.
    #[inline]
    pub fn font_kit_handle(&self) -> &font_kit::font::Font {
        &self.inner.font_kit_font
    }

    /// Reference the cached [`font-kit`](font_kit) metrics.
    #[inline]
    pub fn font_kit_metrics(&self) -> &font_kit::metrics::Metrics {
        &self.inner.metrics
    }

    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &harfbuzz_rs::Shared<harfbuzz_rs::Face> {
        &self.inner.harfbuzz_face
    }

    /// If the font is referenced outside of the cache.
    fn in_use(&self) -> bool {
        Arc::strong_count(&self.inner) > 1
    }

    /// Retain instances in use, register delete for instances removed. Register delete for font if it is not in use also.
    fn retain(&mut self, txn: &mut Transaction) -> bool {
        self.inner.instances.borrow_mut().retain(|_, v| {
            let retain = v.in_use();
            if !retain {
                txn.delete_font_instance(v.instance_key());
            }
            retain
        });

        let retain = self.in_use();
        if !retain {
            txn.delete_font(self.font_key());
        }
        retain
    }
}

pub(super) struct FontInstanceInner {
    instance_key: FontInstanceKey,
    font: Font,
    pub(super) font_size: FontSizePt,
    pub(super) harfbuzz_font: HarfbuzzFont,
    pub(super) metrics: FontMetrics,
}

/// Reference to a specific font instance ([`Font`] + size).
#[derive(Clone)]
pub struct FontInstance {
    pub(super) inner: Arc<FontInstanceInner>,
}
impl FontInstance {
    fn new(font: Font, instance_key: FontInstanceKey, font_size: FontSizePt, metrics: FontMetrics, harfbuzz_font: HarfbuzzFont) -> Self {
        FontInstance {
            inner: Arc::new(FontInstanceInner {
                font,
                instance_key,
                font_size,
                metrics,
                harfbuzz_font,
            }),
        }
    }

    /// Source font reference.
    #[inline]
    pub fn font(&self) -> &Font {
        &self.inner.font
    }

    /// Size of this font instance.
    #[inline]
    pub fn size(&self) -> FontSizePt {
        self.inner.font_size
    }

    /// Various metrics that apply to this font.
    #[inline]
    pub fn metrics(&self) -> &FontMetrics {
        &self.inner.metrics
    }

    /// Gets the font instance key.
    ///
    /// # Careful
    ///
    /// The WebRender font instance resource is managed by this struct, don't manually request a delete with this key.
    ///
    /// Keep a clone of the [`FontInstance`] reference alive for the period you want to render using this font,
    /// otherwise the font may be cleaned-up.
    #[inline]
    pub fn instance_key(&self) -> FontInstanceKey {
        self.inner.instance_key
    }

    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &harfbuzz_rs::Shared<harfbuzz_rs::Font> {
        &self.inner.harfbuzz_font
    }

    /// If the font instance is referenced outside of the cache.
    fn in_use(&self) -> bool {
        Arc::strong_count(&self.inner) > 1
    }
}

type FontQueryKey = (Box<[FontName]>, FontPropertiesKey);

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
struct FontPropertiesKey(u8, u32, u32);
impl FontPropertiesKey {
    pub fn new(properties: FontProperties) -> Self {
        Self(
            match properties.style {
                FontStyle::Normal => 0,
                FontStyle::Italic => 1,
                FontStyle::Oblique => 2,
            },
            (properties.weight.0 * 100.0) as u32,
            (properties.stretch.0 * 100.0) as u32,
        )
    }
}

type HarfbuzzFace = harfbuzz_rs::Shared<harfbuzz_rs::Face<'static>>;

type HarfbuzzFont = harfbuzz_rs::Shared<harfbuzz_rs::Font<'static>>;
