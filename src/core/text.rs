//! Font resolving and text shaping.

use super::units::{layout_length_to_pt, LayoutLength, LayoutPoint, LayoutRect, LayoutSize};
use crate::core::app::AppExtension;
use crate::core::context::{AppInitContext, WindowService};
use crate::core::types::{FontInstanceKey, FontName, FontProperties, FontStyle};
use crate::core::var::ContextVar;
use crate::properties::text_theme::FontFamilyVar;
use fnv::FnvHashMap;
use std::{borrow::Cow, fmt, mem, rc::Rc};
use std::{collections::hash_map::Entry as HEntry, num::NonZeroU32};
use std::{collections::HashMap, sync::Arc};
use webrender::api::units::Au;
use webrender::api::GlyphInstance;
use webrender::api::{FontKey, RenderApi, Transaction};

pub use unicode_script::{self, Script};

/// Application extension that provides the [`Fonts`] window service.
#[derive(Default)]
pub struct FontManager;

impl AppExtension for FontManager {
    fn init(&mut self, r: &mut AppInitContext) {
        r.window_services.register(|ctx| Fonts {
            api: Arc::clone(ctx.render_api),
            fonts: HashMap::default(),
        })
    }
}

/// Fonts cache service.
pub struct Fonts {
    api: Arc<RenderApi>,
    fonts: HashMap<FontQueryKey, FontInstances>,
}
type FontQueryKey = (Box<[FontName]>, FontPropertiesKey);

/// Font size in round points.
pub type FontSizePt = u32;

/// Convert a [`LayoutLength`] to [`FontSizePt`].
#[inline]
pub fn font_size_from_layout_length(length: LayoutLength) -> FontSizePt {
    layout_length_to_pt(length).round().max(0.0) as u32
}

impl Fonts {
    /// Gets a cached font instance or loads a new instance.
    pub fn get(&mut self, font_names: &[FontName], properties: &FontProperties, font_size: FontSizePt) -> Option<FontInstance> {
        let query_key = (font_names.to_vec().into_boxed_slice(), FontPropertiesKey::new(*properties));
        if let Some(font) = self.fonts.get_mut(&query_key) {
            if let Some(instance) = font.instances.get(&font_size) {
                Some(instance.clone())
            } else {
                Some(Self::load_font_size(&self.api, font, font_size))
            }
        } else if let Some(instance) = self.load_font(query_key, font_names, properties, font_size) {
            Some(instance)
        } else {
            None
        }
    }

    /// Gets a font using [`get`](Self::get) or fallback to the any of the default fonts.
    pub fn get_or_default(&mut self, font_names: &[FontName], properties: &FontProperties, font_size: FontSizePt) -> FontInstance {
        self.get(font_names, properties, font_size)
            .or_else(|| {
                warn_println!("did not found font: {:?}", font_names);
                self.get(FontFamilyVar::default_value(), &FontProperties::default(), font_size)
            })
            .expect("did not find any default font")
    }

    fn load_font(
        &mut self,
        query_key: FontQueryKey,
        font_names: &[FontName],
        properties: &FontProperties,
        size: FontSizePt,
    ) -> Option<FontInstance> {
        let family_names: Vec<font_kit::family_name::FamilyName> = font_names.iter().map(|n| n.clone().into()).collect();
        match font_kit::source::SystemSource::new().select_best_match(&family_names, properties) {
            Ok(handle) => {
                let mut txn = Transaction::new();
                let font_key = self.api.generate_font_key();

                let metrics = {
                    let loader = handle.load().expect("cannot load font [2]");
                    loader.metrics()
                };

                let harfbuzz_face = match handle {
                    font_kit::handle::Handle::Path { path, font_index } => {
                        let r = harfbuzz_rs::Face::from_file(&path, font_index).expect("cannot load font [1]");
                        txn.add_native_font(font_key, webrender::api::NativeFontHandle { path, index: font_index });
                        r
                    }
                    font_kit::handle::Handle::Memory { bytes, font_index } => {
                        let blob = harfbuzz_rs::Blob::with_bytes_owned(Arc::clone(&bytes), |a| &*a);
                        let r = harfbuzz_rs::Face::new(blob, font_index);
                        txn.add_raw_font(font_key, (&*bytes).clone(), font_index);
                        r
                    }
                };

                let mut font_instances = FontInstances {
                    font_key,
                    metrics,
                    harfbuzz_face: harfbuzz_face.to_shared(),
                    instances: FnvHashMap::default(),
                };

                self.api.update_resources(txn.resource_updates);
                let instance = Self::load_font_size(&self.api, &mut font_instances, size);
                self.fonts.insert(query_key, font_instances);
                Some(instance)
            }
            Err(font_kit::error::SelectionError::NotFound) => None,
            Err(font_kit::error::SelectionError::CannotAccessSource) => panic!("cannot access system font source"),
        }
    }

    fn load_font_size(api: &RenderApi, font_instances: &mut FontInstances, size: FontSizePt) -> FontInstance {
        let mut txn = Transaction::new();
        let instance_key = api.generate_font_instance_key();

        let size_px = size as f32 * 96.0 / 72.0;
        txn.add_font_instance(
            instance_key,
            font_instances.font_key,
            Au::from_f32_px(size_px),
            None,
            None,
            Vec::new(),
        );
        api.update_resources(txn.resource_updates);

        let mut harfbuzz_font = harfbuzz_rs::Font::new(harfbuzz_rs::Shared::clone(&font_instances.harfbuzz_face));

        harfbuzz_font.set_ppem(size, size);
        harfbuzz_font.set_scale(size as i32 * 64, size as i32 * 64);

        let metrics = FontMetrics::new(size_px, &font_instances.metrics);

        let instance = FontInstance::new(instance_key, size, metrics, harfbuzz_font.to_shared());
        font_instances.instances.insert(size, instance.clone());

        instance
    }
}

impl WindowService for Fonts {}

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
struct FontPropertiesKey(u8, u32, u32);
impl FontPropertiesKey {
    pub fn new(properties: FontProperties) -> Self {
        Self(
            match properties.style {
                FontStyle::Normal => 0,
                FontStyle::Italic => 1,
                FontStyle::Oblique => 2,
            },
            (properties.weight.0 * 100.0) as u32,
            (properties.stretch.0 * 100.0) as u32,
        )
    }
}

/// All instances of a font family.
struct FontInstances {
    pub font_key: FontKey,
    pub metrics: font_kit::metrics::Metrics,
    pub harfbuzz_face: HarfbuzzFace,
    pub instances: FnvHashMap<FontSizePt, FontInstance>,
}

