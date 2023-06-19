use std::{
    collections::{HashMap, HashSet},
    fmt,
    path::PathBuf,
    rc::Rc,
    slice::SliceIndex,
    sync::Arc,
};

use font_kit::properties::Weight;
use parking_lot::{Mutex, RwLock};

use super::{
    font_features::RFontVariations, font_kit_cache::FontKitCache, lang, FontFaceMetrics, FontMetrics, FontName, FontStretch, FontStyle,
    FontSynthesis, FontWeight, Lang, ShapedSegmentData, WordCacheKey,
};
use crate::{
    app::{
        raw_events::{RAW_FONT_AA_CHANGED_EVENT, RAW_FONT_CHANGED_EVENT},
        view_process::{ViewProcessOffline, ViewRenderer, VIEW_PROCESS_INITED_EVENT},
        AppExtension,
    },
    app_local,
    context::UPDATES,
    crate_util::FxHashMap,
    event::{event, event_args, EventUpdate},
    l10n::LangMap,
    task,
    units::*,
    var::{response_done_var, var, AnyVar, ArcEq, ArcVar, ResponseVar, Var},
};

event! {
    /// Change in [`FONTS`] that may cause a font query to now give
    /// a different result.
    ///
    /// # Cache
    ///
    /// Every time this event updates the font cache is cleared. Meaning that even
    /// if the query returns the same font it will be a new reference.
    ///
    /// Fonts only unload when all references to then are dropped, so you can still continue using
    /// old references if you don't want to monitor this event.
    pub static FONT_CHANGED_EVENT: FontChangedArgs;
}

pub use zero_ui_view_api::FontAntiAliasing;

event_args! {
    /// [`FONT_CHANGED_EVENT`] arguments.
    pub struct FontChangedArgs {
        /// The change that happened.
        pub change: FontChange,

        ..

        /// Broadcast to all widgets.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            list.search_all()
        }
    }
}

/// Possible changes in a [`FontChangedArgs`].
#[derive(Clone, Debug)]
pub enum FontChange {
    /// OS fonts change.
    ///
    /// Currently this is only supported in Microsoft Windows.
    SystemFonts,

    /// Custom fonts change caused by call to [`FONTS.register`] or [`FONTS.unregister`].
    CustomFonts,

    /// Custom request caused by call to [`FONTS.refresh`].
    Refesh,

    /// One of the [`GenericFonts`] was set for the language.
    ///
    /// The font name is one of [`FontName`] generic names.
    GenericFont(FontName, Lang),

    /// A new [fallback](GenericFonts::fallback) font was set for the language.
    Fallback(Lang),
}

/// Application extension that manages text fonts.
/// # Services
///
/// Services this extension provides:
///
/// * [`FONTS`] - Service that finds and loads fonts.
///
/// Events this extension provides:
///
/// * [FONT_CHANGED_EVENT] - Font config or system fonts changed.
///
/// # Default
///
/// This extension is included in the [default app].
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct FontManager {}
impl AppExtension for FontManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if RAW_FONT_CHANGED_EVENT.has(update) {
            FONT_CHANGED_EVENT.notify(FontChangedArgs::now(FontChange::SystemFonts));
        } else if let Some(args) = RAW_FONT_AA_CHANGED_EVENT.on(update) {
            FONTS_SV.read().font_aa.set_ne(args.aa);
        } else if FONT_CHANGED_EVENT.has(update) {
            FONTS_SV.write().on_fonts_changed();
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            let mut fonts = FONTS_SV.write();
            fonts.font_aa.set_ne(args.font_aa);
            if args.is_respawn {
                fonts.loader.on_view_process_respawn();
            }
        }
    }

    fn update(&mut self) {
        let mut fonts = FONTS_SV.write();

        for args in fonts.take_updates() {
            FONT_CHANGED_EVENT.notify(args);
        }

        if fonts.prune_requested {
            fonts.on_prune();
        }
    }
}

app_local! {

    static FONTS_SV: FontsService = FontsService {
        loader: FontFaceLoader::new(),
        prune_requested: false,
        font_aa: var(FontAntiAliasing::Default),
    };
}

struct FontsService {
    loader: FontFaceLoader,
    prune_requested: bool,
    font_aa: ArcVar<FontAntiAliasing>,
}
impl FontsService {
    fn on_fonts_changed(&mut self) {
        self.loader.on_refresh();
        self.prune_requested = false;
    }

    fn on_prune(&mut self) {
        self.loader.on_prune();
        self.prune_requested = false;
    }

    fn take_updates(&mut self) -> Vec<FontChangedArgs> {
        std::mem::take(&mut GENERIC_FONTS_SV.write().updates)
    }
}

/// Font loading, custom fonts and app font configuration.
///
/// # Provider
///
/// This service is provided by the [`FontManager`] extension.
pub struct FONTS;
impl FONTS {
    /// Clear cache and notify `Refresh` in [`FONT_CHANGED_EVENT`].
    ///
    /// See the event documentation for more information.
    pub fn refresh(&self) {
        GENERIC_FONTS_SV.write().notify(FontChange::Refesh);
    }

    /// Remove all unused fonts from cache.
    pub fn prune(&self) {
        let mut ft = FONTS_SV.write();
        if !ft.prune_requested {
            ft.prune_requested = true;
            UPDATES.update(None);
        }
    }

