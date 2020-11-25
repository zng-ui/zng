use std::{cell::RefCell, collections::HashMap, path::PathBuf, rc::Rc, sync::Arc};

use fnv::FnvHashMap;
use webrender::api::RenderApi;

use super::{FontMetrics, FontName, FontNames, FontStretch, FontStyle, FontWeight, Script};
use crate::core::app::AppExtension;
use crate::core::context::{AppContext, AppInitContext, UpdateNotifier, UpdateRequest};
use crate::core::event::{event, event_args, EventEmitter};
use crate::core::service::{AppService, WindowService};
use crate::core::units::{layout_to_pt, LayoutLength};
use crate::core::window::WindowId;

event! {
    /// Change in [`Fonts`] that may cause a font query to now give
    /// a different result.
    pub FontChangedEvent: FontChangedArgs;
}

event_args! {
    /// [`FontChangedEvent`] arguments.
    pub struct FontChangedArgs {

        /// The change that happened.
        pub change: FontChange,

        ..

        /// Concerns all widgets.
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            true
        }
    }
}

/// Possible changes in a [`FontChangedArgs`].
#[derive(Clone, Debug)]
pub enum FontChange {
    /// OS fonts change.
    SystemFonts,

    /// Custom fonts change caused by call to [`Fonts::register`] or [`Fonts::unregister`].
    CustomFonts,

    /// Custom request caused by call to [`Fonts::notify_refresh`].
    Refesh,

    /// One of the named [`FontFallbacks`] was set for the script.
    ///
    /// The font name is one of [`FontName`] fallback names.
    NamedFallback(FontName, Script),

    /// A new [fallback](FontFallbacks::fallback) font was set for the script.
    Fallback(Script),
}

/// Application extension that manages text fonts.
/// # Services
///
/// Services this extension provides:
///
/// * [Fonts] - Service that finds and loads fonts.
/// * [FontRenderCache] - Window service that caches fonts for the window renderer.
///
/// Events this extension provides:
///
/// * [FontChangedEvent] - Font fallbacks or system fonts changed.
pub struct FontManager {
    font_changed: EventEmitter<FontChangedArgs>,
}
impl Default for FontManager {
    fn default() -> Self {
        FontManager {
            font_changed: FontChangedEvent::emitter(),
        }
    }
}
impl AppExtension for FontManager {
    fn init(&mut self, ctx: &mut AppInitContext) {
        ctx.events.register::<FontChangedEvent>(self.font_changed.listener());
        ctx.services.register(Fonts::new(ctx.updates.notifier().clone()));
        ctx.window_services
            .register(move |ctx| FontRenderCache::new(Arc::clone(ctx.render_api), ctx.window_id.get()));
    }

    #[cfg(windows)]
    fn on_window_event(&mut self, _window_id: WindowId, _event: &crate::core::types::WindowEvent, _ctx: &mut AppContext) {
        // TODO use Windows sub-classing to listen to WM_FONTCHANGE
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update {
            for args in ctx.services.req::<Fonts>().take_updates() {
                self.font_changed.notify(ctx.events, args);
            }
        }
    }
}

/// Font loading, custom fonts and app font configuration.
pub struct Fonts {
    loader: FontFaceLoader,
    fallbacks: FontFallbacks,
}
impl AppService for Fonts {}
impl Fonts {
    fn new(notifier: UpdateNotifier) -> Self {
        Fonts {
            loader: FontFaceLoader::new(),
            fallbacks: FontFallbacks::new(notifier),
        }
    }

    #[inline]
    fn take_updates(&mut self) -> Vec<FontChangedArgs> {
        std::mem::take(&mut self.fallbacks.updates)
    }