struct FontInstanceInner {
    instance_key: FontInstanceKey,
    font_size: FontSizePt,
    harfbuzz_font: HarfbuzzFont,
    metrics: FontMetrics,
}

type HarfbuzzFace = harfbuzz_rs::Shared<harfbuzz_rs::Face<'static>>;

type HarfbuzzFont = harfbuzz_rs::Shared<harfbuzz_rs::Font<'static>>;

/// Reference to a specific font instance (family and size).
#[derive(Clone)]
pub struct FontInstance {
    inner: Arc<FontInstanceInner>,
}

impl FontInstance {
    fn new(instance_key: FontInstanceKey, font_size: FontSizePt, metrics: FontMetrics, harfbuzz_font: HarfbuzzFont) -> Self {
        FontInstance {
            inner: Arc::new(FontInstanceInner {
                instance_key,
                font_size,
                metrics,
                harfbuzz_font,
            }),
        }
    }

    /// Size of this font instance.
    #[inline]
    pub fn size(&self) -> FontSizePt {
        self.inner.font_size
    }

    /// Various metrics that apply to this font.
    #[inline]
    pub fn metrics(&self) -> &FontMetrics {
        &self.inner.metrics
    }

    /// Shapes the text line using the font.
    ///
    /// The `text` should not contain line breaks, if it does the line breaks are ignored.
    pub fn shape_line(&self, text: &str, config: &ShapingConfig) -> ShapedLine {
        let mut buffer = harfbuzz_rs::UnicodeBuffer::new().set_direction(if config.right_to_left {
            harfbuzz_rs::Direction::Rtl
        } else {
            harfbuzz_rs::Direction::Ltr
        });
        if config.script != Script::Unknown {
            buffer = buffer.set_script(script_to_tag(config.script)).add_str(text);
        } else {
            buffer = buffer.add_str(text).guess_segment_properties();
        }

        let mut features = vec![];
        if config.ignore_ligatures {
            features.push(harfbuzz_rs::Feature::new(b"liga", 0, 0..buffer.len()));
        }
        if config.disable_kerning {
            features.push(harfbuzz_rs::Feature::new(b"kern", 0, 0..buffer.len()));
        }

        let metrics = self.metrics();

        let r = harfbuzz_rs::shape(&self.inner.harfbuzz_font, buffer, &features);

        let baseline = metrics.ascent + metrics.line_gap / 2.0;
        let mut origin = LayoutPoint::new(0.0, baseline);

        let glyphs: Vec<_> = r
            .get_glyph_infos()
            .iter()
            .zip(r.get_glyph_positions())
            .map(|(i, p)| {
                fn to_layout(p: harfbuzz_rs::Position) -> f32 {
                    // remove our scale of 64 and convert to layout pixels
                    (p as f32 / 64.0) * 96.0 / 72.0
                }
                let x_offset = to_layout(p.x_offset);
                let y_offset = to_layout(p.y_offset);
                let x_advance = to_layout(p.x_advance);
                let y_advance = to_layout(p.y_advance);

                let point = LayoutPoint::new(origin.x + x_offset, origin.y + y_offset);
                origin.x += x_advance;
                origin.y += y_advance;
                GlyphInstance { index: i.codepoint, point }
            })
            .collect();

        let bounds = LayoutSize::new(origin.x, config.line_height(metrics));

        ShapedLine { glyphs, baseline, bounds }
    }

    pub fn glyph_outline(&self, _line: &ShapedLine) {
        todo!("Implement this after full text shaping")
        // https://docs.rs/font-kit/0.10.0/font_kit/loaders/freetype/struct.Font.html#method.outline
        // Frame of reference: https://searchfox.org/mozilla-central/source/gfx/2d/ScaledFontDWrite.cpp#148
        // Text shaping: https://crates.io/crates/harfbuzz_rs
    }

    /// Gets the font instance key.
    pub fn instance_key(&self) -> FontInstanceKey {
        self.inner.instance_key
    }
}

fn script_to_tag(script: Script) -> harfbuzz_rs::Tag {
    let mut name = script.short_name().chars();
    harfbuzz_rs::Tag::new(
        name.next().unwrap(),
        name.next().unwrap(),
        name.next().unwrap(),
        name.next().unwrap(),
    )
}

/// Extra configuration for [`shape_line`](FontInstance::shape_line).
#[derive(Debug, Clone, Default)]
pub struct ShapingConfig {
    /// Extra spacing to add between characters.
    pub letter_spacing: Option<f32>,

    /// Spacing to add between each word.
    ///
    /// Use [`word_spacing(..)`](function@Self::word_spacing) to compute the value.
    pub word_spacing: Option<f32>,

    /// Height of each line.
    ///
    /// Use [`line_height(..)`](function@Self::line_height) to compute the value.
    pub line_height: Option<f32>,

    /// Space to add between each line.
    pub line_spacing: f32,

    /// Space to add between each paragraph.
    ///
    /// use [`paragraph_spacing(.).`](function@Self::paragraph_spacing) to compute the value.
    pub paragraph_spacing: Option<f32>,

    /// Unicode script of the text.
    pub script: Script,

    /// Don't use font ligatures.
    pub ignore_ligatures: bool,

    /// Don't use font letter spacing.
    pub disable_kerning: bool,

    /// Text is right-to-left.
    pub right_to_left: bool,

    pub word_break: WordBreak,

    pub line_break: LineBreak,

    pub text_align: TextAlign,

    /// Width of the TAB character.
    ///
    /// By default 3 x space.
    pub tab_size: Option<f32>,

    /// Extra space before the start of the first line.
    pub text_indent: f32,

    /// Collapse/preserve line-breaks/etc.
    pub white_space: WhiteSpace,
}

impl ShapingConfig {
    /// Gets the custom word spacing or 0.25em.
    #[inline]
    pub fn word_spacing(&self, font_size: f32) -> f32 {
        self.word_spacing.unwrap_or(font_size * 0.25)
    }

    /// Gets the custom line height or the font line height.
    #[inline]
    pub fn line_height(&self, metrics: &FontMetrics) -> f32 {
        // servo uses the line-gap as default I think.
        self.line_height.unwrap_or_else(|| metrics.line_height())
    }

    /// Gets the custom paragraph spacing or one line height + two line spacing.
    #[inline]
    pub fn paragraph_spacing(&self, metrics: &FontMetrics) -> f32 {
        self.line_height(metrics) + self.line_spacing * 2.0
    }
}

