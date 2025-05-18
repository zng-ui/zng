#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Font loading, text segmenting and shaping.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use font_features::RFontVariations;
use hashbrown::{HashMap, HashSet};
use std::{borrow::Cow, fmt, ops, path::PathBuf, slice::SliceIndex, sync::Arc};

#[macro_use]
extern crate bitflags;

pub mod font_features;

mod query_util;

mod emoji_util;
pub use emoji_util::*;

mod ligature_util;
use ligature_util::*;

mod unicode_bidi_util;

mod segmenting;
pub use segmenting::*;

mod shaping;
pub use shaping::*;
use zng_clone_move::{async_clmv, clmv};

mod hyphenation;
pub use self::hyphenation::*;

mod unit;
pub use unit::*;

use parking_lot::{Mutex, RwLock};
use pastey::paste;
use zng_app::{
    AppExtension,
    event::{event, event_args},
    render::FontSynthesis,
    update::{EventUpdate, UPDATES},
    view_process::{
        VIEW_PROCESS_INITED_EVENT, ViewRenderer,
        raw_events::{RAW_FONT_AA_CHANGED_EVENT, RAW_FONT_CHANGED_EVENT},
    },
};
use zng_app_context::app_local;
use zng_ext_l10n::{Lang, LangMap, lang};
use zng_layout::unit::{
    EQ_EPSILON, EQ_EPSILON_100, Factor, FactorPercent, Px, PxPoint, PxRect, PxSize, TimeUnits as _, about_eq, about_eq_hash, about_eq_ord,
    euclid,
};
use zng_task as task;
use zng_txt::Txt;
use zng_var::{
    AnyVar, ArcVar, IntoVar, LocalVar, ResponderVar, ResponseVar, Var, animation::Transitionable, impl_from_and_into_var,
    response_done_var, response_var, var,
};
use zng_view_api::config::FontAntiAliasing;

/// Font family name.
///
/// A possible value for the `font_family` property.
///
/// # Case Insensitive
///
/// Font family names are case-insensitive. `"Arial"` and `"ARIAL"` are equal and have the same hash.
#[derive(Clone)]
pub struct FontName {
    txt: Txt,
    is_ascii: bool,
}
impl fmt::Debug for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("FontName")
                .field("txt", &self.txt)
                .field("is_ascii", &self.is_ascii)
                .finish()
        } else {
            write!(f, "{:?}", self.txt)
        }
    }
}
impl PartialEq for FontName {
    fn eq(&self, other: &Self) -> bool {
        self.unicase() == other.unicase()
    }
}
impl Eq for FontName {}
impl PartialEq<str> for FontName {
    fn eq(&self, other: &str) -> bool {
        self.unicase() == unicase::UniCase::<&str>::from(other)
    }
}
impl std::hash::Hash for FontName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.unicase(), state)
    }
}
impl Ord for FontName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self == other {
            // case insensitive eq
            return std::cmp::Ordering::Equal;
        }
        self.txt.cmp(&other.txt)
    }
}
impl PartialOrd for FontName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl FontName {
    fn unicase(&self) -> unicase::UniCase<&str> {
        if self.is_ascii {
            unicase::UniCase::ascii(self)
        } else {
            unicase::UniCase::unicode(self)
        }
    }

    /// New font name from `&'static str`.
    pub const fn from_static(name: &'static str) -> Self {
        FontName {
            txt: Txt::from_static(name),
            is_ascii: {
                // str::is_ascii is not const
                let name_bytes = name.as_bytes();
                let mut i = name_bytes.len();
                let mut is_ascii = true;
                while i > 0 {
                    i -= 1;
                    if !name_bytes[i].is_ascii() {
                        is_ascii = false;
                        break;
                    }
                }
                is_ascii
            },
        }
    }

    /// New font name.
    ///
    /// Note that the inner name value is a [`Txt`] so you can define a font name using `&'static str` or `String`.
    ///
    /// Font names are case insensitive but the input casing is preserved, this casing shows during display and in
    /// the value of [`name`](Self::name).
    ///
    /// [`Txt`]: zng_txt::Txt
    pub fn new(name: impl Into<Txt>) -> Self {
        let txt = name.into();
        FontName {
            is_ascii: txt.is_ascii(),
            txt,
        }
    }

    /// New "serif" font name.
    ///
    /// Serif fonts represent the formal text style for a script.
    pub fn serif() -> Self {
        Self::new("serif")
    }

    /// New "sans-serif" font name.
    ///
    /// Glyphs in sans-serif fonts, are generally low contrast (vertical and horizontal stems have close to the same thickness)
    /// and have stroke endings that are plain — without any flaring, cross stroke, or other ornamentation.
    pub fn sans_serif() -> Self {
        Self::new("sans-serif")
    }

    /// New "monospace" font name.
    ///
    /// The sole criterion of a monospace font is that all glyphs have the same fixed width.
    pub fn monospace() -> Self {
        Self::new("monospace")
    }

    /// New "cursive" font name.
    ///
    /// Glyphs in cursive fonts generally use a more informal script style, and the result looks more
    /// like handwritten pen or brush writing than printed letter-work.
    pub fn cursive() -> Self {
        Self::new("cursive")
    }

    /// New "fantasy" font name.
    ///
    /// Fantasy fonts are primarily decorative or expressive fonts that contain decorative or expressive representations of characters.
    pub fn fantasy() -> Self {
        Self::new("fantasy")
    }

    /// Reference the font name string.
    pub fn name(&self) -> &str {
        &self.txt
    }

    /// Unwraps into a [`Txt`].
    ///
    /// [`Txt`]: zng_txt::Txt
    pub fn into_text(self) -> Txt {
        self.txt
    }
}
impl_from_and_into_var! {
    fn from(s: &'static str) -> FontName {
        FontName::new(s)
    }
    fn from(s: String) -> FontName {
        FontName::new(s)
    }
    fn from(s: Cow<'static, str>) -> FontName {
        FontName::new(s)
    }
    fn from(f: FontName) -> Txt {
        f.into_text()
    }
}
impl fmt::Display for FontName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}
impl std::ops::Deref for FontName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.txt.deref()
    }
}
impl AsRef<str> for FontName {
    fn as_ref(&self) -> &str {
        self.txt.as_ref()
    }
}
impl serde::Serialize for FontName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.txt.serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for FontName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Txt::deserialize(deserializer).map(FontName::new)
    }
}

/// A list of [font names](FontName) in priority order.
///
/// # Examples
///
/// This type is usually initialized using conversion:
///
/// ```
/// # use zng_ext_font::*;
/// fn foo(font_names: impl Into<FontNames>) { }
///
/// foo(["Arial", "sans-serif", "monospace"]);
/// ```
///
/// You can also use the specialized [`push`](Self::push) that converts:
///
/// ```
/// # use zng_ext_font::*;
/// let user_preference = "Comic Sans".to_owned();
///
/// let mut names = FontNames::empty();
/// names.push(user_preference);
/// names.push("Arial");
/// names.extend(FontNames::default());
/// ```
///
/// # Default
///
/// The default value is the [`system_ui`](FontNames::system_ui) for the undefined language (`und`).
#[derive(Eq, PartialEq, Hash, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FontNames(pub Vec<FontName>);
impl FontNames {
    /// Empty list.
    pub fn empty() -> Self {
        FontNames(vec![])
    }

    /// Returns the default UI font names for Windows.
    pub fn windows_ui(lang: &Lang) -> Self {
        // source: VSCode
        // https://github.com/microsoft/vscode/blob/6825c886700ac11d07f7646d8d8119c9cdd9d288/src/vs/code/electron-sandbox/processExplorer/media/processExplorer.css

        if lang!("zh-Hans").matches(lang, true, false) {
            ["Segoe UI", "Microsoft YaHei", "Segoe Ui Emoji", "sans-serif"].into()
        } else if lang!("zh-Hant").matches(lang, true, false) {
            ["Segoe UI", "Microsoft Jhenghei", "Segoe Ui Emoji", "sans-serif"].into()
        } else if lang!(ja).matches(lang, true, false) {
            ["Segoe UI", "Yu Gothic UI", "Meiryo UI", "Segoe Ui Emoji", "sans-serif"].into()
        } else if lang!(ko).matches(lang, true, false) {
            ["Segoe UI", "Malgun Gothic", "Dotom", "Segoe Ui Emoji", "sans-serif"].into()
        } else {
            ["Segoe UI", "Segoe Ui Emoji", "sans-serif"].into()
        }
    }

    /// Returns the default UI font names for MacOS/iOS.
    pub fn mac_ui(lang: &Lang) -> Self {
        // source: VSCode

        if lang!("zh-Hans").matches(lang, true, false) {
            ["PingFang SC", "Hiragino Sans GB", "Apple Color Emoji", "sans-serif"].into()
        } else if lang!("zh-Hant").matches(lang, true, false) {
            ["PingFang TC", "Apple Color Emoji", "sans-serif"].into()
        } else if lang!(ja).matches(lang, true, false) {
            ["Hiragino Kaku Gothic Pro", "Apple Color Emoji", "sans-serif"].into()
        } else if lang!(ko).matches(lang, true, false) {
            [
                "Nanum Gothic",
                "Apple SD Gothic Neo",
                "AppleGothic",
                "Apple Color Emoji",
                "sans-serif",
            ]
            .into()
        } else {
            ["Neue Helvetica", "Lucida Grande", "Apple Color Emoji", "sans-serif"].into()
        }
    }

