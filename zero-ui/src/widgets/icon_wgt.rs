use crate::prelude::new_widget::*;
use crate::widgets::text;

use std::fmt;

/// Render icons defined as glyphs in an icon font.
///
/// Note that no icons are embedded in this crate directly, you can manually create a [`GlyphIcon`]
/// or use an icon set crate. See the [`zero-ui-material-icons`] crate, it provides documented constants for
/// each icon in the fonts.
#[widget($crate::widgets::icon)]
pub mod icon {
    use super::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use super::vis;

    properties! {
        /// The glyph icon.
        pub icon(impl IntoVar<icon::GlyphIcon>);

        /// Icon size, best sizes are 18, 24, 36 or 48dip, default is 24dip.
        ///
        /// This is a single [`Length`] value that sets the "font size" of the icon glyph.
        pub vis::icon_size;

        /// Icon color.
        pub vis::icon_color as color;

        /// Spacing in between the icon and background edges or border.
        ///
        /// Set to `0` by default.
        pub text::properties::text_padding as padding;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(on_build);
    }

    fn on_build(wgt: &mut WidgetBuilding) {
        let icon = if let Some(icon) = wgt.capture_var::<GlyphIcon>(property_id!(self.icon)) {
            icon
        } else {
            tracing::error!("missing `icon` property");
            return;
        };

        wgt.set_child(text::nodes::render_text());

        wgt.push_intrinsic(Priority::Fill, text::nodes::layout_text);
        wgt.push_intrinsic(Priority::Event, |child| {
            let node = text::nodes::resolve_text(child, icon.map(|i| i.glyph.clone().into()));
            let node = text::properties::font_family(node, icon.map(|i| i.font.clone().into()));
            let node = text::properties::font_size(node, vis::ICON_SIZE_VAR);
            text::properties::text_color(node, vis::ICON_COLOR_VAR)
        });
    }

    /// Identifies an icon glyph in the font set.
    #[derive(Clone, PartialEq, Eq, Hash)]
    pub enum GlyphSource {
        /// Code "char" that is mapped to the glyph.
        Code(char),
        /// String that resolves to the glyph due to the default ligature config of the font.
        Ligature(Text),
    }
    impl_from_and_into_var! {
        fn from(code: char) -> GlyphSource {
            GlyphSource::Code(code)
        }
        fn from(ligature: &'static str) -> GlyphSource {
            Text::from_static(ligature).into()
        }
        fn from(ligature: Text) -> GlyphSource {
            GlyphSource::Ligature(ligature)
        }
        fn from(source: GlyphSource) -> Text {
            match source {
                GlyphSource::Code(c) => Text::from_char(c),
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
        fn from<F: Into<FontName> + Clone, G: Into<GlyphSource> + Clone>((name, glyph): (F, G)) -> GlyphIcon {
            GlyphIcon::new(name, glyph)
        }
    }
}
/// Context variables and properties that affect icons.
pub mod vis {
    use super::*;

    use crate::widgets::text::properties::TEXT_COLOR_VAR;

    context_var! {
        /// Defines the size of an icon.
        ///
        /// Default is `24.dip()`.
        pub static ICON_SIZE_VAR: Length = 24.dip();

        /// Defines the color of an icon.
        ///
        /// Inherits from [`TEXT_COLOR_VAR`].
        pub static ICON_COLOR_VAR: Rgba = TEXT_COLOR_VAR;
    }

    /// Sets the [`ICON_SIZE_VAR`] that affects all icons inside the widget.
    #[property(context, default(ICON_SIZE_VAR))]
    pub fn icon_size(child: impl UiNode, size: impl IntoVar<Length>) -> impl UiNode {
        with_context_var(child, ICON_SIZE_VAR, size)
    }

    /// Sets the [`ICON_COLOR_VAR`] that affects all icons inside the widget.
    #[property(context, default(ICON_COLOR_VAR))]
    pub fn icon_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, ICON_COLOR_VAR, color)
    }
}

/// Short form [`icon!`].
///
/// [`icon!`]: mod@icon
pub fn icon(ico: impl IntoVar<icon::GlyphIcon>) -> impl UiNode {
    icon!(icon = ico)
}