/// Result of [`shape_line`](FontInstance::shape_line).
#[derive(Debug, Clone)]
pub struct ShapedLine {
    /// Glyphs for the renderer.
    pub glyphs: Vec<GlyphInstance>,

    /// Baseline within `bounds`.
    ///
    /// This is the font ascent + half the line gap.
    pub baseline: f32,

    /// Size of the text for the layout.
    pub bounds: LayoutSize,
}

/// Configuration of text wrapping for Chinese, Japanese, or Korean text.
#[derive(Debug, Copy, Clone)]
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

/// Hyphenation configuration.
#[derive(Debug, Copy, Clone)]
pub enum Hyphens {
    /// Hyphens are never inserted in word breaks.
    None,
    /// Word breaks only happen in specially marked break characters: `-` and `\u{00AD} SHY`.
    ///
    /// * `U+2010` - The visible hyphen character.
    /// * `U+00AD` - The invisible hyphen character, is made visible in a word break.
    Manual,
    /// Hyphens are inserted like `Manual` and also using language specific hyphenation rules.
    // TODO https://sourceforge.net/projects/hunspell/files/Hyphen/2.8/
    Auto,
}
impl Default for Hyphens {
    /// [`Hyphens::Auto`]
    fn default() -> Self {
        Hyphens::Auto
    }
}

/// Configure line breaks inside words during text wrap.
///
/// This value is only considered if it is impossible to fit the a word to a line.
///
/// Hyphens can be inserted in word breaks using the [`Hyphens`] configuration.
#[derive(Debug, Copy, Clone)]
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

/// Text alignment.
#[derive(Debug, Copy, Clone)]
pub enum TextAlign {
    /// `Left` in LTR or `Right` in RTL.
    Start,
    /// `Right` in LTR or `Left` in RTL.
    End,

    Left,
    Center,
    Right,

    /// Adjust spacing to fill the available width.
    ///
    /// The justify can be configured using [`Justify`].
    Justify(Justify),
}
impl TextAlign {
    /// Justify Auto.
    #[inline]
    pub fn justify() -> Self {
        TextAlign::Justify(Justify::Auto)
    }
}
impl Default for TextAlign {
    /// [`TextAlign::Start`].
    #[inline]
    fn default() -> Self {
        TextAlign::Start
    }
}

/// Text alignment justification mode.
#[derive(Debug, Copy, Clone)]
pub enum Justify {
    /// Selects the justification mode based on the language.
    /// For Chinese/Japanese/Korean uses `InterLetter` for the others uses `InterWord`.
    Auto,
    /// The text is justified by adding space between words.
    ///
    /// This only works if [`word_spacing`](crate::properties::text_theme::word_spacing) is set to auto.
    InterWord,
    /// The text is justified by adding space between letters.
    ///
    /// This only works if [`letter_spacing`](crate::properties::text_theme::letter_spacing) is set to auto.
    InterLetter,
}
impl Default for Justify {
    /// [`Justify::Auto`]
    fn default() -> Self {
        Justify::Auto
    }
}

/// Various metrics about a [`FontInstance`].
#[derive(Clone, Debug)]
pub struct FontMetrics {
    /// The number of font units per em.
    ///
    /// Font sizes are usually expressed in pixels per em; e.g. `12px` means 12 pixels per em.
    pub units_per_em: u32,

    /// The maximum amount the font rises above the baseline, in layout units.
    pub ascent: f32,

    /// The maximum amount the font descends below the baseline, in layout units.
    ///
    /// NB: This is typically a negative value to match the definition of `sTypoDescender` in the
    /// `OS/2` table in the OpenType specification. If you are used to using Windows or Mac APIs,
    /// beware, as the sign is reversed from what those APIs return.
    pub descent: f32,

    /// Distance between baselines, in layout units.
    pub line_gap: f32,

    /// The suggested distance of the top of the underline from the baseline (negative values
    /// indicate below baseline), in layout units.
    pub underline_position: f32,

    /// A suggested value for the underline thickness, in layout units.
    pub underline_thickness: f32,

    /// The approximate amount that uppercase letters rise above the baseline, in layout units.
    pub cap_height: f32,

    /// The approximate amount that non-ascending lowercase letters rise above the baseline, in
    /// font units.
    pub x_height: f32,

    /// A rectangle that surrounds all bounding boxes of all glyphs, in layout units.
    ///
    /// This corresponds to the `xMin`/`xMax`/`yMin`/`yMax` values in the OpenType `head` table.
    pub bounding_box: LayoutRect,
}

impl FontMetrics {
    /// Calculate metrics from global.
    fn new(font_size_px: f32, metrics: &font_kit::metrics::Metrics) -> Self {
        let em = metrics.units_per_em as f32;
        let s = move |f: f32| f / em * font_size_px;
        FontMetrics {
            units_per_em: metrics.units_per_em,
            ascent: s(metrics.ascent),
            descent: s(metrics.descent),
            line_gap: s(metrics.line_gap),
            underline_position: s(metrics.underline_position),
            underline_thickness: s(metrics.underline_thickness),
            cap_height: s(metrics.cap_height),
            x_height: (s(metrics.x_height)),
            bounding_box: {
                let b = metrics.bounding_box;
                LayoutRect::new(
                    LayoutPoint::new(s(b.origin_x()), s(b.origin_y())),
                    LayoutSize::new(s(b.width()), s(b.height())),
                )
            },
        }
    }

    /// The font line height.
    pub fn line_height(&self) -> f32 {
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
    Custom(Rc<dyn Fn(&str) -> Cow<str>>),
}
impl TextTransformFn {
    pub fn transform<'a, 'b>(&'a self, text: &'b str) -> Cow<'b, str> {
        match self {
            TextTransformFn::None => Cow::Borrowed(text),
            TextTransformFn::Uppercase => Cow::Owned(text.to_uppercase()),
            TextTransformFn::Lowercase => Cow::Owned(text.to_lowercase()),
            TextTransformFn::Custom(fn_) => fn_(text),
        }
    }

    pub fn custom(fn_: impl Fn(&str) -> Cow<str> + 'static) -> Self {
        TextTransformFn::Custom(Rc::new(fn_))
    }
}
impl fmt::Debug for TextTransformFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TextTransformFn::None => write!(f, "None"),
            TextTransformFn::Uppercase => write!(f, "Uppercase"),
            TextTransformFn::Lowercase => write!(f, "Lowercase"),
            TextTransformFn::Custom(_) => write!(f, "Custom"),
        }
    }
}

