//! Text input widget, properties and nodes..

use crate::prelude::new_widget::*;

/// Simple text editor widget.
///
/// If `txt` is set to a variable that can be modified the widget becomes interactive, it implements
/// the usual *text box* capabilities, keyboard controlled editing of short text in a single style, mouse
/// selecting and caret positioning.
///
/// You can also use the [`text::commands`] to edit the text.
///
/// # Undo/Redo
///
/// Undo/redo is enabled by default, the widget is an undo scope and handles undo commands. Note that external
/// changes to the `txt` variable will clear the undo stack, only changes done by the widget can be undone.
#[widget($crate::widgets::TextInput)]
pub struct TextInput(StyleMix<UndoMix<EnabledMix<text::Text>>>);
impl TextInput {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            txt_editable = true;
            capture_mouse = true;
            txt_align = Align::TOP_START;
            focusable = true;
            undo_scope = true;
            style_fn = STYLE_VAR;
        }
    }
}

context_var! {
    /// Text input style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// Idle background dark and light color.
    pub static BASE_COLORS_VAR: ColorPair = (rgb(0.12, 0.12, 0.12), rgb(0.88, 0.88, 0.88));
}

/// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the text input style.
#[property(CONTEXT, default(BASE_COLORS_VAR), widget_impl(DefaultStyle))]
pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
    with_context_var(child, BASE_COLORS_VAR, color)
}

/// Sets the text input style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the text input style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Default border color.
pub fn border_color() -> impl Var<Rgba> {
    color_scheme_highlight(BASE_COLORS_VAR, 0.20)
}

/// Border color hovered.
pub fn border_color_hovered() -> impl Var<Rgba> {
    color_scheme_highlight(BASE_COLORS_VAR, 0.30)
}

/// Border color focused.
pub fn border_color_focused() -> impl Var<Rgba> {
    color_scheme_highlight(BASE_COLORS_VAR, 0.40)
}

/// Text input default style.
#[widget($crate::widgets::text_input::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            padding = (7, 15);
            crate::properties::cursor = CursorIcon::Text;
            crate::properties::background_color = color_scheme_pair(BASE_COLORS_VAR);
            crate::properties::border = {
                widths: 1,
                sides: border_color().map_into(),
            };

            when *#is_cap_hovered || *#is_return_focus {
                border = {
                    widths: 1,
                    sides: border_color_hovered().map_into(),
                };
            }

            when *#is_focused {
                border = {
                    widths: 1,
                    sides: border_color_focused().map_into(),
                };
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}