    /// Actual name of generic fonts.
    pub fn generics(&self) -> &'static GenericFonts {
        &GenericFonts {}
    }

    /// Load and register a custom font.
    ///
    /// If the font loads correctly a [`FONT_CHANGED_EVENT`] notification is scheduled.
    /// Fonts sourced from a file are not monitored for changes, you can *reload* the font
    /// by calling `register` again with the same font name.
    ///
    /// The returned response is already set if the font is [`CustomFont::from_bytes`] or [`CustomFont::from_other`] and the other
    /// is already loaded, otherwise the returned response will update once when the font finishes loading.
    pub fn register(&self, custom_font: CustomFont) -> ResponseVar<Result<(), ArcEq<FontLoadingError>>> {
        task::poll_respond(async move {
            FontFaceLoader::register(custom_font).await.map_err(ArcEq::new)?;
            GENERIC_FONTS_SV.write().notify(FontChange::CustomFonts);
            Ok(())
        })
    }

    /// Removes a custom font family. If the font faces are not in use it is also unloaded.
    ///
    /// Returns if any was removed.
    pub fn unregister(&self, custom_family: &FontName) -> bool {
        let mut ft = FONTS_SV.write();
        let unregistered = ft.loader.unregister(custom_family);
        if unregistered {
            GENERIC_FONTS_SV.write().notify(FontChange::CustomFonts);
        }
        unregistered
    }

    /// Gets a font list that best matches the query.
    pub fn list(
        &self,
        families: &[FontName],
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        lang: &Lang,
    ) -> ResponseVar<FontFaceList> {
        // try with shared lock
        if let Some(cached) = FONTS_SV.read().loader.try_list(families, style, weight, stretch, lang) {
            return cached;
        }
        // begin load with exclusive lock (cache is tried again in `load`)
        FONTS_SV.write().loader.load_list(families, style, weight, stretch, lang)
    }

    /// Find a single font face that best matches the query.
    pub fn find(
        &self,
        family: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        lang: &Lang,
    ) -> ResponseVar<Option<FontFace>> {
        // try with shared lock
        if let Some(cached) = FONTS_SV.read().loader.try_cached(family, style, weight, stretch, lang) {
            return cached;
        }
        // begin load with exclusive lock (cache is tried again in `load`)
        FONTS_SV.write().loader.load(family, style, weight, stretch, lang)
    }

    /// Find a single font face with all normal properties.
    pub fn normal(&self, family: &FontName, lang: &Lang) -> ResponseVar<Option<FontFace>> {
        self.find(family, FontStyle::Normal, FontWeight::NORMAL, FontStretch::NORMAL, lang)
    }

    /// Find a single font face with italic italic style and normal weight and stretch.
    pub fn italic(&self, family: &FontName, lang: &Lang) -> ResponseVar<Option<FontFace>> {
        self.find(family, FontStyle::Italic, FontWeight::NORMAL, FontStretch::NORMAL, lang)
    }

    /// Find a single font face with bold weight and normal style and stretch.
    pub fn bold(&self, family: &FontName, lang: &Lang) -> ResponseVar<Option<FontFace>> {
        self.find(family, FontStyle::Normal, FontWeight::BOLD, FontStretch::NORMAL, lang)
    }

    /// Gets all [registered](Self::register) font families.
    pub fn custom_fonts(&self) -> Vec<FontName> {
        FONTS_SV.read().loader.custom_fonts.keys().cloned().collect()
    }

    /// Gets all font families available in the system.
    pub fn system_fonts(&self) -> Vec<FontName> {
        font_kit::source::SystemSource::new()
            .all_families()
            .unwrap_or_default()
            .into_iter()
            .map(FontName::from)
            .collect()
    }

    /// Gets the system font anti-aliasing config as a read-only var.
    ///
    /// The variable updates when the system config changes.
    pub fn system_font_aa(&self) -> impl Var<FontAntiAliasing> {
        FONTS_SV.read().font_aa.read_only()
    }
}

use crate::render::webrender_api::{self as wr, euclid};
pub use font_kit::error::FontLoadingError;

impl From<font_kit::metrics::Metrics> for FontFaceMetrics {
    fn from(m: font_kit::metrics::Metrics) -> Self {
        FontFaceMetrics {
            units_per_em: m.units_per_em,
            ascent: m.ascent,
            descent: m.descent,
            line_gap: m.line_gap,
            underline_position: m.underline_position,
            underline_thickness: m.underline_thickness,
            cap_height: m.cap_height,
            x_height: m.x_height,
            bounds: euclid::rect(
                m.bounding_box.origin_x(),
                m.bounding_box.origin_y(),
                m.bounding_box.width(),
                m.bounding_box.height(),
            ),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct FontInstanceKey(Px, Box<[(harfbuzz_rs::Tag, i32)]>);
impl FontInstanceKey {
    /// Returns the key.
    pub fn new(size: Px, variations: &[harfbuzz_rs::Variation]) -> Self {
        let variations_key: Vec<_> = variations.iter().map(|p| (p.tag(), (p.value() * 1000.0) as i32)).collect();
        FontInstanceKey(size, variations_key.into_boxed_slice())
    }
}

/// A font face selected from a font family.
///
/// Usually this is part of a [`FontList`] that can be requested from
/// the [`FONTS`] service.
///
/// This type is a shared reference to the font data, cloning it is cheap.
#[derive(Clone)]
pub struct FontFace(Arc<LoadedFontFace>);
struct LoadedFontFace {
    data: FontDataRef,
    face: harfbuzz_rs::Shared<harfbuzz_rs::Face<'static>>,
    face_index: u32,
    display_name: FontName,
    family_name: FontName,
    postscript_name: Option<String>,
    is_monospace: bool,
    properties: font_kit::properties::Properties,
    metrics: FontFaceMetrics,
    m: Mutex<FontFaceMut>,
}
struct FontFaceMut {
    font_kit: FontKitCache,
    instances: FxHashMap<FontInstanceKey, Font>,
    render_keys: Vec<RenderFontFace>,
    unregistered: bool,
}

impl fmt::Debug for FontFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = self.0.m.lock();
        f.debug_struct("FontFace")
            .field("display_name", &self.0.display_name)
            .field("family_name", &self.0.family_name)
            .field("postscript_name", &self.0.postscript_name)
            .field("is_monospace", &self.0.is_monospace)
            .field("properties", &self.0.properties)
            .field("metrics", &self.0.metrics)
            .field("instances.len()", &m.instances.len())
            .field("render_keys.len()", &m.render_keys.len())
            .field("unregistered", &m.unregistered)
            .finish_non_exhaustive()
    }
}
impl PartialEq for FontFace {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FontFace {}
impl FontFace {
    /// New empty font face.
    pub fn empty() -> Self {
        FontFace(Arc::new(LoadedFontFace {
            data: FontDataRef::from_static(&[]),
            face: harfbuzz_rs::Face::empty().to_shared(),
            face_index: 0,
            display_name: FontName::from("<empty>"),
            family_name: FontName::from("<empty>"),
            postscript_name: None,
            is_monospace: true,
            properties: font_kit::properties::Properties {
                style: FontStyle::Normal,
                weight: FontWeight::NORMAL,
                stretch: FontStretch::NORMAL,
            },
            // values copied from a monospace font
            metrics: FontFaceMetrics {
                units_per_em: 2048,
                ascent: 1616.0,
                descent: -432.0,
                line_gap: 0.0,
                underline_position: -205.0,
                underline_thickness: 102.0,
                cap_height: 1616.0,
                x_height: 1616.0,
                // `xMin`/`xMax`/`yMin`/`yMax`
                bounds: euclid::Box2D::new(euclid::point2(0.0, -432.0), euclid::point2(1291.0, 1616.0)).to_rect(),
            },
            m: Mutex::new(FontFaceMut {
                font_kit: FontKitCache::default(),
                instances: FxHashMap::default(),
                render_keys: vec![],
                unregistered: false,
            }),
        }))
    }

    /// Is empty font face.
    pub fn is_empty(&self) -> bool {
        self.0.data.is_empty()
    }

