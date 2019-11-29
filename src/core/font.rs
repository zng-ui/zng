use super::{FontInstanceKey, FontKey};
use app_units::Au;
use fnv::FnvHashMap;
use font_loader::system_fonts;
use std::sync::{Arc, Mutex};
use webrender::api::{DocumentId, RenderApi, Transaction};

pub(crate) struct FontInstances {
    pub font_key: FontKey,
    pub instances: FnvHashMap<u32, FontInstanceRef>,
}

#[derive(Clone)]
pub struct FontInstance {
    pub font_key: FontKey,
    pub instance_key: FontInstanceKey,
}

/// Reference to a font instance.
#[derive(Clone)]
pub struct FontInstanceRef {
    inner: Arc<Mutex<Option<FontInstance>>>,
}

impl FontInstanceRef {
    fn new() -> Self {
        FontInstanceRef {
            inner: Default::default(),
        }
    }

    fn load(&self, font_key: FontKey, instance_key: FontInstanceKey) {
        self.inner
            .lock()
            .unwrap()
            .replace(FontInstance { font_key, instance_key });
    }

    /// Gets the font instance key if the font is loaded.
    pub fn instance_key(&self) -> Option<FontInstanceKey> {
        self.inner.lock().unwrap().as_ref().map(|i| i.instance_key)
    }
}

#[derive(Default)]
pub(crate) struct FontCache {
    fonts: FnvHashMap<String, FontInstances>,
    new_font_requests: FnvHashMap<(String, u32), FontInstanceRef>,
    new_size_requests: FnvHashMap<(FontKey, u32), FontInstanceRef>,
}

impl FontCache {
    pub fn get(&mut self, font_family: &str, font_size: u32) -> FontInstanceRef {
        if let Some(font) = self.fonts.get(font_family) {
            if let Some(font_instance) = font.instances.get(&font_size) {
                font_instance.clone()
            } else if let Some(font_instance) = self.new_size_requests.get(&(font.font_key, font_size)) {
                font_instance.clone()
            } else {
                let new_instance = FontInstanceRef::new();
                self.new_size_requests
                    .insert((font.font_key, font_size), new_instance.clone());
                new_instance
            }
        } else {
            let font = (font_family.to_owned(), font_size);
            if let Some(font_instance) = self.new_font_requests.get(&font) {
                font_instance.clone()
            } else {
                let new_instance = FontInstanceRef::new();
                self.new_font_requests.insert(font, new_instance.clone());
                new_instance
            }
        }
    }

    pub fn has_load_requests(&self) -> bool {
        !self.new_font_requests.is_empty() || !self.new_size_requests.is_empty()
    }

    pub fn load_fonts(&mut self, api: &RenderApi, document_id: DocumentId) {
        let mut txn = Transaction::new();

        for ((family, size), instance) in self.new_font_requests.drain() {
            if let Some(font) = self.fonts.get(&family) {
                self.new_size_requests.insert((font.font_key, size), instance);
            } else {
                let property = system_fonts::FontPropertyBuilder::new().family(&family).build();
                let (font, _) = system_fonts::get(&property).unwrap();

                let font_key = api.generate_font_key();

                txn.add_raw_font(font_key, font, 0);

                self.fonts.insert(
                    family.to_owned(),
                    FontInstances {
                        font_key,
                        instances: FnvHashMap::default(),
                    },
                );

                let instance_key = api.generate_font_instance_key();

                txn.add_font_instance(instance_key, font_key, Au::from_px(size as i32), None, None, Vec::new());

                instance.load(font_key, instance_key);
            }
        }

        for ((font_key, size), instance) in self.new_size_requests.drain() {
            let instance_key = api.generate_font_instance_key();

            txn.add_font_instance(instance_key, font_key, Au::from_px(size as i32), None, None, Vec::new());

            instance.load(font_key, instance_key);
        }

        if !txn.is_empty() {
            api.send_transaction(document_id, txn);
        }
    }
}
