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
        /// Set to [`theme::pair`] of [`vis::DARK_THEME_VAR`], [`vis::LIGHT_THEME_VAR`] by default.
        theme = theme::pair(vis::DARK_THEME_VAR, vis::LIGHT_THEME_VAR);
    }
}

/// Button themes, visual properties and context vars.
pub mod vis {
    use super::*;

    use crate::widgets::text::properties::TEXT_COLOR_VAR;

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
            dark_color as base_color;

            /// Button background color.
            ///
            /// Is the base color by default.
            background_color = DARK_COLOR_VAR;

            /// Button border.
            ///
            /// Is widths `1` and sides the base color lighten by 30%.
            border = {
                widths: 1,
                sides: DARK_COLOR_VAR.map_into()
            };

            /// When the pointer device is over this button.
            when self.is_cap_hovered {
                background_color = dark_color_hovered();
                border = {
                    widths: 1,
                    sides: dark_color_pressed().map_into(),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_pressed  {
                background_color = dark_color_pressed();
            }

            /// When the button is disabled.
            when self.is_disabled {
                background_color = dark_color_disabled();
                border = {
                    widths: 1,
                    sides: dark_color_disabled().map_into(),
                };
                text_color = TEXT_COLOR_VAR.map(|&c| colors::BLACK.with_alpha(0.5).mix_normal(c));
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
            light_color as base_color;

            /// Button background color.
            ///
            /// Is the base color by default.
            background_color = LIGHT_COLOR_VAR;

            /// Button border.
            ///
            /// Is widths `1` and sides the base color lighten by 50%.
            border = {
                widths: 1,
                sides: LIGHT_COLOR_VAR.map_into()
            };

            /// When the pointer device is over this button.
            when self.is_cap_hovered {
                background_color = light_color_hovered();
                border = {
                    widths: 1,
                    sides: light_color_pressed().map_into(),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_pressed  {
                background_color = light_color_pressed();
            }

            /// When the button is disabled.
            when self.is_disabled {
                background_color = light_color_disabled();
                border = {
                    widths: 1,
                    sides: light_color_disabled().map_into(),
                };
                text_color = TEXT_COLOR_VAR.map(|&c| colors::WHITE.with_alpha(0.5).mix_normal(c));
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
        pub static DARK_THEME_VAR: ThemeGenerator = ThemeGenerator::new(|_, _| dark_theme!());

        /// Button light theme.
        ///
        /// Use the [`button::vis::light`] property to set.
        ///
        /// [`button::vis::light`]: fn@light
        pub static LIGHT_THEME_VAR: ThemeGenerator = ThemeGenerator::new(|_, _| light_theme!());

        /// Idle background color in the dark theme.
        ///
        /// All other background states are derived by adjusting the brightness of this color.
        pub static DARK_COLOR_VAR: Rgba = rgb(0.18, 0.18, 0.18);

        /// Idle background color in the light theme.
        ///
        /// All other background states are derived by adjusting the brightness of this color.
        pub static LIGHT_COLOR_VAR: Rgba = rgb(0.82, 0.82, 0.82);
    }

    /// Sets the [`DARK_THEME_VAR`] that affects all buttons inside the widget.
    #[property(context, default(DARK_THEME_VAR))]
    pub fn dark(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, DARK_THEME_VAR, theme)
    }

    /// Sets the [`LIGHT_THEME_VAR`] that affects all buttons inside the widget.
    #[property(context, default(LIGHT_THEME_VAR))]
    pub fn light(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, LIGHT_THEME_VAR, theme)
    }

    /// Sets the [`DARK_COLOR_VAR`] that is used to compute all background and border colors in the dark theme.
    #[property(context, default(DARK_COLOR_VAR))]
    pub fn dark_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, DARK_COLOR_VAR, color)
    }

    /// Sets the [`LIGH_COLOR_VAR`] that is used to compute all background and border colors in the light theme.
    #[property(context, default(LIGHT_COLOR_VAR))]
    pub fn light_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, LIGHT_COLOR_VAR, color)
    }
    /// Dark background hovered.
    pub fn dark_color_hovered() -> impl Var<Rgba> {
        DARK_COLOR_VAR.map(|&c| colors::WHITE.with_alpha(0.08).mix_normal(c))
    }

    /// Dark background pressed.
    pub fn dark_color_pressed() -> impl Var<Rgba> {
        DARK_COLOR_VAR.map(|&c| colors::WHITE.with_alpha(0.16).mix_normal(c))
    }

    /// Dark background disabled.
    pub fn dark_color_disabled() -> impl Var<Rgba> {
        DARK_COLOR_VAR.map(|&c| c.desaturate(100.pct()))
    }

    /// Dark background hovered.
    pub fn light_color_hovered() -> impl Var<Rgba> {
        LIGHT_COLOR_VAR.map(|&c| colors::BLACK.with_alpha(0.08).mix_normal(c))
    }

    /// Dark background pressed.
    pub fn light_color_pressed() -> impl Var<Rgba> {
        LIGHT_COLOR_VAR.map(|&c| colors::BLACK.with_alpha(0.16).mix_normal(c))
    }

    /// Dark background disabled.
    pub fn light_color_disabled() -> impl Var<Rgba> {
        LIGHT_COLOR_VAR.map(|&c| c.desaturate(100.pct()))
    }
}
