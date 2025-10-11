//! Glyph icon widget, properties and nodes.
//!
//! Note that no icons are embedded in this crate directly, you can manually create a [`GlyphIcon`]
//! or use an icon set crate. See the `zng::icon::material` module for an example.

use zng_ext_font::{FontName, FontSize, font_features::FontFeatures};
use zng_wgt::prelude::*;

use std::fmt;

use crate::FONT_SIZE_VAR;

/// Render icons defined as glyphs in an icon font.
#[widget($crate::icon::Icon {
    ($ico:expr) => {
        ico = $ico;
    }
})]
pub struct Icon(WidgetBase);
impl Icon {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            crate::txt_align = Align::CENTER;

            // in case the icon is tested on a TextInput
            crate::txt_editable = false;
            crate::txt_selectable = false;
        }
        self.widget_builder().push_build_action(|wgt| {
            let icon = if let Some(icon) = wgt.capture_var::<GlyphIcon>(property_id!(ico)) {
                icon
            } else {
                tracing::error!("missing `icon` property");
                return;
            };

            wgt.set_child(crate::node::render_text());

            wgt.push_intrinsic(NestGroup::CHILD_LAYOUT + 100, "layout_text", move |child| {
                let node = crate::node::layout_text(child);
                icon_size(node)
            });
            wgt.push_intrinsic(NestGroup::EVENT, "resolve_text", move |child| {
                let node = crate::node::resolve_text(child, icon.map(|i| i.glyph.clone().into()));
                let node = crate::font_family(node, icon.map(|i| i.font.clone().into()));
                let node = crate::font_features(node, icon.map(|i| i.features.clone()));
                crate::font_color(node, ICON_COLOR_VAR)
            });
        });
    }
}

/// The glyph icon.
#[property(CONTEXT, widget_impl(Icon))]
pub fn ico(wgt: &mut WidgetBuilding, ico: impl IntoVar<GlyphIcon>) {
    let _ = ico;
    wgt.expect_property_capture();
}

/// Identifies an icon glyph in the font set.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum GlyphSource {
    /// Code "char" that is mapped to the glyph.
    Code(char),
    /// String that resolves to the glyph due to the default ligature config of the font.
    Ligature(Txt),
}
impl_from_and_into_var! {
    fn from(code: char) -> GlyphSource {
        GlyphSource::Code(code)
    }
    fn from(ligature: &'static str) -> GlyphSource {
        Txt::from_static(ligature).into()
    }
    fn from(ligature: Txt) -> GlyphSource {
        GlyphSource::Ligature(ligature)
    }
    fn from(source: GlyphSource) -> Txt {
        match source {
            GlyphSource::Code(c) => Txt::from_char(c),
            GlyphSource::Ligature(l) => l,
        }
    }
}
impl fmt::Debug for GlyphSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "GlyphSource::")?;
        }
        match self {
            GlyphSource::Code(c) => write!(f, "Code({c:?})"),
            GlyphSource::Ligature(l) => write!(f, "Ligature({l:?})"),
        }
    }
}
impl fmt::Display for GlyphSource {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            GlyphSource::Code(c) => write!(f, "{c}"),
            GlyphSource::Ligature(l) => write!(f, "{l}"),
        }
    }
}

/// Represents an icon glyph and font.
#[derive(Clone, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
pub struct GlyphIcon {
    /// Icon set font name.
    pub font: FontName,
    /// Font features, like ligatures.
    pub features: FontFeatures,
    /// Icon glyph.
    pub glyph: GlyphSource,
}
impl GlyphIcon {
    /// New icon.
    pub fn new(font: impl Into<FontName>, glyph: impl Into<GlyphSource>) -> Self {
        GlyphIcon {
            font: font.into(),
            features: FontFeatures::new(),
            glyph: glyph.into(),
        }
    }

    /// Enable all ligatures.
    pub fn with_ligatures(mut self) -> Self {
        self.features.common_lig().enable();
        self.features.historical_lig().enable();
        self.features.discretionary_lig().enable();
        self
    }
}
impl_from_and_into_var! {
    fn from<F: Into<FontName>, G: Into<GlyphSource>>((name, glyph): (F, G)) -> GlyphIcon {
        GlyphIcon::new(name, glyph)
    }
}

context_var! {
    /// Defines the size of an icon.
    ///
    /// Default is auto sized or the font size if cannot auto size.
    pub static ICON_SIZE_VAR: FontSize = FontSize::Default;

    /// Defines the color of an icon.
    ///
    /// Inherits from [`FONT_COLOR_VAR`].
    ///
    /// [`FONT_COLOR_VAR`]: crate::FONT_COLOR_VAR
    pub static ICON_COLOR_VAR: Rgba = crate::FONT_COLOR_VAR;
}

/// Sets the icon font size.
///
/// The [`FontSize::Default`] value enables auto size to fill, or is the `font_size` if cannot auto size.
///
/// Sets the [`ICON_SIZE_VAR`] that affects all icons inside the widget.
#[property(CONTEXT, default(ICON_SIZE_VAR), widget_impl(Icon))]
pub fn ico_size(child: impl IntoUiNode, size: impl IntoVar<FontSize>) -> UiNode {
    with_context_var(child, ICON_SIZE_VAR, size)
}

/// Sets the icon font color.
///
/// Sets the [`ICON_COLOR_VAR`] that affects all icons inside the widget.
#[property(CONTEXT, default(ICON_COLOR_VAR), widget_impl(Icon))]
pub fn ico_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode {
    with_context_var(child, ICON_COLOR_VAR, color)
}

/// Set the font-size from the parent size.
fn icon_size(child: impl IntoUiNode) -> UiNode {
    match_node(child, |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&ICON_SIZE_VAR);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let font_size = ICON_SIZE_VAR.get();
            let s = LAYOUT.constraints().fill_size();
            let mut default_size = s.width.min(s.height);
            if default_size == 0 {
                default_size = FONT_SIZE_VAR.layout_x();
            }
            let font_size_px = font_size.layout_dft_x(default_size);
            *desired_size = if font_size_px >= 0 {
                LAYOUT.with_font_size(font_size_px, || child.measure(wm))
            } else {
                tracing::error!("invalid icon font size {font_size:?} => {font_size_px:?}");
                child.measure(wm)
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            let font_size = ICON_SIZE_VAR.get();
            let s = LAYOUT.constraints().fill_size();
            let mut default_size = s.width.min(s.height);
            if default_size == 0 {
                default_size = FONT_SIZE_VAR.layout_x();
            }
            let font_size_px = font_size.layout_dft_x(default_size);
            *final_size = if font_size_px >= 0 {
                LAYOUT.with_font_size(font_size_px, || child.layout(wl))
            } else {
                tracing::error!("invalid icon font size {font_size:?} => {font_size_px:?}");
                child.layout(wl)
            };
        }
        _ => {}
    })
}