    /// Returns the default UI font names for Linux.
    pub fn linux_ui(lang: &Lang) -> Self {
        // source: VSCode

        if lang!("zh-Hans").matches(lang, true, false) {
            [
                "Ubuntu",
                "Droid Sans",
                "Source Han Sans SC",
                "Source Han Sans CN",
                "Source Han Sans",
                "Noto Color Emoji",
                "sans-serif",
            ]
            .into()
        } else if lang!("zh-Hant").matches(lang, true, false) {
            [
                "Ubuntu",
                "Droid Sans",
                "Source Han Sans TC",
                "Source Han Sans TW",
                "Source Han Sans",
                "Noto Color Emoji",
                "sans-serif",
            ]
            .into()
        } else if lang!(ja).matches(lang, true, false) {
            [
                "system-ui",
                "Ubuntu",
                "Droid Sans",
                "Source Han Sans J",
                "Source Han Sans JP",
                "Source Han Sans",
                "Noto Color Emoji",
                "sans-serif",
            ]
            .into()
        } else if lang!(ko).matches(lang, true, false) {
            [
                "system-ui",
                "Ubuntu",
                "Droid Sans",
                "Source Han Sans K",
                "Source Han Sans JR",
                "Source Han Sans",
                "UnDotum",
                "FBaekmuk Gulim",
                "Noto Color Emoji",
                "sans-serif",
            ]
            .into()
        } else {
            ["system-ui", "Ubuntu", "Droid Sans", "Noto Color Emoji", "sans-serif"].into()
        }
    }

    /// Returns the default UI font names for the current operating system.
    pub fn system_ui(lang: &Lang) -> Self {
        if cfg!(windows) {
            Self::windows_ui(lang)
        } else if cfg!(target_os = "linux") {
            Self::linux_ui(lang)
        } else if cfg!(target_os = "macos") {
            Self::mac_ui(lang)
        } else {
            [FontName::sans_serif()].into()
        }
    }

    /// Push a font name from any type that converts to [`FontName`].
    pub fn push(&mut self, font_name: impl Into<FontName>) {
        self.0.push(font_name.into())
    }
}
impl Default for FontNames {
    fn default() -> Self {
        Self::system_ui(&Lang::default())
    }
}
impl fmt::Debug for FontNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("FontNames").field(&self.0).finish()
        } else if self.0.is_empty() {
            write!(f, "[]")
        } else if self.0.len() == 1 {
            write!(f, "{:?}", self.0[0])
        } else {
            write!(f, "[{:?}, ", self.0[0])?;
            for name in &self.0[1..] {
                write!(f, "{name:?}, ")?;
            }
            write!(f, "]")
        }
    }
}
impl fmt::Display for FontNames {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.0.iter();

        if let Some(name) = iter.next() {
            write!(f, "{name}")?;
            for name in iter {
                write!(f, ", {name}")?;
            }
        }