/// Text white space transform.
#[derive(Debug, Copy, Clone)]
pub enum WhiteSpace {
    /// Text is not changed, all white spaces and line breaks are preserved.
    Preserve,
    /// Replace sequences of white space with a single `U+0020 SPACE` and trim lines. Line breaks are preserved.
    Merge,
    /// Replace sequences of white space and line breaks with `U+0020 SPACE` and trim the text.
    MergeNoBreak,
}
impl Default for WhiteSpace {
    /// [`WhiteSpace::Preserve`].
    #[inline]
    fn default() -> Self {
        WhiteSpace::Preserve
    }
}

/// Name of a font feature.
///
/// # Example
///
/// ```
/// # use zero_ui::core::text::FontFeatureName;
/// let historical_lig: FontFeatureName = b"hlig";
/// ```
pub type FontFeatureName = &'static [u8; 4];

const FEATURE_ENABLED: u32 = 1;
const FEATURE_DISABLED: u32 = 0;

/// Font features configuration.
#[derive(Default, Clone)]
pub struct FontFeatures(FnvHashMap<FontFeatureName, u32>);
impl FontFeatures {
    /// New default.
    #[inline]
    pub fn new() -> FontFeatures {
        FontFeatures::default()
    }

    /// New builder.
    #[inline]
    pub fn builder() -> FontFeaturesBuilder {
        FontFeaturesBuilder::default()
    }

    /// Set or override the features of `self` from `other`.
    ///
    /// Returns the previous state of all affected names.
    #[inline]
    pub fn set_all(&mut self, other: &FontFeatures) -> Vec<(FontFeatureName, Option<u32>)> {
        let mut prev = Vec::with_capacity(other.0.len());
        for (&name, &state) in other.0.iter() {
            prev.push((name, self.0.insert(name, state)));
        }
        prev
    }

    /// Restore feature states that where overridden in [`set_all`](Self::set_all).
    #[inline]
    pub fn restore(&mut self, prev: Vec<(FontFeatureName, Option<u32>)>) {
        for (name, state) in prev {
            match state {
                Some(state) => {
                    self.0.insert(name, state);
                }
                None => {
                    self.0.remove(name);
                }
            }
        }
    }

    /// Access to the named feature.
    #[inline]
    pub fn feature(&mut self, name: FontFeatureName) -> FontFeature {
        FontFeature(self.0.entry(name))
    }

    /// Access to a set of named features that are managed together.
    #[inline]
    pub fn feature_set(&mut self, names: [FontFeatureName; 2]) -> FontFeatureSet {
        FontFeatureSet {
            features: &mut self.0,
            names,
        }
    }

    /// Font capital glyph variants.
    ///
    /// See [`CapsVariant`] for more details.
    #[inline]
    pub fn caps(&mut self) -> CapsVariantFeatures {
        CapsVariantFeatures { features: &mut self.0 }
    }

    /// Font numeric glyph variants.
    ///
    /// See [`NumVariant`] for more details.
    #[inline]
    pub fn numeric(&mut self) -> NumVariantFeatures {
        NumVariantFeatures { features: &mut self.0 }
    }

    /// Font numeric spacing variants.
    ///
    /// See [`NumSpacing`] for more details.
    #[inline]
    pub fn num_spacing(&mut self) -> NumSpacingFeatures {
        NumSpacingFeatures { features: &mut self.0 }
    }

    /// Font numeric spacing variants.
    ///
    /// See [`NumSpacing`] for more details.
    #[inline]
    pub fn num_fraction(&mut self) -> NumFractionFeatures {
        NumFractionFeatures { features: &mut self.0 }
    }

    /// Enables stylistic alternatives for sets of character
    ///
    /// See [`StyleSet`] for more details.
    #[inline]
    pub fn style_set(&mut self) -> StyleSetFeatures {
        StyleSetFeatures { features: &mut self.0 }
    }
}
impl fmt::Debug for FontFeatures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        for (name, state) in self.0.iter() {
            map.entry(&name_to_str(name), state);
        }
        map.finish()
    }
}

fn name_to_str(name: FontFeatureName) -> &'static str {
    std::str::from_utf8(name).unwrap_or_default()
}

/// A builder for [`FontFeatures`].
///
/// # Example
///
/// ```
/// # use zero_ui::core::text::FontFeatures;
/// let features = FontFeatures::builder().kerning(false).build();
/// ```
#[derive(Default)]
pub struct FontFeaturesBuilder(FontFeatures);
impl FontFeaturesBuilder {
    /// Finish building.
    #[inline]
    pub fn build(self) -> FontFeatures {
        self.0
    }

    /// Set the named feature.
    #[inline]
    pub fn feature(mut self, name: FontFeatureName, state: impl Into<FontFeatureState>) -> Self {
        self.0.feature(name).set(state);
        self
    }

    /// Font capital glyph variants.
    ///
    /// See [`CapsVariant`] for more details.
    #[inline]
    pub fn caps(mut self, state: impl Into<CapsVariant>) -> Self {
        self.0.caps().set(state);
        self
    }

    /// Font numeric glyph variants.
    ///
    /// See [`NumVariant`] for more details.
    #[inline]
    pub fn numeric(mut self, state: impl Into<NumVariant>) -> Self {
        self.0.numeric().set(state);
        self
    }

    /// Font numeric spacing variants.
    ///
    /// See [`NumSpacing`] for more details.
    #[inline]
    pub fn num_spacing(mut self, state: impl Into<NumSpacing>) -> Self {
        self.0.num_spacing().set(state);
        self
    }

    /// Font numeric fraction variants.
    ///
    /// See [`NumFraction`] for more details.
    #[inline]
    pub fn num_fraction(mut self, state: impl Into<NumFraction>) -> Self {
        self.0.num_fraction().set(state);
        self
    }

    /// Enables stylistic alternatives for sets of character
    ///
    /// See [`StyleSet`] for more details.
    #[inline]
    pub fn style_set(mut self, state: impl Into<StyleSet>) -> Self {
        self.0.style_set().set(state);
        self
    }
}

