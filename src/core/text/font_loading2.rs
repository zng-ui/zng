use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

use fnv::FnvHashMap;
use webrender::api::RenderApi;

use super::{FontFaceMetrics, FontMetrics, FontName, FontStretch, FontStyle, FontSynthesis, FontWeight, Script};
use crate::core::app::AppExtension;
use crate::core::context::{AppContext, AppInitContext, UpdateNotifier, UpdateRequest};
use crate::core::event::{event, event_args, EventEmitter, EventListener};
use crate::core::service::{AppService, WindowService};
use crate::core::units::{layout_to_pt, LayoutLength};
use crate::core::window::{WindowEventArgs, WindowId, WindowOpenEvent, Windows};

event! {
    /// Change in [`Fonts`] that may cause a font query to now give
    /// a different result.
    ///
    /// # Cache
    ///
    /// Every time this event updates the font cache is cleared. Meaning that even
    /// if the query returns the same font it will be a new reference.
    ///
    /// Fonts only unload when all references to then are dropped, so you can still continue using
    /// old references if you don't want to monitor this event.
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
    ///
    /// Currently this is only supported in Microsoft Windows.
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

    #[cfg(windows)]
    window_open: EventListener<WindowEventArgs>,

    #[cfg(windows)]
    system_fonts_changed: Rc<Cell<bool>>,
}
impl Default for FontManager {
    fn default() -> Self {
        FontManager {
            font_changed: FontChangedEvent::emitter(),

            #[cfg(windows)]
            window_open: WindowOpenEvent::never(),
            #[cfg(windows)]
            system_fonts_changed: Rc::new(Cell::new(false)),
        }
    }
}
impl AppExtension for FontManager {
    fn init(&mut self, ctx: &mut AppInitContext) {
        ctx.events.register::<FontChangedEvent>(self.font_changed.listener());
        ctx.services.register(Fonts::new(ctx.updates.notifier().clone())).unwrap();
        ctx.window_services
            .register(move |ctx| FontRenderCache::new(Arc::clone(ctx.render_api), ctx.window_id.get()))
            .unwrap();

        #[cfg(windows)]
        {
            self.window_open = ctx.events.listen_or_never::<WindowOpenEvent>();
        }
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        if update.update {
            {
                let fonts = ctx.services.req::<Fonts>();

                for args in fonts.take_updates() {
                    self.font_changed.notify(ctx.events, args);
                }

                #[cfg(windows)]
                if self.system_fonts_changed.take() {
                    // subclass monitor flagged a font (un)install.
                    self.font_changed.notify(ctx.events, FontChangedArgs::now(FontChange::SystemFonts));
                    fonts.on_system_fonts_changed();
                }
            }

            #[cfg(windows)]
            if self.window_open.has_updates(ctx.events) {
                // attach subclass WM_FONTCHANGE monitor to new windows.
                let windows = ctx.services.req::<Windows>();
                for w in self.window_open.updates(ctx.events) {
                    if let Ok(w) = windows.window(w.window_id) {
                        let notifier = ctx.updates.notifier().clone();
                        let flag = Rc::clone(&self.system_fonts_changed);
                        let ok = w.set_raw_windows_event_handler(move |_, msg, _, _| {
                            if msg == winapi::um::winuser::WM_FONTCHANGE {
                                flag.set(true);
                                notifier.update();
                                Some(0)
                            } else {
                                None
                            }
                        });
                        if !ok {
                            error_println!("failed to set WM_FONTCHANGE subclass monitor");
                        }
                    }
                }
            }
        }
    }
}

/// Font loading, custom fonts and app font configuration.
#[derive(AppService)]
pub struct Fonts {
    loader: FontFaceLoader,
    fallbacks: FontFallbacks,
}
impl Fonts {
    fn new(notifier: UpdateNotifier) -> Self {
        Fonts {
            loader: FontFaceLoader::new(),
            fallbacks: FontFallbacks::new(notifier),
        }
    }

    fn on_system_fonts_changed(&mut self) {
        self.loader.on_system_fonts_changed();
    }