        Ok(())
    }
}
impl_from_and_into_var! {
    fn from(font_name: &'static str) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_name: String) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_name: Txt) -> FontNames {
        FontNames(vec![FontName::new(font_name)])
    }

    fn from(font_names: Vec<FontName>) -> FontNames {
        FontNames(font_names)
    }

    fn from(font_names: Vec<&'static str>) -> FontNames {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }

    fn from(font_names: Vec<String>) -> FontNames {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }

    fn from(font_name: FontName) -> FontNames {
        FontNames(vec![font_name])
    }
}
impl ops::Deref for FontNames {
    type Target = Vec<FontName>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl ops::DerefMut for FontNames {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl std::iter::Extend<FontName> for FontNames {
    fn extend<T: IntoIterator<Item = FontName>>(&mut self, iter: T) {
        self.0.extend(iter)
    }
}
impl IntoIterator for FontNames {
    type Item = FontName;

    type IntoIter = std::vec::IntoIter<FontName>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<const N: usize> From<[FontName; N]> for FontNames {
    fn from(font_names: [FontName; N]) -> Self {
        FontNames(font_names.into())
    }
}
impl<const N: usize> IntoVar<FontNames> for [FontName; N] {
    type Var = LocalVar<FontNames>;

    fn into_var(self) -> Self::Var {
        LocalVar(self.into())
    }
}
impl<const N: usize> From<[&'static str; N]> for FontNames {
    fn from(font_names: [&'static str; N]) -> Self {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [&'static str; N] {
    type Var = LocalVar<FontNames>;

    fn into_var(self) -> Self::Var {
        LocalVar(self.into())
    }
}
impl<const N: usize> From<[String; N]> for FontNames {
    fn from(font_names: [String; N]) -> Self {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [String; N] {
    type Var = LocalVar<FontNames>;

    fn into_var(self) -> Self::Var {
        LocalVar(self.into())
    }
}
impl<const N: usize> From<[Txt; N]> for FontNames {
    fn from(font_names: [Txt; N]) -> Self {
        FontNames(font_names.into_iter().map(FontName::new).collect())
    }
}
impl<const N: usize> IntoVar<FontNames> for [Txt; N] {
    type Var = LocalVar<FontNames>;

    fn into_var(self) -> Self::Var {
        LocalVar(self.into())
    }
}

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
    ///
    /// [`FONTS.register`]: FONTS::register
    /// [`FONTS.unregister`]: FONTS::unregister
    CustomFonts,

    /// Custom request caused by call to [`FONTS.refresh`].
    ///
    /// [`FONTS.refresh`]: FONTS::refresh
    Refresh,

    /// One of the [`GenericFonts`] was set for the language.
    ///
    /// The font name is one of [`FontName`] generic names.
    ///
    /// [`GenericFonts`]: struct@GenericFonts
    GenericFont(FontName, Lang),

    /// A new [fallback](GenericFonts::fallback) font was set for the language.
    Fallback(Lang),
}

/// Application extension that manages text fonts.
///
/// Services this extension provides:
///
/// * [`FONTS`] - Service that finds and loads fonts.
/// * [`HYPHENATION`] - Service that loads and applies hyphenation dictionaries.
///
/// Events this extension provides:
///
/// * [`FONT_CHANGED_EVENT`] - Font config or system fonts changed.
#[derive(Default)]
#[non_exhaustive]
pub struct FontManager {}
impl AppExtension for FontManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if RAW_FONT_CHANGED_EVENT.has(update) {
            FONT_CHANGED_EVENT.notify(FontChangedArgs::now(FontChange::SystemFonts));
        } else if let Some(args) = RAW_FONT_AA_CHANGED_EVENT.on(update) {
            FONTS_SV.read().font_aa.set(args.aa);
        } else if FONT_CHANGED_EVENT.has(update) {
            FONTS_SV.write().on_fonts_changed();
        } else if let Some(args) = VIEW_PROCESS_INITED_EVENT.on(update) {
            let mut fonts = FONTS_SV.write();
            fonts.font_aa.set(args.font_aa);
            if args.is_respawn {
                fonts.loader.on_view_process_respawn();
            }
        }
    }

    fn update(&mut self) {
        let mut fonts = FONTS_SV.write();

        {
            let mut f = GENERIC_FONTS_SV.write();
            for request in std::mem::take(&mut f.requests) {
                request(&mut f);
            }
        }

        let mut changed = false;
        for (request, responder) in std::mem::take(&mut fonts.loader.unregister_requests) {
            let r = if let Some(removed) = fonts.loader.custom_fonts.remove(&request) {
                // cut circular reference so that when the last font ref gets dropped
                // this font face also gets dropped. Also tag the font as unregistered
                // so it does not create further circular references.
                for removed in removed {
                    removed.on_refresh();
                }

                changed = true;

                true
            } else {
                false
            };
            responder.respond(r);
        }
        if changed {
            FONT_CHANGED_EVENT.notify(FontChangedArgs::now(FontChange::CustomFonts));
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
}

/// Font loading, custom fonts and app font configuration.
pub struct FONTS;
impl FONTS {
    /// Clear cache and notify `Refresh` in [`FONT_CHANGED_EVENT`].
    ///
    /// See the event documentation for more information.
    pub fn refresh(&self) {
        FONT_CHANGED_EVENT.notify(FontChangedArgs::now(FontChange::Refresh));
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
    /// The returned response will update once when the font finishes loading with the new font.
    /// At minimum the new font will be available on the next update.
    pub fn register(&self, custom_font: CustomFont) -> ResponseVar<Result<FontFace, FontLoadingError>> {
        FontFaceLoader::register(custom_font)
    }

    /// Removes a custom font family. If the font faces are not in use it is also unloaded.
    ///
    /// Returns a response var that updates once with a value that indicates if any custom font was removed.
    pub fn unregister(&self, custom_family: FontName) -> ResponseVar<bool> {
        FONTS_SV.write().loader.unregister(custom_family)
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

    /// Find a single font face with italic style, normal weight and stretch.
    pub fn italic(&self, family: &FontName, lang: &Lang) -> ResponseVar<Option<FontFace>> {
        self.find(family, FontStyle::Italic, FontWeight::NORMAL, FontStretch::NORMAL, lang)
    }

    /// Find a single font face with bold weight, normal style and stretch.
    pub fn bold(&self, family: &FontName, lang: &Lang) -> ResponseVar<Option<FontFace>> {
        self.find(family, FontStyle::Normal, FontWeight::BOLD, FontStretch::NORMAL, lang)
    }

    /// Gets all [registered](Self::register) font families.
    pub fn custom_fonts(&self) -> Vec<FontName> {
        FONTS_SV.read().loader.custom_fonts.keys().cloned().collect()
    }

    /// Query all font families available in the system.
    ///
    /// Note that the variable will only update once with the query result, this is not a live view.
    pub fn system_fonts(&self) -> ResponseVar<Vec<FontName>> {
        query_util::system_all()
    }

    /// Gets the system font anti-aliasing config as a read-only var.
    ///
    /// The variable updates when the system config changes.
    pub fn system_font_aa(&self) -> impl Var<FontAntiAliasing> {
        FONTS_SV.read().font_aa.read_only()
    }
}

impl<'a> From<ttf_parser::Face<'a>> for FontFaceMetrics {
    fn from(f: ttf_parser::Face<'a>) -> Self {
        let underline = f
            .underline_metrics()
            .unwrap_or(ttf_parser::LineMetrics { position: 0, thickness: 0 });
        FontFaceMetrics {
            units_per_em: f.units_per_em() as _,
            ascent: f.ascender() as f32,
            descent: f.descender() as f32,
            line_gap: f.line_gap() as f32,
            underline_position: underline.position as f32,
            underline_thickness: underline.thickness as f32,
            cap_height: f.capital_height().unwrap_or(0) as f32,
            x_height: f.x_height().unwrap_or(0) as f32,
            bounds: euclid::rect(
                f.global_bounding_box().x_min as f32,
                f.global_bounding_box().x_max as f32,
                f.global_bounding_box().width() as f32,
                f.global_bounding_box().height() as f32,
            ),
        }
    }
}

#[derive(PartialEq, Eq, Hash)]
struct FontInstanceKey(Px, Box<[(ttf_parser::Tag, i32)]>);
impl FontInstanceKey {
    /// Returns the key.
    pub fn new(size: Px, variations: &[rustybuzz::Variation]) -> Self {
        let variations_key: Vec<_> = variations.iter().map(|p| (p.tag, (p.value * 1000.0) as i32)).collect();
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
    face_index: u32,
    display_name: FontName,
    family_name: FontName,
    postscript_name: Option<Txt>,
    style: FontStyle,
    weight: FontWeight,
    stretch: FontStretch,
    metrics: FontFaceMetrics,
    color_palettes: ColorPalettes,
    color_glyphs: ColorGlyphs,
    lig_carets: LigatureCaretList,
    flags: FontFaceFlags,
    m: Mutex<FontFaceMut>,
}
bitflags! {
    #[derive(Debug, Clone, Copy)]
    struct FontFaceFlags: u8 {
        const IS_MONOSPACE =      0b0000_0001;
        const HAS_LIGATURES =     0b0000_0010;
        const HAS_RASTER_IMAGES = 0b0000_0100;
        const HAS_SVG_IMAGES =    0b0000_1000;
    }
}
struct FontFaceMut {
    instances: HashMap<FontInstanceKey, Font>,
    render_ids: Vec<RenderFontFace>,
    unregistered: bool,
}

impl fmt::Debug for FontFace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let m = self.0.m.lock();
        f.debug_struct("FontFace")
            .field("display_name", &self.0.display_name)
            .field("family_name", &self.0.family_name)
            .field("postscript_name", &self.0.postscript_name)
            .field("flags", &self.0.flags)
            .field("style", &self.0.style)
            .field("weight", &self.0.weight)
            .field("stretch", &self.0.stretch)
            .field("metrics", &self.0.metrics)
            .field("color_palettes.len()", &self.0.color_palettes.len())
            .field("color_glyphs.len()", &self.0.color_glyphs.len())
            .field("instances.len()", &m.instances.len())
            .field("render_keys.len()", &m.render_ids.len())
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
            face_index: 0,
            display_name: FontName::from("<empty>"),
            family_name: FontName::from("<empty>"),
            postscript_name: None,
            flags: FontFaceFlags::IS_MONOSPACE,
            style: FontStyle::Normal,
            weight: FontWeight::NORMAL,
            stretch: FontStretch::NORMAL,
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
            color_palettes: ColorPalettes::empty(),
            color_glyphs: ColorGlyphs::empty(),
            lig_carets: LigatureCaretList::empty(),
            m: Mutex::new(FontFaceMut {
                instances: HashMap::default(),
                render_ids: vec![],
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
        let mut face_index;

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
                        face_index: other_font.0.face_index,
                        display_name: custom_font.name.clone(),
                        family_name: custom_font.name,
                        postscript_name: None,
                        style: other_font.0.style,
                        weight: other_font.0.weight,
                        stretch: other_font.0.stretch,
                        metrics: other_font.0.metrics.clone(),
                        m: Mutex::new(FontFaceMut {
                            instances: Default::default(),
                            render_ids: Default::default(),
                            unregistered: Default::default(),
                        }),
                        color_palettes: other_font.0.color_palettes.clone(),
                        color_glyphs: other_font.0.color_glyphs.clone(),
                        lig_carets: other_font.0.lig_carets.clone(),
                        flags: other_font.0.flags,
                    }))),
                    None => Err(FontLoadingError::NoSuchFontInCollection),
                };
            }
        }

        let ttf_face = match ttf_parser::Face::parse(&bytes, face_index) {
            Ok(f) => f,
            Err(e) => {
                match e {
                    // try again with font 0 (font-kit selects a high index for Ubuntu Font)
                    ttf_parser::FaceParsingError::FaceIndexOutOfBounds => face_index = 0,
                    e => return Err(FontLoadingError::Parse(e)),
                }

                match ttf_parser::Face::parse(&bytes, face_index) {
                    Ok(f) => f,
                    Err(_) => return Err(FontLoadingError::Parse(e)),
                }
            }
        };

        let color_palettes = ColorPalettes::load(ttf_face.raw_face())?;
        let color_glyphs = if color_palettes.is_empty() {
            ColorGlyphs::empty()
        } else {
            ColorGlyphs::load(ttf_face.raw_face())?
        };
        let has_ligatures = ttf_face.tables().gsub.is_some();
        let lig_carets = if has_ligatures {
            LigatureCaretList::empty()
        } else {
            LigatureCaretList::load(ttf_face.raw_face())?
        };

        // all tables used by `ttf_parser::Face::glyph_raster_image`
        let has_raster_images = {
            let t = ttf_face.tables();
            t.sbix.is_some() || t.bdat.is_some() || t.ebdt.is_some() || t.cbdt.is_some()
        };

        let mut flags = FontFaceFlags::empty();
        flags.set(FontFaceFlags::IS_MONOSPACE, ttf_face.is_monospaced());
        flags.set(FontFaceFlags::HAS_LIGATURES, has_ligatures);
        flags.set(FontFaceFlags::HAS_RASTER_IMAGES, has_raster_images);
        flags.set(FontFaceFlags::HAS_SVG_IMAGES, ttf_face.tables().svg.is_some());

        Ok(FontFace(Arc::new(LoadedFontFace {
            face_index,
            display_name: custom_font.name.clone(),
            family_name: custom_font.name,
            postscript_name: None,
            style: custom_font.style,
            weight: custom_font.weight,
            stretch: custom_font.stretch,
            metrics: ttf_face.into(),
            color_palettes,
            color_glyphs,
            lig_carets,
            m: Mutex::new(FontFaceMut {
                instances: Default::default(),
                render_ids: Default::default(),
                unregistered: Default::default(),
            }),
            data: bytes,
            flags,
        })))
    }

    fn load(bytes: FontDataRef, mut face_index: u32) -> Result<Self, FontLoadingError> {
        let _span = tracing::trace_span!("FontFace::load").entered();

        let ttf_face = match ttf_parser::Face::parse(&bytes, face_index) {
            Ok(f) => f,
            Err(e) => {
                match e {
                    // try again with font 0 (font-kit selects a high index for Ubuntu Font)
                    ttf_parser::FaceParsingError::FaceIndexOutOfBounds => face_index = 0,
                    e => return Err(FontLoadingError::Parse(e)),
                }

                match ttf_parser::Face::parse(&bytes, face_index) {
                    Ok(f) => f,
                    Err(_) => return Err(FontLoadingError::Parse(e)),
                }
            }
        };

        let color_palettes = ColorPalettes::load(ttf_face.raw_face())?;
        let color_glyphs = if color_palettes.is_empty() {
            ColorGlyphs::empty()
        } else {
            ColorGlyphs::load(ttf_face.raw_face())?
        };

        let has_ligatures = ttf_face.tables().gsub.is_some();
        let lig_carets = if has_ligatures {
            LigatureCaretList::empty()
        } else {
            LigatureCaretList::load(ttf_face.raw_face())?
        };

        let mut display_name = None;
        let mut family_name = None;
        let mut postscript_name = None;
        let mut any_name = None::<String>;
        for name in ttf_face.names() {
            if let Some(n) = name.to_string() {
                match name.name_id {
                    ttf_parser::name_id::FULL_NAME => display_name = Some(n),
                    ttf_parser::name_id::FAMILY => family_name = Some(n),
                    ttf_parser::name_id::POST_SCRIPT_NAME => postscript_name = Some(n),
                    _ => match &mut any_name {
                        Some(s) => {
                            if n.len() > s.len() {
                                *s = n;
                            }
                        }
                        None => any_name = Some(n),
                    },
                }
            }
        }
        let display_name = FontName::new(Txt::from_str(
            display_name
                .as_ref()
                .or(family_name.as_ref())
                .or(postscript_name.as_ref())
                .or(any_name.as_ref())
                .unwrap(),
        ));
        let family_name = family_name.map(FontName::from).unwrap_or_else(|| display_name.clone());
        let postscript_name = postscript_name.map(Txt::from);

        if ttf_face.units_per_em() == 0 {
            // observed this in Noto Color Emoji (with font_kit)
            tracing::debug!("font {display_name:?} units_per_em 0");
            return Err(FontLoadingError::UnknownFormat);
        }

        // all tables used by `ttf_parser::Face::glyph_raster_image`
        let has_raster_images = {
            let t = ttf_face.tables();
            t.sbix.is_some() || t.bdat.is_some() || t.ebdt.is_some() || t.cbdt.is_some()
        };

        let mut flags = FontFaceFlags::empty();
        flags.set(FontFaceFlags::IS_MONOSPACE, ttf_face.is_monospaced());
        flags.set(FontFaceFlags::HAS_LIGATURES, has_ligatures);
        flags.set(FontFaceFlags::HAS_RASTER_IMAGES, has_raster_images);
        flags.set(FontFaceFlags::HAS_SVG_IMAGES, ttf_face.tables().svg.is_some());

        Ok(FontFace(Arc::new(LoadedFontFace {
            face_index,
            family_name,
            display_name,
            postscript_name,
            style: ttf_face.style().into(),
            weight: ttf_face.weight().into(),
            stretch: ttf_face.width().into(),
            metrics: ttf_face.into(),
            color_palettes,
            color_glyphs,
            lig_carets,
            m: Mutex::new(FontFaceMut {
                instances: Default::default(),
                render_ids: Default::default(),
                unregistered: Default::default(),
            }),
            data: bytes,
            flags,
        })))
    }

    fn on_refresh(&self) {
        let mut m = self.0.m.lock();
        m.instances.clear();
        m.unregistered = true;
    }

    fn render_face(&self, renderer: &ViewRenderer) -> zng_view_api::font::FontFaceId {
        let mut m = self.0.m.lock();
        for r in m.render_ids.iter() {
            if &r.renderer == renderer {
                return r.face_id;
            }
        }

        let key = match renderer.add_font_face((*self.0.data.0).clone(), self.0.face_index) {
            Ok(k) => k,
            Err(_) => {
                tracing::debug!("respawned calling `add_font`, will return dummy font key");
                return zng_view_api::font::FontFaceId::INVALID;
            }
        };

        m.render_ids.push(RenderFontFace::new(renderer, key));

        key
    }

    /// Loads the harfbuzz face.
    ///
    /// Loads from in memory [`bytes`].
    ///
    /// Returns `None` if [`is_empty`].
    ///
    /// [`is_empty`]: Self::is_empty
    /// [`bytes`]: Self::bytes
    pub fn harfbuzz(&self) -> Option<rustybuzz::Face> {
        if self.is_empty() {
            None
        } else {
            Some(rustybuzz::Face::from_slice(&self.0.data.0, self.0.face_index).unwrap())
        }
    }

    /// Loads the full TTF face.
    ///
    /// Loads from in memory [`bytes`].
    ///
    /// Returns `None` if [`is_empty`].
    ///
    /// [`is_empty`]: Self::is_empty
    /// [`bytes`]: Self::bytes
    pub fn ttf(&self) -> Option<ttf_parser::Face> {
        if self.is_empty() {
            None
        } else {
            Some(ttf_parser::Face::parse(&self.0.data.0, self.0.face_index).unwrap())
        }
    }

    /// Reference the font file bytes.
    pub fn bytes(&self) -> &FontDataRef {
        &self.0.data
    }
    /// Index of the font face in the [font file](Self::bytes).
    pub fn index(&self) -> u32 {
        self.0.face_index
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

    /// Font style.
    pub fn style(&self) -> FontStyle {
        self.0.style
    }

    /// Font weight.
    pub fn weight(&self) -> FontWeight {
        self.0.weight
    }

    /// Font stretch.
    pub fn stretch(&self) -> FontStretch {
        self.0.stretch
    }

    /// Font is monospace (fixed-width).
    pub fn is_monospace(&self) -> bool {
        self.0.flags.contains(FontFaceFlags::IS_MONOSPACE)
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
    /// [font variations]: crate::font_features::FontVariations::finalize
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
            synth |= FontSynthesis::OBLIQUE;
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

    /// CPAL table.
    ///
    /// Is empty if not provided by the font.
    pub fn color_palettes(&self) -> &ColorPalettes {
        &self.0.color_palettes
    }

    /// COLR table.
    ///
    /// Is empty if not provided by the font.
    pub fn color_glyphs(&self) -> &ColorGlyphs {
        &self.0.color_glyphs
    }

    /// If the font provides glyph substitutions.
    pub fn has_ligatures(&self) -> bool {
        self.0.flags.contains(FontFaceFlags::HAS_LIGATURES)
    }

    /// If this font provides custom positioned carets for some or all ligature glyphs.
    ///
    /// If `true` the [`Font::ligature_caret_offsets`] method can be used to get the caret offsets, otherwise
    /// it always returns empty.
    pub fn has_ligature_caret_offsets(&self) -> bool {
        !self.0.lig_carets.is_empty()
    }

    /// If this font has bitmap images associated with some glyphs.
    pub fn has_raster_images(&self) -> bool {
        self.0.flags.contains(FontFaceFlags::HAS_RASTER_IMAGES)
    }

    /// If this font has SVG images associated with some glyphs.
    pub fn has_svg_images(&self) -> bool {
        self.0.flags.contains(FontFaceFlags::HAS_SVG_IMAGES)
    }
}

/// A sized font face.
///
/// A sized font can be requested from a [`FontFace`].
///
/// This type is a shared reference to the loaded font data, cloning it is cheap.
#[derive(Clone)]
pub struct Font(Arc<LoadedFont>);
struct LoadedFont {
    face: FontFace,
    size: Px,
    variations: RFontVariations,
    metrics: FontMetrics,
    render_keys: Mutex<Vec<RenderFont>>,
    small_word_cache: RwLock<HashMap<WordCacheKey<[u8; Font::SMALL_WORD_LEN]>, ShapedSegmentData>>,
    word_cache: RwLock<HashMap<WordCacheKey<String>, ShapedSegmentData>>,
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
    const SMALL_WORD_LEN: usize = 8;

    fn to_small_word(s: &str) -> Option<[u8; Self::SMALL_WORD_LEN]> {
        if s.len() <= Self::SMALL_WORD_LEN {
            let mut a = [b'\0'; Self::SMALL_WORD_LEN];
            a[..s.len()].copy_from_slice(s.as_bytes());
            Some(a)
        } else {
            None
        }
    }

    fn new(face: FontFace, size: Px, variations: RFontVariations) -> Self {
        Font(Arc::new(LoadedFont {
            metrics: face.metrics().sized(size),
            face,
            size,
            variations,
            render_keys: Mutex::new(vec![]),
            small_word_cache: RwLock::default(),
            word_cache: RwLock::default(),
        }))
    }

    fn render_font(&self, renderer: &ViewRenderer, synthesis: FontSynthesis) -> zng_view_api::font::FontId {
        let _span = tracing::trace_span!("Font::render_font").entered();

        let mut render_keys = self.0.render_keys.lock();
        for r in render_keys.iter() {
            if &r.renderer == renderer && r.synthesis == synthesis {
                return r.font_id;
            }
        }

        let font_key = self.0.face.render_face(renderer);

        let mut opt = zng_view_api::font::FontOptions::default();
        opt.synthetic_oblique = synthesis.contains(FontSynthesis::OBLIQUE);
        opt.synthetic_bold = synthesis.contains(FontSynthesis::BOLD);
        let variations = self.0.variations.iter().map(|v| (v.tag.to_bytes(), v.value)).collect();

        let key = match renderer.add_font(font_key, self.0.size, opt, variations) {
            Ok(k) => k,
            Err(_) => {
                tracing::debug!("respawned calling `add_font_instance`, will return dummy font key");
                return zng_view_api::font::FontId::INVALID;
            }
        };

        render_keys.push(RenderFont::new(renderer, synthesis, key));

        key
    }

    /// Reference the font face source of this font.
    pub fn face(&self) -> &FontFace {
        &self.0.face
    }

    /// Gets the sized harfbuzz font.
    pub fn harfbuzz(&self) -> Option<rustybuzz::Face> {
        let ppem = self.0.size.0 as u16;

        let mut font = self.0.face.harfbuzz()?;

        font.set_pixels_per_em(Some((ppem, ppem)));
        font.set_variations(&self.0.variations);

        Some(font)
    }

    /// Font size.
    ///
    /// This is also the *pixels-per-em* value.
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

    /// Iterate over pixel offsets relative to `lig` glyph start that represents the
    /// caret offset for each cluster that is covered by the ligature, after the first.
    ///
    /// The caret offset for the first cluster is the glyph offset and is not yielded in the iterator. The
    /// yielded offsets are relative to the glyph position.
    pub fn ligature_caret_offsets(
        &self,
        lig: zng_view_api::font::GlyphIndex,
    ) -> impl ExactSizeIterator<Item = f32> + DoubleEndedIterator + '_ {
        let face = &self.0.face.0;
        face.lig_carets.carets(lig).iter().map(move |&o| match o {
            ligature_util::LigatureCaret::Coordinate(o) => {
                let size_scale = 1.0 / face.metrics.units_per_em as f32 * self.0.size.0 as f32;
                o as f32 * size_scale
            }
            ligature_util::LigatureCaret::GlyphContourPoint(i) => {
                if let Some(f) = self.harfbuzz() {
                    struct Search {
                        i: u16,
                        s: u16,
                        x: f32,
                    }
                    impl Search {
                        fn check(&mut self, x: f32) {
                            self.s = self.s.saturating_add(1);
                            if self.s == self.i {
                                self.x = x;
                            }
                        }
                    }
                    impl ttf_parser::OutlineBuilder for Search {
                        fn move_to(&mut self, x: f32, _y: f32) {
                            self.check(x);
                        }

                        fn line_to(&mut self, x: f32, _y: f32) {
                            self.check(x);
                        }

                        fn quad_to(&mut self, _x1: f32, _y1: f32, x: f32, _y: f32) {
                            self.check(x)
                        }

                        fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, x: f32, _y: f32) {
                            self.check(x);
                        }

                        fn close(&mut self) {}
                    }
                    let mut search = Search { i, s: 0, x: 0.0 };
                    if f.outline_glyph(ttf_parser::GlyphId(lig as _), &mut search).is_some() && search.s >= search.i {
                        return search.x * self.0.metrics.size_scale;
                    }
                }
                0.0
            }
        })
    }
}
impl zng_app::render::Font for Font {
    fn is_empty_fallback(&self) -> bool {
        self.face().is_empty()
    }

    fn renderer_id(&self, renderer: &ViewRenderer, synthesis: FontSynthesis) -> zng_view_api::font::FontId {
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
    /// This is at least `1`, but can be the empty face.
    pub fn len(&self) -> usize {
        self.fonts.len()
    }

    /// Is length `1` and only contains the empty face.
    pub fn is_empty(&self) -> bool {
        self.fonts[0].is_empty() && self.fonts.len() == 1
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
#[expect(clippy::len_without_is_empty)] // cannot be empty.
impl FontList {
    /// The font that best matches the requested properties.
    pub fn best(&self) -> &Font {
        &self.fonts[0]
    }

    /// Font size requested in the query that generated this font list.
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
    unregister_requests: Vec<(FontName, ResponderVar<bool>)>,

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
            unregister_requests: vec![],
            system_fonts_cache: HashMap::new(),
            list_cache: HashMap::new(),
        }
    }

    fn on_view_process_respawn(&mut self) {
        let sys_fonts = self.system_fonts_cache.values().flatten().filter_map(|f| f.result.rsp().flatten());
        for face in self.custom_fonts.values().flatten().cloned().chain(sys_fonts) {
            let mut m = face.0.m.lock();
            m.render_ids.clear();
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

    fn register(custom_font: CustomFont) -> ResponseVar<Result<FontFace, FontLoadingError>> {
        // start loading
        let resp = task::respond(FontFace::load_custom(custom_font));

        // modify loader.custom_fonts at the end of whatever update is happening when finishes loading.
        resp.hook(|args| {
            if let Some(done) = args.value().done() {
                if let Ok(face) = done {
                    let mut fonts = FONTS_SV.write();
                    let family = fonts.loader.custom_fonts.entry(face.0.family_name.clone()).or_default();
                    let existing = family
                        .iter()
                        .position(|f| f.0.weight == face.0.weight && f.0.style == face.0.style && f.0.stretch == face.0.stretch);

                    if let Some(i) = existing {
                        family[i] = face.clone();
                    } else {
                        family.push(face.clone());
                    }

                    FONT_CHANGED_EVENT.notify(FontChangedArgs::now(FontChange::CustomFonts));
                }
                false
            } else {
                true
            }
        })
        .perm();
        resp
    }

    fn unregister(&mut self, custom_family: FontName) -> ResponseVar<bool> {
        let (responder, response) = response_var();

        if !self.unregister_requests.is_empty() {
            UPDATES.update(None);
        }
        self.unregister_requests.push((custom_family, responder));

        response
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

        let mut list = Vec::with_capacity(families.len() + 1);
        let mut pending = vec![];

        {
            let fallback = [GenericFonts {}.fallback(lang)];
            let mut used = HashSet::with_capacity(families.len());
            for name in families.iter().chain(&fallback) {
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
        }

        let r = if pending.is_empty() {
            if list.is_empty() {
                tracing::error!(target: "font_loading", "failed to load fallback font");
                list.push(FontFace::empty());
            }
            response_done_var(FontFaceList {
                fonts: list.into_boxed_slice(),
                requested_style: style,
                requested_weight: weight,
                requested_stretch: stretch,
            })
        } else {
            task::respond(async move {
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
            })
        };

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

        let load = task::wait(clmv!(font_name, || {
            let (bytes, face_index) = match Self::get_system(&font_name, style, weight, stretch) {
                Some(h) => h,
                None => {
                    #[cfg(debug_assertions)]
                    static NOT_FOUND: Mutex<Option<HashSet<FontName>>> = Mutex::new(None);

                    #[cfg(debug_assertions)]
                    if NOT_FOUND.lock().get_or_insert_with(HashSet::default).insert(font_name.clone()) {
                        tracing::debug!(r#"font "{font_name}" not found"#);
                    }

                    return None;
                }
            };
            match FontFace::load(bytes, face_index) {
                Ok(f) => Some(f),
                Err(FontLoadingError::UnknownFormat) => None,
                Err(e) => {
                    tracing::error!(target: "font_loading", "failed to load system font, {e}\nquery: {:?}", (font_name, style, weight, stretch));
                    None
                }
            }
        }));
        let result = task::respond(async_clmv!(font_name, {
            match task::with_deadline(load, 10.secs()).await {
                Ok(r) => r,
                Err(_) => {
                    tracing::error!(target: "font_loading", "timeout loading {font_name:?}");
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

    fn get_system(font_name: &FontName, style: FontStyle, weight: FontWeight, stretch: FontStretch) -> Option<(FontDataRef, u32)> {
        let _span = tracing::trace_span!("FontFaceLoader::get_system").entered();
        match query_util::best(font_name, style, weight, stretch) {
            Ok(r) => r,
            Err(e) => {
                tracing::error!("cannot get `{font_name}` system font, {e}");
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
            |face: &FontFace, weight: FontWeight, dist: &mut f64| {
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
            |face: &FontFace, weight: FontWeight, dist: &mut f64| {
                if face.weight() > weight {
                    *dist += weight.0 as f64;
                }
            }
        } else {
            debug_assert!(weight.0 > 500.0);
            // b:
            |face: &FontFace, weight: FontWeight, dist: &mut f64| {
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
    face_id: zng_view_api::font::FontFaceId,
}
impl RenderFontFace {
    fn new(renderer: &ViewRenderer, face_id: zng_view_api::font::FontFaceId) -> Self {
        RenderFontFace {
            renderer: renderer.clone(),
            face_id,
        }
    }
}
impl Drop for RenderFontFace {
    fn drop(&mut self) {
        // error here means the entire renderer was already dropped.
        let _ = self.renderer.delete_font_face(self.face_id);
    }
}

struct RenderFont {
    renderer: ViewRenderer,
    synthesis: FontSynthesis,
    font_id: zng_view_api::font::FontId,
}
impl RenderFont {
    fn new(renderer: &ViewRenderer, synthesis: FontSynthesis, font_id: zng_view_api::font::FontId) -> RenderFont {
        RenderFont {
            renderer: renderer.clone(),
            synthesis,
            font_id,
        }
    }
}
impl Drop for RenderFont {
    fn drop(&mut self) {
        // error here means the entire renderer was already dropped.
        let _ = self.renderer.delete_font(self.font_id);
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

    requests: Vec<Box<dyn FnOnce(&mut GenericFontsService) + Send + Sync>>,
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

            requests: vec![],
        }
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
/// [`FontNames::system_ui`]: crate::FontNames::system_ui
#[non_exhaustive]
pub struct GenericFonts {}
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
    /// The change applied for the next update.
    ///
    /// Use `lang!(und)` to set name used when no language matches.
    pub fn [<set_ $name>]<F: Into<FontName>>(&self, lang: Lang, font_name: F) {
        let mut g = GENERIC_FONTS_SV.write();
        let font_name = font_name.into();
        if g.requests.is_empty() {
            UPDATES.update(None);
        }
        g.requests.push(Box::new(move |g| {
            g.$name.insert(lang.clone(), font_name);
            FONT_CHANGED_EVENT.notify(FontChangedArgs::now(FontChange::GenericFont(FontName::$name(), lang)));
        }));
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
    /// The change applies for the next update.
    ///
    /// Use `lang!(und)` to set name used when no language matches.
    pub fn set_fallback<F: Into<FontName>>(&self, lang: Lang, font_name: F) {
        let mut g = GENERIC_FONTS_SV.write();
        if g.requests.is_empty() {
            UPDATES.update(None);
        }
        let font_name = font_name.into();
        g.requests.push(Box::new(move |g| {
            FONT_CHANGED_EVENT.notify(FontChangedArgs::now(FontChange::Fallback(lang.clone())));
            g.fallback.insert(lang, font_name);
        }));
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

#[derive(Debug, Clone)]
enum FontSource {
    File(PathBuf, u32),
    Memory(FontDataRef, u32),
    Alias(FontName),
}

/// Custom font builder.
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
    ///
    /// [`FONTS.register`]: FONTS::register
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
    ///
    /// [`FONTS.register`]: FONTS::register
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
    ///
    /// [`FONTS.register`]: FONTS::register
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

/// The width of a font as an approximate fraction of the normal width.
///
/// Widths range from 0.5 to 2.0 inclusive, with 1.0 as the normal width.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, Transitionable)]
#[serde(transparent)]
pub struct FontStretch(pub f32);
impl fmt::Debug for FontStretch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if name.is_empty() {
            f.debug_tuple("FontStretch").field(&self.0).finish()
        } else {
            if f.alternate() {
                write!(f, "FontStretch::")?;
            }
            write!(f, "{name}")
        }
    }
}
impl PartialOrd for FontStretch {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(about_eq_ord(self.0, other.0, EQ_EPSILON))
    }
}
impl PartialEq for FontStretch {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_EPSILON)
    }
}
impl std::hash::Hash for FontStretch {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EQ_EPSILON, state)
    }
}
impl Default for FontStretch {
    fn default() -> FontStretch {
        FontStretch::NORMAL
    }
}
impl FontStretch {
    /// Ultra-condensed width (50%), the narrowest possible.
    pub const ULTRA_CONDENSED: FontStretch = FontStretch(0.5);
    /// Extra-condensed width (62.5%).
    pub const EXTRA_CONDENSED: FontStretch = FontStretch(0.625);
    /// Condensed width (75%).
    pub const CONDENSED: FontStretch = FontStretch(0.75);
    /// Semi-condensed width (87.5%).
    pub const SEMI_CONDENSED: FontStretch = FontStretch(0.875);
    /// Normal width (100%).
    pub const NORMAL: FontStretch = FontStretch(1.0);
    /// Semi-expanded width (112.5%).
    pub const SEMI_EXPANDED: FontStretch = FontStretch(1.125);
    /// Expanded width (125%).
    pub const EXPANDED: FontStretch = FontStretch(1.25);
    /// Extra-expanded width (150%).
    pub const EXTRA_EXPANDED: FontStretch = FontStretch(1.5);
    /// Ultra-expanded width (200%), the widest possible.
    pub const ULTRA_EXPANDED: FontStretch = FontStretch(2.0);

    /// Gets the const name, if this value is one of the constants.
    pub fn name(self) -> &'static str {
        macro_rules! name {
            ($($CONST:ident;)+) => {$(
                if self == Self::$CONST {
                    return stringify!($CONST);
                }
            )+}
        }
        name! {
            ULTRA_CONDENSED;
            EXTRA_CONDENSED;
            CONDENSED;
            SEMI_CONDENSED;
            NORMAL;
            SEMI_EXPANDED;
            EXPANDED;
            EXTRA_EXPANDED;
            ULTRA_EXPANDED;
        }
        ""
    }
}
impl_from_and_into_var! {
    fn from(fct: Factor) -> FontStretch {
        FontStretch(fct.0)
    }
    fn from(pct: FactorPercent) -> FontStretch {
        FontStretch(pct.fct().0)
    }
    fn from(fct: f32) -> FontStretch {
        FontStretch(fct)
    }
}
impl From<ttf_parser::Width> for FontStretch {
    fn from(value: ttf_parser::Width) -> Self {
        use ttf_parser::Width::*;
        match value {
            UltraCondensed => FontStretch::ULTRA_CONDENSED,
            ExtraCondensed => FontStretch::EXTRA_CONDENSED,
            Condensed => FontStretch::CONDENSED,
            SemiCondensed => FontStretch::SEMI_CONDENSED,
            Normal => FontStretch::NORMAL,
            SemiExpanded => FontStretch::SEMI_EXPANDED,
            Expanded => FontStretch::EXPANDED,
            ExtraExpanded => FontStretch::EXTRA_EXPANDED,
            UltraExpanded => FontStretch::ULTRA_EXPANDED,
        }
    }
}
impl From<FontStretch> for ttf_parser::Width {
    fn from(value: FontStretch) -> Self {
        if value <= FontStretch::ULTRA_CONDENSED {
            ttf_parser::Width::UltraCondensed
        } else if value <= FontStretch::EXTRA_CONDENSED {
            ttf_parser::Width::ExtraCondensed
        } else if value <= FontStretch::CONDENSED {
            ttf_parser::Width::Condensed
        } else if value <= FontStretch::SEMI_CONDENSED {
            ttf_parser::Width::SemiCondensed
        } else if value <= FontStretch::NORMAL {
            ttf_parser::Width::Normal
        } else if value <= FontStretch::SEMI_EXPANDED {
            ttf_parser::Width::SemiExpanded
        } else if value <= FontStretch::EXPANDED {
            ttf_parser::Width::Expanded
        } else if value <= FontStretch::EXTRA_EXPANDED {
            ttf_parser::Width::ExtraExpanded
        } else {
            ttf_parser::Width::UltraExpanded
        }
    }
}

/// The italic or oblique form of a font.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum FontStyle {
    /// The regular form.
    #[default]
    Normal,
    /// A form that is generally cursive in nature.
    Italic,
    /// A skewed version of the regular form.
    Oblique,
}
impl fmt::Debug for FontStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "FontStyle::")?;
        }
        match self {
            Self::Normal => write!(f, "Normal"),
            Self::Italic => write!(f, "Italic"),
            Self::Oblique => write!(f, "Oblique"),
        }
    }
}
impl From<ttf_parser::Style> for FontStyle {
    fn from(value: ttf_parser::Style) -> Self {
        use ttf_parser::Style::*;
        match value {
            Normal => FontStyle::Normal,
            Italic => FontStyle::Italic,
            Oblique => FontStyle::Oblique,
        }
    }
}

impl From<FontStyle> for ttf_parser::Style {
    fn from(value: FontStyle) -> Self {
        match value {
            FontStyle::Normal => Self::Normal,
            FontStyle::Italic => Self::Italic,
            FontStyle::Oblique => Self::Oblique,
        }
    }
}

/// The degree of stroke thickness of a font. This value ranges from 100.0 to 900.0,
/// with 400.0 as normal.
#[derive(Clone, Copy, Transitionable, serde::Serialize, serde::Deserialize)]
pub struct FontWeight(pub f32);
impl Default for FontWeight {
    fn default() -> FontWeight {
        FontWeight::NORMAL
    }
}
impl fmt::Debug for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name();
        if name.is_empty() {
            f.debug_tuple("FontWeight").field(&self.0).finish()
        } else {
            if f.alternate() {
                write!(f, "FontWeight::")?;
            }
            write!(f, "{name}")
        }
    }
}
impl PartialOrd for FontWeight {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(about_eq_ord(self.0, other.0, EQ_EPSILON_100))
    }
}
impl PartialEq for FontWeight {
    fn eq(&self, other: &Self) -> bool {
        about_eq(self.0, other.0, EQ_EPSILON_100)
    }
}
impl std::hash::Hash for FontWeight {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        about_eq_hash(self.0, EQ_EPSILON_100, state)
    }
}
impl FontWeight {
    /// Thin weight (100), the thinnest value.
    pub const THIN: FontWeight = FontWeight(100.0);
    /// Extra light weight (200).
    pub const EXTRA_LIGHT: FontWeight = FontWeight(200.0);
    /// Light weight (300).
    pub const LIGHT: FontWeight = FontWeight(300.0);
    /// Normal (400).
    pub const NORMAL: FontWeight = FontWeight(400.0);
    /// Medium weight (500, higher than normal).
    pub const MEDIUM: FontWeight = FontWeight(500.0);
    /// Semi-bold weight (600).
    pub const SEMIBOLD: FontWeight = FontWeight(600.0);
    /// Bold weight (700).
    pub const BOLD: FontWeight = FontWeight(700.0);
    /// Extra-bold weight (800).
    pub const EXTRA_BOLD: FontWeight = FontWeight(800.0);
    /// Black weight (900), the thickest value.
    pub const BLACK: FontWeight = FontWeight(900.0);

    /// Gets the const name, if this value is one of the constants.
    pub fn name(self) -> &'static str {
        macro_rules! name {
                ($($CONST:ident;)+) => {$(
                    if self == Self::$CONST {
                        return stringify!($CONST);
                    }
                )+}
            }
        name! {
            THIN;
            EXTRA_LIGHT;
            LIGHT;
            NORMAL;
            MEDIUM;
            SEMIBOLD;
            BOLD;
            EXTRA_BOLD;
            BLACK;
        }
        ""
    }
}
impl_from_and_into_var! {
    fn from(weight: u32) -> FontWeight {
        FontWeight(weight as f32)
    }
    fn from(weight: f32) -> FontWeight {
        FontWeight(weight)
    }
}
impl From<ttf_parser::Weight> for FontWeight {
    fn from(value: ttf_parser::Weight) -> Self {
        use ttf_parser::Weight::*;
        match value {
            Thin => FontWeight::THIN,
            ExtraLight => FontWeight::EXTRA_LIGHT,
            Light => FontWeight::LIGHT,
            Normal => FontWeight::NORMAL,
            Medium => FontWeight::MEDIUM,
            SemiBold => FontWeight::SEMIBOLD,
            Bold => FontWeight::BOLD,
            ExtraBold => FontWeight::EXTRA_BOLD,
            Black => FontWeight::BLACK,
            Other(o) => FontWeight(o as f32),
        }
    }
}
impl From<FontWeight> for ttf_parser::Weight {
    fn from(value: FontWeight) -> Self {
        ttf_parser::Weight::from(value.0 as u16)
    }
}

/// Configuration of text wrapping for Chinese, Japanese, or Korean text.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LineBreak {
    /// The same rule used by other languages.
    Auto,
    /// The least restrictive rule, good for short lines.
    Loose,
    /// The most common rule.
    Normal,
    /// The most stringent rule.
    Strict,
    /// Allow line breaks in between any character including punctuation.
    Anywhere,
}
impl Default for LineBreak {
    /// [`LineBreak::Auto`]
    fn default() -> Self {
        LineBreak::Auto
    }
}
impl fmt::Debug for LineBreak {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "LineBreak::")?;
        }
        match self {
            LineBreak::Auto => write!(f, "Auto"),
            LineBreak::Loose => write!(f, "Loose"),
            LineBreak::Normal => write!(f, "Normal"),
            LineBreak::Strict => write!(f, "Strict"),
            LineBreak::Anywhere => write!(f, "Anywhere"),
        }
    }
}

