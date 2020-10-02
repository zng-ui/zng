use crate::core::color::{web_colors, Rgba};
use crate::core::context::*;
use crate::core::text::*;
use crate::core::impl_ui_node;
use crate::core::profiler::profile_scope;
use crate::core::render::FrameBuilder;
use crate::core::types::Text;
use crate::core::types::*;
use crate::core::units::*;
use crate::core::var::{IntoVar, ObjVar, Var};
use crate::core::{UiNode, Widget};
use crate::properties::{capture_only::text_value, text_theme::*};
use zero_ui_macros::widget;

struct TextNode<T: Var<Text>> {
    text: T,

    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font: Option<FontInstance>,
    color: Rgba,
}
impl<T: Var<Text>> TextNode<T> {
    fn aligned_size(&self, pixels: PixelGrid) -> LayoutSize {
        self.size.snap_to(pixels)
    }
}
#[impl_ui_node(none)]
impl<T: Var<Text>> UiNode for TextNode<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        profile_scope!("text::init");

        self.color = *TextColorVar::var().get(ctx.vars);
        let font_size = *FontSizeVar::var().get(ctx.vars);
        let font_properties = FontProperties {
            style: *FontStyleVar::var().get(ctx.vars),
            weight: *FontWeightVar::var().get(ctx.vars),
            stretch: *FontStretchVar::var().get(ctx.vars),
        };

        let font_family = FontFamilyVar::var();
        let font_family = font_family.get(ctx.vars);
        let font = ctx
            .window_services
            .req::<Fonts>()
            .get_or_default(font_family, &font_properties, font_size);

        let text = self.text.get(ctx.vars);
        let text = TextTransformVar::var().get(ctx.vars).transform(text);

        let r = font.shape_text(&text, &Default::default());
        self.glyphs = r.glyphs;

        self.size = r.bounds;
        self.font = Some(font);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        profile_scope!("text::update");

        if self.text.is_new(ctx.vars)
            || FontFamilyVar::var().is_new(ctx.vars)
            || FontSizeVar::var().is_new(ctx.vars)
            || TextTransformVar::var().is_new(ctx.vars)
        {
            self.init(ctx);
            ctx.updates.push_layout();
        }

        if let Some(&color) = TextColorVar::var().update(ctx.vars) {
            self.color = color;
            ctx.updates.push_render();
        }
    }

    fn measure(&mut self, _: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        self.aligned_size(ctx.pixel_grid())
    }

    fn render(&self, frame: &mut FrameBuilder) {
        profile_scope!("text::render");
        let size = self.aligned_size(frame.pixel_grid());
        frame.push_text(
            LayoutRect::from_size(size),
            &self.glyphs,
            self.font.as_ref().unwrap().instance_key(),
            self.color.into(),
            None,
        )
    }
}

widget! {
    /// A configured [`text`](../fn.text.html).
    ///
    /// # Example
    ///
    /// ```
    /// use zero_ui::widgets::text;
    ///
    /// let hello_txt = text! {
    ///     font_family: "Arial";
    ///     font_size: 18;
    ///     text: "Hello!";
    /// };
    /// ```
    /// # `text()`
    ///
    /// If you don't need to configure the text, you can just use the function [`text`](../fn.text.html).
    pub text;

    default_child {
        /// The [`Text`](crate::core::types::Text) value.
        ///
        /// Set to an empty string (`""`).
        text -> text_value: "";
    }

    default {
        /// The text font. If not set inherits the `font_family` from the parent widget.
        font_family;
        /// The font style. If not set inherits the `font_style` from the parent widget.
        font_style;
        /// The font weight. If not set inherits the `font_weight` from the parent widget.
        font_weight;
        /// The font stretch. If not set inherits the `font_stretch` from the parent widget.
        font_stretch;
        /// The font size. If not set inherits the `font_size` from the parent widget.
        font_size;
        /// The text color. If not set inherits the `text_color` from the parent widget.
        color -> text_color;
    }

    /// Creates a [`text`](../fn.text.html).
    #[inline]
    fn new_child(text) -> impl UiNode {
        TextNode {
            text: text.unwrap().into_var(),

            glyphs: vec![],
            size: LayoutSize::default(),
            font: None,
            color: web_colors::BLACK,
        }
    }
}

/// Simple text run.
///
/// # Configure
///
/// Text spans can be configured by setting [`font_family`](crate::properties::text_theme::font_family),
/// [`font_size`](crate::properties::text_theme::font_size) or [`text_color`](crate::properties::text_theme::text_color)
/// in parent widgets.
///
/// # Example
/// ```
/// # fn main() -> () {
/// use zero_ui::widgets::{container, text};
/// use zero_ui::properties::{font_family, font_size};
///
/// let hello_txt = container! {
///     font_family: "Arial";
///     font_size: 18;
///     content: text("Hello!");
/// };
/// # }
/// ```
///
/// # `text!`
///
/// There is a specific widget for creating configured text runs: [`text!`](text/index.html).
pub fn text(text: impl IntoVar<Text> + 'static) -> impl Widget {
    // TODO remove 'static when rust issue #42940 is fixed.
    text! {
        text;
    }
}
