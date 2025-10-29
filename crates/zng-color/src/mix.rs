use super::{Hsla, Hsva, PreMulRgba, Rgba, clamp_normal};
use zng_layout::unit::Factor;

use pastey::*;

pub use zng_view_api::MixBlendMode;

macro_rules! impl_mix {
    (
        separable {$(
            $Mode:ident => |$fg:ident, $bg:ident, $ca0:ident, $ca1:ident| $mix:expr,
        )+}

        non_separable {$(
            $NsMode:ident => |[$fgh:ident, $fgs:ident, $fgl:ident], [$bgh:ident, $bgs:ident, $bgl:ident]| $ns_mix:expr,
        )+}
    ) => {
        /// Color mix and adjustment methods.
        pub trait MixAdjust {
            /// MixAdjust `self` over `background` using the `mode`.
            fn mix(self, mode: MixBlendMode, background: Self) -> Self where Self:Sized {
                match mode {
                    $(MixBlendMode::$Mode => paste!(self.[<mix_ $Mode:lower>](background)),)+
                    $(MixBlendMode::$NsMode => paste!(self.[<mix_ $NsMode:lower>](background)),)+
                    _ => unreachable!()
                }
            }

            $(
                paste! {
                    #[doc = "MixAdjust `self` over `background` using the [`MixBlendMode::" $Mode "`]."]
                    ///
                    fn [<mix_ $Mode:lower>](self, background: Self) -> Self;
                }
            )+

            $(
                paste! {
                    #[doc = "MixAdjust `self` over `background` using the [`MixBlendMode::`" $NsMode "`]."]
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
                                (fga + bga - fga * bga).clamp(0.0, 1.0)
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

#[rustfmt::skip]// zng fmt can't handle this syntax and is slightly slower because it causes rustfmt errors
impl_mix! {
    separable {
        Normal => |fg, bg, fga, _bga| fg + bg * (1.0 - fga),

        Multiply => |fg, bg, fga, bga| fg * bg + fg * (1.0 - bga) + bg * (1.0 - fga),

        Screen => |fg, bg, _fga, _bga| fg + bg - fg * bg,

        Overlay => |fg, bg, fga, bga| if bg * 2.0 <= bga {
            2.0 * fg * bg + fg * (1.0 - bga) + bg * (1.0 - fga)
        } else {
            fg * (1.0 + bga) + bg * (1.0 + fga) - 2.0 * fg * bg - fga * bga
        },

        Darken => |fg, bg, fga, bga| (fg * bga).min(bg * fga) + fg * (1.0 - bga) + bg * (1.0 - fga),

        Lighten => |fg, bg, fga, bga| (fg * bga).max(bg * fga) + fg * (1.0 - bga) + bg * (1.0 - fga),

        ColorDodge => |fg, bg, fga, bga| if fg == fga {
            fga * bga + fg * (1.0 - bga) + bg * (1.0 - fga)
        } else {
            fga * bga * 1.0_f32.min((bg / bga) * fga / (fga - fg)) + fg * (1.0 - bga) + bg * (1.0 - fga)
        },

        ColorBurn => |fg, bg, fga, bga| fga * bga * (1.0 - 1.0_f32.min((1.0 - bg / bga) * fga / fg)) + fg * (1.0 - bga) + bg * (1.0 - fga),

        HardLight => |fg, bg, fga, bga| if fg * 2.0 <= fga {
            2.0 * fg * bg + fg * (1.0 - bga) + bg * (1.0 - fga)
        } else {
            fg * (1.0 + bga) + bg * (1.0 + fga) - 2.0 * fg * bg - fga * bga
        },

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

        Difference => |fg, bg, fga, bga| fg + bg - 2.0 * (fg * bga).min(bg * fga),

        Exclusion => |fg, bg, _fga, _bga| fg + bg - 2.0 * fg * bg,

        PlusLighter => |fg, bg, fga, bga| (bg * bga + fg * fga).clamp(0.0, 1.0),
    }

    non_separable {
        Hue => |[fgh, _fgs, _fgl], [_bgh, bgs, bgl]| [fgh, bgs, bgl],

        Saturation => |[_fgh, fgs, _fgl], [bgh, _bgs, bgl]| [bgh, fgs, bgl],

        Color => |[fgh, fgs, _fgl], [_bgh, _bgs, bgl]| [fgh, fgs, bgl],

        Luminosity => |[fgh, _fgs, _fbl], [bgh, bgs, _bgl]| [bgh, bgs, fgh],
    }
}