/// Hyphenation mode.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Hyphens {
    /// Hyphens are never inserted in word breaks.
    None,
    /// Word breaks only happen in specially marked break characters: `-` and `\u{00AD} SHY`.
    ///
    /// * `U+2010` - The visible hyphen character.
    /// * `U+00AD` - The invisible hyphen character, is made visible in a word break.
    Manual,
    /// Hyphens are inserted like `Manual` and also using language specific hyphenation rules.
    Auto,
}
impl Default for Hyphens {
    /// [`Hyphens::Auto`]
    fn default() -> Self {
        Hyphens::Auto
    }
}
impl fmt::Debug for Hyphens {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Hyphens::")?;
        }
        match self {
            Hyphens::None => write!(f, "None"),
            Hyphens::Manual => write!(f, "Manual"),
            Hyphens::Auto => write!(f, "Auto"),
        }
    }
}

/// Configure line breaks inside words during text wrap.
///
/// This value is only considered if it is impossible to fit a full word to a line.
///
/// Hyphens can be inserted in word breaks using the [`Hyphens`] configuration.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WordBreak {
    /// Line breaks can be inserted in between letters of Chinese/Japanese/Korean text only.
    Normal,
    /// Line breaks can be inserted between any letter.
    BreakAll,
    /// Line breaks are not inserted between any letter.
    KeepAll,
}
impl Default for WordBreak {
    /// [`WordBreak::Normal`]
    fn default() -> Self {
        WordBreak::Normal
    }
}
impl fmt::Debug for WordBreak {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WordBreak::")?;
        }
        match self {
            WordBreak::Normal => write!(f, "Normal"),
            WordBreak::BreakAll => write!(f, "BreakAll"),
            WordBreak::KeepAll => write!(f, "KeepAll"),
        }
    }
}

