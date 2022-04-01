use crate::prelude::new_widget::*;

/// A clickable container.
#[widget($crate::widgets::button)]
pub mod button {
    use super::*;
    use crate::properties::capture_mouse;

    pub use super::theme;

    inherit!(focusable_mixin);
    inherit!(container);

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

        /// Button background color.
        background_color = theme::BackgroundColorVar;

        /// Button border.
        border = {
            widths: theme::BorderWidthsVar,
            sides: theme::BorderSidesVar,
        };

        /// Button corner radius.
        corner_radius = theme::CornerRadiusVar;

        /// Color of text inside the button [`content`](#wp-content).
        text_color = theme::TextColorVar;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the button is pressed.
        capture_mouse = true;

        /// Content padding.
        padding = theme::PaddingVar;

        /// When the pointer device is over this button.
        when self.is_cap_hovered {
            background_color = theme::hovered::BackgroundColorVar;
            border = {
                widths: theme::BorderWidthsVar,
                sides: theme::hovered::BorderSidesVar,
            };
            text_color = theme::hovered::TextColorVar;
        }

        /// When the button is pressed in a way that press release will cause a button click.
        when self.is_pressed  {
            background_color = theme::pressed::BackgroundColorVar;
            border = {
                widths: theme::BorderWidthsVar,
                sides: theme::pressed::BorderSidesVar,
            };
            text_color = theme::pressed::TextColorVar;
        }

        /// When the button is disabled.
        when self.is_disabled {
            background_color = theme::disabled::BackgroundColorVar;
            border = {
                widths: theme::BorderWidthsVar,
                sides: theme::disabled::BorderSidesVar,
            };
            text_color = theme::disabled::TextColorVar;
        }
    }
}

/// Context variables and properties that affect the button appearance from parent widgets.
pub mod theme {
    use super::*;