/// Generate `FontFeature` methods in `FontFeatures` and builder methods in `FontFeaturesBuilder`
/// that set the feature.
macro_rules! font_features {
    ($(
        $(#[$docs:meta])*
        fn $name:ident($feat0:tt $(, $feat1:tt)?);
    )+) => {
        $(
            font_features!{feature $(#[$docs])* fn $name($feat0 $(, $feat1)?); }
            font_features!{builder $(#[$docs])* fn $name(); }
        )+
    };

    (feature $(#[$docs:meta])* fn $name:ident($feat0:tt, $feat1:tt); ) => {
        impl FontFeatures {
            $(#[$docs])*
            #[inline]
            pub fn $name(&mut self) -> FontFeatureSet {
                self.feature_set([$feat0, $feat1])
            }
        }
    };

    (feature $(#[$docs:meta])* fn $name:ident($feat0:tt);) => {
        impl FontFeatures {
            $(#[$docs])*
            #[inline]
            pub fn $name(&mut self) -> FontFeature {
                self.feature($feat0)
            }
        }
    };

    (builder $(#[$docs:meta])* fn $name:ident();) => {
        impl FontFeaturesBuilder {
            $(#[$docs])*
            #[inline]
            pub fn $name(mut self, state: impl Into<FontFeatureState>) -> Self {
                self.0.$name().set(state);
                self
            }
        }
    };
}

font_features! {
    /// Allow glyphs boundaries to overlap for a more pleasant reading.
    ///
    /// This corresponds to the `kern` feature.
    ///
    /// `Auto` always activates these kerning.
    fn kerning(b"kern");

    /// The most common ligatures, like for `fi`, `ffi`, `th` or similar.
    ///
    /// This corresponds to OpenType `liga` and `clig` features.
    ///
    /// `Auto` always activates these ligatures.
    fn common_lig(b"liga", b"clig");

    /// Ligatures specific to the font, usually decorative.
    ///
    /// This corresponds to OpenType `dlig` feature.
    ///
    /// `Auto` usually disables these ligatures.
    fn discretionary_lig(b"dlig");

    /// Ligatures used historically, in old books, like the German tz digraph being displayed ß.
    ///
    /// This corresponds to OpenType `hlig` feature.
    ///
    /// `Auto` usually disables these ligatures.
    fn historical_lig(b"hlig");

    /// Alternative letters that adapt to their surrounding letters.
    ///
    /// This corresponds to OpenType `calt` feature.
    ///
    /// `Auto` usually activates this feature.
    fn contextual_alt(b"calt");

    /// Force usage of ordinal special glyphs, 1a becomes 1ª.
    ///
    /// This corresponds to OpenType `ordn` feature.
    ///
    /// `Auto` deactivates this feature.
    fn ordinal(b"ordn");

    /// Force use of a slashed zero for `0`.
    ///
    /// This corresponds to OpenType `zero` feature.
    ///
    /// `Auto` deactivates this feature.
    fn slashed_zero(b"zero");

    /// Use swashes flourish style.
    ///
    /// Fonts can have alternative swash styles, you can select then by enabling a number.
    ///
    /// This corresponds to OpenType `swsh` and `cswh` feature.
    ///
    /// `Auto` does not use swashes.
    fn swash(b"swsh", b"cswh");

    /// Use stylistic alternatives.
    ///
    /// Fonts can have multiple alternative styles, you can select then by enabling a number.
    ///
    /// This corresponds to OpenType `salt` feature.
    ///
    /// `Auto` does not use alternative styles.
    fn stylistic(b"salt");

    /// Use glyphs that were common in the past but not today.
    ///
    /// This corresponds to OpenType `hist` feature.
    ///
    /// `Auto` does not use alternative styles.
    fn historical_forms(b"hist");

    /// Replace letter with fleurons, dingbats and border elements.
    ///
    /// Fonts can have multiple alternative styles, you can select then by enabling a number.
    ///
    /// This corresponds to OpenType `ornm` feature.
    ///
    /// `Auto` does not enable this by default, but some fonts are purely dingbats glyphs.
    fn ornaments(b"ornm");

    /// Enables annotation alternatives, like circled digits or inverted characters.
    ///
    /// Fonts can have multiple alternative styles, you can select then by enabling a number.
    ///
    /// This corresponds to OpenType `nalt` feature.
    ///
    /// `Auto` does not use alternative styles.
    fn annotation(b"nalt");
}

// TODO
// main: https://developer.mozilla.org/en-US/docs/Web/CSS/font-feature-settings
// 1 - https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-east-asian
// 2 - https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-alternates
// 4 - https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-position
// 5 - https://helpx.adobe.com/pt/fonts/user-guide.html/pt/fonts/using/open-type-syntax.ug.html#calt
// review - https://harfbuzz.github.io/shaping-opentype-features.html

/// Represents a feature in a [`FontFeatures`] configuration.
pub struct FontFeature<'a>(HEntry<'a, FontFeatureName, u32>);
impl<'a> FontFeature<'a> {
    /// Gets the OpenType name of the feature.
    #[inline]
    pub fn name(&self) -> FontFeatureName {
        self.0.key()
    }

    /// Gets the current state of the feature.
    pub fn state(&self) -> FontFeatureState {
        match &self.0 {
            HEntry::Occupied(e) => FontFeatureState(Some(*e.get())),
            HEntry::Vacant(_) => FontFeatureState::auto(),
        }
    }

    /// If the feature is explicitly enabled.
    pub fn is_enabled(&self) -> bool {
        self.state().is_enabled()
    }

    /// If the feature is explicitly disabled.
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.state().is_disabled()
    }

    /// If the feature is auto enabled zero-ui.
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.state().is_auto()
    }

    /// Set the feature state.
    ///
    /// Returns the previous state.
    #[inline]
    pub fn set(self, state: impl Into<FontFeatureState>) -> FontFeatureState {
        let prev = self.state();
        match state.into().0 {
            Some(n) => self.set_explicit(n),
            None => self.auto(),
        }
        prev
    }

    fn set_explicit(self, state: u32) {
        match self.0 {
            HEntry::Occupied(mut e) => {
                e.insert(state);
            }
            HEntry::Vacant(e) => {
                e.insert(state);
            }
        }
    }

    /// Enable the feature.
    #[inline]
    pub fn enable(self) {
        self.set_explicit(FEATURE_ENABLED);
    }

    /// Enable the feature with alternative selection.
    #[inline]
    pub fn enable_alt(self, alt: NonZeroU32) {
        self.set_explicit(alt.get())
    }

    /// Disable the feature.
    #[inline]
    pub fn disable(self) {
        self.set_explicit(FEATURE_DISABLED);
    }

    /// Set the feature to auto.
    #[inline]
    pub fn auto(self) {
        if let HEntry::Occupied(e) = self.0 {
            e.remove();
        }
    }
}
impl<'a> fmt::Debug for FontFeature<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"{}\": {:?}", name_to_str(self.name()), self.state())
    }
}

/// Represents a set of boolean features in a [`FontFeatures`] configuration, the features state is managed together.
pub struct FontFeatureSet<'a> {
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
    names: [FontFeatureName; 2],
}
impl<'a> FontFeatureSet<'a> {
    /// Gets the OpenType name of the features.
    #[inline]
    pub fn names(&self) -> &[FontFeatureName] {
        &self.names
    }

    /// Gets the current state of the features.
    ///
    /// Returns `Auto` if the features are mixed.
    #[inline]
    pub fn state(&self) -> FontFeatureState {
        match (self.features.get(self.names[0]), self.features.get(self.names[1])) {
            (Some(&a), Some(&b)) if a == b => FontFeatureState(Some(a)),
            _ => FontFeatureState::auto(),
        }
    }

    /// If the features are explicitly enabled.
    pub fn is_enabled(&self) -> bool {
        self.state().is_enabled()
    }

    /// If the features are explicitly disabled.
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.state().is_disabled()
    }

    /// If the features are auto enabled zero-ui, or in a mixed state.
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.state().is_auto()
    }

    /// Set the feature state.
    ///
    /// Returns the previous state.
    #[inline]
    pub fn set(self, state: impl Into<FontFeatureState>) -> FontFeatureState {
        let prev = self.state();
        match state.into().0 {
            Some(n) => self.set_explicit(n),
            None => self.auto(),
        }
        prev
    }

    fn set_explicit(self, state: u32) {
        for name in &self.names {
            self.features.insert(name, state);
        }
    }

    /// Enable the feature.
    #[inline]
    pub fn enable(self) {
        self.set_explicit(FEATURE_ENABLED);
    }

    /// Disable the feature.
    #[inline]
    pub fn disable(self) {
        self.set_explicit(FEATURE_DISABLED);
    }

    /// Set the feature to auto.
    #[inline]
    pub fn auto(self) {
        for name in &self.names {
            self.features.remove(name);
        }
    }
}
impl<'a> fmt::Debug for FontFeatureSet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: {:?}",
            self.names().iter().map(|s| name_to_str(s)).collect::<Vec<_>>(),
            self.state()
        )
    }
}