    async fn load_custom(custom_font: CustomFont) -> Result<Self, FontLoadingError> {
        let bytes;
        let face_index;

        match custom_font.source {
            FontSource::File(path, index) => {
                bytes = FontDataRef(Arc::new(task::wait(|| std::fs::read(path)).await?));
                face_index = index;
            }
            FontSource::Memory(arc, index) => {
                bytes = arc;
                face_index = index;
            }
            FontSource::Alias(other_font) => {
                let result = FONTS_SV
                    .write()
                    .loader
                    .load_resolved(&other_font, custom_font.style, custom_font.weight, custom_font.stretch);
                return match result.wait_into_rsp().await {
                    Some(other_font) => Ok(FontFace(Arc::new(LoadedFontFace {
                        data: other_font.0.data.clone(),
                        face: harfbuzz_rs::Face::new(other_font.0.data.clone(), other_font.0.face_index).to_shared(),
                        face_index: other_font.0.face_index,
                        display_name: custom_font.name.clone(),
                        family_name: custom_font.name,
                        postscript_name: None,
                        properties: other_font.0.properties,
                        is_monospace: other_font.0.is_monospace,
                        metrics: other_font.0.metrics.clone(),
                        m: Mutex::new(FontFaceMut {
                            font_kit: other_font.0.m.lock().font_kit.clone(),
                            instances: Default::default(),
                            render_keys: Default::default(),
                            unregistered: Default::default(),
                        }),
                    }))),
                    None => Err(FontLoadingError::NoSuchFontInCollection),
                };
            }
        }

        let font = font_kit::handle::Handle::Memory {
            bytes: Arc::clone(&bytes.0),
            font_index: face_index,
        }
        .load()?;

        let face = harfbuzz_rs::Face::new(bytes.clone(), face_index);
        if face.glyph_count() == 0 {
            // Harfbuzz returns the empty face if data is not a valid font,
            // font-kit already successfully parsed the font above so if must be
            // a format not supported by Harfbuzz.
            return Err(FontLoadingError::UnknownFormat);
        }

        Ok(FontFace(Arc::new(LoadedFontFace {
            data: bytes,
            face: face.to_shared(),
            face_index,
            display_name: custom_font.name.clone(),
            family_name: custom_font.name,
            postscript_name: None,
            properties: font_kit::properties::Properties {
                style: custom_font.style,
                weight: custom_font.weight,
                stretch: custom_font.stretch,
            },
            is_monospace: font.is_monospace(),
            metrics: font.metrics().into(),
            m: Mutex::new(FontFaceMut {
                font_kit: {
                    let mut font_kit = FontKitCache::default();
                    font_kit.get_or_init(move || font);
                    font_kit
                },
                instances: Default::default(),
                render_keys: Default::default(),
                unregistered: Default::default(),
            }),
        })))
    }

    fn load(handle: font_kit::handle::Handle) -> Result<Self, FontLoadingError> {
        let _span = tracing::trace_span!("FontFace::load").entered();

        let bytes;
        let face_index;

        match handle {
            font_kit::handle::Handle::Path { path, font_index } => {
                bytes = FontDataRef(Arc::new(std::fs::read(path)?));
                face_index = font_index;
            }
            font_kit::handle::Handle::Memory { bytes: arc, font_index } => {
                bytes = FontDataRef(arc);
                face_index = font_index;
            }
        };

        let font = font_kit::handle::Handle::Memory {
            bytes: Arc::clone(&bytes.0),
            font_index: face_index,
        }
        .load()?;

        let face = harfbuzz_rs::Face::new(bytes.clone(), face_index);
        if face.glyph_count() == 0 {
            // Harfbuzz returns the empty face if data is not a valid font,
            // font-kit already successfully parsed the font above so if must be
            // a format not supported by Harfbuzz.
            return Err(FontLoadingError::UnknownFormat);
        }

        Ok(FontFace(Arc::new(LoadedFontFace {
            data: bytes,
            face: face.to_shared(),
            face_index,
            display_name: font.full_name().into(),
            family_name: font.family_name().into(),
            postscript_name: font.postscript_name(),
            properties: font.properties(),
            is_monospace: font.is_monospace(),
            metrics: font.metrics().into(),
            m: Mutex::new(FontFaceMut {
                font_kit: {
                    let mut font_kit = FontKitCache::default();
                    font_kit.get_or_init(move || font);
                    font_kit
                },
                instances: Default::default(),
                render_keys: Default::default(),
                unregistered: Default::default(),
            }),
        })))
    }

    fn on_refresh(&self) {
        let mut m = self.0.m.lock();
        m.instances.clear();
        m.unregistered = true;
    }

    const DUMMY_FONT_KEY: wr::FontKey = wr::FontKey(wr::IdNamespace(0), 0);

    fn render_face(&self, renderer: &ViewRenderer) -> wr::FontKey {
        let namespace = match renderer.namespace_id() {
            Ok(n) => n,
            Err(ViewProcessOffline) => {
                tracing::debug!("respawned calling `namespace_id`, will return dummy font key");
                return Self::DUMMY_FONT_KEY;
            }
        };

        let mut m = self.0.m.lock();
        for r in m.render_keys.iter() {
            if r.key.0 == namespace {
                return r.key;
            }
        }

        let key = match renderer.add_font((*self.0.data.0).clone(), self.0.face_index) {
            Ok(k) => k,
            Err(ViewProcessOffline) => {
                tracing::debug!("respawned calling `add_font`, will return dummy font key");
                return Self::DUMMY_FONT_KEY;
            }
        };

        m.render_keys.push(RenderFontFace::new(renderer, key));

        key
    }

    /// Reference the `harfbuzz` face.
    pub fn harfbuzz_face(&self) -> &harfbuzz_rs::Shared<harfbuzz_rs::Face<'static>> {
        &self.0.face
    }

    /// Get the `font_kit` loaded in the current thread, or loads it.
    ///
    /// Loads from the cached [`bytes`], unfortunately the font itself is `!Send`,
    /// so a different instance is generated each call.
    ///
    /// Returns `None` if [`is_empty`].
    ///
    /// [`is_empty`]: Self::is_empty
    /// [`bytes`]: Self::bytes
    pub fn font_kit(&self) -> Option<Rc<font_kit::font::Font>> {
        if self.is_empty() {
            None
        } else {
            Some(self.0.m.lock().font_kit.get_or_init(|| {
                font_kit::handle::Handle::Memory {
                    bytes: Arc::clone(&self.0.data.0),
                    font_index: self.0.face_index,
                }
                .load()
                .unwrap()
            }))
        }
    }

    /// Reference the font file bytes.
    pub fn bytes(&self) -> &FontDataRef {
        &self.0.data
    }

    /// Font full name.
    pub fn display_name(&self) -> &FontName {
        &self.0.display_name
    }

    /// Font family name.
    pub fn family_name(&self) -> &FontName {
        &self.0.family_name
    }

    /// Font globally unique name.
    pub fn postscript_name(&self) -> Option<&str> {
        self.0.postscript_name.as_deref()
    }

    /// Index of the font face in the [font file](Self::bytes).
    pub fn index(&self) -> u32 {
        self.0.face_index
    }

    /// Font style.
    pub fn style(&self) -> FontStyle {
        self.0.properties.style
    }

    /// Font weight.
    pub fn weight(&self) -> FontWeight {
        self.0.properties.weight
    }

    /// Font stretch.
    pub fn stretch(&self) -> FontStretch {
        self.0.properties.stretch
    }

    /// Font is monospace (fixed-width).
    pub fn is_monospace(&self) -> bool {
        self.0.is_monospace
    }

    /// Font metrics in font units.
    pub fn metrics(&self) -> &FontFaceMetrics {
        &self.0.metrics
    }