    /// Raises [`FontChangedEvent`].
    #[inline]
    pub fn notify_refresh(&mut self) {
        self.fallbacks.notify(FontChange::Refesh);
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
    ///
    /// If the font loads correctly a [`FontChangedEvent`] notification is scheduled.
    /// Fonts sourced from a file are not monitored for changes, you can *reload* the font
    /// by calling `register` again with the same font name.
    #[inline]
    pub fn register(&mut self, custom_font: CustomFont) -> Result<(), FontLoadingError> {
        self.loader.register(custom_font)?;
        self.fallbacks.notify(FontChange::CustomFonts);
        Ok(())
    }

    /// Removes a custom font. If the font is not in use it is also unloaded.
    ///
    /// Returns if any font was removed.
    #[inline]
    pub fn unregister(&mut self, custom_font: &FontName) -> bool {
        let unregistered = self.loader.unregister(custom_font);
        if unregistered {
            self.fallbacks.notify(FontChange::CustomFonts);
        }
        unregistered
    }

    /// Gets a font list that best matches the query.
    #[inline]
    pub fn get(&self, families: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontList {
        self.loader.get(families, style, weight, stretch)
    }
}

pub use font_kit::error::FontLoadingError;

type HarfbuzzFace = harfbuzz_rs::Shared<harfbuzz_rs::Face<'static>>;
type HarfbuzzFont = harfbuzz_rs::Shared<harfbuzz_rs::Font<'static>>;
type FontKitFont = font_kit::font::Font;

/// A font face selected from a font family.
///
/// Usually this is part of a [`FontList`] that can be requested from
/// the [`Fonts`] service.
#[derive(Debug)]
pub struct FontFace {
    h_face: HarfbuzzFace,
    kit_font: FontKitFont,
    display_name: FontName,
    family_name: FontName,
    postscript_name: Option<String>,
    style: FontStyle,
    weight: FontWeight,
    stretch: FontStretch,
    instances: RefCell<FnvHashMap<u32, FontRef>>,
}
impl FontFace {
    fn load_custom(custom_font: CustomFont, loader: &FontFaceLoader) -> Result<Self, FontLoadingError> {
        let (kit_font, h_face) = match custom_font.source {
            FontSource::File(path, font_index) => (
                font_kit::handle::Handle::Path {
                    path: path.clone(),
                    font_index,
                }
                .load()?,
                harfbuzz_rs::Face::from_file(path, font_index).map_err(FontLoadingError::Io)?,
            ),
            FontSource::Memory(bytes, font_index) => (
                font_kit::handle::Handle::Memory {
                    bytes: Arc::clone(&bytes),
                    font_index,
                }
                .load()?,
                harfbuzz_rs::Face::new(harfbuzz_rs::Blob::with_bytes_owned(bytes, |b| &b[..]), font_index),
            ),
            FontSource::Alias(other_font) => {
                let other_font = loader.get_exact(&other_font).ok_or(FontLoadingError::NoSuchFontInCollection)?;
                return Ok(FontFace {
                    h_face: other_font.h_face.clone(),
                    kit_font: other_font.kit_font.clone(),
                    display_name: custom_font.name.clone(),
                    family_name: custom_font.name,
                    postscript_name: None,
                    style: custom_font.style,
                    weight: custom_font.weight,
                    stretch: custom_font.stretch,
                    instances: Default::default(),
                });
            }
        };

        // loaded external
        Ok(FontFace {
            h_face: h_face.to_shared(),
            kit_font,
            display_name: custom_font.name.clone(),
            family_name: custom_font.name,
            postscript_name: None,
            style: custom_font.style,
            weight: custom_font.weight,
            stretch: custom_font.stretch,
            instances: Default::default(),
        })
    }

    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font face handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &HarfbuzzFace {
        &self.h_face
    }

    /// Reference the underlying [`font-kit`](font_kit) font handle.
    #[inline]
    pub fn font_kit_handle(&self) -> &FontKitFont {
        &self.kit_font
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

    /// Index of the font face in the font file.
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

    /// Gets a cached sized [`Font`].
    pub fn sized(self: &Rc<Self>, font_size: LayoutLength) -> FontRef {
        let font_size = font_size.get() as u32;
        let mut instances = self.instances.borrow_mut();
        let f = instances
            .entry(font_size)
            .or_insert_with(|| Rc::new(Font::new(Rc::clone(self), LayoutLength::new(font_size as f32))));
        Rc::clone(f)
    }
}

/// A shared [`FontFace`].
pub type FontFaceRef = Rc<FontFace>;

const HARFBUZZ_FONT_SCALE: i32 = 64;

/// A sized font face.
///
/// A sized font can be requested from a [`FontFace`].
#[derive(Debug)]
pub struct Font {
    face: FontFaceRef,
    h_font: HarfbuzzFont,
    size: LayoutLength,
    metrics: FontMetrics,
}
impl Font {
    fn new(face: FontFaceRef, size: LayoutLength) -> Self {
        let mut h_font = harfbuzz_rs::Font::new(HarfbuzzFace::clone(&face.h_face));
        let metrics = FontMetrics::new(size.get(), &face.kit_font.metrics());

        let font_size_pt = layout_to_pt(size) as u32;
        h_font.set_ppem(font_size_pt, font_size_pt);
        h_font.set_scale(font_size_pt as i32 * HARFBUZZ_FONT_SCALE, font_size_pt as i32 * HARFBUZZ_FONT_SCALE);

        Font {
            face,
            h_font: h_font.into(),
            size,
            metrics,
        }
    }

    /// Reference the font face source of this font.
    #[inline]
    pub fn face(&self) -> &FontFaceRef {
        &self.face
    }

    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &HarfbuzzFont {
        &self.h_font
    }

    /// Font size.
    #[inline]
    pub fn size(&self) -> LayoutLength {
        self.size
    }

    /// Font size in point units.
    #[inline]
    pub fn size_pt(&self) -> f32 {
        layout_to_pt(self.size)
    }

    /// Sized font metrics.
    #[inline]
    pub fn metrics(&self) -> &FontMetrics {
        &self.metrics
    }
}

/// A shared [`Font`].
pub type FontRef = Rc<Font>;

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

struct FontFaceLoader {
    custom_fonts: HashMap<FontName, FontFaceRef>,
}
impl FontFaceLoader {
    fn new() -> Self {
        FontFaceLoader {
            custom_fonts: HashMap::new(),
        }
    }

    fn register(&mut self, custom_font: CustomFont) -> Result<(), FontLoadingError> {
        let name = custom_font.name.clone();
        let face = FontFace::load_custom(custom_font, self)?;
        self.custom_fonts.insert(name, Rc::new(face));
        Ok(())
    }

    fn unregister(&mut self, custom_font: &FontName) -> bool {
        self.custom_fonts.remove(custom_font).is_some()
    }

    fn get(&self, families: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontList {
        todo!()
    }

    fn get_exact(&self, font_name: &FontName) -> Option<&FontFaceRef> {
        self.custom_fonts.get(font_name)
    }
}

/// Per-window font glyph cache.
pub struct FontRenderCache {
    api: Arc<RenderApi>,
    window_id: WindowId,
    fonts: FnvHashMap<*const FontFace, webrender::api::FontKey>,
    instances: FnvHashMap<*const Font, super::FontInstanceKey>,
}

impl WindowService for FontRenderCache {}
impl FontRenderCache {
    fn new(api: Arc<RenderApi>, window_id: WindowId) -> Self {
        FontRenderCache {
            api,
            window_id,
            fonts: FnvHashMap::default(),
            instances: FnvHashMap::default(),
        }
    }

    /// Gets a font list with the cached renderer data for each font.
    #[inline]
    pub fn get(&mut self, font_list: &FontList, font_size: LayoutLength) -> RenderFontList {
        todo!()
    }

    /// Gets a [`RenderFont`] cached in the window renderer.
    pub fn render_font(&mut self, font: FontRef) -> RenderFont {
        let instance_key = *self.instances.entry(Rc::as_ptr(&font)).or_insert_with(|| todo!());

        RenderFont {
            font,
            window_id: self.window_id,
            instance_key,
            synthesis_used: super::FontSynthesis::DISABLED, //TODO
        }
    }
}

/// A [`Font`] cached in a window renderer.
#[derive(Debug, Clone)]
pub struct RenderFont {
    font: FontRef,
    window_id: WindowId,
    instance_key: super::FontInstanceKey,
    synthesis_used: super::FontSynthesis,
}
impl RenderFont {
    /// Reference the font.
    #[inline]
    pub fn font(&self) -> &FontRef {
        &self.font
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

    /// What synthetic properties are used in this instance.
    #[inline]
    pub fn synthesis_used(&self) -> super::FontSynthesis {
        self.synthesis_used
    }
}

/// A shared [`RenderFont`].
pub type RenderFontRef = Rc<RenderFont>;

pub struct RenderFontList(Vec<RenderFontRef>);

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
    updates: Vec<FontChangedArgs>,
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
            updates: vec![],
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
        self.notify(FontChange::NamedFallback(FontName::$name(), script));
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
        self.notify(FontChange::Fallback(script));
        self.fallback.insert(script, font_name.into())
    }

    fn notify(&mut self, change: FontChange) {
        if self.updates.is_empty() {
            self.notifier.update();
        }
        self.updates.push(FontChangedArgs::now(change));
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
