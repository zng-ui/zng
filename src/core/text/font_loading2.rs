use std::{collections::HashMap, path::PathBuf, rc::Rc, sync::Arc};

use fnv::FnvHashMap;
use webrender::api::RenderApi;

use super::{FontName, FontNames, FontStretch, FontStyle, FontWeight, Script};
use crate::core::app::AppExtension;
use crate::core::context::{AppContext, AppInitContext, UpdateNotifier, UpdateRequest};
use crate::core::service::{AppService, WindowService};
use crate::core::units::{layout_to_pt, LayoutLength};
use crate::core::var::{RcVar, Vars};
use crate::core::window::WindowId;

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
    display_name: FontName,
    family_name: FontName,
    postscript_name: Option<String>,
    style: FontStyle,
    weight: FontWeight,
    stretch: FontStretch,
}
impl FontFace {
    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &harfbuzz_rs::Shared<harfbuzz_rs::Face> {
        &self.h_face
    }

    /// Font full name.
    #[inline]
    pub fn display_name(&self) -> &FontName {
        &self.display_name
    }

    /// Font family name.
    #[inline]
    pub fn family_name(&self) -> &FontName {
        &self.family_name
    }

    /// Font globally unique name.
    #[inline]
    pub fn postscript_name(&self) -> Option<&str> {
        self.postscript_name.as_deref()
    }

    /// Index of the font in the font file.
    #[inline]
    pub fn index(&self) -> u32 {
        self.h_face.index()
    }

    /// Number of glyphs in the font.
    #[inline]
    pub fn glyph_count(&self) -> u32 {
        self.h_face.glyph_count()
    }

    /// Font style.
    #[inline]
    pub fn style(&self) -> FontStyle {
        self.style
    }

    /// Font weight.
    #[inline]
    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    /// Font stretch.
    #[inline]
    pub fn stretch(&self) -> FontStretch {
        self.stretch
    }
}

/// A shared [`FontFace`].
pub type FontFaceRef = Rc<FontFace>;

/// A list of [`FontFaceRef`] resolved from a [`FontName`] list, plus the [fallback](FontFallbacks::fallback) font.
///
/// Glyphs that are not resolved by the first font fallback to the second font and so on.
#[derive(Debug, Clone)]
pub struct FontList {
    fonts: Box<[FontFaceRef]>,
}
#[allow(clippy::len_without_is_empty)] // is never empty.
impl FontList {
    /// The font face that best matches the requested properties.
    #[inline]
    pub fn best(&self) -> &FontFaceRef {
        &self.fonts[0]
    }

    /// Iterate over font faces, more specific first.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<FontFaceRef> {
        self.fonts.iter()
    }

    /// Number of font faces in the list.
    ///
    /// This is at least `1`.
    #[inline]
    pub fn len(&self) -> usize {
        self.fonts.len()
    }
}
impl PartialEq for FontList {
    /// Both are equal if each point to the same fonts in the same order.
    fn eq(&self, other: &Self) -> bool {
        self.fonts.len() == other.fonts.len() && self.fonts.iter().zip(other.fonts.iter()).all(|(a, b)| Rc::ptr_eq(a, b))
    }
}
impl Eq for FontList {}
impl std::ops::Deref for FontList {
    type Target = [FontFaceRef];

    fn deref(&self) -> &Self::Target {
        &self.fonts
    }
}
impl<'a> std::iter::IntoIterator for &'a FontList {
    type Item = &'a FontFaceRef;

    type IntoIter = std::slice::Iter<'a, FontFaceRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl std::ops::Index<usize> for FontList {
    type Output = FontFaceRef;

    fn index(&self, index: usize) -> &Self::Output {
        &self.fonts[index]
    }
}

struct FontLoader {}
impl FontLoader {
    fn new() -> Self {
        FontLoader {}
    }

    fn register(&mut self, custom_font: CustomFont) -> Result<(), FontLoadingError> {
        todo!()
    }

    fn get(&self, families: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontList {
        todo!()
    }

    fn get_query(&self, query: &FontQueryKey) -> FontList {
        todo!()
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
    pub fn get(&mut self, font_list: &FontList, font_size: LayoutLength) -> RenderFontList {
        todo!()
    }
}

type HarfbuzzFont = harfbuzz_rs::Shared<harfbuzz_rs::Font<'static>>;

/// A [`FontFace`] + font size cached in a window renderer.
pub struct RenderFont {
    face: FontFaceRef,
    h_font: HarfbuzzFont,
    window_id: WindowId,
    instance_key: super::FontInstanceKey,
    size: LayoutLength,
    synthesis_used: super::FontSynthesis,
    metrics: super::FontMetrics,
}
impl RenderFont {
    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &harfbuzz_rs::Shared<harfbuzz_rs::Font<'static>> {
        &self.h_font
    }

    /// Reference the font face from which this font was created.
    #[inline]
    pub fn face(&self) -> &FontFaceRef {
        &self.face
    }

    /// Owner window id.
    ///
    /// Render fonts can only be used in the same window that created then.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Gets the font instance key.
    ///
    /// # Careful
    ///
    /// The WebRender font instance resource is managed by this struct, don't manually request a delete with this key.
    ///
    /// Keep a clone of the font reference alive for the period you want to render using this font,
    /// otherwise the font may be cleaned-up.
    #[inline]
    pub fn instance_key(&self) -> super::FontInstanceKey {
        self.instance_key
    }

    /// Font size.
    #[inline]
    pub fn size(&self) -> LayoutLength {
        self.size
    }

    /// Font size in point units.
    #[inline]
    pub fn size_pt(&self) -> f32 {
        layout_to_pt(self.size())
    }

    /// Various metrics that apply to this font.
    #[inline]
    pub fn metrics(&self) -> &super::FontMetrics {
        &self.metrics
    }

    /// What synthetic properties are used in this instance.
    #[inline]
    pub fn synthesis_used(&self) -> super::FontSynthesis {
        self.synthesis_used
    }
}

/// A shared [`RenderFont`].
pub type RenderFontRef = Rc<RenderFont>;

pub struct RenderFontList(Vec<RenderFont>);

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