/// Text alignment justification mode.
#[derive(Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Justify {
    /// Selects the justification mode based on the language.
    ///
    /// For Chinese/Japanese/Korean uses `InterLetter` for the others uses `InterWord`.
    Auto,
    /// The text is justified by adding space between words.
    InterWord,
    /// The text is justified by adding space between letters.
    InterLetter,
}
impl Default for Justify {
    /// [`Justify::Auto`]
    fn default() -> Self {
        Justify::Auto
    }
}
impl Justify {
    /// Resolve `Auto` for the given language.
    pub fn resolve(self, lang: &Lang) -> Self {
        match self {
            Self::Auto => match lang.language.as_str() {
                "zh" | "ja" | "ko" => Self::InterLetter,
                _ => Self::InterWord,
            },
            m => m,
        }
    }
}
impl fmt::Debug for Justify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "Justify::")?;
        }
        match self {
            Justify::Auto => write!(f, "Auto"),
            Justify::InterWord => write!(f, "InterWord"),
            Justify::InterLetter => write!(f, "InterLetter"),
        }
    }
}

/// Various metrics that apply to the entire [`FontFace`].
///
/// For OpenType fonts, these mostly come from the `OS/2` table.
///
/// See the [`FreeType Glyph Metrics`] documentation for an explanation of the various metrics.
///
/// [`FreeType Glyph Metrics`]: https://freetype.org/freetype2/docs/glyphs/glyphs-3.html
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct FontFaceMetrics {
    /// The number of font units per em.
    ///
    /// Font sizes are usually expressed in pixels per em; e.g. `12px` means 12 pixels per em.
    pub units_per_em: u32,

    /// The maximum amount the font rises above the baseline, in font units.
    pub ascent: f32,

    /// The maximum amount the font descends below the baseline, in font units.
    ///
    /// This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: f32,

    /// Distance between baselines, in font units.
    pub line_gap: f32,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in font units.
    pub underline_position: f32,

    /// A suggested value for the underline thickness, in font units.
    pub underline_thickness: f32,

    /// The approximate amount that uppercase letters rise above the baseline, in font units.
    pub cap_height: f32,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: f32,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in font units.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounds: euclid::Rect<f32, ()>,
}
impl FontFaceMetrics {
    /// Compute [`FontMetrics`] given a font size in pixels.
    pub fn sized(&self, font_size_px: Px) -> FontMetrics {
        let size_scale = 1.0 / self.units_per_em as f32 * font_size_px.0 as f32;
        let s = move |f: f32| Px((f * size_scale).round() as i32);
        FontMetrics {
            size_scale,
            ascent: s(self.ascent),
            descent: s(self.descent),
            line_gap: s(self.line_gap),
            underline_position: s(self.underline_position),
            underline_thickness: s(self.underline_thickness),
            cap_height: s(self.cap_height),
            x_height: (s(self.x_height)),
            bounds: {
                let b = self.bounds;
                PxRect::new(
                    PxPoint::new(s(b.origin.x), s(b.origin.y)),
                    PxSize::new(s(b.size.width), s(b.size.height)),
                )
            },
        }
    }
}

