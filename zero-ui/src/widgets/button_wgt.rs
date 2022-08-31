use crate::prelude::new_widget::*;

/// A clickable element.
#[widget($crate::widgets::button)]
pub mod button {
    use super::*;
    use crate::properties::capture_mouse;

    #[doc(inline)]
    pub use super::vis;

    inherit!(focusable_mixin);
    inherit!(element);

    properties! {
        /// Button click event.
        ///
        /// # Examples
        ///
        /// ```
        /// use zero_ui::prelude::*;
        ///
        /// button! {
        ///     on_click = hn!(|ctx, args: &ClickArgs| {
        ///         assert!(args.is_primary());
        ///         println!("button {:?} clicked!", ctx.path.widget_id());
        ///     });
        ///     content = text("Click Me!");
        /// }
        /// # ;
        /// ```
        on_click;

        /// If pointer interaction with other widgets is blocked while the button is pressed.
        ///
        /// Enabled by default.
        capture_mouse = true;

        /// Button dark and light themes.
        ///
        /// Set to [`theme::pair`] of [`vis::DarkThemeVar`], [`vis::LightThemeVar`] by default.
        theme = theme::pair(vis::DarkThemeVar, vis::LightThemeVar);
    }
}

/// Button themes, visual properties and context vars.
pub mod vis {
    use super::*;

    use crate::widgets::text::properties::TextColorVar;

    /// Button base theme.
    #[widget($crate::widgets::button::vis::base_theme)]
    pub mod base_theme {
        use super::*;

        inherit!(theme);

        properties! {
            /// Button padding.
            ///
            /// Is `(7, 15)` by default.
            padding = (7, 15);

            /// Button corner radius.
            ///
            /// Is `4` by default.
            corner_radius = 4;

            /// Button content align.
            child_align as content_align = Align::CENTER;
        }
    }

    /// Default button dark theme.
    #[widget($crate::widgets::button::vis::dark_theme)]
    pub mod dark_theme {
        use super::*;

        inherit!(base_theme);

        properties! {
            /// Button base color, all background and border colors are derived from this color.
            dark_base_color as base_color;

            /// Button background color.
            ///
            /// Is the base color by default.
            background_color = DarkColorVar;

            /// Button border.
            ///
            /// Is widths `1` and sides the base color lighten by 30%.
            border = {
                widths: 1,
                sides: DarkColorVar::new().map_into()
            };

            /// When the pointer device is over this button.
            when self.is_cap_hovered {
                background_color = DarkColorVar::hovered();
                border = {
                    widths: 1,
                    sides: DarkColorVar::pressed().map_into(),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_pressed  {
                background_color = DarkColorVar::pressed();
            }

            /// When the button is disabled.
            when self.is_disabled {
                background_color = DarkColorVar::disabled();
                border = {
                    widths: 1,
                    sides: DarkColorVar::disabled().map_into(),
                };
                //text_color = TextColorVar::new().map(|c| c.darken(50.pct()).desaturate(100.pct()));
                text_color = TextColorVar::new().map(|&c| colors::BLACK.with_alpha(0.5).mix_normal(c));
                cursor = CursorIcon::NotAllowed;
            }
        }
    }

    /// Default button light theme.
    #[widget($crate::widgets::button::vis::light_theme)]
    pub mod light_theme {
        use super::*;

        inherit!(base_theme);

        properties! {
            /// Button base color, all background and border colors are derived from this color.
            light_base_color as base_color;

            /// Button background color.
            ///
            /// Is the base color by default.
            background_color = LightColorVar;

            /// Button border.
            ///
            /// Is widths `1` and sides the base color lighten by 50%.
            border = {
                widths: 1,
                sides: LightColorVar::new().map_into()
            };

            /// When the pointer device is over this button.
            when self.is_cap_hovered {
                background_color = LightColorVar::hovered();
                border = {
                    widths: 1,
                    sides: LightColorVar::pressed().map_into(),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_pressed  {
                background_color = LightColorVar::pressed();
            }

            /// When the button is disabled.
            when self.is_disabled {
                background_color = LightColorVar::disabled();
                border = {
                    widths: 1,
                    sides: LightColorVar::disabled().map_into(),
                };
                //text_color = TextColorVar::new().map(|c| c.lighten(50.pct()).desaturate(100.pct()));
                text_color = TextColorVar::new().map(|&c| colors::WHITE.with_alpha(0.5).mix_normal(c));
                cursor = CursorIcon::NotAllowed;
            }
        }
    }

    context_var! {
        /// Button dark theme.
        ///
        /// Use the [`button::vis::dark`] property to set.
        ///
        /// [`button::vis::dark`]: fn@dark
        pub struct DarkThemeVar: ThemeGenerator = ThemeGenerator::new(|_, _| dark_theme!());

        /// Button light theme.
        ///
        /// Use the [`button::vis::light`] property to set.
        ///
        /// [`button::vis::light`]: fn@light
        pub struct LightThemeVar: ThemeGenerator = ThemeGenerator::new(|_, _| light_theme!());

        /// Idle background color in the dark theme.
        ///
        /// All other background states are derived by adjusting the brightness of this color.
        pub struct DarkColorVar: Rgba = rgb(0.18, 0.18, 0.18);

        /// Idle background color in the light theme.
        ///
        /// All other background states are derived by adjusting the brightness of this color.
        pub struct LightColorVar: Rgba = rgb(0.82, 0.82, 0.82);
    }

    /// Sets the [`DarkThemeVar`] that affects all buttons inside the widget.
    #[property(context, default(DarkThemeVar))]
    pub fn dark(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, DarkThemeVar, theme)
    }

    /// Sets the [`LightThemeVar`] that affects all buttons inside the widget.
    #[property(context, default(LightThemeVar))]
    pub fn light(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, LightThemeVar, theme)
    }

    /// Sets the [`DarkBaseColorVar`] that is used to compute all background and border colors in the dark theme.
    #[property(context, default(DarkColorVar))]
    pub fn dark_base_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, DarkColorVar, color)
    }

    /// Sets the [`LightBaseColorVar`] that is used to compute all background and border colors in the light theme.
    #[property(context, default(LightColorVar))]
    pub fn light_base_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, LightColorVar, color)
    }

    impl DarkColorVar {
        /// Dark background hovered.
        pub fn hovered() -> impl Var<Rgba> {
            DarkColorVar::new().map(|&c| colors::WHITE.with_alpha(0.08).mix_normal(c))
        }

        /// Dark background pressed.
        pub fn pressed() -> impl Var<Rgba> {
            DarkColorVar::new().map(|&c| colors::WHITE.with_alpha(0.16).mix_normal(c))
        }

        /// Dark background disabled.
        pub fn disabled() -> impl Var<Rgba> {
            DarkColorVar::new().map(|&c| c.desaturate(100.pct()))
        }
    }

    impl LightColorVar {
        /// Dark background hovered.
        pub fn hovered() -> impl Var<Rgba> {
            LightColorVar::new().map(|&c| colors::BLACK.with_alpha(0.08).mix_normal(c))
        }

        /// Dark background pressed.
        pub fn pressed() -> impl Var<Rgba> {
            LightColorVar::new().map(|&c| colors::BLACK.with_alpha(0.16).mix_normal(c))
        }

        /// Dark background disabled.
        pub fn disabled() -> impl Var<Rgba> {
            LightColorVar::new().map(|&c| c.desaturate(100.pct()))
        }
    }
}
