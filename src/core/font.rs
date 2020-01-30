use crate::core::app::AppExtension;
use crate::core::context::{AppInitContext, Service};
use crate::core::types::FontInstanceKey;

use fnv::FnvHashMap;
use font_loader::system_fonts;
use std::sync::Arc;
use webrender::api::units::Au;
use webrender::api::{FontKey, GlyphDimensions, RenderApi, Transaction};

#[derive(Default)]
pub struct FontCache;

impl AppExtension for FontCache {
    fn init(&mut self, r: &mut AppInitContext) {
        r.services.register_wnd(|ctx| Fonts {
            api: Arc::clone(ctx.render_api),
            fonts: FnvHashMap::default(),
        })
    }
}

/// Fonts service.
pub struct Fonts {
    api: Arc<RenderApi>,
    fonts: FnvHashMap<String, FontInstances>,
}

impl Fonts {
    /// Gets a cached font instance or loads a new instance.
    pub fn get(&mut self, font_family: &str, font_size: u32) -> FontInstance {
        if let Some(font) = self.fonts.get_mut(font_family) {
            if let Some(font_instance) = font.instances.get(&font_size) {
                font_instance.clone()
            } else {
                Self::load_font_size(self.api.clone(), font, font_size)
            }
        } else {
            self.load_font(font_family, font_size)
        }
    }

    fn load_font(&mut self, family: &str, size: u32) -> FontInstance {
        let mut txn = Transaction::new();
        let property = system_fonts::FontPropertyBuilder::new().family(&family).build();
        let (font, _) = system_fonts::get(&property).unwrap();

        let font_key = self.api.generate_font_key();

        txn.add_raw_font(font_key, font, 0);

        let mut font_instances = FontInstances {
            font_key,
            instances: FnvHashMap::default(),
        };
        self.api.update_resources(txn.resource_updates);
        let instance = Self::load_font_size(self.api.clone(), &mut font_instances, size);

        self.fonts.insert(family.to_owned(), font_instances);

        instance
    }

    fn load_font_size(api: Arc<RenderApi>, font_instances: &mut FontInstances, size: u32) -> FontInstance {
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

impl Service for Fonts {}

/// All instances of a font family.
struct FontInstances {
    pub font_key: FontKey,
    pub instances: FnvHashMap<u32, FontInstance>,
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

        let dimensions = self
            .inner
            .api
            .get_glyph_dimensions(self.inner.instance_key, indices.clone());
        (indices, dimensions)
    }

    /// Gets the font instance key.
    pub fn instance_key(&self) -> FontInstanceKey {
        self.inner.instance_key
    }
}
