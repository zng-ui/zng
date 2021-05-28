use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

use fnv::FnvHashMap;
use font_kit::properties::Weight;
use webrender::api::RenderApi;

use super::{
    font_features::RFontVariations, FontFaceMetrics, FontMetrics, FontName, FontStretch, FontStyle, FontSynthesis, FontWeight, Script,
};
use crate::app::AppExtension;
use crate::context::{AppContext, AppInitContext, UpdateNotifier, UpdateRequest};
use crate::event::{event, event_args, EventEmitter};
use crate::service::Service;
use crate::units::{layout_to_pt, LayoutLength};

#[cfg(windows)]
use crate::{
    event::EventListener,
    window::{WindowEventArgs, WindowOpenEvent, Windows},
};

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
        fn concerns_widget(&self, _ctx: &mut WidgetContext) -> bool {
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

    /// Custom request caused by call to [`Fonts::refresh`].
    Refesh,

    /// One of the [`GenericFonts`] was set for the script.
    ///
    /// The font name is one of [`FontName`] generic names.
    GenericFont(FontName, Script),

    /// A new [fallback](GenericFonts::fallback) font was set for the script.
    Fallback(Script),
}

/// Application extension that manages text fonts.
/// # Services
///
/// Services this extension provides:
///
/// * [Fonts] - Service that finds and loads fonts.
///
/// Events this extension provides:
///
/// * [FontChangedEvent] - Font config or system fonts changed.
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
        ctx.services.register(Fonts::new(ctx.updates.notifier().clone()));

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
                } else if fonts.prune_requested {
                    fonts.on_prune();
                }
            }

            #[cfg(windows)]
            if self.window_open.has_updates(ctx.events) {
                // attach subclass WM_FONTCHANGE monitor to new headed windows.
                let windows = ctx.services.req::<Windows>();
                for w in self.window_open.updates(ctx.events) {
                    if let Ok(w) = windows.window(w.window_id) {
                        if w.mode().is_headed() {
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
                                log::error!(target: "font_loading", "failed to set WM_FONTCHANGE subclass monitor");
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Font loading, custom fonts and app font configuration.
#[derive(Service)]
pub struct Fonts {
    loader: FontFaceLoader,
    generics: GenericFonts,
    prune_requested: bool,
}
impl Fonts {
    fn new(notifier: UpdateNotifier) -> Self {
        Fonts {
            loader: FontFaceLoader::new(),
            generics: GenericFonts::new(notifier),
            prune_requested: false,
        }
    }

    #[cfg(windows)]
    fn on_system_fonts_changed(&mut self) {
        self.loader.on_system_fonts_changed();
        self.prune_requested = false;
    }

    #[cfg(windows)]
    fn on_prune(&mut self) {
        self.loader.on_prune();
        self.prune_requested = false;
    }

    #[inline]
    fn take_updates(&mut self) -> Vec<FontChangedArgs> {
        std::mem::take(&mut self.generics.updates)
    }

    /// Clear cache and notify `Refresh` in [`FontChangedEvent`].
    ///
    /// See the event documentation for more information.
    #[inline]
    pub fn refresh(&mut self) {
        self.generics.notify(FontChange::Refesh);
    }

    /// Remove all unused fonts from cache.
    #[inline]
    pub fn prune(&mut self) {
        if !self.prune_requested {
            self.prune_requested = true;
            self.generics.notifier.update();
        }
    }

    /// Actual name of generic fonts.
    #[inline]
    pub fn generics(&self) -> &GenericFonts {
        &self.generics
    }

    /// Configure the actual name of generic fonts.
    #[inline]
    pub fn generics_mut(&mut self) -> &mut GenericFonts {
        &mut self.generics
    }

    /// Load and register a custom font.
    ///
    /// If the font loads correctly a [`FontChangedEvent`] notification is scheduled.
    /// Fonts sourced from a file are not monitored for changes, you can *reload* the font
    /// by calling `register` again with the same font name.
    #[inline]
    pub fn register(&mut self, custom_font: CustomFont) -> Result<(), FontLoadingError> {
        self.loader.register(custom_font)?;
        self.generics.notify(FontChange::CustomFonts);
        Ok(())
    }

    /// Removes a custom font family. If the font faces are not in use it is also unloaded.
    ///
    /// Returns if any was removed.
    #[inline]
    pub fn unregister(&mut self, custom_family: &FontName) -> bool {
        let unregistered = self.loader.unregister(custom_family);
        if unregistered {
            self.generics.notify(FontChange::CustomFonts);
        }
        unregistered
    }

    /// Gets a font list that best matches the query.
    #[inline]
    pub fn get_list(&mut self, families: &[FontName], style: FontStyle, weight: FontWeight, stretch: FontStretch) -> FontFaceList {
        self.loader
            .get_list(families, style, weight, stretch, &self.generics.fallback[&Script::Unknown])
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
            bounding_box: webrender::euclid::rect(
                m.bounding_box.origin_x(),
                m.bounding_box.origin_y(),
                m.bounding_box.width(),
                m.bounding_box.height(),
            ),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct FontInstanceKey(u32, Box<[(rustybuzz::Tag, i32)]>);
impl FontInstanceKey {
    /// Returns the key and adjusts the values to match the key rounding.
    pub fn new(size: &mut LayoutLength, variations: &mut RFontVariations) -> Self {
        let size_key = (size.get() * 2.0) as u32;
        let variations_key: Vec<_> = variations.iter().map(|p| (p.tag, (p.value * 1000.0) as i32)).collect();
        let key = FontInstanceKey(size_key, variations_key.into_boxed_slice());

        *size = LayoutLength::new(key.0 as f32 / 2.0);
        *variations = key
            .1
            .iter()
            .map(|&(n, v)| rustybuzz::Variation {
                tag: n,
                value: (v as f32 / 1000.0),
            })
            .collect();

        key
    }
}

/// A font face selected from a font family.
///
/// Usually this is part of a [`FontList`] that can be requested from
/// the [`Fonts`] service.
pub struct FontFace {
    bytes: Arc<Vec<u8>>,
    face_index: u32,
    display_name: FontName,
    family_name: FontName,
    postscript_name: Option<String>,
    is_monospace: bool,
    properties: font_kit::properties::Properties,
    metrics: FontFaceMetrics,

    instances: RefCell<FnvHashMap<FontInstanceKey, FontRef>>,
    render_keys: RefCell<Vec<RenderFontFace>>,

    unregistered: Cell<bool>,
}

impl fmt::Debug for FontFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FontFace")
            .field("display_name", &self.display_name)
            .field("family_name", &self.family_name)
            .field("postscript_name", &self.postscript_name)
            .field("is_monospace", &self.is_monospace)
            .field("properties", &self.properties)
            .field("metrics", &self.metrics)
            .field("instances.len()", &self.instances.borrow().len())
            .field("render_keys.len()", &self.render_keys.borrow().len())
            .field("unregistered", &self.unregistered.get())
            .finish()
    }
}
impl FontFace {
    fn load_custom(custom_font: CustomFont, loader: &mut FontFaceLoader) -> Result<Self, FontLoadingError> {
        let bytes;
        let face_index;

        match custom_font.source {
            FontSource::File(path, font_index) => {
                bytes = Arc::new(std::fs::read(path)?);
                face_index = font_index;
            }
            FontSource::Memory(arc, font_index) => {
                bytes = arc;
                face_index = font_index;
            }
            FontSource::Alias(other_font) => {
                let other_font = loader
                    .get(&other_font, custom_font.style, custom_font.weight, custom_font.stretch)
                    .ok_or(FontLoadingError::NoSuchFontInCollection)?;
                return Ok(FontFace {
                    bytes: Arc::clone(&other_font.bytes),
                    face_index: other_font.face_index,
                    display_name: custom_font.name.clone(),
                    family_name: custom_font.name,
                    postscript_name: None,
                    properties: other_font.properties,
                    is_monospace: other_font.is_monospace,
                    metrics: other_font.metrics.clone(),
                    instances: Default::default(),
                    render_keys: Default::default(),
                    unregistered: Cell::new(false),
                });
            }
        }

        let kit_font = font_kit::handle::Handle::Memory {
            bytes: Arc::clone(&bytes),
            font_index: face_index,
        }
        .load()?;

        if rustybuzz::Face::from_slice(&bytes, face_index).is_none() {
            return Err(FontLoadingError::Parse);
        }

        Ok(FontFace {
            bytes,
            face_index,
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
            render_keys: Default::default(),
            unregistered: Cell::new(false),
        })
    }

    fn load(handle: font_kit::handle::Handle) -> Result<Self, FontLoadingError> {
        let bytes;
        let face_index;

        match handle {
            font_kit::handle::Handle::Path { path, font_index } => {
                bytes = Arc::new(std::fs::read(path)?);
                face_index = font_index;
            }
            font_kit::handle::Handle::Memory { bytes: vec, font_index } => {
                bytes = vec;
                face_index = font_index;
            }
        };

        let kit_font = font_kit::handle::Handle::Memory {
            bytes: Arc::clone(&bytes),
            font_index: face_index,
        }
        .load()?;

        if rustybuzz::Face::from_slice(&bytes, face_index).is_none() {
            return Err(FontLoadingError::Parse);
        }

        Ok(FontFace {
            bytes,
            face_index,
            display_name: kit_font.full_name().into(),
            family_name: kit_font.family_name().into(),
            postscript_name: kit_font.postscript_name(),
            properties: kit_font.properties(),
            is_monospace: kit_font.is_monospace(),
            metrics: kit_font.metrics().into(),
            instances: Default::default(),
            render_keys: Default::default(),
            unregistered: Cell::new(false),
        })
    }

    fn empty() -> Self {
        FontFace {
            bytes: Arc::new(vec![]),
            face_index: 0,
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
                bounding_box: webrender::euclid::rect(1524.0, 3483.0, -249.0, -1392.0),
            },
            instances: Default::default(),
            render_keys: Default::default(),
            unregistered: Cell::new(false),
        }
    }

    fn on_unregistered(&self) {
        self.instances.borrow_mut().clear();
        self.unregistered.set(true);
    }

    fn render_face(&self, api: &Arc<RenderApi>, txn: &mut webrender::api::Transaction) -> webrender::api::FontKey {
        let namespace = api.get_namespace_id();
        let mut keys = self.render_keys.borrow_mut();
        for r in keys.iter() {
            if r.key.0 == namespace {
                return r.key;
            }
        }

        let key = api.generate_font_key();
        txn.add_raw_font(key, (*self.bytes).clone(), self.face_index);

        keys.push(RenderFontFace::new(api, key));

        key
    }

    /// Reference the underlying font bytes.
    #[inline]
    pub fn bytes(&self) -> &Arc<Vec<u8>> {
        &self.bytes
    }

    /// The font face index in the [font file](Self::bytes).
    #[inline]
    pub fn face_index(&self) -> u32 {
        self.face_index
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

    /// Index of the font face in the [font file](Self::bytes).
    #[inline]
    pub fn index(&self) -> u32 {
        self.face_index
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
    ///
    /// The `font_size` is the size of `1 font EM` in layout pixels.
    ///
    /// The `variations` are custom [font variations](crate::text::font_features::FontVariations::finalize) that will be used
    /// during shaping and rendering.
    pub fn sized(self: &Rc<Self>, mut font_size: LayoutLength, mut variations: RFontVariations) -> FontRef {
        let key = FontInstanceKey::new(&mut font_size, &mut variations);
        if !self.unregistered.get() {
            let mut instances = self.instances.borrow_mut();
            let f = instances
                .entry(key)
                .or_insert_with(|| Rc::new(Font::new(Rc::clone(self), font_size, variations)));
            Rc::clone(f)
        } else {
            log::warn!(target: "font_loading", "creating font from unregistered `{}`, will not cache", self.display_name);
            Rc::new(Font::new(Rc::clone(self), font_size, variations))
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

    /// If both font faces are the same.
    #[inline]
    pub fn ptr_eq(self: &Rc<Self>, other: &Rc<Self>) -> bool {
        Rc::ptr_eq(self, other)
    }
}

/// A shared [`FontFace`].
pub type FontFaceRef = Rc<FontFace>;

/// A sized font face.
///
/// A sized font can be requested from a [`FontFace`].
pub struct Font {
    face: FontFaceRef,
    size: LayoutLength,
    variations: RFontVariations,
    metrics: FontMetrics,
    render_keys: RefCell<Vec<RenderFont>>,
}
impl fmt::Debug for Font {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Font")
            .field("face", &self.face)
            .field("size", &self.size)
            .field("metrics", &self.metrics)
            .field("render_keys.len()", &self.render_keys.borrow().len())
            .finish()
    }
}
impl Font {
    fn new(face: FontFaceRef, size: LayoutLength, variations: RFontVariations) -> Self {
        Font {
            metrics: face.metrics().sized(size.get()),
            face,
            size,
            variations,
            render_keys: Default::default(),
        }
    }

    fn render_font(&self, api: &Arc<RenderApi>, synthesis: FontSynthesis) -> webrender::api::FontInstanceKey {
        let namespace = api.get_namespace_id();
        let mut keys = self.render_keys.borrow_mut();
        for r in keys.iter() {
            if r.key.0 == namespace && r.synthesis == synthesis {
                return r.key;
            }
        }

        let mut txn = webrender::api::Transaction::new();

        let font_key = self.face.render_face(api, &mut txn);

        let key = api.generate_font_instance_key();

        let mut opt = webrender::api::FontInstanceOptions::default();
        if synthesis.contains(FontSynthesis::STYLE) {
            opt.synthetic_italics = webrender::api::SyntheticItalics::enabled();
        }
        if synthesis.contains(FontSynthesis::BOLD) {
            opt.flags |= webrender::api::FontInstanceFlags::SYNTHETIC_BOLD;
        }
        txn.add_font_instance(
            key,
            font_key,
            webrender::api::units::Au::from_f32_px(self.size.get()),
            Some(opt),
            None,
            self.variations
                .iter()
                .map(|v| webrender::api::FontVariation {
                    tag: v.tag.0,
                    value: v.value,
                })
                .collect(),
        );

        api.update_resources(txn.resource_updates);

        keys.push(RenderFont::new(api, synthesis, key));

        key
    }

    /// Reference the font face source of this font.
    #[inline]
    pub fn face(&self) -> &FontFaceRef {
        &self.face
    }

    /// Font size.
    #[inline]
    pub fn size(&self) -> LayoutLength {
        self.size
    }

    /// Custom font variations.
    #[inline]
    pub fn variations(&self) -> &RFontVariations {
        &self.variations
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

    /// If both fonts are the same.
    #[inline]
    pub fn ptr_eq(self: &Rc<Self>, other: &Rc<Self>) -> bool {
        Rc::ptr_eq(self, other)
    }
}
impl crate::render::Font for Font {
    fn instance_key(&self, api: &Arc<RenderApi>, synthesis: FontSynthesis) -> webrender::api::FontInstanceKey {
        // how does cache clear works with this?
        self.render_font(api, synthesis)
    }
}

/// A shared [`Font`].
pub type FontRef = Rc<Font>;

impl crate::render::Font for Rc<Font> {
    fn instance_key(&self, api: &Arc<RenderApi>, synthesis: FontSynthesis) -> webrender::api::FontInstanceKey {
        self.render_font(api, synthesis)
    }
}

/// A list of [`FontFaceRef`] resolved from a [`FontName`] list, plus the [fallback](GenericFonts::fallback) font.
///
/// Glyphs that are not resolved by the first font fallback to the second font and so on.
#[derive(Debug, Clone)]
pub struct FontFaceList {
    fonts: Box<[FontFaceRef]>,
    requested_style: FontStyle,
    requested_weight: FontWeight,
    requested_stretch: FontStretch,
}
#[allow(clippy::len_without_is_empty)] // is never empty.
impl FontFaceList {
    /// Style requested in the query that generated this font face list.
    #[inline]
    pub fn requested_style(&self) -> FontStyle {
        self.requested_style
    }

    /// Weight requested in the query that generated this font face list.
    #[inline]
    pub fn requested_weight(&self) -> FontWeight {
        self.requested_weight
    }

    /// Stretch requested in the query that generated this font face list.
    #[inline]
    pub fn requested_stretch(&self) -> FontStretch {
        self.requested_stretch
    }

    /// The font face that best matches the requested properties.
    #[inline]
    pub fn best(&self) -> &FontFaceRef {
        &self.fonts[0]
    }

    /// Gets the font synthesis to use to better render the given font face on the list.
    #[inline]
    pub fn face_synthesis(&self, face_index: usize) -> FontSynthesis {
        if let Some(face) = self.fonts.get(face_index) {
            face.synthesis_for(self.requested_style, self.requested_weight)
        } else {
            FontSynthesis::DISABLED
        }
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
    ///
    /// This calls [`FontFace::sized`] for each font in the list.
    #[inline]
    pub fn sized(&self, font_size: LayoutLength, variations: RFontVariations) -> FontList {
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
            && self.fonts.iter().zip(other.fonts.iter()).all(|(a, b)| Rc::ptr_eq(a, b))
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
    requested_style: FontStyle,
    requested_weight: FontWeight,
    requested_stretch: FontStretch,
}
#[allow(clippy::len_without_is_empty)] // cannot be empty.
impl FontList {
    /// The font that best matches the requested properties.
    #[inline]
    pub fn best(&self) -> &FontRef {
        &self.fonts[0]
    }

    /// Style requested in the query that generated this font list.
    #[inline]
    pub fn requested_style(&self) -> FontStyle {
        self.requested_style
    }

    /// Weight requested in the query that generated this font list.
    #[inline]
    pub fn requested_weight(&self) -> FontWeight {
        self.requested_weight
    }

    /// Stretch requested in the query that generated this font list.
    #[inline]
    pub fn requested_stretch(&self) -> FontStretch {
        self.requested_stretch
    }

    /// Gets the font synthesis to use to better render the given font on the list.
    #[inline]
    pub fn face_synthesis(&self, font_index: usize) -> FontSynthesis {
        if let Some(font) = self.fonts.get(font_index) {
            font.face.synthesis_for(self.requested_style, self.requested_weight)
        } else {
            FontSynthesis::DISABLED
        }
    }

    /// Iterate over font faces, more specific first.
    #[inline]
    pub fn iter(&self) -> std::slice::Iter<FontRef> {
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
    /// Both are equal if each point to the same fonts in the same order and have the same requested properties.
    fn eq(&self, other: &Self) -> bool {
        self.requested_style == other.requested_style
            && self.requested_weight == other.requested_weight
            && self.requested_stretch == other.requested_stretch
            && self.fonts.len() == other.fonts.len()
            && self.fonts.iter().zip(other.fonts.iter()).all(|(a, b)| Rc::ptr_eq(a, b))
    }
}
impl Eq for FontList {}
impl std::ops::Deref for FontList {
    type Target = [FontRef];

    fn deref(&self) -> &Self::Target {
        &self.fonts
    }
}
impl<'a> std::iter::IntoIterator for &'a FontList {
    type Item = &'a FontRef;

    type IntoIter = std::slice::Iter<'a, FontRef>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl std::ops::Index<usize> for FontList {
    type Output = FontRef;

    fn index(&self, index: usize) -> &Self::Output {
        &self.fonts[index]
    }
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

    #[cfg(windows)]
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
    #[cfg(windows)]
    fn on_prune(&mut self) {
        self.system_fonts_cache.retain(|_, v| {
            v.retain(|sff| match sff {
                SystemFontFace::Found(.., font_face) => Rc::strong_count(font_face) > 1,
                SystemFontFace::NotFound(..) => true,
            });
            !v.is_empty()
        });
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
            log::error!(target: "font_loading", "failed to load fallback font");
            r.push(Rc::new(FontFace::empty()));
        }

        FontFaceList {
            fonts: r.into_boxed_slice(),
            requested_style: style,
            requested_weight: weight,
            requested_stretch: stretch,
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
                    log::error!(target: "font_loading", "failed to load system font, {}", e);
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
                log::error!(target: "font_loading", "failed to select system font, {}", e);
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

        Rc::clone(best)
    }
}

struct RenderFontFace {
    api: std::sync::Weak<RenderApi>,
    key: webrender::api::FontKey,
}
impl RenderFontFace {
    fn new(api: &Arc<RenderApi>, key: webrender::api::FontKey) -> Self {
        RenderFontFace {
            api: Arc::downgrade(api),
            key,
        }
    }
}
impl Drop for RenderFontFace {
    fn drop(&mut self) {
        if let Some(api) = self.api.upgrade() {
            let mut txn = webrender::api::Transaction::new();
            txn.delete_font(self.key);
            api.update_resources(txn.resource_updates);
        }
    }
}

struct RenderFont {
    api: std::sync::Weak<RenderApi>,
    synthesis: FontSynthesis,
    key: webrender::api::FontInstanceKey,
}
impl RenderFont {
    fn new(api: &Arc<RenderApi>, synthesis: FontSynthesis, key: webrender::api::FontInstanceKey) -> RenderFont {
        RenderFont {
            api: Arc::downgrade(api),
            synthesis,
            key,
        }
    }
}
impl Drop for RenderFont {
    fn drop(&mut self) {
        if let Some(api) = self.api.upgrade() {
            let mut txn = webrender::api::Transaction::new();
            txn.delete_font_instance(self.key);
            api.update_resources(txn.resource_updates);
        }
    }
}

/// Generic fonts configuration for the app.
///
/// This type can be accessed from the [`Fonts`] service.
pub struct GenericFonts {
    serif: FnvHashMap<Script, FontName>,
    sans_serif: FnvHashMap<Script, FontName>,
    monospace: FnvHashMap<Script, FontName>,
    cursive: FnvHashMap<Script, FontName>,
    fantasy: FnvHashMap<Script, FontName>,
    fallback: FnvHashMap<Script, FontName>,
    notifier: UpdateNotifier,
    updates: Vec<FontChangedArgs>,
}
impl GenericFonts {
    fn new(notifier: UpdateNotifier) -> Self {
        fn default(name: impl Into<FontName>) -> FnvHashMap<Script, FontName> {
            let mut f = FnvHashMap::with_capacity_and_hasher(1, fnv::FnvBuildHasher::default());
            f.insert(Script::Unknown, name.into());
            f
        }
        GenericFonts {
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
    ($($name:ident),+ $(,)?) => {$($crate::paste! {
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
        self.notify(FontChange::GenericFont(FontName::$name(), script));
        self.$name.insert(script, font_name.into())
    }
    })+};
}
impl GenericFonts {
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