    context_var! {
        /// Button background color.
        ///
        /// Use the [`button::theme::background_color`] property to set.
        ///
        /// [`button::theme::background_color`]: fn@background_color
        pub struct BackgroundColorVar: Rgba = rgb(0.2, 0.2, 0.2);

        /// Button border widths.
        ///
        /// Use the [`button::theme::border`] property to set.
        ///
        /// [`button::theme::border`]: fn@border
        pub struct BorderWidthsVar: SideOffsets = SideOffsets::new_all(1.0);
        /// Button border sides.
        ///
        /// Use the [`button::theme::border`] property to set.
        ///
        /// [`button::theme::border`]: fn@border
        pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.2, 0.2, 0.2));
        /// Button corner radius.
        ///
        /// Use the [`button::theme::corner_radius`] property to set.
        ///
        /// [`button::theme::corner_radius`]: fn@corner_radius
        pub struct CornerRadiusVar: CornerRadius = CornerRadius::new_all(4);

        /// Button padding.
        ///
        /// Use the [`button::theme::padding`] property to set.
        ///
        /// [`button::theme::border`]: fn@border
        pub struct PaddingVar: SideOffsets = SideOffsets::new(7.0, 15.0, 7.0, 15.0);

        /// Button text color.
        ///
        /// Use the [`button::theme::text_color`] property to set.
        ///
        /// [`button::theme::text_color`]: fn@text_color
        pub struct TextColorVar: Rgba = colors::WHITE;
    }

    /// Sets the [`BackgroundColorVar`] that affects all buttons inside the widget.
    #[property(context, default(BackgroundColorVar))]
    pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, BackgroundColorVar, color)
    }

    /// Sets the [`BorderWidthsVar`], [`BorderSidesVar`] that affects all buttons inside the widget.
    #[property(context, default(BorderWidthsVar, BorderSidesVar))]
    pub fn border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
        let child = with_context_var(child, BorderWidthsVar, widths);
        with_context_var(child, BorderSidesVar, sides)
    }

    /// Sets the [`CornerRadiusVar`] that affects all buttons inside the widget.
    #[property(context, default(CornerRadiusVar))]
    pub fn corner_radius(child: impl UiNode, radius: impl IntoVar<CornerRadius>) -> impl UiNode {
        with_context_var(child, CornerRadiusVar, radius)
    }

    /// Sets the [`PaddingVar`] that affects all buttons inside the widget.
    #[property(context, default(PaddingVar))]
    pub fn padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
        with_context_var(child, PaddingVar, padding)
    }

    /// Sets the [`TextColorVar`] that affects all texts inside buttons inside the widget.
    #[property(context, default(TextColorVar))]
    pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, TextColorVar, color)
    }

    /// Pointer hovered values.
    pub mod hovered {
        use super::*;

        context_var! {
            /// Hovered button background color.
            ///
            /// Use the [`button::theme::hovered::background_color`] property to set.
            ///
            /// [`button::theme::hovered::background_color`]: fn@background_color
            pub struct BackgroundColorVar: Rgba = rgb(0.25, 0.25, 0.25);

            /// Hovered button border sides.
            ///
            /// Use the [`button::theme::hovered::border_sides`] property to set.
            ///
            /// [`button::theme::hovered::border_sides`]: fn@border_sides
            pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.4, 0.4, 0.4));

            /// Hovered button text color.
            ///
            /// Use the [`button::theme::hovered::text_color`] property to set.
            ///
            /// [`button::theme::hovered::text_color`]: fn@text_color
            pub struct TextColorVar: Rgba = colors::WHITE;
        }

        /// Sets the hovered [`BackgroundColorVar`] that affects all buttons inside the widget.
        #[property(context, default(BackgroundColorVar))]
        pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, BackgroundColorVar, color)
        }

        /// Sets the hovered [`BorderSidesVar`] that affects all buttons inside the widget.
        #[property(context, default(BorderSidesVar))]
        pub fn border_sides(child: impl UiNode, sides: impl IntoVar<BorderSides>) -> impl UiNode {
            with_context_var(child, BorderSidesVar, sides)
        }

        /// Sets the hovered [`TextColorVar`] that affects all texts inside buttons inside the widget.
        #[property(context, default(TextColorVar))]
        pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, TextColorVar, color)
        }
    }

    /// Button pressed values.
    pub mod pressed {
        use super::*;

        context_var! {
            /// Pressed button background color.
            ///
            /// Use the [`button::theme::pressed::background_color`] property to set.
            ///
            /// [`button::theme::pressed::background_color`]: fn@background_color
            pub struct BackgroundColorVar: Rgba = rgb(0.3, 0.3, 0.3);
            /// Pressed button border sides.
            ///
            /// Use the [`button::theme::pressed::border`] property to set.
            ///
            /// [`button::theme::pressed::border`]: fn@border
            pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.6, 0.6, 0.6));

            /// Pressed button text color.
            ///
            /// Use the [`button::theme::pressed::text_color`] property to set.
            ///
            /// [`button::theme::pressed::text_color`]: fn@text_color
            pub struct TextColorVar: Rgba = colors::WHITE;
        }

        /// Sets the pressed [`BackgroundColorVar`] that affects all buttons inside the widget.
        #[property(context, default(BackgroundColorVar))]
        pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, BackgroundColorVar, color)
        }

        /// Sets the pressed [`BorderSidesVar`] that affects all buttons inside the widget.
        #[property(context, default(BorderSidesVar))]
        pub fn border_sides(child: impl UiNode, sides: impl IntoVar<BorderSides>) -> impl UiNode {
            with_context_var(child, BorderSidesVar, sides)
        }

        /// Sets the pressed [`TextColorVar`] that affects all texts inside buttons inside the widget.
        #[property(context, default(TextColorVar))]
        pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, TextColorVar, color)
        }
    }

    /// Button disabled values.
    pub mod disabled {
        use super::*;

        context_var! {
            /// Disabled button background color.
            ///
            /// Use the [`button::theme::disabled::background_color`] property to set.
            ///
            /// [`button::theme::disabled::background_color`]: fn@background_color
            pub struct BackgroundColorVar: Rgba = rgb(0.2, 0.2, 0.2);
            /// Disabled button border sides.
            ///
            /// Use the [`button::theme::disabled::border`] property to set.
            ///
            /// [`button::theme::disabled::border`]: fn@border
            pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.2, 0.2, 0.2));

            /// Disabled button text color.
            ///
            /// Use the [`button::theme::disabled::text_color`] property to set.
            ///
            /// [`button::theme::disabled::text_color`]: fn@text_color
            pub struct TextColorVar: Rgba = colors::WHITE.darken(40.pct());
        }

        /// Sets the disabled [`BackgroundColorVar`] that affects all buttons inside the widget.
        #[property(context, default(BackgroundColorVar))]
        pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, BackgroundColorVar, color)
        }

        /// Sets the disabled [`BorderSidesVar`] that affects all buttons inside the widget.
        #[property(context, default(BorderSidesVar))]
        pub fn border_sides(child: impl UiNode, sides: impl IntoVar<BorderSides>) -> impl UiNode {
            with_context_var(child, BorderSidesVar, sides)
        }

        /// Sets the disabled [`TextColorVar`] that affects all texts inside buttons inside the widget.
        #[property(context, default(TextColorVar))]
        pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, TextColorVar, color)
        }
    }
}