    #[inline]
    fn take_updates(&mut self) -> Vec<FontChangedArgs> {
        std::mem::take(&mut self.fallbacks.updates)
    }

    /// Clear cache and notify `Refresh` in [`FontChangedEvent`].
    ///
    /// See the event documentation for more information.
    #[inline]
    pub fn refresh(&mut self) {
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

    /// Removes a custom font family. If the font faces are not in use it is also unloaded.
    ///
    /// Returns if any was removed.
    #[inline]
    pub fn unregister(&mut self, custom_family: &FontName) -> bool {
        let unregistered = self.loader.unregister(custom_family);
        if unregistered {
            self.fallbacks.notify(FontChange::CustomFonts);
        }
        unregistered
    }

    /// Gets a font list that best matches the query.
    #[inline]
    pub fn get_list(&mut self, families: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontFaceList {
        self.loader
            .get_list(families, style, weight, stretch, &self.fallbacks.fallback[&Script::Unknown])
    }

    /// Gets a single font face that best matches the query.
    #[inline]
    pub fn get(&mut self, family: &FontName, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<FontFaceRef> {
        self.loader.get(family, style, weight, stretch)
    }

    /// Gets a single font face with all normal properties.
    #[inline]
    pub fn get_normal(&mut self, family: &FontName) -> Option<FontFaceRef> {
        self.get(family, FontStyle::Normal, FontWeight::NORMAL, FontStretch::NORMAL)
    }

    /// Gets a single font face with italic italic style and normal weight and stretch.
    #[inline]
    pub fn get_italic(&mut self, family: &FontName) -> Option<FontFaceRef> {
        self.get(family, FontStyle::Italic, FontWeight::NORMAL, FontStretch::NORMAL)
    }

    /// Gets a single font face with bold weight and normal style and stretch.
    #[inline]
    pub fn get_bold(&mut self, family: &FontName) -> Option<FontFaceRef> {
        self.get(family, FontStyle::Normal, FontWeight::BOLD, FontStretch::NORMAL)
    }

    /// Gets all [registered](Self::register) font families.
    #[inline]
    pub fn custom_fonts(&self) -> Vec<FontName> {
        self.loader.custom_fonts.keys().cloned().collect()
    }

    /// Gets all font families available in the system.
    #[inline]
    pub fn system_fonts(&self) -> Vec<FontName> {
        self.loader
            .system_fonts
            .all_families()
            .unwrap_or_default()
            .into_iter()
            .map(FontName::from)
            .collect()
    }
}

pub use font_kit::error::FontLoadingError;

type HarfbuzzFace = harfbuzz_rs::Shared<harfbuzz_rs::Face<'static>>;
type HarfbuzzFont = harfbuzz_rs::Shared<harfbuzz_rs::Font<'static>>;

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
            bounding_box: euclid::rect(
                m.bounding_box.origin_x(),
                m.bounding_box.origin_y(),
                m.bounding_box.width(),
                m.bounding_box.height(),
            ),
        }
    }
}

/// A font face selected from a font family.
///
/// Usually this is part of a [`FontList`] that can be requested from
/// the [`Fonts`] service.
#[derive(Debug)]
pub struct FontFace {
    h_face: HarfbuzzFace,
    display_name: FontName,
    family_name: FontName,
    postscript_name: Option<String>,
    is_monospace: bool,
    properties: font_kit::properties::Properties,
    metrics: FontFaceMetrics,

