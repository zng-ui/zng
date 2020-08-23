use crate::core::app::AppExtension;
use crate::core::context::{AppInitContext, WindowService};
use crate::core::types::{FontInstanceKey, FontName, FontProperties, FontSize, FontStyle};
use crate::core::var::ContextVar;
use crate::widgets::FontFamilyVar;

use fnv::FnvHashMap;
use std::{collections::HashMap, sync::Arc};
use webrender::api::units::Au;
use webrender::api::{FontKey, GlyphDimensions, RenderApi, Transaction};

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
type FontQueryKey = (Box<[FontName]>, FontPropertiesKey);

impl Fonts {
    /// Gets a cached font instance or loads a new instance.
    pub fn get(&mut self, font_names: &[FontName], properties: &FontProperties, font_size: FontSize) -> Option<FontInstance> {
        let query_key = (font_names.to_vec().into_boxed_slice(), FontPropertiesKey::new(*properties));
        if let Some(font) = self.fonts.get_mut(&query_key) {
            if let Some(instance) = font.instances.get(&font_size) {
                Some(instance.clone())
            } else {
                Some(Self::load_font_size(self.api.clone(), font, font_size))
            }
        } else if let Some(instance) = self.load_font(query_key, font_names, properties, font_size) {
            Some(instance)
        } else {
            None
        }
    }

    /// Gets a font using [`get`](Self::get) or fallback to the any of the default fonts.
    pub fn get_or_default(&mut self, font_names: &[FontName], properties: &FontProperties, font_size: FontSize) -> FontInstance {
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
        size: FontSize,
    ) -> Option<FontInstance> {
        let family_names: Vec<font_kit::family_name::FamilyName> = font_names.iter().map(|n| n.clone().into()).collect();
        match font_kit::source::SystemSource::new().select_best_match(&family_names, properties) {
            Ok(handle) => {
                let mut txn = Transaction::new();
                let font_key = self.api.generate_font_key();

                match handle {
                    font_kit::handle::Handle::Path { path, font_index } => {
                        txn.add_native_font(font_key, webrender::api::NativeFontHandle { path, index: font_index })
                    }
                    font_kit::handle::Handle::Memory { bytes, font_index } => txn.add_raw_font(font_key, (&*bytes).clone(), font_index),
                }

                let mut font_instances = FontInstances {
                    font_key,
                    instances: FnvHashMap::default(),
                };
                self.api.update_resources(txn.resource_updates);
                let instance = Self::load_font_size(self.api.clone(), &mut font_instances, size);
                self.fonts.insert(query_key, font_instances);
                Some(instance)
            }
            Err(font_kit::error::SelectionError::NotFound) => None,
            Err(font_kit::error::SelectionError::CannotAccessSource) => panic!("cannot access system font source"),
        }
    }

    fn load_font_size(api: Arc<RenderApi>, font_instances: &mut FontInstances, size: FontSize) -> FontInstance {
        let mut txn = Transaction::new();
        let instance_key = api.generate_font_instance_key();

        txn.add_font_instance(
            instance_key,
            font_instances.font_key,
            Au::from_px(size as i32),
            None,
            None,
            Vec::new(),
        );
        api.update_resources(txn.resource_updates);

        let instance = FontInstance::new(api, font_instances.font_key, instance_key);
        font_instances.instances.insert(size, instance.clone());

        instance
    }
}

impl WindowService for Fonts {}

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

/// All instances of a font family.
struct FontInstances {
    pub font_key: FontKey,
    pub instances: FnvHashMap<FontSize, FontInstance>,
}

#[derive(Clone)]
struct FontInstanceInner {
    api: Arc<RenderApi>,
    font_key: FontKey,
    instance_key: FontInstanceKey,
}

/// Reference to a specific font instance (family and size).
#[derive(Clone)]
pub struct FontInstance {
    inner: Arc<FontInstanceInner>,
}

impl FontInstance {
    fn new(api: Arc<RenderApi>, font_key: FontKey, instance_key: FontInstanceKey) -> Self {
        FontInstance {
            inner: Arc::new(FontInstanceInner {
                api,
                font_key,
                instance_key,
            }),
        }
    }

    /// Gets the glyphs and glyph dimensions required for drawing the given `text`.
    pub fn glyph_layout(&self, text: &str) -> (Vec<u32>, Vec<Option<GlyphDimensions>>) {
        let indices: Vec<_> = self
            .inner
            .api
            .get_glyph_indices(self.inner.font_key, text)
            .into_iter()
            .filter_map(|i| i)
            .collect();

        let dimensions = self.inner.api.get_glyph_dimensions(self.inner.instance_key, indices.clone());
        (indices, dimensions)
    }

    /// Gets the font instance key.
    pub fn instance_key(&self) -> FontInstanceKey {
        self.inner.instance_key
    }
}
