//! Color and gradient types, functions, properties and macros.
//!
//! # Colors
//!
//! The example demonstrates multiple ways of declaring or selecting a color, all blue in this case.
//!  
//! ```
//! use zng::prelude::*;
//!
//! fn sample(color: impl IntoVar<color::Rgba>) -> UiNode {
//!     Wgt! {
//!         layout::size = (100, 40);
//!         widget::background_color = color;
//!     }
//! }
//!
//! # fn example() {
//! # let _ =
//! Window! {
//!     child = Stack!(
//!         top_to_bottom,
//!         5,
//!         ui_vec![
//!             sample(hex!(#00F)),
//!             sample(rgb(0, 0, 255)),
//!             sample(rgb(0.0, 0.0, 1.0)),
//!             sample(colors::BLUE),
//!             sample(hsv(240.deg(), 100.pct(), 100.pct())),
//!             sample(hsl(240.deg(), 100.pct(), 50.pct())),
//!         ]
//!     );
//! }
//! # ; }
//! ```
//!
//! The [`Rgba`] type also provides methods for basic color manipulation and mixing.
//!
//! ```rust,no_fmt
//! # use zng::prelude::*;
//! # fn sample(_: impl IntoVar<color::Rgba>) -> UiNode {
//! # widget::node::UiNode::nil()
//! # }
//! # let _ = ui_vec![
//! sample(colors::GREEN.darken(50.pct())),
//! sample(colors::GREEN),
//! sample(colors::GREEN.lighten(50.pct())),
//! sample(colors::GREEN.desaturate(50.pct())),
//! sample(colors::GREEN.with_alpha(50.pct()).mix_normal(colors::BLUE)),
//! # ];
//! ```
//!
//! Color mixing methods apply the color over the parameter, that is `foreground.mix_normal(background)`.
//!
//! # Color Filters
//!
//! The [`filter`] module provides implementation of pixel filter graphical effects that you may be
//! familiar with from CSS.
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! # let _ =
//! Window! {
//!     clear_color = colors::BLACK.transparent();
//!     color::filter::opacity = 50.pct();
//!     child = Text!("translucent window");
//! }
//! # ; }
//! ```
//!
//! The example above applies [`filter::opacity`] on the window, making it translucent in view-process
//! implementations that support transparent windows.
//!
//! [`filter::opacity`]: fn@filter::opacity
//!
//! # Gradients
//!
//! The [`gradient`] module provides implementation of linear, radial and conic gradients. Usually the
//! gradient nodes are wrapped in some other property like [`widget::background_conic`], but they can be used directly.
//!
//! [`widget::background_conic`]: fn@crate::widget::background_conic
//!
//! ```
//! use zng::prelude::*;
//!
//! # fn example() {
//! # let _ =
//! Window! {
//!     widget::background = color::gradient::conic_gradient(
//!         50.pct(),
//!         45.deg(),
//!         color::gradient::stops![colors::GREEN, (colors::RED, 30.pct()), colors::BLUE],
//!     );
//!     // OR
//!     widget::background_conic = {
//!         center: 50.pct(),
//!         angle: 45.deg(),
//!         stops: color::gradient::stops![colors::GREEN, (colors::RED, 30.pct()), colors::BLUE],
//!     };
//! }
//! # ; }
//! ```
//!
//! See [`gradient::stops!`] for the macro syntax.
//!
//! # Full API
//!
//! See [`zng_color`], [`zng_wgt_filter`] and [`zng_wgt_fill`] for the full API.

pub use zng_color::{
    COLOR_SCHEME_VAR, ColorScheme, Hsla, Hsva, LerpSpace, LightDark, LightDarkVarExt, MixAdjust, MixBlendMode, PreMulRgba,
    RenderMixBlendMode, Rgba, colors, hex, hsl, hsla, hsla_linear_sampler, hsla_sampler, hsv, hsva, lerp_space, light_dark, rgb, rgba,
    rgba_sampler, web_colors, with_lerp_space,
};

pub use zng_wgt::{accent_color, base_color, color_scheme};

pub use zng_wgt_fill::node::flood;

/// Color filter types and properties.
#[cfg(feature = "color_filter")]
pub mod filter {
    pub use zng_color::filter::{ColorMatrix, Filter, RenderFilter};

    pub use zng_wgt_filter::{
        backdrop_blur, backdrop_brightness, backdrop_color_matrix, backdrop_contrast, backdrop_filter, backdrop_grayscale,
        backdrop_hue_rotate, backdrop_invert, backdrop_saturate, backdrop_sepia, blur, brightness, child_filter, child_mix_blend,
        child_opacity, color_matrix, contrast, drop_shadow, filter, grayscale, hue_rotate, invert_color, mix_blend, opacity, saturate,
        sepia,
    };
}

/// Color gradient types and nodes.
pub mod gradient {
    pub use zng_color::gradient::{
        ColorStop, ExtendMode, GradientRadius, GradientRadiusBase, GradientStop, GradientStops, LinearGradientAxis, RenderExtendMode,
        RenderGradientStop, stops,
    };

    pub use zng_wgt_fill::node::{
        ConicGradient, GradientBuilder, LinearGradient, RadialGradient, TiledConicGradient, TiledLinearGradient, TiledRadialGradient,
        conic_gradient, gradient, linear_gradient, radial_gradient,
    };
}
