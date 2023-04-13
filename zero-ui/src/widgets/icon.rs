//! Glyph icon widget, properties and nodes..

use crate::prelude::new_widget::*;
use crate::widgets::text;

use std::fmt;

/// Render icons defined as glyphs in an icon font.
///
/// Note that no icons are embedded in this crate directly, you can manually create a [`GlyphIcon`]
/// or use an icon set crate. See the [`zero-ui-material-icons`] crate, it provides documented constants for
/// each icon in the fonts.
#[widget($crate::widgets::Icon {
    ($ico:expr) => {
        ico = $ico;
    }
})]
pub struct Icon(WidgetBase);
impl Icon {
    #[widget(on_start)]
    fn on_start(&mut self) {
        self.builder().push_build_action(on_build);
    }

    impl_properties! {
        /// Spacing in between the icon and background edges or border.
        pub fn crate::properties::padding(padding: impl IntoVar<SideOffsets>);
    }
}

/// The glyph icon.
#[property(CONTEXT, capture, impl(Icon))]
pub fn ico(child: impl UiNode, ico: impl IntoVar<GlyphIcon>) -> impl UiNode {}

fn on_build(wgt: &mut WidgetBuilding) {
    let icon = if let Some(icon) = wgt.capture_var::<GlyphIcon>(property_id!(ico)) {
        icon
    } else {
        tracing::error!("missing `icon` property");
        return;
    };

    wgt.set_child(text::nodes::render_text());

    wgt.push_intrinsic(NestGroup::FILL, "layout_text", text::nodes::layout_text);
    wgt.push_intrinsic(NestGroup::EVENT, "resolve_text", move |child| {
        let node = text::nodes::resolve_text(child, icon.map(|i| i.glyph.clone().into()));
        let node = text::font_family(node, icon.map(|i| i.font.clone().into()));
        let node = text::font_size(node, ICON_SIZE_VAR);
        text::txt_color(node, ICON_COLOR_VAR)
    });
}

/// Identifies an icon glyph in the font set.
#[derive(Clone, PartialEq, Eq, Hash)]
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
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct GlyphIcon {
    /// Icon set font name.
    pub font: FontName,
    /// Icon glyph.
    pub glyph: GlyphSource,
}
impl GlyphIcon {
    /// New icon.
    pub fn new(font: impl Into<FontName>, glyph: impl Into<GlyphSource>) -> Self {
        GlyphIcon {
            font: font.into(),
            glyph: glyph.into(),
        }
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
    /// Default is `24.dip()`.
    pub static ICON_SIZE_VAR: Length = 24.dip();

    /// Defines the color of an icon.
    ///
    /// Inherits from [`TEXT_COLOR_VAR`].
    pub static ICON_COLOR_VAR: Rgba = text::TEXT_COLOR_VAR;
}

/// Sets the [`ICON_SIZE_VAR`] that affects all icons inside the widget.
#[property(CONTEXT, default(ICON_SIZE_VAR), impl(Icon))]
pub fn ico_size(child: impl UiNode, size: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, ICON_SIZE_VAR, size)
}

/// Sets the [`ICON_COLOR_VAR`] that affects all icons inside the widget.
#[property(CONTEXT, default(ICON_COLOR_VAR), impl(Icon))]
pub fn ico_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, ICON_COLOR_VAR, color)
}
