use std::{collections::HashMap, path::PathBuf, rc::Rc, sync::Arc};

use fnv::FnvHashMap;
use webrender::api::RenderApi;

use super::{FontName, FontNames, FontStretch, FontStyle, FontWeight, Script};
use crate::core::app::AppExtension;
use crate::core::context::{AppContext, AppInitContext, UpdateNotifier, UpdateRequest};
use crate::core::service::{AppService, WindowService};
use crate::core::var::{RcVar, Vars};

/// Application extension that manages text fonts.
/// # Services
///
/// Services this extension provides:
///
/// * [Fonts] - Service that finds and loads fonts.
/// * [FontCache] - Window service that caches fonts for the window renderer.
#[derive(Default)]
pub struct FontManager;
impl AppExtension for FontManager {
    fn init(&mut self, ctx: &mut AppInitContext) {
        ctx.services.register(Fonts::new(ctx.updates.notifier().clone()));
        ctx.window_services.register(move |ctx| FontCache::new(Arc::clone(ctx.render_api)));
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update {
            ctx.services.req::<Fonts>().update(ctx.vars);
        }
    }
}

/// Font loading, custom fonts and app font configuration.
pub struct Fonts {
    loader: FontLoader,
    fallbacks: FontFallbacks,
    live_queries: HashMap<FontQueryKey, RcVar<FontList>>,
}
impl AppService for Fonts {}
impl Fonts {
    fn new(notifier: UpdateNotifier) -> Self {
        Fonts {
            loader: FontLoader::new(),
            fallbacks: FontFallbacks::new(notifier),
            live_queries: HashMap::new(),
        }
    }

    /// Actual name of fallback fonts.
    #[inline]
    pub fn fallbacks(&self) -> &FontFallbacks {
        &self.fallbacks
    }

    /// Configure the actual name of fallback fonts.
    #[inline]
    pub fn fallbacks_mut(&mut self) -> &mut FontFallbacks {
        &mut self.fallbacks
    }

    /// Load and register a custom font.
    #[inline]
    pub fn register(&mut self, custom_font: CustomFont) -> Result<(), FontLoadingError> {
        self.request_update();
        self.loader.register(custom_font)
    }

    /// Gets a font list that best matches the query.
    #[inline]
    pub fn get(&self, families: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontList {
        self.loader.get(families, style, weight, stretch)
    }

    /// Gets a font list that best matches the query in a variable.
    /// The variable updates if new fonts registered changes the query result.
    pub fn get_var(&mut self, families: FontNames, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> RcVar<FontList> {
        let query = FontQueryKey::new(families, style, weight, stretch);
        match self.live_queries.entry(query) {
            std::collections::hash_map::Entry::Occupied(e) => e.get().clone(),
            std::collections::hash_map::Entry::Vacant(e) => {
                let result = RcVar::new(self.loader.get(&e.key().0, style, weight, stretch));
                e.insert(result).clone()
            }
        }
    }

    fn request_update(&mut self) {
        self.fallbacks.request_update();
    }

    fn update(&mut self, vars: &Vars) {
        if self.fallbacks.pending_update {
            self.fallbacks.pending_update = false;

            // 1 - Retain only vars with more then one reference.
            // 2 - Rerun the query and update the var if the result changes.

            let loader = &self.loader;
            self.live_queries.retain(|query, var| {
                let retain = var.ptr_count() > 1;
                if retain {
                    let result = loader.get_query(query);
                    if &result != var.get(vars) {
                        var.set(vars, result);
                    }
                }
                retain
            });
        }
    }
}

pub use font_kit::error::FontLoadingError;

type HarfbuzzFace = harfbuzz_rs::Shared<harfbuzz_rs::Face<'static>>;

#[derive(Debug)]
pub struct FontFace {
    h_face: HarfbuzzFace,
}

/// A shared [`FontFace`].
pub type FontFaceRef = Rc<FontFace>;

/// A list of [`FontFaceRef`] resolved from a [`FontName`] list, plus the [fallback](FontFallbacks::fallback) font.
///
/// Glyphs that are not resolved by the first font fallback to the second font and so on.
#[derive(Debug, Clone)]
pub struct FontList(Vec<FontFaceRef>);
impl PartialEq for FontList {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.iter().zip(other.0.iter()).all(|(a, b)| Rc::ptr_eq(a, b))
    }
}
impl Eq for FontList {}

struct FontLoader {
    system: font_kit::source::SystemSource,
    custom: font_kit::sources::mem::MemSource,
}
impl FontLoader {
    fn new() -> Self {
        FontLoader {
            system: font_kit::source::SystemSource::new(),
            custom: font_kit::sources::mem::MemSource::from_fonts(std::iter::empty()).unwrap(),
        }
    }

