use super::{FontInstanceKey, FontMetrics, FontName, FontProperties, FontSizePt, FontStyle};
use crate::core::{app::AppExtension, context::AppInitContext, context::WindowService, var::ContextVar};
use crate::properties::text_theme::FontFamilyVar;
use fnv::FnvHashMap;
use std::{collections::HashMap, sync::Arc};
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
    fonts: HashMap<FontQueryKey, FontInstances>,
}
impl Fonts {
    /// Gets a cached font instance or loads a new instance.
    pub fn get(&mut self, font_names: &[FontName], properties: &FontProperties, font_size: FontSizePt) -> Option<FontInstance> {
        let query_key = (font_names.to_vec().into_boxed_slice(), FontPropertiesKey::new(*properties));
        if let Some(font) = self.fonts.get_mut(&query_key) {
            if let Some(instance) = font.instances.get(&font_size) {
                Some(instance.clone())
            } else {
                Some(Self::load_font_size(&self.api, font, font_size))
            }
        } else if let Some(instance) = self.load_font(query_key, font_names, properties, font_size) {
            Some(instance)
        } else {
            None
        }
    }

    /// Gets a font using [`get`](Self::get) or fallback to the any of the default fonts.
    pub fn get_or_default(&mut self, font_names: &[FontName], properties: &FontProperties, font_size: FontSizePt) -> FontInstance {
        self.get(font_names, properties, font_size)
            .or_else(|| {
                warn_println!("did not found font: {:?}", font_names);
                self.get(FontFamilyVar::default_value(), &FontProperties::default(), font_size)
            })
            .expect("did not find any default font")
    }

    fn load_font(
        &mut self,
        query_key: FontQueryKey,
        font_names: &[FontName],
        properties: &FontProperties,
        size: FontSizePt,
    ) -> Option<FontInstance> {
        let family_names: Vec<font_kit::family_name::FamilyName> = font_names.iter().map(|n| n.clone().into()).collect();
        match font_kit::source::SystemSource::new().select_best_match(&family_names, properties) {
            Ok(handle) => {
                let mut txn = Transaction::new();
                let font_key = self.api.generate_font_key();

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

                let mut font_instances = FontInstances {
                    font_key,
                    metrics,
                    harfbuzz_face: harfbuzz_face.to_shared(),
                    instances: FnvHashMap::default(),
                };

                self.api.update_resources(txn.resource_updates);
                let instance = Self::load_font_size(&self.api, &mut font_instances, size);
                self.fonts.insert(query_key, font_instances);
                Some(instance)
            }
            Err(font_kit::error::SelectionError::NotFound) => None,
            Err(font_kit::error::SelectionError::CannotAccessSource) => panic!("cannot access system font source"),
        }
    }

    fn load_font_size(api: &RenderApi, font_instances: &mut FontInstances, size: FontSizePt) -> FontInstance {
        let mut txn = Transaction::new();
        let instance_key = api.generate_font_instance_key();

        let size_px = size as f32 * 96.0 / 72.0;
        txn.add_font_instance(
            instance_key,
            font_instances.font_key,
            Au::from_f32_px(size_px),
            None,
            None,
            Vec::new(),
        );
        api.update_resources(txn.resource_updates);

        let mut harfbuzz_font = harfbuzz_rs::Font::new(harfbuzz_rs::Shared::clone(&font_instances.harfbuzz_face));

        harfbuzz_font.set_ppem(size, size);
        harfbuzz_font.set_scale(size as i32 * 64, size as i32 * 64);

        let metrics = FontMetrics::new(size_px, &font_instances.metrics);

        let instance = FontInstance::new(instance_key, size, metrics, harfbuzz_font.to_shared());
        font_instances.instances.insert(size, instance.clone());

        instance
    }
}
impl WindowService for Fonts {}

/// All instances of a font family.
struct FontInstances {
    pub font_key: FontKey,
    pub metrics: font_kit::metrics::Metrics,
    pub harfbuzz_face: HarfbuzzFace,
    pub instances: FnvHashMap<FontSizePt, FontInstance>,
}

pub(super) struct FontInstanceInner {
    instance_key: FontInstanceKey,
    pub(super) font_size: FontSizePt,
    pub(super) harfbuzz_font: HarfbuzzFont,
    pub(super) metrics: FontMetrics,
}

/// Reference to a specific font instance (family and size).
#[derive(Clone)]
pub struct FontInstance {
    pub(super) inner: Arc<FontInstanceInner>,
}
impl FontInstance {
    fn new(instance_key: FontInstanceKey, font_size: FontSizePt, metrics: FontMetrics, harfbuzz_font: HarfbuzzFont) -> Self {
        FontInstance {
            inner: Arc::new(FontInstanceInner {
                instance_key,
                font_size,
                metrics,
                harfbuzz_font,
            }),
        }
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
