use zero_ui_view_api::webrender_api;

use super::{Hsla, PreMulRgba, Rgba};

use paste::*;

/// Webrender [`MixBlendMode`].
pub type RenderMixBlendMode = webrender_api::MixBlendMode;

macro_rules! impl_mix {
    (
        separable {$(
            $(#[$meta:meta])*
            $Mode:ident => |$c0:ident, $c1:ident, $ca0:ident, $ca1:ident| $mix:expr,
        )+}

        non_separable {$(
            $(#[$ns_meta:meta])*
            $NsMode:ident => |[$h0:ident, $s0:ident, $l0:ident], [$h1:ident, $s1:ident, $l1:ident]| $ns_mix:expr,
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

        impl PreMulRgba {
            /// Mix `other` over `self` using the `mode`.
            pub fn mix(self, mode: MixBlendMode, other: PreMulRgba) -> PreMulRgba {
                match mode {
                    $(MixBlendMode::$Mode => paste!(self.[<mix_ $Mode:lower>](other)),)+
                    $(MixBlendMode::$NsMode => paste!(self.to_rgba().[<mix_ $NsMode:lower>](other.to_rgba()).pre_mul()),)+
                }
            }

            $(
                paste! {
                    #[doc = "Mix `other` over `self` using the [`MixBlendMode::`" $Mode "`]."]
                    ///
                    $(#[$meta])*
                    pub fn [<mix_ $Mode:lower>](self, other: PreMulRgba) -> PreMulRgba {
                        PreMulRgba {
                            red: {
                                let $c0 = self.red;
                                let $c1 = other.red;
                                let $ca0 = self.alpha;
                                let $ca1 = other.alpha;
                                $mix
                            },
                            green: {
                                let $c0 = self.green;
                                let $c1 = other.green;
                                let $ca0 = self.alpha;
                                let $ca1 = other.alpha;
                                $mix
                            },
                            blue: {
                                let $c0 = self.blue;
                                let $c1 = other.blue;
                                let $ca0 = self.alpha;
                                let $ca1 = other.alpha;
                                $mix
                            },
                            alpha: {
                                let a0 = self.alpha;
                                let a1 = other.alpha;
                                (a0 + a1 - a0 * a1).max(0.0).min(1.0)
                            },
                        }
                    }
                }
            )+
        }

        impl Rgba {
            /// Mix `other` over `self` using the `mode`.
            pub fn mix(self, mode: MixBlendMode, other: Rgba) -> Rgba {
                match mode {
                    $(MixBlendMode::$Mode => paste!(self.[<mix_ $Mode:lower>](other)),)+
                    $(MixBlendMode::$NsMode => paste!(self.[<mix_ $NsMode:lower>](other)),)+
                }
            }

            $(
                paste! {
                    #[doc = "Mix `other` over `self` using the [`MixBlendMode::`" $Mode "`]."]
                    ///
                    $(#[$meta])*
                    ///
                    /// This method converts both [`PreMulRgba`] and the result back to `Rgba`.
                    pub fn [<mix_ $Mode:lower>](self, other: Rgba) -> Rgba {
                        self.pre_mul().[<mix_ $Mode:lower>](other.pre_mul()).to_rgba()
                    }
                }
            )+
            $(
                paste! {
                    #[doc = "Mix `other` over `self` using the [`MixBlendMode::`" $NsMode "`]."]
                    ///
                    $(#[$ns_meta])*
                    ///
                    /// This method converts both [`Hsla`] and the result back to `Rgba`.
                    pub fn [<mix_ $NsMode:lower>](self, other: Rgba) -> Rgba {
                        self.to_hsla().[<mix_ $NsMode:lower>](other.to_hsla()).to_rgba()
                    }
                }
            )+
        }

        impl Hsla {
            $(
                paste! {
                    #[doc = "Mix `other` over `self` using the [`MixBlendMode::`" $NsMode "`]."]
                    ///
                    $(#[$ns_meta])*
                    pub fn [<mix_ $NsMode:lower>](self, other: Hsla) -> Hsla {
                        let $h0 = self.hue;
                        let $s0 = self.saturation;
                        let $l0 = self.lightness;

                        let $h1 = other.hue;
                        let $s1 = other.saturation;
                        let $l1 = other.lightness;

                        let [h, s, l] = { $ns_mix };

                        Hsla {
                            hue: h,
                            saturation: s,
                            lightness: l,
                            alpha: {
                                let a0 = self.alpha;
                                let a1 = other.alpha;
                                (a0 + a1 - a0 * a1).max(0.0).min(1.0)
                            }
                        }
                    }
                }
            )+
        }
    };
}

impl_mix! {
    separable {
        /// Normal alpha blend of the second color over the first.
        Normal => |c0, c1, a0, _a1| c0 + c1 * (1.0 - a0),

        /// Multiply the colors.
        ///
        /// The resultant color is always at least as dark as either color.
        /// Multiplying any color with black results in black.
        /// Multiplying any color with white preserves the original color.
        Multiply => |c0, c1, a0, a1| c0 * c1 + c0 * (1.0 - a1) + c1 * (1.0 - a0),

        /// Multiply the colors, then complements the result.
        ///
        /// The result color is always at least as light as either of the two constituent colors.
        /// Screening any color with white produces white; screening with black leaves the original color unchanged.
        /// The effect is similar to projecting multiple photographic slides simultaneously onto a single screen.
        Screen => |c0, c1, _a0, _a1| c0 + c1 - c0 * c1,

        /// Multiplies or screens the colors, depending on the first color value.
        ///
        /// Second color overlays the first while preserving its highlights and shadows.
        /// The first color is not replaced but is mixed with the second color to reflect the lightness or darkness
        /// of the first.
        ///
        /// This is the inverse of *hardlight*.
        Overlay => |c0, c1, a0, a1| if c1 * 2.0 <= a1 {
            2.0 * c0 * c1 + c0 * (1.0 - a1) + c1 * (1.0 - a0)
        } else {
            c0 * (1.0 + a1) + c1 * (1.0 + a0) - 2.0 * c0 * c1 - a0 * a1
        },

        /// Selects the darker of the colors.
        ///
        /// The first color is replaced with the second where the first is darker; otherwise, it is left unchanged.
        Darken => |c0, c1, a0, a1| (c0 * a1).min(c1 * a0) + c0 * (1.0 - a1) + c1 * (1.0 - a0),

        /// Selects the lighter of the colors.
        ///
        /// The first color is replaced with the second where the first is lighter; otherwise, it is left unchanged.
        Lighten => |c0, c1, a0, a1| (c0 * a1).max(c1 * a0) + c0 * (1.0 - a1) + c1 * (1.0 - a0),

        /// Brightens the first color to reflect the second color. Painting with black produces no changes.
        ColorDodge => |c0, c1, a0, a1| if c0 == a0 {
            a0 * a1 + c0 * (1.0 - a1) + c1 * (1.0 - a0)
        } else {
            a0 * a1 * 1.0_f32.min((c1 / a1) * a0 / (a0 - c0)) + c0 * (1.0 - a1) + c1 * (1.0 - a0)
        },

        /// Darkens the first color to reflect the second color. Painting with white produces no change.
        ColorBurn => |c0, c1, a0, a1| a0 * a1 * (1.0 - 1.0_f32.min((1.0 - c1 / a1) * a0 / c0)) + c0 * (1.0 - a1) + c1 * (1.0 - a0),

        /// Multiplies or screens the colors, depending on the second color value.
        ///
        /// The effect is similar to shining a harsh spotlight on the first color.
        HardLight => |c0, c1, a0, a1| if c0 * 2.0 <= a0 {
            2.0 * c0 * c1 + c0 * (1.0 - a1) + c1 * (1.0 - a0)
        } else {
            c0 * (1.0 + a1) + c1 * (1.0 + a0) - 2.0 * c0 * c1 - a0 * a1
        },

        /// Darkens or lightens the colors, depending on the second color value.
        ///
        /// The effect is similar to shining a diffused spotlight on the first color.
        SoftLight => |c0, c1, a0, a1| {
            let m = c1 / a1;

            if c0 * 2.0 <= a0 {
                c1 * (a0 + (2.0 * c0 - a0) * (1.0 - m)) + c0 * (1.0 - a1) + c1 * (1.0 - a0)
            } else if c1 * 4.0 <= a1 {
                let m2 = m * m;
                let m3 = m2 * m;
                a1 * (2.0 * c0 - a0) * (m3 * 16.0 - m2 * 12.0 - m * 3.0) + c0 - c0 * a1 + c1
            } else {
                a1 * (2.0 * c0 - a0) * (m.sqrt() - m) + c0 - c0 * a1 + c1
            }
        },

        /// Subtracts the darker of the two constituent colors from the lighter color.
        ///
        /// Painting with white inverts the first color; painting with black produces no change.
        Difference => |c0, c1, a0, a1| c0 + c1 - 2.0 * (c0 * a1).min(c1 * a0),

        /// Produces an effect similar to that of the *difference* mode but lower in contrast.
        ///
        /// Painting with white inverts the first color; painting with black produces no change.
        Exclusion => |c0, c1, _a0, _a1| c0 + c1 - 2.0 * c0 * c1,
    }

    non_separable {
        /// Creates a color with the hue of the second color and the saturation and luminosity of the first color.
        Hue => |[_h0, s0, l0], [h1, _s1, _l1]| [h1, s0, l0],

        /// Creates a color with the saturation of the second color and the hue and luminosity of the first color.
        Saturation => |[h0, _s0, l0], [_h1, s1, _l1]| [h0, s1, l0],

        /// Creates a color with the hue and saturation of the second color and the luminosity of the first color.
        Color => |[_h0, _s0, l0], [h1, s1, _l1]| [h1, s1, l0],

        /// Creates a color with the luminosity of the second color and the hue and saturation of the first color.
        Luminosity => |[h0, s0, _l0], [_h1, _s1, l1]| [h0, s0, l1],
    }
}