    fn register(&mut self, custom_font: CustomFont) -> Result<(), FontLoadingError> {
        let font = match custom_font.source {
            FontSource::File(path, i) => font_kit::handle::Handle::Path { path, font_index: i }.load()?,
            FontSource::Memory(bytes, i) => font_kit::handle::Handle::Memory { bytes, font_index: i }.load()?,
            FontSource::Alias(name) => self
                .system
                .select_family_by_name(&name)
                .map(|f| f.fonts()[0].clone())
                .map_err(|_| FontLoadingError::NoSuchFontInCollection)?
                .load()?,
        };

        Ok(())
    }

    fn get(&self, families: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontList {
        todo!()
    }

    fn get_query(&self, query: &FontQueryKey) -> FontList {
        todo!()
    }
}
impl font_kit::source::Source for FontLoader {
    fn all_fonts(&self) -> Result<Vec<font_kit::handle::Handle>, font_kit::error::SelectionError> {
        let mut r = self.custom.all_fonts()?;
        r.extend(self.system.all_fonts()?);
        Ok(r)
    }

    fn all_families(&self) -> Result<Vec<String>, font_kit::error::SelectionError> {
        let mut r = self.custom.all_families()?;
        r.extend(self.system.all_families()?);
        Ok(r)
    }

    fn select_family_by_name(&self, family_name: &str) -> Result<font_kit::family_handle::FamilyHandle, font_kit::error::SelectionError> {
        match self.custom.select_family_by_name(family_name) {
            Ok(r) => return Ok(r),
            Err(e) => match e {
                font_kit::error::SelectionError::NotFound => {}
                font_kit::error::SelectionError::CannotAccessSource => return Err(e),
            },
        }
        self.system.select_family_by_name(family_name)
    }

    fn select_by_postscript_name(&self, postscript_name: &str) -> Result<font_kit::handle::Handle, font_kit::error::SelectionError> {
        match self.custom.select_by_postscript_name(postscript_name) {
            Ok(r) => return Ok(r),
            Err(e) => match e {
                font_kit::error::SelectionError::NotFound => {}
                font_kit::error::SelectionError::CannotAccessSource => return Err(e),
            },
        }
        self.system.select_by_postscript_name(postscript_name)
    }
}

/// Per-window font glyph cache.
pub struct FontCache {
    api: Arc<RenderApi>,
}
impl WindowService for FontCache {}
impl FontCache {
    fn new(api: Arc<RenderApi>) -> Self {
        FontCache { api }
    }

    /// Gets a font list with the cached renderer data for each font.
    pub fn get(&mut self, font_list: &FontList, font_size: f32) -> RenderFontList {
        todo!()
    }
}

pub struct RenderFontList;

/// Fallback fonts configuration for the app.
///
/// This type can be accessed from the [`Fonts`] service.
///
/// # Fallback Fonts
///
/// TODO
pub struct FontFallbacks {
    serif: FnvHashMap<Script, FontName>,
    sans_serif: FnvHashMap<Script, FontName>,
    monospace: FnvHashMap<Script, FontName>,
    cursive: FnvHashMap<Script, FontName>,
    fantasy: FnvHashMap<Script, FontName>,
    fallback: FnvHashMap<Script, FontName>,
    notifier: UpdateNotifier,
    pending_update: bool,
}
impl FontFallbacks {
    fn new(notifier: UpdateNotifier) -> Self {
        fn default(name: impl Into<FontName>) -> FnvHashMap<Script, FontName> {
            let mut f = FnvHashMap::with_capacity_and_hasher(1, fnv::FnvBuildHasher::default());
            f.insert(Script::Unknown, name.into());
            f
        }
        FontFallbacks {
            serif: default("Times New Roman"),
            sans_serif: default("Arial"),
            monospace: default("Courier New"),
            cursive: default("Comic Sans MS"),
            #[cfg(target_family = "windows")]
            fantasy: default("Impact"),
            #[cfg(not(target_family = "windows"))]
            fantasy: default("Papyrus"),
            fallback: default("Segoe UI Symbol"),
            notifier,
            pending_update: false,
        }
    }
}
macro_rules! impl_fallback_accessors {
    ($($name:ident),+ $(,)?) => {$(paste::paste! {
    #[doc = "Gets the fallback *" $name "* font for the given script."]
    ///
    /// Returns a font name and the [`Script`] it was registered with. The script
    /// can be the same as requested or [`Script::Unknown`].
    #[inline]
    pub fn $name(&self, script: Script) -> (&FontName, Script) {
        Self::get_fallback(&self.$name, script)
    }

    #[doc = "Sets the fallback *" $name "* font for the given script."]
    ///
    /// Returns the previous registered font for the script.
    ///
    /// Set [`Script::Unknown`] for all scripts that don't have a specific font association.
    pub fn [<set_ $name>]<F: Into<FontName>>(&mut self, script: Script, font_name: F) -> Option<FontName> {
        self.request_update();
        self.$name.insert(script, font_name.into())
    }
    })+};
}
impl FontFallbacks {
    fn get_fallback(map: &FnvHashMap<Script, FontName>, script: Script) -> (&FontName, Script) {
        map.get(&script)
            .map(|f| (f, script))
            .unwrap_or_else(|| (&map[&Script::Unknown], Script::Unknown))
    }