/// Represents the [capitalization variant](FontFeatures::caps) features. At any time only one of
/// these features are be enabled.
pub struct CapsVariantFeatures<'a> {
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
}
impl<'a> CapsVariantFeatures<'a> {
    /// Gets the OpenType names of all the features affected.
    #[inline]
    pub fn names(&self) -> [FontFeatureName; 6] {
        // the order of names if required by `take_state`.
        [b"c2sc", b"smcp", b"c2pc", b"pcap", b"unic", b"titl"]
    }

    /// Gets the current state of the features.
    pub fn state(&self) -> CapsVariant {
        let enabled = |n| self.features.get(n).copied().unwrap_or_default() == FEATURE_ENABLED;

        if enabled(b"c2sc") {
            CapsVariant::AllSmallCaps
        } else if enabled(b"smcp") {
            CapsVariant::SmallCaps
        } else if enabled(b"c2pc") {
            CapsVariant::AllPetite
        } else if enabled(b"pcap") {
            CapsVariant::Petite
        } else if enabled(b"unic") {
            CapsVariant::Unicase
        } else {
            match self.features.get(b"titl") {
                Some(&FEATURE_ENABLED) => CapsVariant::TitlingCaps,
                Some(&FEATURE_DISABLED) => CapsVariant::None,
                _ => CapsVariant::Auto,
            }
        }
    }
    fn take_state(&mut self) -> CapsVariant {
        let names = self.names();
        // Returns if the feature is enabled and removes all tailing features.
        let mut enabled = |i, expected| {
            let name = names[i];
            debug_assert_eq!(name, expected);
            if self.features.remove(name).unwrap_or_default() == FEATURE_ENABLED {
                for name in &names[(i + 1)..] {
                    self.features.remove(name);
                }
                true
            } else {
                false
            }
        };

        if enabled(0, b"c2sc") {
            CapsVariant::AllSmallCaps
        } else if enabled(1, b"smcp") {
            CapsVariant::SmallCaps
        } else if enabled(2, b"c2pc") {
            CapsVariant::AllPetite
        } else if enabled(3, b"pcap") {
            CapsVariant::Petite
        } else if enabled(4, b"unic") {
            CapsVariant::Unicase
        } else {
            match self.features.remove(b"titl") {
                Some(FEATURE_ENABLED) => CapsVariant::TitlingCaps,
                Some(FEATURE_DISABLED) => CapsVariant::None,
                _ => CapsVariant::Auto,
            }
        }
    }

    /// If no feature is explicitly enabled/disabled.
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.state() == CapsVariant::Auto
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    pub fn set(mut self, state: impl Into<CapsVariant>) -> CapsVariant {
        let prev = self.take_state();

        let mut enable = |n| {
            self.features.insert(n, FEATURE_ENABLED);
        };

        match state.into() {
            CapsVariant::SmallCaps => enable(b"smcp"),
            CapsVariant::AllSmallCaps => {
                enable(b"smcp");
                enable(b"c2sc");
            }
            CapsVariant::Petite => enable(b"pcap"),
            CapsVariant::AllPetite => {
                enable(b"pcap");
                enable(b"c2pc");
            }
            CapsVariant::Unicase => enable(b"unic"),
            CapsVariant::TitlingCaps => enable(b"titl"),
            CapsVariant::None => {
                self.features.insert(b"titl", FEATURE_DISABLED);
            }
            CapsVariant::Auto => {}
        }

        prev
    }
}
impl<'a> fmt::Debug for CapsVariantFeatures<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.state(), f)
    }
}

/// Represents the [numeric variant](FontFeatures::numeric) features. At any time only one of
/// these features are be enabled.
pub struct NumVariantFeatures<'a> {
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
}
impl<'a> NumVariantFeatures<'a> {
    /// Gets the OpenType names of all the features affected.
    #[inline]
    pub fn names(&self) -> [FontFeatureName; 2] {
        [b"lnum", b"onum"]
    }

    /// Gets the current state of the features.
    #[inline]
    pub fn state(&self) -> NumVariant {
        let enabled = |n| self.features.get(n).copied().unwrap_or_default() == FEATURE_ENABLED;

        if enabled(b"lnum") {
            NumVariant::Lining
        } else if enabled(b"onum") {
            NumVariant::OldStyle
        } else {
            NumVariant::Auto
        }
    }