/// Various metrics about a [`Font`].
///
/// You can compute these metrics from a [`FontFaceMetrics`]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct FontMetrics {
    /// Multiply this to a font EM value to get the size in pixels.
    pub size_scale: f32,

    /// The maximum amount the font rises above the baseline, in pixels.
    pub ascent: Px,

    /// The maximum amount the font descends below the baseline, in pixels.
    ///
    /// This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: Px,

    /// Distance between baselines, in pixels.
    pub line_gap: Px,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in pixels.
    pub underline_position: Px,

    /// A suggested value for the underline thickness, in pixels.
    pub underline_thickness: Px,

    /// The approximate amount that uppercase letters rise above the baseline, in pixels.
    pub cap_height: Px,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in pixels.
    pub x_height: Px,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in pixels.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounds: PxRect,
}
impl FontMetrics {
    /// The font line height.
    pub fn line_height(&self) -> Px {
        self.ascent - self.descent + self.line_gap
    }
}

/// Text transform function.
#[derive(Clone)]
pub enum TextTransformFn {
    /// No transform.
    None,
    /// To UPPERCASE.
    Uppercase,
    /// to lowercase.
    Lowercase,
    /// Custom transform function.
    Custom(Arc<dyn Fn(&Txt) -> Cow<Txt> + Send + Sync>),
}
impl TextTransformFn {
    /// Apply the text transform.
    ///
    /// Returns [`Cow::Owned`] if the text was changed.
    pub fn transform<'t>(&self, text: &'t Txt) -> Cow<'t, Txt> {
        match self {
            TextTransformFn::None => Cow::Borrowed(text),
            TextTransformFn::Uppercase => {
                if text.chars().any(|c| !c.is_uppercase()) {
                    Cow::Owned(text.to_uppercase().into())
                } else {
                    Cow::Borrowed(text)
                }
            }
            TextTransformFn::Lowercase => {
                if text.chars().any(|c| !c.is_lowercase()) {
                    Cow::Owned(text.to_lowercase().into())
                } else {
                    Cow::Borrowed(text)
                }
            }
            TextTransformFn::Custom(fn_) => fn_(text),
        }
    }

    /// New [`Custom`](Self::Custom).
    pub fn custom(fn_: impl Fn(&Txt) -> Cow<Txt> + Send + Sync + 'static) -> Self {
        TextTransformFn::Custom(Arc::new(fn_))
    }
}
impl fmt::Debug for TextTransformFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "TextTransformFn::")?;
        }
        match self {
            TextTransformFn::None => write!(f, "None"),
            TextTransformFn::Uppercase => write!(f, "Uppercase"),
            TextTransformFn::Lowercase => write!(f, "Lowercase"),
            TextTransformFn::Custom(_) => write!(f, "Custom"),
        }
    }
}
impl PartialEq for TextTransformFn {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Custom(l0), Self::Custom(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}

/// Text white space transform.
#[derive(Copy, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum WhiteSpace {
    /// Text is not changed, all white spaces and line breaks are preserved.
    Preserve,
    /// Replace white spaces with a single `U+0020 SPACE` and trim lines. Line breaks are preserved.
    Merge,
    /// Replace white spaces and line breaks with `U+0020 SPACE` and trim the text.
    MergeAll,
}
impl Default for WhiteSpace {
    /// [`WhiteSpace::Preserve`].
    fn default() -> Self {
        WhiteSpace::Preserve
    }
}
impl WhiteSpace {
    /// Transform the white space of the text.
    ///
    /// Returns [`Cow::Owned`] if the text was changed.
    pub fn transform(self, text: &Txt) -> Cow<Txt> {
        match self {
            WhiteSpace::Preserve => Cow::Borrowed(text),
            WhiteSpace::Merge => {
                let is_white_space = |c: char| c.is_whitespace() && !"\n\r\u{85}".contains(c);
                let t = text.trim_matches(is_white_space);

                let mut prev_space = false;
                for c in t.chars() {
                    if is_white_space(c) {
                        if prev_space || c != '\u{20}' {
                            // collapse spaces or replace non ' ' white space with ' '.

                            let mut r = String::new();
                            let mut sep = "";
                            for part in t.split(is_white_space).filter(|s| !s.is_empty()) {
                                r.push_str(sep);
                                r.push_str(part);
                                sep = "\u{20}";
                            }
                            return Cow::Owned(Txt::from_str(&r));
                        } else {
                            prev_space = true;
                        }
                    } else {
                        prev_space = false;
                    }
                }

                if t.len() != text.len() {
                    Cow::Owned(Txt::from_str(t))
                } else {
                    Cow::Borrowed(text)
                }
            }
            WhiteSpace::MergeAll => {
                let t = text.trim();

                let mut prev_space = false;
                for c in t.chars() {
                    if c.is_whitespace() {
                        if prev_space || c != '\u{20}' {
                            // collapse spaces or replace non ' ' white space with ' '.

                            let mut r = String::new();
                            let mut sep = "";
                            for part in t.split_whitespace() {
                                r.push_str(sep);
                                r.push_str(part);
                                sep = "\u{20}";
                            }
                            return Cow::Owned(Txt::from_str(&r));
                        } else {
                            prev_space = true;
                        }
                    } else {
                        prev_space = false;
                    }
                }

                if t.len() != text.len() {
                    Cow::Owned(Txt::from_str(t))
                } else {
                    Cow::Borrowed(text)
                }
            }
        }
    }
}
impl fmt::Debug for WhiteSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WhiteSpace::")?;
        }
        match self {
            WhiteSpace::Preserve => write!(f, "Preserve"),
            WhiteSpace::Merge => write!(f, "Merge"),
            WhiteSpace::MergeAll => write!(f, "MergeAll"),
        }
    }
}

