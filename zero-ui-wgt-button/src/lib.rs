#![warn(unused_extern_crates)]
#![warn(missing_docs)]

//! Button widget.

zero_ui_wgt::enable_widget_macros!();

use zero_ui_wgt::{border, corner_radius, is_disabled, prelude::*};
use zero_ui_wgt_access::{access_role, labelled_by_child, AccessRole};
use zero_ui_wgt_container::{child_align, padding, Container};
use zero_ui_wgt_fill::background_color;
use zero_ui_wgt_filters::{child_opacity, saturate};
use zero_ui_wgt_input::{
    cursor,
    focus::FocusableMix,
    gesture::{on_click, ClickArgs},
    is_cap_hovered, is_pressed,
    pointer_capture::{capture_pointer, CaptureMode},
    CursorIcon,
};
use zero_ui_wgt_style::{Style, StyleFn, StyleMix};

/// A clickable container.
#[widget($crate::Button)]
pub struct Button(FocusableMix<StyleMix<Container>>);
impl Button {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;
            capture_pointer = true;
            labelled_by_child = true;
        }
    }

    widget_impl! {
        /// Button click event.
        pub on_click(handler: impl WidgetHandler<ClickArgs>);

        /// If pointer interaction with other widgets is blocked while the button is pressed.
        ///
        /// Enabled by default in this widget.
        pub capture_pointer(mode: impl IntoVar<CaptureMode>);
    }
}

context_var! {
    /// Button style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// Idle background dark and light color.
    pub static BASE_COLORS_VAR: ColorPair = (rgb(0.18, 0.18, 0.18), rgb(0.82, 0.82, 0.82));
}

/// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the button style.
#[property(CONTEXT, default(BASE_COLORS_VAR), widget_impl(DefaultStyle))]
pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
    with_context_var(child, BASE_COLORS_VAR, color)
}

/// Sets the button style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the button style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    zero_ui_wgt_style::with_style_extension(child, STYLE_VAR, style)
}

/// Create a [`color_scheme_highlight`] of `0.08`.
pub fn color_scheme_hovered(pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
    color_scheme_highlight(pair, 0.08)
}

/// Create a [`color_scheme_highlight`] of `0.16`.
pub fn color_scheme_pressed(pair: impl IntoVar<ColorPair>) -> impl Var<Rgba> {
    color_scheme_highlight(pair, 0.16)
}

/// Button default style.
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            access_role = AccessRole::Button;

            padding = (7, 15);
            corner_radius = 4;
            child_align = Align::CENTER;

            #[easing(150.ms())]
            background_color = color_scheme_pair(BASE_COLORS_VAR);

            #[easing(150.ms())]
            border = {
                widths: 1,
                sides: color_scheme_pair(BASE_COLORS_VAR).map_into()
            };

            when *#is_cap_hovered {
                #[easing(0.ms())]
                background_color = color_scheme_hovered(BASE_COLORS_VAR);
                #[easing(0.ms())]
                border = {
                    widths: 1,
                    sides: color_scheme_pressed(BASE_COLORS_VAR).map_into(),
                };
            }

            when *#is_pressed {
                #[easing(0.ms())]
                background_color = color_scheme_pressed(BASE_COLORS_VAR);
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}