    fn take_state(&mut self) -> NumVariant {
        let lnum = self.features.remove(b"lnum");
        let onum = self.features.remove(b"onum");

        if lnum.unwrap_or_default() == FEATURE_ENABLED {
            NumVariant::Lining
        } else if onum.unwrap_or_default() == FEATURE_ENABLED {
            NumVariant::OldStyle
        } else {
            NumVariant::Auto
        }
    }

    /// If no feature is explicitly enabled/disabled.
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.state() == NumVariant::Auto
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    #[inline]
    pub fn set(&mut self, state: impl Into<NumVariant>) -> NumVariant {
        let prev = self.take_state();

        match state.into() {
            NumVariant::OldStyle => {
                self.features.insert(b"onum", FEATURE_ENABLED);
            }
            NumVariant::Lining => {
                self.features.insert(b"lnum", FEATURE_ENABLED);
            }
            NumVariant::Auto => {}
        }

        prev
    }
}
impl<'a> fmt::Debug for NumVariantFeatures<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.state(), f)
    }
}

/// Represents the [numeric spacing](FontFeatures::num_spacing) features. At any time only one of
/// these features are be enabled.
pub struct NumSpacingFeatures<'a> {
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
}
impl<'a> NumSpacingFeatures<'a> {
    /// Gets the OpenType names of all the features affected.
    #[inline]
    pub fn names(&self) -> [FontFeatureName; 2] {
        [b"pnum", b"tnum"]
    }

    /// Gets the current state of the features.
    #[inline]
    pub fn state(&self) -> NumSpacing {
        let enabled = |n| self.features.get(n).copied().unwrap_or_default() == FEATURE_ENABLED;

        if enabled(b"pnum") {
            NumSpacing::Proportional
        } else if enabled(b"tnum") {
            NumSpacing::Tabular
        } else {
            NumSpacing::Auto
        }
    }

    fn take_state(&mut self) -> NumSpacing {
        let pnum = self.features.remove(b"pnum");
        let tnum = self.features.remove(b"tnum");

        if pnum.unwrap_or_default() == FEATURE_ENABLED {
            NumSpacing::Proportional
        } else if tnum.unwrap_or_default() == FEATURE_ENABLED {
            NumSpacing::Tabular
        } else {
            NumSpacing::Auto
        }
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    #[inline]
    pub fn set(&mut self, state: impl Into<NumSpacing>) -> NumSpacing {
        let prev = self.take_state();
        match state.into() {
            NumSpacing::Tabular => {
                self.features.insert(b"tnum", FEATURE_ENABLED);
            }
            NumSpacing::Proportional => {
                self.features.insert(b"pnum", FEATURE_ENABLED);
            }
            NumSpacing::Auto => {}
        }
        prev
    }
}
impl<'a> fmt::Debug for NumSpacingFeatures<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.state(), f)
    }
}

/// Represents the [numeric fraction](FontFeatures::num_fraction) features. At any time only one of
/// these features are be enabled.
pub struct NumFractionFeatures<'a> {
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
}
impl<'a> NumFractionFeatures<'a> {
    /// Gets the OpenType names of all the features affected.
    #[inline]
    pub fn names(&self) -> [FontFeatureName; 2] {
        [b"frac", b"afrc"]
    }

    /// Gets the current state of the features.
    #[inline]
    pub fn state(&self) -> NumFraction {
        let enabled = |n| self.features.get(n).copied().unwrap_or_default() == FEATURE_ENABLED;

        if enabled(b"frac") {
            NumFraction::Diagonal
        } else if enabled(b"afrc") {
            NumFraction::Stacked
        } else {
            NumFraction::Auto
        }
    }

    fn take_state(&mut self) -> NumFraction {
        let frac = self.features.remove(b"frac");
        let afrc = self.features.remove(b"afrc");

        if frac.unwrap_or_default() == FEATURE_ENABLED {
            NumFraction::Diagonal
        } else if afrc.unwrap_or_default() == FEATURE_ENABLED {
            NumFraction::Stacked
        } else {
            NumFraction::Auto
        }
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    #[inline]
    pub fn set(&mut self, state: impl Into<NumFraction>) -> NumFraction {
        let prev = self.take_state();
        match state.into() {
            NumFraction::Diagonal => {
                self.features.insert(b"frac", FEATURE_ENABLED);
            }
            NumFraction::Stacked => {
                self.features.insert(b"afrc", FEATURE_ENABLED);
            }
            NumFraction::Auto => {}
        }
        prev
    }
}
impl<'a> fmt::Debug for NumFractionFeatures<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.state(), f)
    }
}
/// Represents the [style_set](FontFeatures::style_set) features. At any time only one of
/// these features are be enabled.
pub struct StyleSetFeatures<'a> {
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
}
impl<'a> StyleSetFeatures<'a> {
    /// Gets the OpenType names of all the features affected.
    #[inline]
    pub fn names(&self) -> [FontFeatureName; 20] {
        StyleSet::NAMES
    }

    /// Gets the current state of the features.
    #[inline]
    pub fn state(&self) -> StyleSet {
        for (i, name) in self.names().iter().enumerate() {
            if self.features.get(name) == Some(&FEATURE_ENABLED) {
                return (i as u8 + 1).into();
            }
        }
        StyleSet::Auto
    }
    fn take_state(&mut self) -> StyleSet {
        let mut state = StyleSet::Auto;
        for (i, name) in self.names().iter().enumerate() {
            if self.features.get(name) == Some(&FEATURE_ENABLED) {
                state = (i as u8 + 1).into()
            }
        }
        state
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    #[inline]
    pub fn set(&mut self, state: impl Into<StyleSet>) -> StyleSet {
        let prev = self.take_state();
        if let Some(name) = state.into().name() {
            self.features.insert(name, FEATURE_ENABLED);
        }
        prev
    }
}
impl<'a> fmt::Debug for StyleSetFeatures<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.state(), f)
    }
}

/// State of a [font feature](FontFeatures).
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FontFeatureState(Option<u32>);
impl FontFeatureState {
    /// Automatic state.
    #[inline]
    pub const fn auto() -> Self {
        FontFeatureState(None)
    }

    /// Enabled state.
    #[inline]
    pub const fn enabled() -> Self {
        FontFeatureState(Some(1))
    }

    /// Enabled state with alternative selected.
    #[inline]
    pub const fn enabled_alt(alt: NonZeroU32) -> Self {
        FontFeatureState(Some(alt.get()))
    }

    /// Disabled state.
    #[inline]
    pub const fn disabled() -> Self {
        FontFeatureState(Some(0))
    }

    /// Is [`auto`](Self::auto).
    #[inline]
    pub fn is_auto(self) -> bool {
        self == Self::auto()
    }

