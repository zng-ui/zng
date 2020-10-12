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

                let metrics = {
                    let loader = handle.load().expect("cannot load font [2]");
                    loader.metrics()
                };

                let harfbuzz_face = match handle {
                    font_kit::handle::Handle::Path { path, font_index } => {
                        let r = harfbuzz_rs::Face::from_file(&path, font_index).expect("cannot load font [1]");
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

                Some(Font::new(api, font_key, properties, metrics, harfbuzz_face.into()))
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
    harfbuzz_face: HarfbuzzFace,
    instances: RefCell<FnvHashMap<FontSizePt, FontInstance>>,
}

/// Reference to a specific font (family + style, weight and stretch).
#[derive(Clone)]
pub struct Font {
    inner: Arc<FontInner>,
}
impl Font {
    fn new(
        api: Arc<RenderApi>,
        font_key: FontKey,
        properties: FontProperties,
        metrics: font_kit::metrics::Metrics,
        harfbuzz_face: HarfbuzzFace,
    ) -> Self {
        Font {
            inner: Arc::new(FontInner {
                api,
                font_key,
                metrics,
                properties,
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

    ///// Gets the font name.
    //#[inline]
    //pub fn font_name(&self) -> &FontName {
    //    &self.inner.font_name
    //}
    //
    ///// Gets the index of the font in the font file.
    //#[inline]
    //pub fn font_index(&self) -> u32 {
    //    self.inner.font_index
    //}

    /// Gets the WebRender font key.
    #[inline]
    pub fn font_key(&self) -> FontKey {
        self.inner.font_key
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
    pub fn instance_key(&self) -> FontInstanceKey {
        self.inner.instance_key
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
