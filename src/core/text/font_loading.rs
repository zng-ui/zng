use super::{FontInstanceKey, FontMetrics, FontName, FontSizePt, FontStretch, FontStyle, FontSynthesis, FontWeight};
use crate::core::{app::AppExtension, context::AppInitContext, context::WindowService, var::ContextVar};
use crate::properties::text_theme::FontFamilyVar;
use fnv::FnvHashMap;
use font_kit::properties::Properties as FontProperties;
use std::{cell::RefCell, collections::HashMap, sync::Arc};
use std::{collections::hash_map::Entry as HEntry, rc::Rc};
use webrender::api::{units::Au, FontInstanceFlags, FontInstanceOptions, FontKey, RenderApi, SyntheticItalics, Transaction};

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
    fonts: HashMap<FontQueryKey, FontRef>,
}
impl Fonts {
    /// Gets a cached font or loads a font.
    #[inline]
    pub fn get(&mut self, font_names: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<FontRef> {
        self.get_font(font_names, FontProperties { style, weight, stretch })
    }

    /// Gets a font using [`get`](Self::get) or fallback to the any of the default fonts.
    #[inline]
    pub fn get_or_default(&mut self, font_names: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontRef {
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

    fn get_font(&mut self, font_names: &[FontName], properties: FontProperties) -> Option<FontRef> {
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

    fn load_font(api: Arc<RenderApi>, font_names: &[FontName], properties: FontProperties) -> Option<FontRef> {
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

                Some(FontRef::new(api, font_key, font_kit_font, properties, harfbuzz_face.into()))
            }
            Err(font_kit::error::SelectionError::NotFound) => None,
            Err(font_kit::error::SelectionError::CannotAccessSource) => panic!("cannot access system font source"),
        }
    }
}
impl WindowService for Fonts {}

struct Font {
    api: Arc<RenderApi>,
    font_key: FontKey,
    properties: FontProperties,
    req_properties: FontProperties,
    metrics: font_kit::metrics::Metrics,
    display_name: FontName,
    family_name: FontName,
    postscript_name: Option<String>,
    font_kit_font: font_kit::font::Font,
    harfbuzz_face: HarfbuzzFace,
    instances: RefCell<FnvHashMap<(FontSizePt, FontSynthesis), FontInstanceRef>>,
}

/// Reference to a specific font (family + style, weight and stretch).
#[derive(Clone)]
pub struct FontRef(Rc<Font>);
impl FontRef {
    fn new(
        api: Arc<RenderApi>,
        font_key: FontKey,
        font_kit_font: font_kit::font::Font,
        requested_properties: FontProperties,
        harfbuzz_face: HarfbuzzFace,
    ) -> Self {
        FontRef(Rc::new(Font {
            api,
            font_key,
            metrics: font_kit_font.metrics(),
            display_name: FontName::new(font_kit_font.full_name()),
            family_name: FontName::new(font_kit_font.family_name()),
            postscript_name: font_kit_font.postscript_name(),
            properties: font_kit_font.properties(),
            req_properties: requested_properties,
            font_kit_font,
            harfbuzz_face,
            instances: RefCell::default(),
        }))
    }

    /// Instantiate the font at the size.
    pub fn instance(&self, font_size: FontSizePt, synthesis_allowed: FontSynthesis) -> FontInstanceRef {
        let synthesis_used = self.synthesis_required() & synthesis_allowed;

        if let Some(instance) = self.0.instances.borrow().get(&(font_size, synthesis_used)) {
            return instance.clone();
        }

        let api = &self.0.api;
        let mut txn = Transaction::new();
        let instance_key = api.generate_font_instance_key();

        let size_px = font_size as f32 * 96.0 / 72.0;

        let mut opt = FontInstanceOptions::default();
        if synthesis_used.contains(FontSynthesis::STYLE) {
            opt.synthetic_italics = SyntheticItalics::enabled();
        }
        if synthesis_used.contains(FontSynthesis::BOLD) {
            opt.flags |= FontInstanceFlags::SYNTHETIC_BOLD;
        }
        txn.add_font_instance(instance_key, self.0.font_key, Au::from_f32_px(size_px), Some(opt), None, Vec::new());
        api.update_resources(txn.resource_updates);

        let mut harfbuzz_font = harfbuzz_rs::Font::new(harfbuzz_rs::Shared::clone(&self.0.harfbuzz_face));

        harfbuzz_font.set_ppem(font_size, font_size);
        harfbuzz_font.set_scale(font_size as i32 * 64, font_size as i32 * 64);

        let metrics = FontMetrics::new(size_px, &self.0.metrics);

        let instance = FontInstanceRef::new(
            self.clone(),
            instance_key,
            font_size,
            metrics,
            synthesis_used,
            harfbuzz_font.to_shared(),
        );
        self.0.instances.borrow_mut().insert((font_size, synthesis_used), instance.clone());

        instance
    }

    /// Font full name.
    #[inline]
    pub fn display_name(&self) -> &FontName {
        &self.0.display_name
    }

    /// Font family name.
    #[inline]
    pub fn family_name(&self) -> &FontName {
        &self.0.family_name
    }

    /// Font globally unique name.
    #[inline]
    pub fn postscript_name(&self) -> Option<&str> {
        self.0.postscript_name.as_deref()
    }

    /// Index of the font in the font file.
    #[inline]
    pub fn index(&self) -> u32 {
        self.0.harfbuzz_face.index()
    }

    /// Number of glyphs in the font.
    #[inline]
    pub fn glyph_count(&self) -> u32 {
        self.0.harfbuzz_face.glyph_count()
    }

    /// Font style.
    #[inline]
    pub fn style(&self) -> FontStyle {
        self.0.properties.style
    }

    /// Font weight.
    #[inline]
    pub fn weight(&self) -> FontWeight {
        self.0.properties.weight
    }

    /// Font stretch.
    #[inline]
    pub fn stretch(&self) -> FontStretch {
        self.0.properties.stretch
    }

    /// Font style that was requested.
    ///
    /// If it does not match [`style`](Self::style) synthetic styling may be used in instances.
    #[inline]
    pub fn requested_style(&self) -> FontStyle {
        self.0.req_properties.style
    }

    /// Font style that was requested.
    ///
    /// If it does not match [`weight`](Self::weight) synthetic bolding may be used in instances.
    #[inline]
    pub fn requested_weight(&self) -> FontWeight {
        self.0.req_properties.weight
    }

    /// Font synthesis required to fulfill the requested properties.
    #[inline]
    pub fn synthesis_required(&self) -> FontSynthesis {
        let mut r = FontSynthesis::empty();
        if self.requested_style() != self.style() {
            r = FontSynthesis::STYLE;
        }
        if self.requested_weight() != self.weight() {
            r |= FontSynthesis::BOLD;
        }
        r
    }

    /// If the font is fixed-width.
    #[inline]
    pub fn is_monospace(&self) -> bool {
        self.0.font_kit_font.is_monospace()
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
        self.0.font_key
    }

    /// Reference the underlying [`font-kit`](font_kit) font handle.
    #[inline]
    pub fn font_kit_handle(&self) -> &font_kit::font::Font {
        &self.0.font_kit_font
    }

    /// Reference the cached [`font-kit`](font_kit) metrics.
    #[inline]
    pub fn font_kit_metrics(&self) -> &font_kit::metrics::Metrics {
        &self.0.metrics
    }

    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &harfbuzz_rs::Shared<harfbuzz_rs::Face> {
        &self.0.harfbuzz_face
    }

    /// If the font is referenced outside of the cache.
    fn in_use(&self) -> bool {
        Rc::strong_count(&self.0) > 1
    }

    /// Retain instances in use, register delete for instances removed. Register delete for font if it is not in use also.
    fn retain(&mut self, txn: &mut Transaction) -> bool {
        self.0.instances.borrow_mut().retain(|_, v| {
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
impl PartialEq for FontRef {
    /// If both point to the same font.
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FontRef {}

pub(super) struct FontInstance {
    instance_key: FontInstanceKey,
    font: FontRef,
    font_size: FontSizePt,
    synthesis_used: FontSynthesis,
    harfbuzz_font: HarfbuzzFont,
    metrics: FontMetrics,
}

/// Reference to a specific font instance ([`Font`] + size).
#[derive(Clone)]
pub struct FontInstanceRef(Rc<FontInstance>);
impl FontInstanceRef {
    fn new(
        font: FontRef,
        instance_key: FontInstanceKey,
        font_size: FontSizePt,
        metrics: FontMetrics,
        synthesis_used: FontSynthesis,
        harfbuzz_font: HarfbuzzFont,
    ) -> Self {
        FontInstanceRef(Rc::new(FontInstance {
            font,
            instance_key,
            font_size,
            metrics,
            synthesis_used,
            harfbuzz_font,
        }))
    }

    /// Source font reference.
    #[inline]
    pub fn font(&self) -> &FontRef {
        &self.0.font
    }

    /// Size of this font instance.
    #[inline]
    pub fn size(&self) -> FontSizePt {
        self.0.font_size
    }

    /// Various metrics that apply to this font.
    #[inline]
    pub fn metrics(&self) -> &FontMetrics {
        &self.0.metrics
    }

    /// What synthetic properties are used in this instance.
    #[inline]
    pub fn synthesis_used(&self) -> FontSynthesis {
        self.0.synthesis_used
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
        self.0.instance_key
    }

    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &harfbuzz_rs::Shared<harfbuzz_rs::Font> {
        &self.0.harfbuzz_font
    }

    /// If the font instance is referenced outside of the cache.
    fn in_use(&self) -> bool {
        Rc::strong_count(&self.0) > 1
    }
}
impl PartialEq for FontInstanceRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FontInstanceRef {}

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