    /// Gets a cached sized [`Font`].
    ///
    /// The `font_size` is the size of `1 font EM` in pixels.
    ///
    /// The `variations` are custom [font variations] that will be used
    /// during shaping and rendering.
    ///
    /// [font variations]: crate::text::font_features::FontVariations::finalize
    pub fn sized(&self, font_size: Px, variations: RFontVariations) -> Font {
        let key = FontInstanceKey::new(font_size, &variations);
        let mut m = self.0.m.lock();
        if !m.unregistered {
            m.instances
                .entry(key)
                .or_insert_with(|| Font::new(self.clone(), font_size, variations))
                .clone()
        } else {
            tracing::debug!(target: "font_loading", "creating font from unregistered `{}`, will not cache", self.0.display_name);
            Font::new(self.clone(), font_size, variations)
        }
    }

    /// Gets what font synthesis to use to better render this font face given the style and weight.
    pub fn synthesis_for(&self, style: FontStyle, weight: FontWeight) -> FontSynthesis {
        let mut synth = FontSynthesis::DISABLED;

        if style != FontStyle::Normal && self.style() == FontStyle::Normal {
            // if requested oblique or italic and the face is neither.
            synth |= FontSynthesis::STYLE;
        }
        if weight > self.weight() {
            // if requested a weight larger then the face weight the renderer can
            // add extra stroke outlines to compensate.
            synth |= FontSynthesis::BOLD;
        }

        synth
    }

    /// If this font face is cached. All font faces are cached by default, a font face can be detached from
    /// cache when a [`FONT_CHANGED_EVENT`] event happens, in this case the font can still be used normally, but
    /// a request for the same font name will return a different reference.
    pub fn is_cached(&self) -> bool {
        !self.0.m.lock().unregistered
    }
}

/// A sized font face.
///
/// A sized font can be requested from a [`FontFace`].
///
/// This type is a shared reference to the loaded font data, cloning it is cheap.
#[derive(Clone)]
pub struct Font(pub(super) Arc<LoadedFont>);
pub(super) struct LoadedFont {
    face: FontFace,
    pub(super) font: harfbuzz_rs::Shared<harfbuzz_rs::Font<'static>>,
    size: Px,
    variations: RFontVariations,
    metrics: FontMetrics,
    render_keys: Mutex<Vec<RenderFont>>,
    pub(super) small_word_cache: RwLock<FxHashMap<WordCacheKey<[u8; Font::SMALL_WORD_LEN]>, ShapedSegmentData>>,
    pub(super) word_cache: RwLock<hashbrown::HashMap<WordCacheKey<String>, ShapedSegmentData>>,
}
impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font")
            .field("face", &self.0.face)
            .field("size", &self.0.size)
            .field("metrics", &self.0.metrics)
            .field("render_keys.len()", &self.0.render_keys.lock().len())
            .field("small_word_cache.len()", &self.0.small_word_cache.read().len())
            .field("word_cache.len()", &self.0.word_cache.read().len())
            .finish()
    }
}
impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for Font {}
impl Font {
    pub(super) const SMALL_WORD_LEN: usize = 8;

    pub(super) fn to_small_word(s: &str) -> Option<[u8; Self::SMALL_WORD_LEN]> {
        if s.len() <= Self::SMALL_WORD_LEN {
            let mut a = [b'\0'; Self::SMALL_WORD_LEN];
            a[..s.len()].copy_from_slice(s.as_bytes());
            Some(a)
        } else {
            None
        }
    }

    fn new(face: FontFace, size: Px, variations: RFontVariations) -> Self {
        let ppem = size.0 as u32;

        let mut font = harfbuzz_rs::Font::new(face.harfbuzz_face().clone());
        font.set_ppem(ppem, ppem);
        font.set_variations(&variations);

        Font(Arc::new(LoadedFont {
            metrics: face.metrics().sized(size),
            font: font.to_shared(),
            face,
            size,
            variations,
            render_keys: Mutex::new(vec![]),
            small_word_cache: RwLock::default(),
            word_cache: RwLock::default(),
        }))
    }

    const DUMMY_FONT_KEY: wr::FontInstanceKey = wr::FontInstanceKey(wr::IdNamespace(0), 0);

    fn render_font(&self, renderer: &ViewRenderer, synthesis: FontSynthesis) -> (wr::FontInstanceKey, wr::FontInstanceFlags) {
        let _span = tracing::trace_span!("Font::render_font").entered();

        let namespace = match renderer.namespace_id() {
            Ok(n) => n,
            Err(ViewProcessOffline) => {
                tracing::debug!("respawned calling `namespace_id`, will return dummy font key");
                return (Self::DUMMY_FONT_KEY, wr::FontInstanceFlags::empty());
            }
        };
        let mut render_keys = self.0.render_keys.lock();
        for r in render_keys.iter() {
            if r.key.0 == namespace && r.synthesis == synthesis {
                return (r.key, r.flags);
            }
        }

        let font_key = self.0.face.render_face(renderer);

        let mut flags = wr::FontInstanceFlags::empty();

        let mut opt = wr::FontInstanceOptions::default();
        if synthesis.contains(FontSynthesis::STYLE) {
            opt.synthetic_italics = wr::SyntheticItalics::enabled();
        }
        if synthesis.contains(FontSynthesis::BOLD) {
            opt.flags |= wr::FontInstanceFlags::SYNTHETIC_BOLD;
            flags |= wr::FontInstanceFlags::SYNTHETIC_BOLD;
        }
        let variations = self
            .0
            .variations
            .iter()
            .map(|v| wr::FontVariation {
                tag: v.tag().0,
                value: v.value(),
            })
            .collect();

        let key = match renderer.add_font_instance(font_key, self.0.size, Some(opt), None, variations) {
            Ok(k) => k,
            Err(ViewProcessOffline) => {
                tracing::debug!("respawned calling `add_font_instance`, will return dummy font key");
                return (Self::DUMMY_FONT_KEY, wr::FontInstanceFlags::empty());
            }
        };

        render_keys.push(RenderFont::new(renderer, synthesis, key, flags));

        (key, flags)
    }

    /// Reference the font face source of this font.
    pub fn face(&self) -> &FontFace {
        &self.0.face
    }

    /// Reference the `harfbuzz` font.
    pub fn harfbuzz_font(&self) -> &harfbuzz_rs::Shared<harfbuzz_rs::Font<'static>> {
        &self.0.font
    }

    /// Font size.
    pub fn size(&self) -> Px {
        self.0.size
    }

    /// Custom font variations.
    pub fn variations(&self) -> &RFontVariations {
        &self.0.variations
    }

    /// Sized font metrics.
    pub fn metrics(&self) -> &FontMetrics {
        &self.0.metrics
    }
}
impl crate::render::Font for Font {
    fn is_empty_fallback(&self) -> bool {
        self.face().is_empty()
    }

    fn instance_key(&self, renderer: &ViewRenderer, synthesis: FontSynthesis) -> (wr::FontInstanceKey, wr::FontInstanceFlags) {
        self.render_font(renderer, synthesis)
    }
}

