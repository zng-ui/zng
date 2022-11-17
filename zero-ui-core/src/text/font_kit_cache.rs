// * font_kit::font::Font is !Send, see issue: https://github.com/servo/font-kit/issues/108
// * loading for every use is not feasible, even from in memory font data.

use std::{cell::RefCell, rc::Rc, sync::Arc, thread::ThreadId};

use linear_map::LinearMap;

thread_local! {
    static FONT_KIT_CACHE: RefCell<Vec<FontKitThreadLocalEntry>>  = RefCell::new(vec![]);
}
struct FontKitThreadLocalEntry {
    key: Arc<usize>,
    font: Option<Rc<font_kit::font::Font>>,
}
impl FontKitThreadLocalEntry {
    pub fn cleanup(&mut self) {
        if self.font.is_some() && Arc::strong_count(&self.key) == 1 {
            self.font = None;
        }
    }
}

fn cleanup_current_font_kit_cache() {
    FONT_KIT_CACHE.with(|c| {
        let mut c = c.borrow_mut();

        let mut clean = 0;
        for e in c.iter_mut() {
            e.cleanup();
            if e.font.is_none() {
                clean += 1;
            }
        }
        if clean == c.len() {
            c.clear();
        }
    })
}

#[derive(Default, Clone)]
pub struct FontKitCache {
    threads: LinearMap<ThreadId, Arc<usize>>,
}
impl FontKitCache {
    pub fn get_or_init(&mut self, init: impl FnOnce() -> font_kit::font::Font) -> Rc<font_kit::font::Font> {
        match self.threads.entry(std::thread::current().id()) {
            linear_map::Entry::Occupied(e) => {
                let i = **e.get();
                FONT_KIT_CACHE.with(|c| c.borrow()[i].font.clone().unwrap())
            }
            linear_map::Entry::Vacant(e) => {
                cleanup_current_font_kit_cache();
                FONT_KIT_CACHE.with(|c| {
                    let mut c = c.borrow_mut();
                    let key = Arc::new(c.len());
                    let font = Rc::new(init());
                    c.push(FontKitThreadLocalEntry {
                        key: key.clone(),
                        font: Some(font.clone()),
                    });
                    e.insert(key);
                    font
                })
            }
        }
    }
}
impl Drop for FontKitCache {
    fn drop(&mut self) {
        self.threads.clear();
        cleanup_current_font_kit_cache();
    }
}
