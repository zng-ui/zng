//! Font resolving and text shaping.

use super::units::{layout_length_to_pt, LayoutLength, LayoutPoint, LayoutRect, LayoutSize};
use crate::core::app::AppExtension;
use crate::core::context::{AppInitContext, WindowService};
use crate::core::types::{FontInstanceKey, FontName, FontProperties, FontStyle};
use crate::core::var::ContextVar;
use crate::properties::text_theme::FontFamilyVar;
use fnv::FnvHashMap;
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

        let r = harfbuzz_rs::shape(&self.inner.harfbuzz_font, buffer, &features);

        let mut origin = LayoutPoint::new(0.0, self.metrics().ascent);

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

        let font_size = self.inner.font_size as f32;
        let bounds = if glyphs.is_empty() {
            LayoutSize::new(0.0, font_size)
        } else {
            LayoutSize::new(origin.x, config.line_height(font_size))
        };

        ShapedLine { glyphs, bounds }
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

/// Extra configuration for [`shape_text`](FontInstance::shape_text).
#[derive(Debug, Clone, Default)]
pub struct ShapingConfig {
    /// Extra spacing to add between characters.
    pub letter_spacing: Option<f32>,

    /// Spacing to add between each word.
    ///
    /// Use [`word_spacing(..)`](function@Self::word_spacing) to compute the value.
    pub word_spacing: Option<f32>,

    /// Space to add between each line.
    ///
    /// Use [`line_height(..)`](function@Self::line_height) to compute the value.
    pub line_height: Option<f32>,

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

    pub word_break: (),

    pub line_break: LineBreak,

    pub justify: Option<Justify>,

    /// Width of the TAB character.
    ///
    /// By default 8 x space.
    pub tab_size: Option<f32>,

    /// Extra space before the start of the first line.
    pub text_indent: f32,
}

impl ShapingConfig {
    /// Gets the custom word spacing or 0.25em.
    #[inline]
    pub fn word_spacing(&self, font_size: f32) -> f32 {
        self.word_spacing.unwrap_or(font_size * 0.25)
    }

    /// Gets the custom line height or 1.3em.
    #[inline]
    pub fn line_height(&self, font_size: f32) -> f32 {
        self.line_height.unwrap_or(font_size * 1.3)
    }

    /// Gets the custom paragraph spacing or one line height.
    #[inline]
    pub fn paragraph_spacing(&self, font_size: f32) -> f32 {
        self.line_height(font_size)
    }
}

/// Result of [`shape_text`](FontInstance::shape_text).
#[derive(Debug, Clone)]
pub struct ShapedLine {
    /// Glyphs for the renderer.
    pub glyphs: Vec<GlyphInstance>,
    /// Size of the text for the layout.
    pub bounds: LayoutSize,
}

#[derive(Debug, Copy, Clone)]
pub enum LineBreak {
    Auto,
    Loose,
    Normal,
    Strict,
    Anywhere,
}
impl Default for LineBreak {
    /// [`LineBreak::Auto`]
    fn default() -> Self {
        LineBreak::Auto
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Hyphenation {
    None,
    /// `\u{2010}` HYPHEN, `\u{00AD}` SHY
    Manual,
    Auto,
}
impl Default for Hyphenation {
    /// [`Hyphenation::Auto`]
    fn default() -> Self {
        Hyphenation::Auto
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WordBreak {
    Normal,
    BreakAll,
    KeepAll,
}
impl Default for WordBreak {
    /// [`WordBreak::Normal`]
    fn default() -> Self {
        WordBreak::Normal
    }
}
pub enum TextAlign {
    /// `Left` in LTR or `Right` in RTL.
    Start,
    /// `Right` in LTR or `Left` in RTL.
    End,

    Left,
    Center,
    Right,

    Justify,
}

#[derive(Debug, Copy, Clone)]
pub enum Justify {
    Auto,
    InterWord,
    InterCharacter,
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
            ascent: s(dbg!(metrics.ascent)),
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
}
