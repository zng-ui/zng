use super::{Hsla, Hsva, PreMulRgba, Rgba, clamp_normal};
use zng_layout::unit::Factor;

use paste::*;

/// Webrender [`MixBlendMode`].
pub type RenderMixBlendMode = zng_view_api::MixBlendMode;

macro_rules! impl_mix {
    (
        separable {$(
            $(#[$meta:meta])*
            $Mode:ident => |$fg:ident, $bg:ident, $ca0:ident, $ca1:ident| $mix:expr,
        )+}

        non_separable {$(
            $(#[$ns_meta:meta])*
            $NsMode:ident => |[$fgh:ident, $fgs:ident, $fgl:ident], [$bgh:ident, $bgs:ident, $bgl:ident]| $ns_mix:expr,
        )+}
    ) => {
        /// Color mix blend mode.
        #[repr(u8)]
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        pub enum MixBlendMode {
            $(
                $(#[$meta])*
                $Mode,
            )+
            $(
                $(#[$ns_meta])*
                $NsMode,
            )+
        }

        impl From<MixBlendMode> for RenderMixBlendMode {
            fn from(mode: MixBlendMode) -> Self {
                match mode {
                    $(MixBlendMode::$Mode => RenderMixBlendMode::$Mode,)+
                    $(MixBlendMode::$NsMode => RenderMixBlendMode::$NsMode,)+
                }
            }
        }

        /// Color mix and adjustment methods.
        pub trait MixAdjust {
            /// MixAdjust `background` over `self` using the `mode`.
            fn mix(self, mode: MixBlendMode, background: Self) -> Self where Self:Sized {
                match mode {
                    $(MixBlendMode::$Mode => paste!(self.[<mix_ $Mode:lower>](background)),)+
                    $(MixBlendMode::$NsMode => paste!(self.[<mix_ $NsMode:lower>](background)),)+
                }
            }

            $(
                paste! {
                    #[doc = "MixAdjust `background` over `self` using the [`MixBlendMode::" $Mode "`]."]
                    ///
                    $(#[$meta])*
                    fn [<mix_ $Mode:lower>](self, background: Self) -> Self;
                }
            )+

            $(
                paste! {
                    #[doc = "MixAdjust `self` over `background` using the [`MixBlendMode::`" $NsMode "`]."]
                    ///
                    $(#[$ns_meta])*
                    ///
                    /// This method converts both inputs to [`Hsla`] and the result back to `Rgba`.
                    fn [<mix_ $NsMode:lower>](self, background: Self) -> Self;
                }
            )+

            /// Adds the `amount` to the color *lightness*.
            ///
            /// # Examples
            ///
            /// Add `10%` of the current lightness to the `DARK_RED` color:
            ///
            /// ```
            /// # use zng_color::*;
            /// # use zng_layout::unit::*;
            /// web_colors::DARK_RED.lighten(10.pct())
            /// # ;
            /// ```
            fn lighten<A: Into<Factor>>(self, amount: A) -> Self;

            /// Subtracts the `amount` from the color *lightness*.
            ///
            /// # Examples
            ///
            /// Removes `10%` of the current lightness from the `DARK_RED` color:
            ///
            /// ```
            /// # use zng_color::*;
            /// # use zng_layout::unit::*;
            /// web_colors::DARK_RED.darken(10.pct())
            /// # ;
            fn darken<A: Into<Factor>>(self, amount: A) -> Self;

            /// Subtracts the `amount` from the color *saturation*.
            ///
            /// # Examples
            ///
            /// Removes `10%` of the current saturation from the `RED` color:
            ///
            /// ```
            /// # use zng_color::*;
            /// # use zng_layout::unit::*;
            /// colors::RED.desaturate(10.pct())
            /// # ;
            fn desaturate<A: Into<Factor>>(self, amount: A) -> Self;

            /// Returns a copy of this color with a new `lightness`.
            fn with_lightness<L: Into<Factor>>(self, lightness: L) -> Self;
        }

        impl PreMulRgba {
            $(
                paste! {
                    fn [<mix_ $Mode:lower _impl>](self, background: PreMulRgba) -> PreMulRgba {
                        PreMulRgba {
                            red: {
                                let $fg = self.red;
                                let $bg = background.red;
                                let $ca0 = self.alpha;
                                let $ca1 = background.alpha;
                                $mix
                            },
                            green: {
                                let $fg = self.green;
                                let $bg = background.green;
                                let $ca0 = self.alpha;
                                let $ca1 = background.alpha;
                                $mix
                            },
                            blue: {
                                let $fg = self.blue;
                                let $bg = background.blue;
                                let $ca0 = self.alpha;
                                let $ca1 = background.alpha;
                                $mix
                            },
                            alpha: {
                                let fga = self.alpha;
                                let bga = background.alpha;
                                (fga + bga - fga * bga).max(0.0).min(1.0)
                            },
                        }
                    }
                }
            )+
        }

        impl Hsla {
            $(
                paste! {
                    fn [<mix_ $NsMode:lower _impl>](self, background: Hsla) -> Hsla {
                        let $fgh = self.hue;
                        let $fgs = self.saturation;
                        let $fgl = self.lightness;

                        let $bgh = background.hue;
                        let $bgs = background.saturation;
                        let $bgl = background.lightness;

                        let [h, s, l] = { $ns_mix };

                        Hsla {
                            hue: h,
                            saturation: s,
                            lightness: l,
                            alpha: {
                                let fga = self.alpha;
                                let bga = background.alpha;
                                (fga + bga - fga * bga).max(0.0).min(1.0)
                            }
                        }
                    }
                }
            )+
        }

        impl MixAdjust for PreMulRgba {
            $(
                paste! {
                    fn [<mix_ $Mode:lower>](self, background: Self) -> Self {
                       self.[<mix_ $Mode:lower _impl>](background)
                    }
                }
            )+
            $(
                paste! {
                    fn [<mix_ $NsMode:lower>](self, background: Self) -> Self {
                        Hsla::from(self).[<mix_ $NsMode:lower>](Hsla::from(background)).into()
                    }
                }
            )+

            fn lighten<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).lighten(amount.into()).into()
            }

            fn darken<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).darken(amount.into()).into()
            }

            fn desaturate<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).desaturate(amount).into()
            }

            fn with_lightness<L: Into<Factor>>(self, lightness: L) -> Self {
                Hsla::from(self).with_lightness(lightness).into()
            }
        }

        impl MixAdjust for Hsla {
            $(
                paste! {
                    fn [<mix_ $Mode:lower>](self, background: Self) -> Self {
                        PreMulRgba::from(self).[<mix_ $Mode:lower>](PreMulRgba::from(background)).into()
                    }
                }
            )+

            $(
                paste! {
                    fn [<mix_ $NsMode:lower>](self, background: Self) -> Self {
                       self.[<mix_ $NsMode:lower _impl>](background)
                    }
                }
            )+

            fn lighten<A: Into<Factor>>(self, amount: A) -> Self {
                let mut lighter = self;
                lighter.lightness = clamp_normal(lighter.lightness + (lighter.lightness * amount.into().0));
                lighter
            }

            fn darken<A: Into<Factor>>(self, amount: A) -> Self {
                let mut darker = self;
                darker.lightness = clamp_normal(darker.lightness - (darker.lightness * amount.into().0));
                darker
            }

            fn desaturate<A: Into<Factor>>(self, amount: A) -> Self {
                let mut d = self;
                d.saturation = clamp_normal(d.saturation - (d.saturation * amount.into().0));
                d
            }

            fn with_lightness<L: Into<Factor>>(mut self, lightness: L) -> Self {
                self.set_lightness(lightness);
                self
            }
        }

        impl MixAdjust for Rgba {
            $(
                paste! {
                    fn [<mix_ $Mode:lower>](self, background: Self) -> Self {
                        PreMulRgba::from(self).[<mix_ $Mode:lower>](PreMulRgba::from(background)).into()
                    }
                }
            )+
            $(
                paste! {
                    fn [<mix_ $NsMode:lower>](self, background: Self) -> Self {
                        Hsla::from(self).[<mix_ $NsMode:lower>](Hsla::from(background)).into()
                    }
                }
            )+

            fn lighten<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).lighten(amount.into()).into()
            }

            fn darken<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).darken(amount.into()).into()
            }

            fn desaturate<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).desaturate(amount).into()
            }

            fn with_lightness<L: Into<Factor>>(self, lightness: L) -> Self {
                Hsla::from(self).with_lightness(lightness).into()
            }
        }

        impl MixAdjust for Hsva {
            $(
                paste! {
                    fn [<mix_ $Mode:lower>](self, background: Self) -> Self {
                        PreMulRgba::from(self).[<mix_ $Mode:lower>](PreMulRgba::from(background)).into()
                    }
                }
            )+
            $(
                paste! {
                    fn [<mix_ $NsMode:lower>](self, background: Self) -> Self {
                        Hsla::from(self).[<mix_ $NsMode:lower>](Hsla::from(background)).into()
                    }
                }
            )+

            fn lighten<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).lighten(amount.into()).into()
            }

            fn darken<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).darken(amount.into()).into()
            }

            fn desaturate<A: Into<Factor>>(self, amount: A) -> Self {
                Hsla::from(self).desaturate(amount).into()
            }

            fn with_lightness<L: Into<Factor>>(self, lightness: L) -> Self {
                Hsla::from(self).with_lightness(lightness).into()
            }
        }
    };
}

