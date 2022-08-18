use zero_ui_view_api::webrender_api;

use super::{Hsla, PreMulRgba, Rgba};

use paste::*;

/// Webrender [`MixBlendMode`].
pub type RenderMixBlendMode = webrender_api::MixBlendMode;

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

        impl PreMulRgba {
            /// Mix `background` over `self` using the `mode`.
            pub fn mix(self, mode: MixBlendMode, background: PreMulRgba) -> PreMulRgba {
                match mode {
                    $(MixBlendMode::$Mode => paste!(self.[<mix_ $Mode:lower>](background)),)+
                    $(MixBlendMode::$NsMode => paste!(self.to_rgba().[<mix_ $NsMode:lower>](background.to_rgba()).pre_mul()),)+
                }
            }

            $(
                paste! {
                    #[doc = "Mix `background` over `self` using the [`MixBlendMode::`" $Mode "`]."]
                    ///
                    $(#[$meta])*
                    pub fn [<mix_ $Mode:lower>](self, background: PreMulRgba) -> PreMulRgba {
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

        impl Rgba {
            /// Mix `self` over `background` using the `mode`.
            pub fn mix(self, mode: MixBlendMode, background: Rgba) -> Rgba {
                match mode {
                    $(MixBlendMode::$Mode => paste!(self.[<mix_ $Mode:lower>](background)),)+
                    $(MixBlendMode::$NsMode => paste!(self.[<mix_ $NsMode:lower>](background)),)+
                }
            }

            $(
                paste! {
                    #[doc = "Mix `self` over `background` using the [`MixBlendMode::`" $Mode "`]."]
                    ///
                    $(#[$meta])*
                    ///
                    /// This method converts both [`PreMulRgba`] and the result back to `Rgba`.
                    pub fn [<mix_ $Mode:lower>](self, background: Rgba) -> Rgba {
                        self.pre_mul().[<mix_ $Mode:lower>](background.pre_mul()).to_rgba()
                    }
                }
            )+
            $(
                paste! {
                    #[doc = "Mix `self` over `background` using the [`MixBlendMode::`" $NsMode "`]."]
                    ///
                    $(#[$ns_meta])*
                    ///
                    /// This method converts both [`Hsla`] and the result back to `Rgba`.
                    pub fn [<mix_ $NsMode:lower>](self, background: Rgba) -> Rgba {
                        self.to_hsla().[<mix_ $NsMode:lower>](background.to_hsla()).to_rgba()
                    }
                }
            )+
        }

        impl Hsla {
            $(
                paste! {
                    #[doc = "Mix `self` over `background` using the [`MixBlendMode::`" $NsMode "`]."]
                    ///
                    $(#[$ns_meta])*
                    pub fn [<mix_ $NsMode:lower>](self, background: Hsla) -> Hsla {
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