    /// Is [`enabled`](Self::enabled) or [`enabled_alt`](Self::enabled_alt).
    #[inline]
    pub fn is_enabled(self) -> bool {
        if let Some(n) = self.0 {
            if n >= 1 {
                return true;
            }
        }
        false
    }

    /// Is [`disabled`](Self::disabled).
    #[inline]
    pub fn is_disabled(self) -> bool {
        self == Self::disabled()
    }

    /// Gets the enabled alternative.
    #[inline]
    pub fn alt(self) -> Option<u32> {
        if let Some(n) = self.0 {
            if n >= 1 {
                return Some(n);
            }
        }
        None
    }
}
impl fmt::Debug for FontFeatureState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(n) => {
                if n == FEATURE_DISABLED {
                    write!(f, "FontFeatureState::disabled()")
                } else if n == FEATURE_ENABLED {
                    write!(f, "FontFeatureState::enabled()")
                } else {
                    write!(f, "FontFeatureState::enabled_alt({})", n)
                }
            }
            None => write!(f, "FontFeatureState::auto()"),
        }
    }
}
impl_from_and_into_var! {
    fn from(enabled: bool) -> FontFeatureState {
        if enabled {
            FontFeatureState::enabled()
        } else {
            FontFeatureState::disabled()
        }
    }

    /// `0` is disabled, `>=1` is enabled with the alt value.
    fn from(alt: u32) -> FontFeatureState {
        FontFeatureState(Some(alt))
    }
}

/// Font capital letters variant features.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CapsVariant {
    /// Disable all caps variant.
    None,

    /// No caps variant, for most text. `TitlingCaps` if the text is all in uppercase.
    Auto,

    /// Enable small caps alternative for lowercase letters.
    ///
    /// This corresponds to OpenType `smcp` feature.
    SmallCaps,

    /// Enable small caps alternative for lower and upper case letters.
    ///
    /// This corresponds to OpenType `smcp` and `c2sc` features.
    AllSmallCaps,

    /// Enable petite caps alternative for lowercase letters.
    ///
    /// This corresponds to OpenType `pcap` feature.
    Petite,

    /// Enable petite caps alternative for lower and upper case letters.
    ///
    /// This corresponds to OpenType `pcap` and `c2pc` features.
    AllPetite,

    /// Enables unicase, using small caps for upper case letters mixed with normal lowercase letters.
    ///
    /// This corresponds to OpenType `unic` feature.
    Unicase,

    /// Enable title caps alternatives. This uses alternative uppercase glyphs designed for all uppercase words.
    ///
    /// This corresponds to OpenType `titl` feature.
    TitlingCaps,
}

/// Font numeric variant features.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum NumVariant {
    /// Uses the default numeric glyphs, in most fonts this is the same as `Lining`, some fonts use the `OldStyle`.
    Auto,
    /// Uses numeric glyphs that rest on the baseline.
    ///
    /// This corresponds to OpenType `lnum` feature.
    Lining,
    /// Uses old-style numeric glyphs, where some numbers, like 3, 4, 7, 9 have descenders.
    ///
    /// This corresponds to OpenType `onum` feature.
    OldStyle,
}

/// Font numeric spacing features.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum NumSpacing {
    /// Uses the default numeric width, usually this is `Tabular` for *monospace* fonts and `Proportional` for the others.
    Auto,
    /// Numeric glyphs take different space depending on the design of the glyph.
    ///
    /// This corresponds to OpenType `pnum` feature.
    Proportional,
    /// Numeric glyphs take the same space even if the glyphs design width is different.
    ///
    /// This corresponds to OpenType `tnum` feature.
    Tabular,
}

/// Font numeric fraction features.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum NumFraction {
    /// Don't use fraction variants.
    Auto,
    /// Variant where the numerator and denominator are made smaller and separated by a slash.
    ///
    /// This corresponds to OpenType `frac` feature.
    Diagonal,
    /// Variant where the numerator and denominator are made smaller, stacked and separated by a horizontal line.
    ///
    /// This corresponds to OpenType `afrc` feature.
    Stacked,
}

/// All possible [style_set](FontFeatures::style_set) features.
///
/// The styles depend on the font, it is recommended you create an `enum` with named sets that
/// converts into this one for each font you wish to use.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum StyleSet {
    /// Don't use alternative style set.
    Auto = 0,

    S01,
    S02,
    S03,
    S04,
    S05,
    S06,
    S07,
    S08,
    S09,
    S10,

    S11,
    S12,
    S13,
    S14,
    S15,
    S16,
    S17,
    S18,
    S19,
    S20,
}
impl_from_and_into_var! {
    /// `set == 0 || set > 20` is Auto, `set >= 1 && set <= 20` maps to their variant.
    fn from(set: u8) -> StyleSet {
        if set > 20 {
            StyleSet::Auto
        } else {
            // SAFETY: We eliminated the bad values in the `if`.
            unsafe { mem::transmute(set) }
        }
    }
}
impl StyleSet {
    pub fn name(self) -> Option<FontFeatureName> {
        if self == StyleSet::Auto {
            None
        } else {
            Some(Self::NAMES[self as usize - 1])
        }
    }

    const NAMES: [FontFeatureName; 20] = [
        b"ss01", b"ss02", b"ss03", b"ss04", b"ss05", b"ss06", b"ss07", b"ss08", b"ss09", b"ss10", b"ss11", b"ss12", b"ss13", b"ss14",
        b"ss15", b"ss16", b"ss17", b"ss18", b"ss19", b"ss20",
    ];
}

/// All possible [character_variant](FontFeatures::character_variant) features (`cv00..=cv99`).
///
/// The styles depend on the font, it is recommended you create `const`s with named variants to use with a specific font.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct CharacterVariant(u8);
impl CharacterVariant {
    /// New variant.
    ///
    /// Returns auto if `v == 0 || v > 99`.
    #[inline]
    pub const fn new(v: u8) -> Self {
        if v > 99 {
            CharacterVariant(0)
        } else {
            CharacterVariant(v)
        }
    }

    /// New auto.
    #[inline]
    pub const fn auto() -> Self {
        CharacterVariant(0)
    }

    /// Is auto.
    #[inline]
    pub const fn is_auto(self) -> bool {
        self.0 == 0
    }

    /// Gets the feature name if it is not auto.
    #[inline]
    pub fn name(self) -> Option<FontFeatureName> {
        todo!()
    }

    /// Gets the variant number, if it is not auto.
    #[inline]
    pub const fn variant(self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            Some(self.0)
        }
    }
}