/// A list of [`FontFace`] resolved from a [`FontName`] list, plus the [fallback](GenericFonts::fallback) font.
///
/// Glyphs that are not resolved by the first font fallback to the second font and so on.
#[derive(Debug, Clone)]
pub struct FontFaceList {
    fonts: Box<[FontFace]>,
    requested_style: FontStyle,
    requested_weight: FontWeight,
    requested_stretch: FontStretch,
}
#[allow(clippy::len_without_is_empty)] // is never empty.
impl FontFaceList {
    /// New list with only the [`FontFace::empty`].
    pub fn empty() -> Self {
        Self {
            fonts: Box::new([FontFace::empty()]),
            requested_style: FontStyle::Normal,
            requested_weight: FontWeight::NORMAL,
            requested_stretch: FontStretch::NORMAL,
        }
    }

    /// Style requested in the query that generated this font face list.
    pub fn requested_style(&self) -> FontStyle {
        self.requested_style
    }

    /// Weight requested in the query that generated this font face list.
    pub fn requested_weight(&self) -> FontWeight {
        self.requested_weight
    }

    /// Stretch requested in the query that generated this font face list.
    pub fn requested_stretch(&self) -> FontStretch {
        self.requested_stretch
    }

    /// The font face that best matches the requested properties.
    pub fn best(&self) -> &FontFace {
        &self.fonts[0]
    }

    /// Gets the font synthesis to use to better render the given font face on the list.
    pub fn face_synthesis(&self, face_index: usize) -> FontSynthesis {
        if let Some(face) = self.fonts.get(face_index) {
            face.synthesis_for(self.requested_style, self.requested_weight)
        } else {
            FontSynthesis::DISABLED
        }
    }

    /// Iterate over font faces, more specific first.
    pub fn iter(&self) -> std::slice::Iter<FontFace> {
        self.fonts.iter()
    }

    /// Number of font faces in the list.
    ///
    /// This is at least `1`.
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Gets a sized font list.
    ///
    /// This calls [`FontFace::sized`] for each font in the list.
    pub fn sized(&self, font_size: Px, variations: RFontVariations) -> FontList {
        FontList {
            fonts: self.fonts.iter().map(|f| f.sized(font_size, variations.clone())).collect(),
            requested_style: self.requested_style,
            requested_weight: self.requested_weight,
            requested_stretch: self.requested_stretch,
        }
    }
}
impl PartialEq for FontFaceList {
    /// Both are equal if each point to the same fonts in the same order and have the same requested properties.
    fn eq(&self, other: &Self) -> bool {
        self.requested_style == other.requested_style
            && self.requested_weight == other.requested_weight
            && self.requested_stretch == other.requested_stretch
            && self.fonts.len() == other.fonts.len()
            && self.fonts.iter().zip(other.fonts.iter()).all(|(a, b)| a == b)
    }
}
impl Eq for FontFaceList {}
impl std::ops::Deref for FontFaceList {
    type Target = [FontFace];

    fn deref(&self) -> &Self::Target {
        &self.fonts
    }
}
impl<'a> std::iter::IntoIterator for &'a FontFaceList {
    type Item = &'a FontFace;

    type IntoIter = std::slice::Iter<'a, FontFace>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl std::ops::Index<usize> for FontFaceList {
    type Output = FontFace;

    fn index(&self, index: usize) -> &Self::Output {
        &self.fonts[index]
    }
}

/// A list of [`Font`] created from a [`FontFaceList`].
#[derive(Debug, Clone)]
pub struct FontList {
    fonts: Box<[Font]>,
    requested_style: FontStyle,
    requested_weight: FontWeight,
    requested_stretch: FontStretch,
}
#[allow(clippy::len_without_is_empty)] // cannot be empty.
impl FontList {
    /// The font that best matches the requested properties.
    pub fn best(&self) -> &Font {
        &self.fonts[0]
    }

    /// Font size requested in the query that generated  this font list.
    pub fn requested_size(&self) -> Px {
        self.fonts[0].size()
    }

    /// Style requested in the query that generated this font list.
    pub fn requested_style(&self) -> FontStyle {
        self.requested_style
    }

    /// Weight requested in the query that generated this font list.
    pub fn requested_weight(&self) -> FontWeight {
        self.requested_weight
    }

    /// Stretch requested in the query that generated this font list.
    pub fn requested_stretch(&self) -> FontStretch {
        self.requested_stretch
    }

    /// Gets the font synthesis to use to better render the given font on the list.
    pub fn face_synthesis(&self, font_index: usize) -> FontSynthesis {
        if let Some(font) = self.fonts.get(font_index) {
            font.0.face.synthesis_for(self.requested_style, self.requested_weight)
        } else {
            FontSynthesis::DISABLED
        }
    }

    /// Iterate over font faces, more specific first.
    pub fn iter(&self) -> std::slice::Iter<Font> {
        self.fonts.iter()
    }

    /// Number of font faces in the list.
    ///
    /// This is at least `1`.
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Returns `true` is `self` is sized from the `faces` list.
    pub fn is_sized_from(&self, faces: &FontFaceList) -> bool {
        if self.len() != faces.len() {
            return false;
        }

        for (font, face) in self.iter().zip(faces.iter()) {
            if font.face() != face {
                return false;
            }
        }

        true
    }
}
impl PartialEq for FontList {
    /// Both are equal if each point to the same fonts in the same order and have the same requested properties.
    fn eq(&self, other: &Self) -> bool {
        self.requested_style == other.requested_style
            && self.requested_weight == other.requested_weight
            && self.requested_stretch == other.requested_stretch
            && self.fonts.len() == other.fonts.len()
            && self.fonts.iter().zip(other.fonts.iter()).all(|(a, b)| a == b)
    }
}
impl Eq for FontList {}
impl std::ops::Deref for FontList {
    type Target = [Font];

    fn deref(&self) -> &Self::Target {
        &self.fonts
    }
}
impl<'a> std::iter::IntoIterator for &'a FontList {
    type Item = &'a Font;

    type IntoIter = std::slice::Iter<'a, Font>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<I: SliceIndex<[Font]>> std::ops::Index<I> for FontList {
    type Output = I::Output;

    fn index(&self, index: I) -> &I::Output {
        &self.fonts[index]
    }
}

struct FontFaceLoader {
    custom_fonts: HashMap<FontName, Vec<FontFace>>,
    system_fonts_cache: HashMap<FontName, Vec<SystemFontFace>>,
    list_cache: HashMap<Box<[FontName]>, Vec<FontFaceListQuery>>,
}
struct SystemFontFace {
    properties: (FontStyle, FontWeight, FontStretch),
    result: ResponseVar<Option<FontFace>>,
}
struct FontFaceListQuery {
    properties: (FontStyle, FontWeight, FontStretch),
    lang: Lang,
    result: ResponseVar<FontFaceList>,
}
impl FontFaceLoader {
    fn new() -> Self {
        FontFaceLoader {
            custom_fonts: HashMap::new(),
            system_fonts_cache: HashMap::new(),
            list_cache: HashMap::new(),
        }
    }