    impl_fallback_accessors! {
        serif, sans_serif, monospace, cursive, fantasy
    }

    /// Gets the ultimate fallback font used when none of the other fonts support a glyph.
    ///
    /// Returns a font name and the [`Script`] it was registered with. The script
    /// can be the same as requested or [`Script::Unknown`].
    #[inline]
    pub fn fallback(&self, script: Script) -> (&FontName, Script) {
        Self::get_fallback(&self.fallback, script)
    }

    /// Sets the ultimate fallback font used when none of other fonts support a glyph.
    ///
    /// This should be a font that cover as many glyphs as possible.
    ///
    /// Returns the previous registered font for the script.
    ///
    /// Set [`Script::Unknown`] for all scripts that don't have a specific font association.
    pub fn set_fallback<F: Into<FontName>>(&mut self, script: Script, font_name: F) -> Option<FontName> {
        self.request_update();
        self.fallback.insert(script, font_name.into())
    }

    fn request_update(&mut self) {
        if !self.pending_update {
            self.pending_update = true;
            self.notifier.update();
        }
    }
}

/// Reference to in memory font data.
// We can't use Arc<[u8]> here because of compatibility with the font-kit crate.
#[allow(clippy::clippy::rc_buffer)]
pub type FontDataRef = Arc<Vec<u8>>;

#[derive(Debug, Clone)]
enum FontSource {
    File(PathBuf, u32),
    Memory(FontDataRef, u32),
    Alias(FontName),
}

/// Custom font builder.
///
/// A custom font has a name and a source,
#[derive(Debug, Clone)]
pub struct CustomFont {
    name: FontName,
    source: FontSource,
    stretch: FontStretch,
    style: FontStyle,
    weight: FontWeight,
}
impl CustomFont {
    /// A custom font loaded from a file.
    ///
    /// If the file is a collection of fonts, `font_index` determines which, otherwise just pass `0`.
    ///
    /// The font is loaded in [`AppFonts::register`].
    pub fn from_file<N: Into<FontName>, P: Into<PathBuf>>(name: N, path: P, font_index: u32) -> Self {
        CustomFont {
            name: name.into(),
            source: FontSource::File(path.into(), font_index),
            stretch: FontStretch::NORMAL,
            style: FontStyle::Normal,
            weight: FontWeight::NORMAL,
        }
    }

    /// A custom font loaded from a shared byte slice.
    ///
    /// If the font data is a collection of fonts, `font_index` determines which, otherwise just pass `0`.
    ///
    /// The font is loaded in [`AppFonts::register`].
    pub fn from_bytes<N: Into<FontName>>(name: N, data: FontDataRef, font_index: u32) -> Self {
        CustomFont {
            name: name.into(),
            source: FontSource::Memory(data, font_index),
            stretch: FontStretch::NORMAL,
            style: FontStyle::Normal,
            weight: FontWeight::NORMAL,
        }
    }

    /// A custom font that maps to another font.
    ///
    /// The font is loaded in [`AppFonts::register`].
    pub fn from_other<N: Into<FontName>, O: Into<FontName>>(name: N, other_font: O) -> Self {
        CustomFont {
            name: name.into(),
            source: FontSource::Alias(other_font.into()),
            stretch: FontStretch::NORMAL,
            style: FontStyle::Normal,
            weight: FontWeight::NORMAL,
        }
    }

    /// Set the [`FontStretch`].
    ///
    /// Default is [`FontStretch::NORMAL`].
    #[inline]
    pub fn stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// Set the [`FontStyle`].
    ///
    /// Default is [`FontStyle::Normal`].
    #[inline]
    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the [`FontWeight`].
    ///
    /// Default is [`FontWeight::NORMAL`].
    #[inline]
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }
}

#[derive(Eq, PartialEq, Hash)]
struct FontQueryKey(FontNames, u8, u32, u32);
impl FontQueryKey {
    #[inline]
    fn new(names: FontNames, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Self {
        FontQueryKey(names, style as u8, (weight.0 * 100.0) as u32, (stretch.0 * 100.0) as u32)
    }
}
