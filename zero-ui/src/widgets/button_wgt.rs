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
            background_color = DarkBaseColorVar;

            /// Button border.
            ///
            /// Is widths `1` and sides the base color lighten by 30%.
            border = {
                widths: 1,
                sides: DarkBaseColorVar::new().map(|c| c.lighten(30.pct()).into())
            };

            /// When the pointer device is over this button.
            when self.is_cap_hovered {
                background_color = DarkBaseColorVar::new().map(|c| c.lighten(15.pct()));
                border = {
                    widths: 1,
                    sides: DarkBaseColorVar::new().map(|c| c.lighten(45.pct()).into()),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_pressed  {
                background_color = DarkBaseColorVar::new().map(|c| c.lighten(60.pct()));
                border = {
                    widths: 1,
                    sides: DarkBaseColorVar::new().map(|c| c.lighten(60.pct()).into()),
                };
            }

            /// When the button is disabled.
            when self.is_disabled {
                background_color = DarkBaseColorVar::new().map(|c| c.desaturate(10.pct()));
                border = {
                    widths: 1,
                    sides: DarkBaseColorVar::new().map(|c| c.lighten(20.pct()).into()),
                };
                text_color = TextColorVar::new().map(|c| c.darken(50.pct()).desaturate(100.pct()));
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
            background_color = LightBaseColorVar;

            /// Button border.
            ///
            /// Is widths `1` and sides the base color lighten by 50%.
            border = {
                widths: 1,
                sides: LightBaseColorVar::new().map(|c| c.lighten(50.pct()).into())
            };

            /// When the pointer device is over this button.
            when self.is_cap_hovered {
                background_color = LightBaseColorVar::new().map(|c| c.lighten(5.pct()));
                border = {
                    widths: 1,
                    sides: LightBaseColorVar::new().map(|c| c.lighten(55.pct()).into()),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_pressed  {
                background_color = LightBaseColorVar::new().map(|c| c.lighten(10.pct()));
                border = {
                    widths: 1,
                    sides: LightBaseColorVar::new().map(|c| c.lighten(60.pct()).into()),
                };
            }

            /// When the button is disabled.
            when self.is_disabled {
                background_color = LightBaseColorVar::new().map(|c| c.desaturate(10.pct()));
                border = {
                    widths: 1,
                    sides: LightBaseColorVar::new().map(|c| c.lighten(50.pct()).desaturate(10.pct()).into()),
                };
                text_color = TextColorVar::new().map(|c| c.darken(10.pct()).desaturate(100.pct()));
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
        pub struct DarkThemeVar: ThemeGenerator = ThemeGenerator::new(|_| dark_theme!());

        /// Button light theme.
        ///
        /// Use the [`button::vis::light`] property to set.
        ///
        /// [`button::vis::light`]: fn@light
        pub struct LightThemeVar: ThemeGenerator = ThemeGenerator::new(|_| light_theme!());

        /// Idle background color in the dark theme.
        ///
        /// All other background states are derived by adjusting the brightness of this color.
        pub struct DarkBaseColorVar: Rgba = rgb(0.18, 0.18, 0.18);

        /// Idle background color in the light theme.
        ///
        /// All other background states are derived by adjusting the brightness of this color.
        pub struct LightBaseColorVar: Rgba = rgb(0.9, 0.9, 0.9);
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
    #[property(context, default(DarkBaseColorVar))]
    pub fn dark_base_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, DarkBaseColorVar, color)
    }

    /// Sets the [`LightBaseColorVar`] that is used to compute all background and border colors in the light theme.
    #[property(context, default(LightBaseColorVar))]
    pub fn light_base_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, LightBaseColorVar, color)
    }
}