    fn on_view_process_respawn(&mut self) {
        let sys_fonts = self.system_fonts_cache.values().flatten().filter_map(|f| f.result.rsp().flatten());
        for face in self.custom_fonts.values().flatten().cloned().chain(sys_fonts) {
            let mut m = face.0.m.lock();
            m.render_keys.clear();
            for inst in m.instances.values() {
                inst.0.render_keys.lock().clear();
            }
        }
    }

    fn on_refresh(&mut self) {
        for (_, sys_family) in self.system_fonts_cache.drain() {
            for sys_font in sys_family {
                sys_font.result.with(|r| {
                    if let Some(Some(face)) = r.done() {
                        face.on_refresh();
                    }
                });
            }
        }
    }
    fn on_prune(&mut self) {
        self.system_fonts_cache.retain(|_, v| {
            v.retain(|sff| {
                if sff.result.strong_count() == 1 {
                    sff.result.with(|r| {
                        match r.done() {
                            Some(Some(face)) => Arc::strong_count(&face.0) > 1, // face shared
                            Some(None) => false,                                // loading for no one
                            None => true,                                       // retain not found
                        }
                    })
                } else {
                    // response var shared
                    true
                }
            });
            !v.is_empty()
        });

        self.list_cache.clear();
    }

    async fn register(custom_font: CustomFont) -> Result<(), FontLoadingError> {
        let face = FontFace::load_custom(custom_font).await?;

        let mut fonts = FONTS_SV.write();
        let family = fonts.loader.custom_fonts.entry(face.0.family_name.clone()).or_default();

        let existing = family.iter().position(|f| f.0.properties == face.0.properties);

        if let Some(i) = existing {
            family[i] = face;
        } else {
            family.push(face);
        }
        Ok(())
    }

    fn unregister(&mut self, custom_family: &FontName) -> bool {
        if let Some(removed) = self.custom_fonts.remove(custom_family) {
            // cut circular reference so that when the last font ref gets dropped
            // this font face also gets dropped. Also tag the font as unregistered
            // so it does not create further circular references.
            for removed in removed {
                removed.on_refresh();
            }
            true
        } else {
            false
        }
    }

    fn try_list(
        &self,
        families: &[FontName],
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        lang: &Lang,
    ) -> Option<ResponseVar<FontFaceList>> {
        if let Some(queries) = self.list_cache.get(families) {
            for q in queries {
                if q.properties == (style, weight, stretch) && &q.lang == lang {
                    return Some(q.result.clone());
                }
            }
        }
        None
    }

    fn load_list(
        &mut self,
        families: &[FontName],
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        lang: &Lang,
    ) -> ResponseVar<FontFaceList> {
        if let Some(r) = self.try_list(families, style, weight, stretch, lang) {
            return r;
        }

        let mut used = HashSet::with_capacity(families.len());

        let mut list = Vec::with_capacity(families.len() + 1);
        let mut pending = vec![];

        let fallback = GenericFonts {}.fallback(lang);
        for name in families.iter().chain([&fallback]) {
            if !used.insert(name) {
                continue;
            }

            let face = self.load(name, style, weight, stretch, lang);
            if face.is_done() {
                if let Some(face) = face.into_rsp().unwrap() {
                    list.push(face);
                }
            } else {
                pending.push((list.len(), face));
            }
        }

        if pending.is_empty() {
            if list.is_empty() {
                tracing::error!(target: "font_loading", "failed to load fallback font");
                list.push(FontFace::empty());
            } else {
                return response_done_var(FontFaceList {
                    fonts: list.into_boxed_slice(),
                    requested_style: style,
                    requested_weight: weight,
                    requested_stretch: stretch,
                });
            }
        }

        let r = task::respond(async move {
            for (i, pending) in pending.into_iter().rev() {
                if let Some(rsp) = pending.wait_into_rsp().await {
                    list.insert(i, rsp);
                }
            }

            if list.is_empty() {
                tracing::error!(target: "font_loading", "failed to load fallback font");
                list.push(FontFace::empty());
            }

            FontFaceList {
                fonts: list.into_boxed_slice(),
                requested_style: style,
                requested_weight: weight,
                requested_stretch: stretch,
            }
        });

        self.list_cache
            .entry(families.iter().cloned().collect())
            .or_insert_with(|| Vec::with_capacity(1))
            .push(FontFaceListQuery {
                properties: (style, weight, stretch),
                lang: lang.clone(),
                result: r.clone(),
            });

        r
    }

    fn try_cached(
        &self,
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        lang: &Lang,
    ) -> Option<ResponseVar<Option<FontFace>>> {
        let resolved = GenericFonts {}.resolve(font_name, lang);
        let font_name = resolved.as_ref().unwrap_or(font_name);
        self.try_resolved(font_name, style, weight, stretch)
    }

    /// Try cached again, otherwise begins loading and inserts the response in the cache.
    fn load(
        &mut self,
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        lang: &Lang,
    ) -> ResponseVar<Option<FontFace>> {
        let resolved = GenericFonts {}.resolve(font_name, lang);
        let font_name = resolved.as_ref().unwrap_or(font_name);
        self.load_resolved(font_name, style, weight, stretch)
    }

