//! Glyph icon widget, properties and nodes..

use zero_ui_app::event::{CommandMetaVar, StaticCommandMetaVarId};
use zero_ui_ext_font::{font_features::FontFeatures, FontName};
use zero_ui_wgt::prelude::*;

use std::fmt;

/// Render icons defined as glyphs in an icon font.
///
/// Note that no icons are embedded in this crate directly, you can manually create a [`GlyphIcon`]
/// or use an icon set crate. See the [`zero-ui-material-icons`] crate, it provides documented constants for
/// each icon in the fonts.
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
        }
        self.widget_builder().push_build_action(on_build);
    }
}

/// The glyph icon.
#[property(CONTEXT, capture, widget_impl(Icon))]
pub fn ico(ico: impl IntoVar<GlyphIcon>) {}

fn on_build(wgt: &mut WidgetBuilding) {
    let icon = if let Some(icon) = wgt.capture_var::<GlyphIcon>(property_id!(ico)) {
        icon
    } else {
        tracing::error!("missing `icon` property");
        return;
    };

    wgt.set_child(crate::node::render_text());

    wgt.push_intrinsic(NestGroup::CHILD_LAYOUT + 100, "layout_text", crate::node::layout_text);
    wgt.push_intrinsic(NestGroup::EVENT, "resolve_text", move |child| {
        let node = crate::node::resolve_text(child, icon.map(|i| i.glyph.clone().into()));
        let node = crate::font_family(node, icon.map(|i| i.font.clone().into()));
        let node = crate::font_size(node, ICON_SIZE_VAR);
        let node = crate::font_features(node, icon.map_ref(|i| &i.features));
        crate::font_color(node, ICON_COLOR_VAR)
    });
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
    /// Default is `24.dip()`.
    pub static ICON_SIZE_VAR: Length = 24.dip();

    /// Defines the color of an icon.
    ///
    /// Inherits from [`FONT_COLOR_VAR`].
    pub static ICON_COLOR_VAR: Rgba = crate::FONT_COLOR_VAR;
}

/// Sets the [`ICON_SIZE_VAR`] that affects all icons inside the widget.
#[property(CONTEXT, default(ICON_SIZE_VAR), widget_impl(Icon))]
pub fn ico_size(child: impl UiNode, size: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, ICON_SIZE_VAR, size)
}

/// Sets the [`ICON_COLOR_VAR`] that affects all icons inside the widget.
#[property(CONTEXT, default(ICON_COLOR_VAR), widget_impl(Icon))]
pub fn ico_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
    with_context_var(child, ICON_COLOR_VAR, color)
}

/// Adds the [`icon`](CommandIconExt) metadata.
///
/// The value is an [`WidgetFn<()>`] that can generate any icon widget, recommended widget is [`Icon!`].
///
/// [`Icon!`]: struct@Icon
pub trait CommandIconExt {
    /// Gets a read-write variable that is the icon for the command.
    fn icon(self) -> CommandMetaVar<WidgetFn<()>>;

    /// Sets the initial icon if it is not set.
    fn init_icon(self, icon: WidgetFn<()>) -> Self;
}
static COMMAND_ICON_ID: StaticCommandMetaVarId<WidgetFn<()>> = StaticCommandMetaVarId::new_unique();
impl CommandIconExt for Command {
    fn icon(self) -> CommandMetaVar<WidgetFn<()>> {
        self.with_meta(|m| m.get_var_or_default(&COMMAND_ICON_ID))
    }

    fn init_icon(self, icon: WidgetFn<()>) -> Self {
        self.with_meta(|m| m.init_var(&COMMAND_ICON_ID, icon));
        self
    }
}