/// Defines an insert offset in a shaped text.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct CaretIndex {
    /// Char byte offset in the full text.
    ///
    /// This index can be computed using the [`SegmentedText`].
    pub index: usize,
    /// Line index in the shaped text.
    ///
    /// This value is only used to disambiguate between the *end* of a wrap and
    /// the *start* of the next, the text itself does not have any line
    /// break but visually the user interacts with two lines. Note that this
    /// counts wrap lines, and that this value is not required to define a valid
    /// CaretIndex.
    ///
    /// This index can be computed using the [`ShapedText::snap_caret_line`].
    pub line: usize,
}

impl PartialEq for CaretIndex {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}
impl Eq for CaretIndex {}
impl CaretIndex {
    /// First position.
    pub const ZERO: CaretIndex = CaretIndex { index: 0, line: 0 };
}
impl PartialOrd for CaretIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for CaretIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.index.cmp(&other.index)
    }
}

/// Reasons why a loader might fail to load a font.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum FontLoadingError {
    /// The data was of a format the loader didn't recognize.
    UnknownFormat,
    /// Attempted to load an invalid index in a TrueType or OpenType font collection.
    ///
    /// For example, if a `.ttc` file has 2 fonts in it, and you ask for the 5th one, you'll get
    /// this error.
    NoSuchFontInCollection,
    /// Attempted to load a malformed or corrupted font.
    Parse(ttf_parser::FaceParsingError),
    /// Attempted to load a font from the filesystem, but there is no filesystem (e.g. in
    /// WebAssembly).
    NoFilesystem,
    /// A disk or similar I/O error occurred while attempting to load the font.
    Io(Arc<std::io::Error>),
}
impl PartialEq for FontLoadingError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Io(l0), Self::Io(r0)) => Arc::ptr_eq(l0, r0),
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
impl From<std::io::Error> for FontLoadingError {
    fn from(error: std::io::Error) -> FontLoadingError {
        Self::Io(Arc::new(error))
    }
}
impl fmt::Display for FontLoadingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFormat => write!(f, "unknown format"),
            Self::NoSuchFontInCollection => write!(f, "no such font in the collection"),
            Self::NoFilesystem => write!(f, "no filesystem present"),
            Self::Parse(e) => fmt::Display::fmt(e, f),
            Self::Io(e) => fmt::Display::fmt(e, f),
        }
    }
}
impl std::error::Error for FontLoadingError {
    fn cause(&self) -> Option<&dyn std::error::Error> {
        match self {
            FontLoadingError::Parse(e) => Some(e),
            FontLoadingError::Io(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use zng_app::APP;

    use super::*;

    #[test]
    fn generic_fonts_default() {
        let _app = APP.minimal().extend(FontManager::default()).run_headless(false);

        assert_eq!(FontName::sans_serif(), GenericFonts {}.sans_serif(&lang!(und)))
    }

    #[test]
    fn generic_fonts_fallback() {
        let _app = APP.minimal().extend(FontManager::default()).run_headless(false);

        assert_eq!(FontName::sans_serif(), GenericFonts {}.sans_serif(&lang!(en_US)));
        assert_eq!(FontName::sans_serif(), GenericFonts {}.sans_serif(&lang!(es)));
    }

    #[test]
    fn generic_fonts_get1() {
        let mut app = APP.minimal().extend(FontManager::default()).run_headless(false);
        GenericFonts {}.set_sans_serif(lang!(en_US), "Test Value");
        app.update(false).assert_wait();

        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en-US")), "Test Value");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en")), "Test Value");
    }

    #[test]
    fn generic_fonts_get2() {
        let mut app = APP.minimal().extend(FontManager::default()).run_headless(false);
        GenericFonts {}.set_sans_serif(lang!(en), "Test Value");
        app.update(false).assert_wait();

        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en-US")), "Test Value");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en")), "Test Value");
    }

    #[test]
    fn generic_fonts_get_best() {
        let mut app = APP.minimal().extend(FontManager::default()).run_headless(false);
        GenericFonts {}.set_sans_serif(lang!(en), "Test Value");
        GenericFonts {}.set_sans_serif(lang!(en_US), "Best");
        app.update(false).assert_wait();

        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en-US")), "Best");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en")), "Test Value");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("und")), "sans-serif");
    }

    #[test]
    fn generic_fonts_get_no_lang_match() {
        let mut app = APP.minimal().extend(FontManager::default()).run_headless(false);
        GenericFonts {}.set_sans_serif(lang!(es_US), "Test Value");
        app.update(false).assert_wait();

        assert_eq!(&GenericFonts {}.sans_serif(&lang!("en-US")), "sans-serif");
        assert_eq!(&GenericFonts {}.sans_serif(&lang!("es")), "Test Value");
    }
}