    /// Get a `font_name` that already resolved generic names if it is already in cache.
    fn try_resolved(
        &self,
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<ResponseVar<Option<FontFace>>> {
        if let Some(custom_family) = self.custom_fonts.get(font_name) {
            let custom = Self::match_custom(custom_family, style, weight, stretch);
            return Some(response_done_var(Some(custom)));
        }

        if let Some(cached_sys_family) = self.system_fonts_cache.get(font_name) {
            for sys_face in cached_sys_family.iter() {
                if sys_face.properties == (style, weight, stretch) {
                    return Some(sys_face.result.clone());
                }
            }
        }

        None
    }

    /// Load a `font_name` that already resolved generic names.
    fn load_resolved(
        &mut self,
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> ResponseVar<Option<FontFace>> {
        if let Some(cached) = self.try_resolved(font_name, style, weight, stretch) {
            return cached;
        }

        let result = task::wait_respond(clmv!(font_name, || {
            let handle = match Self::get_system(&font_name, style, weight, stretch) {
                Some(h) => h,
                None => {
                    #[cfg(debug_assertions)]
                    static NOT_FOUND: Mutex<crate::crate_util::FxHashSet<FontName>> = Mutex::new(crate::crate_util::fx_set_new());

                    #[cfg(debug_assertions)]
                    if NOT_FOUND.lock().insert(font_name.clone()) {
                        tracing::warn!(r#"font "{font_name}" not found"#);
                    }

                    return None;
                }
            };
            match FontFace::load(handle) {
                Ok(f) => Some(f),
                Err(FontLoadingError::UnknownFormat) => None,
                Err(e) => {
                    tracing::error!(target: "font_loading", "failed to load system font, {e}\nquery: {:?}", (font_name, style, weight, stretch));
                    None
                }
            }
        }));

        self.system_fonts_cache
            .entry(font_name.clone())
            .or_insert_with(|| Vec::with_capacity(1))
            .push(SystemFontFace {
                properties: (style, weight, stretch),
                result: result.clone(),
            });

        result
    }

    fn get_system(font_name: &FontName, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<font_kit::handle::Handle> {
        let _span = tracing::trace_span!("FontFaceLoader::get_system").entered();
        let family_name = font_kit::family_name::FamilyName::from(font_name.clone());
        match font_kit::source::SystemSource::new()
            .select_best_match(&[family_name], &font_kit::properties::Properties { style, weight, stretch })
        {
            Ok(handle) => Some(handle),
            Err(font_kit::error::SelectionError::NotFound) => {
                tracing::debug!(target: "font_loading", "system font not found\nquery: {:?}", (font_name, style, weight, stretch));
                None
            }
            Err(e) => {
                tracing::error!(target: "font_loading", "failed to select system font, {e}\nquery: {:?}", (font_name, style, weight, stretch));
                None
            }
        }
    }

    fn match_custom(faces: &[FontFace], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontFace {
        if faces.len() == 1 {
            // it is common for custom font names to only have one face.
            return faces[0].clone();
        }

        let mut set = Vec::with_capacity(faces.len());
        let mut set_dist = 0.0f64; // stretch distance of current set if it is not empty.

        // # Filter Stretch
        //
        // Closest to query stretch, if the query is narrow, closest narrow then
        // closest wide, if the query is wide the reverse.
        let wrong_side = if stretch <= FontStretch::NORMAL {
            |s| s > FontStretch::NORMAL
        } else {
            |s| s <= FontStretch::NORMAL
        };
        for face in faces {
            let mut dist = (face.stretch().0 - stretch.0).abs() as f64;
            if wrong_side(face.stretch()) {
                dist += f32::MAX as f64 + 1.0;
            }

            if set.is_empty() {
                set.push(face);
                set_dist = dist;
            } else if dist < set_dist {
                // better candidate found, restart closest set.
                set_dist = dist;
                set.clear();
                set.push(face);
            } else if (dist - set_dist).abs() < 0.0001 {
                // another candidate, same distance.
                set.push(face);
            }
        }
        if set.len() == 1 {
            return set[0].clone();
        }

        // # Filter Style
        //
        // Each query style has a fallback preference, we retain the faces that have the best
        // style given the query preference.
        let style_pref = match style {
            FontStyle::Normal => [FontStyle::Normal, FontStyle::Oblique, FontStyle::Italic],
            FontStyle::Italic => [FontStyle::Italic, FontStyle::Oblique, FontStyle::Normal],
            FontStyle::Oblique => [FontStyle::Oblique, FontStyle::Italic, FontStyle::Normal],
        };
        let mut best_style = style_pref.len();
        for face in &set {
            let i = style_pref.iter().position(|&s| s == face.style()).unwrap();
            if i < best_style {
                best_style = i;
            }
        }
        set.retain(|f| f.style() == style_pref[best_style]);
        if set.len() == 1 {
            return set[0].clone();
        }

        // # Filter Weight
        //
        // a: under 400 query matches query then descending under query then ascending over query.
        // b: over 500 query matches query then ascending over query then descending under query.
        //
        // c: in 400..=500 query matches query then ascending to 500 then descending under query
        //     then ascending over 500.
        let add_penalty = if weight.0 >= 400.0 && weight.0 <= 500.0 {
            // c:
            |face: &FontFace, weight: Weight, dist: &mut f64| {
                // Add penalty for:
                if face.weight() < weight {
                    // Not being in search up to 500
                    *dist += 100.0;
                } else if face.weight().0 > 500.0 {
                    // Not being in search down to 0
                    *dist += 600.0;
                }
            }
        } else if weight.0 < 400.0 {
            // a:
            |face: &FontFace, weight: Weight, dist: &mut f64| {
                if face.weight() > weight {
                    *dist += weight.0 as f64;
                }
            }
        } else {
            debug_assert!(weight.0 > 500.0);
            // b:
            |face: &FontFace, weight: Weight, dist: &mut f64| {
                if face.weight() < weight {
                    *dist += f32::MAX as f64;
                }
            }
        };

        let mut best = set[0];
        let mut best_dist = f64::MAX;

        for face in &set {
            let mut dist = (face.weight().0 - weight.0).abs() as f64;

            add_penalty(face, weight, &mut dist);

            if dist < best_dist {
                best_dist = dist;
                best = face;
            }
        }

        best.clone()
    }
}

struct RenderFontFace {
    renderer: ViewRenderer,
    key: wr::FontKey,
}
impl RenderFontFace {
    fn new(renderer: &ViewRenderer, key: wr::FontKey) -> Self {
        RenderFontFace {
            renderer: renderer.clone(),
            key,
        }
    }
}
impl Drop for RenderFontFace {
    fn drop(&mut self) {
        // error here means the entire renderer was already dropped.
        let _ = self.renderer.delete_font(self.key);
    }
}

struct RenderFont {
    renderer: ViewRenderer,
    synthesis: FontSynthesis,
    key: wr::FontInstanceKey,
    flags: wr::FontInstanceFlags,
}
impl RenderFont {
    fn new(renderer: &ViewRenderer, synthesis: FontSynthesis, key: wr::FontInstanceKey, flags: wr::FontInstanceFlags) -> RenderFont {
        RenderFont {
            renderer: renderer.clone(),
            synthesis,
            key,
            flags,
        }
    }
}
impl Drop for RenderFont {
    fn drop(&mut self) {
        // error here means the entire renderer was already dropped.
        let _ = self.renderer.delete_font_instance(self.key);
    }
}

app_local! {
    static GENERIC_FONTS_SV: GenericFontsService = GenericFontsService::new();
}

struct GenericFontsService {
    serif: LangMap<FontName>,
    sans_serif: LangMap<FontName>,
    monospace: LangMap<FontName>,
    cursive: LangMap<FontName>,
    fantasy: LangMap<FontName>,
    fallback: LangMap<FontName>,
    updates: Vec<FontChangedArgs>,
}
impl GenericFontsService {
    fn new() -> Self {
        fn default(name: impl Into<FontName>) -> LangMap<FontName> {
            let mut f = LangMap::with_capacity(1);
            f.insert(lang!(und), name.into());
            f
        }

        let serif = "serif";
        let sans_serif = "sans-serif";
        let monospace = "monospace";
        let cursive = "cursive";
        let fantasy = "fantasy";
        let fallback = if cfg!(windows) {
            "Segoe UI Symbol"
        } else if cfg!(target_os = "linux") {
            "Standard Symbols PS"
        } else {
            "sans-serif"
        };

        GenericFontsService {
            serif: default(serif),
            sans_serif: default(sans_serif),
            monospace: default(monospace),
            cursive: default(cursive),
            fantasy: default(fantasy),

            fallback: default(fallback),

            updates: vec![],
        }
    }

    fn notify(&mut self, change: FontChange) {
        if self.updates.is_empty() {
            UPDATES.update(None);
        }
        self.updates.push(FontChangedArgs::now(change));
    }
}

/// Generic fonts configuration for the app.
///
/// This type can be accessed from the [`FONTS`] service.
///
/// # Defaults
///
/// By default the `serif`, `sans_serif`, `monospace`, `cursive` and `fantasy` are set to their own generic name,
/// this delegates the resolution to the operating system.
///
/// The default `fallback` font is "Segoe UI Symbol" for Windows, "Standard Symbols PS" for Linux and "sans-serif" for others.
///
/// See also [`FontNames::system_ui`] for the default font selection for UIs.
///
/// [`FontNames::system_ui`]: crate::text::FontNames::system_ui
pub struct GenericFonts {}
impl GenericFonts {}
macro_rules! impl_fallback_accessors {
    ($($name:ident=$name_str:tt),+ $(,)?) => {$($crate::paste! {
    #[doc = "Gets the fallback *"$name_str "* font for the given language."]
    ///
    /// Returns a font name for the best `lang` match.
    ///
    #[doc = "Note that the returned name can still be the generic `\""$name_str "\"`, this delegates the resolution to the operating system."]

    pub fn $name(&self, lang: &Lang) -> FontName {
        GENERIC_FONTS_SV.read().$name.get(lang).unwrap().clone()
    }

    #[doc = "Sets the fallback *"$name_str "* font for the given language."]
    ///
    /// Returns the previous registered font for the language.
    ///
    /// Use `lang!(und)` to set name used when no language matches.
    pub fn [<set_ $name>]<F: Into<FontName>>(&self, lang: Lang, font_name: F) -> Option<FontName> {
        let mut g = GENERIC_FONTS_SV.write();
        g.notify(FontChange::GenericFont(FontName::$name(), lang.clone()));
        g.$name.insert(lang, font_name.into())
    }
    })+};
}
impl GenericFonts {
    impl_fallback_accessors! {
        serif="serif", sans_serif="sans-serif", monospace="monospace", cursive="cursive", fantasy="fantasy"
    }

    /// Gets the ultimate fallback font used when none of the other fonts support a glyph.
    ///
    /// Returns a font name.
    pub fn fallback(&self, lang: &Lang) -> FontName {
        GENERIC_FONTS_SV.read().fallback.get(lang).unwrap().clone()
    }

    /// Sets the ultimate fallback font used when none of other fonts support a glyph.
    ///
    /// This should be a font that cover as many glyphs as possible.
    ///
    /// Returns the previous registered font for the language.
    ///
    /// Use `lang!(und)` to set name used when no language matches.
    pub fn set_fallback<F: Into<FontName>>(&self, lang: Lang, font_name: F) -> Option<FontName> {
        let mut g = GENERIC_FONTS_SV.write();
        g.notify(FontChange::Fallback(lang.clone()));
        g.fallback.insert(lang, font_name.into())
    }

    /// Returns the font name registered for the generic `name` and `lang`.
    ///
    /// Returns `None` if `name` if not one of the generic font names.
    pub fn resolve(&self, name: &FontName, lang: &Lang) -> Option<FontName> {
        if name == &FontName::serif() {
            Some(self.serif(lang))
        } else if name == &FontName::sans_serif() {
            Some(self.sans_serif(lang))
        } else if name == &FontName::monospace() {
            Some(self.monospace(lang))
        } else if name == &FontName::cursive() {
            Some(self.cursive(lang))
        } else if name == &FontName::fantasy() {
            Some(self.fantasy(lang))
        } else {
            None
        }
    }
}

/// Reference to in memory font data.
#[derive(Clone)]
#[allow(clippy::rc_buffer)]
pub struct FontDataRef(pub Arc<Vec<u8>>);
impl FontDataRef {
    /// Copy bytes from embedded font.
    pub fn from_static(data: &'static [u8]) -> Self {
        FontDataRef(Arc::new(data.to_vec()))
    }
}
impl fmt::Debug for FontDataRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FontDataRef(Arc<{} bytes>>)", self.0.len())
    }
}
impl std::ops::Deref for FontDataRef {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl From<FontDataRef> for harfbuzz_rs::Shared<harfbuzz_rs::Blob<'static>> {
    fn from(d: FontDataRef) -> Self {
        harfbuzz_rs::Blob::with_bytes_owned(d, |d| d).to_shared()
    }
}

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
    /// The font is loaded in [`FONTS.register`].
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
    /// The font is loaded in [`FONTS.register`].
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
    /// The font is loaded in [`FONTS.register`].
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
    pub fn stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// Set the [`FontStyle`].
    ///
    /// Default is [`FontStyle::Normal`].
    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the [`FontWeight`].
    ///
    /// Default is [`FontWeight::NORMAL`].
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }
}

impl From<font_kit::family_name::FamilyName> for FontName {
    fn from(family_name: font_kit::family_name::FamilyName) -> Self {
        use font_kit::family_name::FamilyName::*;

        match family_name {
            Title(title) => FontName::new(title),
            Serif => FontName::serif(),
            SansSerif => FontName::sans_serif(),
            Monospace => FontName::monospace(),
            Cursive => FontName::cursive(),
            Fantasy => FontName::fantasy(),
        }
    }
}
impl From<FontName> for font_kit::family_name::FamilyName {
    fn from(font_name: FontName) -> Self {
        use font_kit::family_name::FamilyName::*;
        match font_name.name() {
            "serif" => Serif,
            "sans-serif" => SansSerif,
            "monospace" => Monospace,
            "cursive" => Cursive,
            "fantasy" => Fantasy,
            _ => Title(font_name.text.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;

    use super::*;

    #[test]
    fn generic_fonts_default() {
        let _app = App::minimal().run_headless(false);

        assert_eq!(FontName::sans_serif(), GenericFonts {}.sans_serif(&lang!(und)))
    }

    #[test]
    fn generic_fonts_fallback() {
        let _app = App::minimal().run_headless(false);

        assert_eq!(FontName::sans_serif(), GenericFonts {}.sans_serif(&lang!(en_US)));
        assert_eq!(FontName::sans_serif(), GenericFonts {}.sans_serif(&lang!(es)));
    }

    #[test]
    fn generic_fonts_get1() {
        let _app = App::minimal().run_headless(false);
        GenericFonts {}.set_sans_serif(lang!(en_US), "Test Value");

        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en-US")), "Test Value");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en")), "Test Value");
    }

    #[test]
    fn generic_fonts_get2() {
        let _app = App::minimal().run_headless(false);
        GenericFonts {}.set_sans_serif(lang!(en), "Test Value");

        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en-US")), "Test Value");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en")), "Test Value");
    }

    #[test]
    fn generic_fonts_get_best() {
        let _app = App::minimal().run_headless(false);
        GenericFonts {}.set_sans_serif(lang!(en), "Test Value");
        GenericFonts {}.set_sans_serif(lang!(en_US), "Best");

        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en-US")), "Best");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en")), "Test Value");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("und")), "sans-serif");
    }

    #[test]
    fn generic_fonts_get_no_lang_match() {
        let _app = App::minimal().run_headless(false);
        GenericFonts {}.set_sans_serif(lang!(es_US), "Test Value");

        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en-US")), "sans-serif");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("es")), "Test Value");
    }
}