    instances: RefCell<FnvHashMap<u32, FontRef>>,
    unregistered: Cell<bool>,
}
impl FontFace {
    fn load_custom(custom_font: CustomFont, loader: &mut FontFaceLoader) -> Result<Self, FontLoadingError> {
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
                let other_font = loader
                    .get(&other_font, custom_font.style, custom_font.weight, custom_font.stretch)
                    .ok_or(FontLoadingError::NoSuchFontInCollection)?;
                return Ok(FontFace {
                    h_face: other_font.h_face.clone(),
                    display_name: custom_font.name.clone(),
                    family_name: custom_font.name,
                    postscript_name: None,
                    properties: other_font.properties,
                    is_monospace: other_font.is_monospace,
                    metrics: other_font.metrics.clone(),
                    instances: Default::default(),
                    unregistered: Cell::new(false),
                });
            }
        };

        Ok(FontFace {
            h_face: h_face.to_shared(),
            display_name: custom_font.name.clone(),
            family_name: custom_font.name,
            postscript_name: None,
            properties: font_kit::properties::Properties {
                style: custom_font.style,
                weight: custom_font.weight,
                stretch: custom_font.stretch,
            },
            is_monospace: kit_font.is_monospace(),
            metrics: kit_font.metrics().into(),
            instances: Default::default(),
            unregistered: Cell::new(false),
        })
    }

    fn load(handle: font_kit::handle::Handle) -> Result<Self, FontLoadingError> {
        let kit_font = handle.load()?;
        let h_face = match handle {
            font_kit::handle::Handle::Path { path, font_index } => {
                harfbuzz_rs::Face::from_file(path, font_index).map_err(FontLoadingError::Io)?
            }
            font_kit::handle::Handle::Memory { bytes, font_index } => {
                harfbuzz_rs::Face::new(harfbuzz_rs::Blob::with_bytes_owned(bytes, |b| &b[..]), font_index)
            }
        };
        Ok(FontFace {
            h_face: h_face.to_shared(),
            display_name: kit_font.full_name().into(),
            family_name: kit_font.family_name().into(),
            postscript_name: kit_font.postscript_name(),
            properties: kit_font.properties(),
            is_monospace: kit_font.is_monospace(),
            metrics: kit_font.metrics().into(),
            instances: Default::default(),
            unregistered: Cell::new(false),
        })
    }

    fn empty() -> Self {
        FontFace {
            h_face: harfbuzz_rs::Face::empty().into(),
            display_name: "<empty>".into(),
            family_name: "<empty>".into(),
            postscript_name: None,
            properties: font_kit::properties::Properties::default(),
            is_monospace: true,
            // copied from the default Windows "monospace".
            metrics: FontFaceMetrics {
                units_per_em: 2048,
                ascent: 1705.0,
                descent: -615.0,
                line_gap: 0.0,
                underline_position: -477.0,
                underline_thickness: 84.0,
                cap_height: 1170.0,
                x_height: 866.0,
                bounding_box: euclid::rect(1524.0, 3483.0, -249.0, -1392.0),
            },
            instances: Default::default(),
            unregistered: Cell::new(false),
        }
    }

    fn on_unregistered(&self) {
        self.instances.borrow_mut().clear();
        self.unregistered.set(true);
    }

    /// Reference the underlying [`harfbuzz`](harfbuzz_rs) font face handle.
    #[inline]
    pub fn harfbuzz_handle(&self) -> &HarfbuzzFace {
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
        self.properties.style
    }

    /// Font weight.
    #[inline]
    pub fn weight(&self) -> FontWeight {
        self.properties.weight
    }

    /// Font stretch.
    #[inline]
    pub fn stretch(&self) -> FontStretch {
        self.properties.stretch
    }

    /// Font is monospace (fixed-width).
    #[inline]
    pub fn is_monospace(&self) -> bool {
        self.is_monospace
    }

    /// Font metrics in font units.
    #[inline]
    pub fn metrics(&self) -> &FontFaceMetrics {
        &self.metrics
    }

    /// Gets a cached sized [`Font`].
    pub fn sized(self: &Rc<Self>, font_size: LayoutLength) -> FontRef {
        if !self.unregistered.get() {
            let font_size = font_size.get() as u32;
            let mut instances = self.instances.borrow_mut();
            let f = instances
                .entry(font_size)
                .or_insert_with(|| Rc::new(Font::new(Rc::clone(self), LayoutLength::new(font_size as f32))));
            Rc::clone(f)
        } else {
            warn_println!("creating font from unregistered `{}`, will not cache", self.display_name);
            Rc::new(Font::new(Rc::clone(self), font_size))
        }
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
        let metrics = face.metrics().sized(size.get());

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
pub struct FontFaceList {
    fonts: Box<[FontFaceRef]>,
}
#[allow(clippy::len_without_is_empty)] // is never empty.
impl FontFaceList {
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

    /// Gets a sized font list.
    #[inline]
    pub fn sized(&self, font_size: LayoutLength) -> FontList {
        FontList {
            fonts: self.fonts.iter().map(|f| f.sized(font_size)).collect(),
        }
    }
}
impl PartialEq for FontFaceList {
    /// Both are equal if each point to the same fonts in the same order.
    fn eq(&self, other: &Self) -> bool {
        self.fonts.len() == other.fonts.len() && self.fonts.iter().zip(other.fonts.iter()).all(|(a, b)| Rc::ptr_eq(a, b))
    }
}
impl Eq for FontFaceList {}
impl std::ops::Deref for FontFaceList {
    type Target = [FontFaceRef];

    fn deref(&self) -> &Self::Target {
        &self.fonts
    }
}
impl<'a> std::iter::IntoIterator for &'a FontFaceList {
    type Item = &'a FontFaceRef;

    type IntoIter = std::slice::Iter<'a, FontFaceRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl std::ops::Index<usize> for FontFaceList {
    type Output = FontFaceRef;

    fn index(&self, index: usize) -> &Self::Output {
        &self.fonts[index]
    }
}

/// A list of [`FontRef`] created from a [`FontFaceList`].
#[derive(Debug, Clone)]
pub struct FontList {
    fonts: Box<[FontRef]>,
}

struct FontFaceLoader {
    custom_fonts: HashMap<FontName, Vec<FontFaceRef>>,
    system_fonts: font_kit::source::SystemSource,
    system_fonts_cache: HashMap<FontName, Vec<SystemFontFace>>,
}
enum SystemFontFace {
    /// Properties queried and face returned by system.
    Found(FontStyle, FontWeight, FontStretch, FontFaceRef),
    NotFound(FontStyle, FontWeight, FontStretch),
}
impl FontFaceLoader {
    fn new() -> Self {
        FontFaceLoader {
            custom_fonts: HashMap::new(),
            system_fonts: font_kit::source::SystemSource::new(),
            system_fonts_cache: HashMap::new(),
        }
    }

    fn on_system_fonts_changed(&mut self) {
        self.system_fonts = font_kit::source::SystemSource::new();
        for (_, sys_family) in self.system_fonts_cache.drain() {
            for sys_font in sys_family {
                if let SystemFontFace::Found(_, _, _, ref_) = sys_font {
                    ref_.on_unregistered();
                }
            }
        }
    }

    fn register(&mut self, custom_font: CustomFont) -> Result<(), FontLoadingError> {
        let face = Rc::new(FontFace::load_custom(custom_font, self)?);

        let family = self.custom_fonts.entry(face.family_name.clone()).or_default();

        let existing = family.iter().position(|f| f.properties == face.properties);

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
                removed.on_unregistered();
            }
            true
        } else {
            false
        }
    }

    fn get_list(
        &mut self,
        families: &[FontName],
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        fallback: &FontName,
    ) -> FontFaceList {
        let mut r = Vec::with_capacity(families.len() + 1);
        r.extend(families.iter().filter_map(|name| self.get(name, style, weight, stretch)));
        if !families.contains(fallback) {
            if let Some(fallback) = self.get(fallback, style, weight, stretch) {
                r.push(fallback);
            }
        }

        if r.is_empty() {
            error_println!("failed to load fallback font");
            r.push(Rc::new(FontFace::empty()));
        }

        FontFaceList {
            fonts: r.into_boxed_slice(),
        }
    }

    fn get(&mut self, font_name: &FontName, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<FontFaceRef> {
        if let Some(custom_family) = self.custom_fonts.get(font_name) {
            return Some(Self::match_custom(custom_family, style, weight, stretch));
        }

        if let Some(cached_sys_family) = self.system_fonts_cache.get_mut(font_name) {
            for sys_face in cached_sys_family.iter() {
                match sys_face {
                    SystemFontFace::Found(m_style, m_weight, m_stretch, face) => {
                        if *m_style == style && *m_weight == weight && *m_stretch == stretch {
                            return Some(Rc::clone(face)); // cached match
                        }
                    }
                    SystemFontFace::NotFound(n_style, n_weight, n_stretch) => {
                        if *n_style == style && *n_weight == weight && *n_stretch == stretch {
                            return None; // cached not match
                        }
                    }
                }
            }
        }

        let handle = self.get_system(font_name, style, weight, stretch);

        let sys_family = self
            .system_fonts_cache
            .entry(font_name.clone())
            .or_insert_with(|| Vec::with_capacity(1));

        if let Some(handle) = handle {
            match FontFace::load(handle) {
                Ok(f) => {
                    let f = Rc::new(f);
                    sys_family.push(SystemFontFace::Found(style, weight, stretch, Rc::clone(&f)));
                    return Some(f); // new match
                }
                Err(e) => {
                    error_println!("failed to load system font, {}", e);
                }
            }
        } else {
            sys_family.push(SystemFontFace::NotFound(style, weight, stretch));
        }

        None // no new match
    }

    fn get_system(
        &self,
        font_name: &FontName,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<font_kit::handle::Handle> {
        let family_name = font_kit::family_name::FamilyName::from(font_name.clone());
        match self
            .system_fonts
            .select_best_match(&[family_name], &font_kit::properties::Properties { style, weight, stretch })
        {
            Ok(handle) => Some(handle),
            Err(e) => {
                error_println!("failed to select system font, {}", e);
                None
            }
        }
    }

    fn match_custom(faces: &[FontFaceRef], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontFaceRef {
        if faces.len() == 1 {
            // it is common for custom font names to only have one face.
            return Rc::clone(&faces[0]);
        }

        let mut set = Vec::with_capacity(faces.len());
        let mut set_dist = 0.0f64; // stretch distance of current set if it is not empty.

        // # Filter Stretch
        //
        // Closest to query stretch, if the query is narrow, closest narrow then
        // closest wide, if the query is wide the reverse.
        let wrong_side = |s| {
            if stretch <= FontStretch::NORMAL {
                s > FontStretch::NORMAL
            } else {
                s <= FontStretch::NORMAL
            }
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
            return Rc::clone(set[0]);
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
            return Rc::clone(set[0]);
        }

        // # Filter Weight
        //
        // a - under 400 query matches query then descending under query then ascending over query.
        // b - over 500 query matches query then ascending over query then descending under query.
        //
        // c - 400-450 matches query then 500 then descending under 400 then ascending over 400.
        // d - 450-500 matches query then 400 then descending under 500 then ascending over 500.
        if let Some(exact) = set.iter().find(|f| f.weight() == weight) {
            return Rc::clone(exact);
        }
        let mut descending_first = true;
        let mut weight = weight;
        if weight.0 >= 400.0 && weight.0 <= 450.0 {
            // c
            if let Some(special) = set.iter().find(|f| f.weight() == FontWeight(500.0)) {
                return Rc::clone(special);
            } else {
                weight = FontWeight(400.0);
            }
        } else if weight.0 <= 500.0 && weight.0 > 450.0 {
            // d
            if let Some(special) = set.iter().find(|f| f.weight() == FontWeight(400.0)) {
                return Rc::clone(special);
            } else {
                weight = FontWeight(500.0);
            }
        } else if weight.0 > 500.0 {
            // b
            descending_first = false;
        } // else a
        
        let wrong_side = |w| if descending_first { w > weight } else { w < weight };
        let mut best = set[0];
        let mut best_dist = f64::MAX;
        for face in set {
            let mut dist = (face.weight().0 - weight.0).abs() as f64;
            if wrong_side(face.weight()) {
                dist += f32::MAX as f64 + 1.0;
            }
            if dist < best_dist {
                best = face;
                best_dist = dist;
            }
        }
        Rc::clone(best)
    }
}

/// Per-window font glyph cache.
#[derive(WindowService)]
pub struct FontRenderCache {
    api: Arc<RenderApi>,
    window_id: WindowId,
    fonts: FnvHashMap<*const FontFace, webrender::api::FontKey>,
    instances: FnvHashMap<*const Font, super::FontInstanceKey>,
}
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
    pub fn get_list(&mut self, font_list: &FontList) -> RenderFontList {
        RenderFontList {
            fonts: font_list.fonts.iter().map(|f| self.get(Rc::clone(f))).collect(),
        }
    }

    /// Gets a [`RenderFont`] cached in the window renderer.
    pub fn get(&mut self, font: FontRef) -> RenderFont {
        #[allow(clippy::mutable_key_type)] // Hash impl for *const T hashes the pointer usize value.
        let fonts = &mut self.fonts;
        let api = &self.api;

        // get or cache the font render key.
        let instance_key = *self.instances.entry(Rc::as_ptr(&font)).or_insert_with(|| {
            // font not cached

            let mut txn = webrender::api::Transaction::new();

            // get or cache the font face render key.
            let font_key = *fonts.entry(Rc::as_ptr(font.face())).or_insert_with(|| {
                // font face not cached

                let font_key = api.generate_font_key();
                txn.add_raw_font(font_key, font.face.h_face.face_data().get_data().into(), font.face.index());
                font_key
            });

            let instance_key = api.generate_font_instance_key();

            let mut opt = webrender::api::FontInstanceOptions::default();
            // TODO
            // if font.face.synthesis.contains(FontSynthesis::STYLE) {
            //     opt.synthetic_italics = webrender::api::SyntheticItalics::enabled();
            // }
            // if font.face.synthesis.contains(FontSynthesis::BOLD) {
            //     opt.flags |= webrender::api::FontInstanceFlags::SYNTHETIC_BOLD;
            // }

            txn.add_font_instance(
                instance_key,
                font_key,
                webrender::api::units::Au::from_f32_px(font.size.get()),
                Some(opt),
                None,
                vec![],
            );

            api.update_resources(txn.resource_updates);

            instance_key
        });

        RenderFont {
            font,
            window_id: self.window_id,
            instance_key,
        }
    }
}

/// A [`Font`] cached in a window renderer.
#[derive(Debug, Clone)]
pub struct RenderFont {
    font: FontRef,
    window_id: WindowId,
    instance_key: super::FontInstanceKey,
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
}

pub struct RenderFontList {
    fonts: Box<[RenderFont]>,
}

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
    /// The font is loaded in [`Fonts::register`].
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
    /// The font is loaded in [`Fonts::register`].
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
    /// The font is loaded in [`Fonts::register`].
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

impl From<font_kit::family_name::FamilyName> for FontName {
    #[inline]
    fn from(family_name: font_kit::family_name::FamilyName) -> Self {
        match family_name {
            font_kit::family_name::FamilyName::Title(title) => FontName::new(title),
            font_kit::family_name::FamilyName::Serif => FontName::serif(),
            font_kit::family_name::FamilyName::SansSerif => FontName::sans_serif(),
            font_kit::family_name::FamilyName::Monospace => FontName::monospace(),
            font_kit::family_name::FamilyName::Cursive => FontName::cursive(),
            font_kit::family_name::FamilyName::Fantasy => FontName::fantasy(),
        }
    }
}
impl From<FontName> for font_kit::family_name::FamilyName {
    fn from(font_name: FontName) -> Self {
        match font_name.name() {
            "serif" => font_kit::family_name::FamilyName::Serif,
            "sans-serif" => font_kit::family_name::FamilyName::SansSerif,
            "monospace" => font_kit::family_name::FamilyName::Monospace,
            "cursive" => font_kit::family_name::FamilyName::Cursive,
            "fantasy" => font_kit::family_name::FamilyName::Fantasy,
            _ => font_kit::family_name::FamilyName::Title(font_name.text.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_font() {
        let _empty = FontFace::empty();
    }
}