impl_mix! {
    separable {
        /// Normal alpha blend of the foreground color over the background.
        Normal => |fg, bg, fga, _bga| fg + bg * (1.0 - fga),

        /// Multiply the colors.
        ///
        /// The resultant color is always at least as dark as either color.
        /// Multiplying any color with black results in black.
        /// Multiplying any color with white preserves the original color.
        Multiply => |fg, bg, fga, bga| fg * bg + fg * (1.0 - bga) + bg * (1.0 - fga),

        /// Multiply the colors, then complements the result.
        ///
        /// The result color is always at least as light as either of the two constituent colors.
        /// Screening any color with white produces white; screening with black leaves the original color unchanged.
        /// The effect is similar to projecting multiple photographic slides simultaneously onto a single screen.
        Screen => |fg, bg, _fga, _bga| fg + bg - fg * bg,

        /// Multiplies or screens the colors, depending on the background color value.
        ///
        /// Foreground color overlays the background while preserving its highlights and shadows.
        /// The background color is not replaced but is mixed with the foreground color to reflect the lightness or darkness
        /// of the background.
        ///
        /// This is the inverse of *hardlight*.
        Overlay => |fg, bg, fga, bga| if bg * 2.0 <= bga {
            2.0 * fg * bg + fg * (1.0 - bga) + bg * (1.0 - fga)
        } else {
            fg * (1.0 + bga) + bg * (1.0 + fga) - 2.0 * fg * bg - fga * bga
        },

        /// Selects the darker of the colors.
        ///
        /// The background color is replaced with the foreground where the background is darker; otherwise, it is left unchanged.
        Darken => |fg, bg, fga, bga| (fg * bga).min(bg * fga) + fg * (1.0 - bga) + bg * (1.0 - fga),

        /// Selects the lighter of the colors.
        ///
        /// The background color is replaced with the foreground where the background is lighter; otherwise, it is left unchanged.
        Lighten => |fg, bg, fga, bga| (fg * bga).max(bg * fga) + fg * (1.0 - bga) + bg * (1.0 - fga),

        /// Brightens the background color to reflect the foreground color. Painting with black produces no changes.
        ColorDodge => |fg, bg, fga, bga| if fg == fga {
            fga * bga + fg * (1.0 - bga) + bg * (1.0 - fga)
        } else {
            fga * bga * 1.0_f32.min((bg / bga) * fga / (fga - fg)) + fg * (1.0 - bga) + bg * (1.0 - fga)
        },

        /// Darkens the background color to reflect the foreground color. Painting with white produces no change.
        ColorBurn => |fg, bg, fga, bga| fga * bga * (1.0 - 1.0_f32.min((1.0 - bg / bga) * fga / fg)) + fg * (1.0 - bga) + bg * (1.0 - fga),

        /// Multiplies or screens the colors, depending on the foreground color value.
        ///
        /// The effect is similar to shining a harsh spotlight on the background color.
        HardLight => |fg, bg, fga, bga| if fg * 2.0 <= fga {
            2.0 * fg * bg + fg * (1.0 - bga) + bg * (1.0 - fga)
        } else {
            fg * (1.0 + bga) + bg * (1.0 + fga) - 2.0 * fg * bg - fga * bga
        },

        /// Darkens or lightens the colors, depending on the foreground color value.
        ///
        /// The effect is similar to shining a diffused spotlight on the background color.
        SoftLight => |fg, bg, fga, bga| {
            let m = bg / bga;

            if fg * 2.0 <= fga {
                bg * (fga + (2.0 * fg - fga) * (1.0 - m)) + fg * (1.0 - bga) + bg * (1.0 - fga)
            } else if bg * 4.0 <= bga {
                let m2 = m * m;
                let m3 = m2 * m;
                bga * (2.0 * fg - fga) * (m3 * 16.0 - m2 * 12.0 - m * 3.0) + fg - fg * bga + bg
            } else {
                bga * (2.0 * fg - fga) * (m.sqrt() - m) + fg - fg * bga + bg
            }
        },

        /// Subtracts the darker of the two constituent colors from the lighter color.
        ///
        /// Painting with white inverts the background color; painting with black produces no change.
        Difference => |fg, bg, fga, bga| fg + bg - 2.0 * (fg * bga).min(bg * fga),

        /// Produces an effect similar to that of the *difference* mode but lower in contrast.
        ///
        /// Painting with white inverts the background color; painting with black produces no change.
        Exclusion => |fg, bg, _fga, _bga| fg + bg - 2.0 * fg * bg,
    }

    non_separable {
        /// Creates a color with the hue of the foreground color and the saturation and luminosity of the background color.
        Hue => |[fgh, _fgs, _fgl], [_bgh, bgs, bgl]| [fgh, bgs, bgl],

        /// Creates a color with the saturation of the foreground color and the hue and luminosity of the background color.
        Saturation => |[_fgh, fgs, _fgl], [bgh, _bgs, bgl]| [bgh, fgs, bgl],

        /// Creates a color with the hue and saturation of the foreground color and the luminosity of the background color.
        Color => |[fgh, fgs, _fgl], [_bgh, _bgs, bgl]| [fgh, fgs, bgl],

        /// Creates a color with the luminosity of the foreground color and the hue and saturation of the background color.
        Luminosity => |[fgh, _fgs, _fbl], [bgh, bgs, _bgl]| [bgh, bgs, fgh],
    }
}
#[expect(clippy::derivable_impls)] // macro generated enum
impl Default for MixBlendMode {
    fn default() -> Self {
        MixBlendMode::Normal
    }
}
