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

        /// Button theme.
        ///
        /// Set to [`vis::THEME_VAR`] by default, setting this property directly completely replaces the button theme,
        /// see [`vis::replace_theme`] and [`vis::extend_theme`] for other ways of modifying the button theme.
        theme = vis::THEME_VAR;
    }
}

/// Button theme, visual properties and context vars.
pub mod vis {
    use super::*;

    context_var! {
        /// Button theme in a context.
        ///
        /// Is the [`default_theme!`] by default.
        ///
        /// [`default_theme!`]: mod@default_theme
        pub static THEME_VAR: ThemeGenerator = ThemeGenerator::new(|_, _| default_theme!());

        /// Idle background dark and light color.
        pub static BASE_COLORS_VAR: theme::ColorPair = (rgb(0.18, 0.18, 0.18), rgb(0.82, 0.82, 0.82));
    }

    /// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the button theme.
    #[property(context, default(BASE_COLORS_VAR))]
    pub fn base_colors(child: impl UiNode, color: impl IntoVar<theme::ColorPair>) -> impl UiNode {
        with_context_var(child, BASE_COLORS_VAR, color)
    }

    /// Sets the button theme in a context, the parent theme is fully replaced.
    #[property(context, default(THEME_VAR))]
    pub fn replace_theme(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, THEME_VAR, theme)
    }

    /// Extends the button theme in a context, the parent theme is used, properties of the same name set in
    /// `theme` override the parent theme.
    #[property(context, default(ThemeGenerator::nil()))]
    pub fn extend_theme(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        themable::with_theme_extension(child, THEME_VAR, theme)
    }

    /// Button default theme.
    #[widget($crate::widgets::button::vis::default_theme)]
    pub mod default_theme {
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

            /// Button theme base dark and light colors.
            ///
            /// All other button theme colors are derived from this pair.
            base_colors;

            /// Button background.
            background_color = theme::color(BASE_COLORS_VAR);

            /// Button border.
            ///
            /// Is widths `1`.
            border = {
                widths: 1,
                sides: theme::color(BASE_COLORS_VAR).map_into()
            };

            /// When the pointer device is over this button.
            when self.is_cap_hovered {
                background_color = theme::color_hovered(BASE_COLORS_VAR);
                border = {
                    widths: 1,
                    sides: theme::color_pressed(BASE_COLORS_VAR).map_into(),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_pressed  {
                background_color = theme::color_pressed(BASE_COLORS_VAR);
            }

            /// When the button is disabled.
            when self.is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}
