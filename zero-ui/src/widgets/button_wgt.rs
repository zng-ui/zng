use crate::prelude::new_widget::*;

/// A clickable container.
#[widget($crate::widgets::button)]
pub mod button {
    use super::*;
    use crate::properties::capture_mouse;

    #[doc(inline)]
    pub use super::vis;

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
        background_color = vis::BackgroundColorVar;

        /// Button border.
        border = {
            widths: vis::BorderWidthsVar,
            sides: vis::BorderSidesVar,
        };

        /// Button corner radius.
        corner_radius = vis::CornerRadiusVar;

        /// Color of text inside the button [`content`](#wp-content).
        text_color = vis::TextColorVar;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the button is pressed.
        capture_mouse = true;

        /// Content padding.
        padding = vis::PaddingVar;

        /// Content align.
        content_align = vis::ContentAlignVar;

        /// Button cursor.
        cursor = vis::CursorIconVar;

        /// When the pointer device is over this button.
        when self.is_cap_hovered {
            background_color = vis::hovered::BackgroundColorVar;
            border = {
                widths: vis::BorderWidthsVar,
                sides: vis::hovered::BorderSidesVar,
            };
            text_color = vis::hovered::TextColorVar;
        }

        /// When the button is pressed in a way that press release will cause a button click.
        when self.is_pressed  {
            background_color = vis::pressed::BackgroundColorVar;
            border = {
                widths: vis::BorderWidthsVar,
                sides: vis::pressed::BorderSidesVar,
            };
            text_color = vis::pressed::TextColorVar;
        }

        /// When the button is disabled.
        when self.is_disabled {
            background_color = vis::disabled::BackgroundColorVar;
            border = {
                widths: vis::BorderWidthsVar,
                sides: vis::disabled::BorderSidesVar,
            };
            text_color = vis::disabled::TextColorVar;
            cursor = vis::disabled::CursorIconVar;
        }
    }
}

/// Button themes, visual properties and context vars.
pub mod vis {
    use super::*;

    /// Button base theme.
    #[widget($crate::widgets::button::vis::base_theme)]
    pub mod base_theme {
        use super::*;

        inherit!(theme);
    }

    /// Default button dark theme.
    #[widget($crate::widgets::button::vis::dark_theme)]
    pub mod dark_theme {
        use super::*;

        inherit!(base_theme);
    }

    /// Default button light theme.
    #[widget($crate::widgets::button::vis::light_theme)]
    pub mod light_theme {
        use super::*;

        inherit!(base_theme);
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

    context_var! {
        /// Button background color.
        ///
        /// Use the [`button::vis::background_color`] property to set.
        ///
        /// [`button::vis::background_color`]: fn@background_color
        pub struct BackgroundColorVar: Rgba = rgb(0.2, 0.2, 0.2);

        /// Button border widths.
        ///
        /// Use the [`button::vis::border`] property to set.
        ///
        /// [`button::vis::border`]: fn@border
        pub struct BorderWidthsVar: SideOffsets = SideOffsets::new_all(1);
        /// Button border sides.
        ///
        /// Use the [`button::vis::border`] property to set.
        ///
        /// [`button::vis::border`]: fn@border
        pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.2, 0.2, 0.2));
        /// Button corner radius.
        ///
        /// Use the [`button::vis::corner_radius`] property to set.
        ///
        /// [`button::vis::corner_radius`]: fn@corner_radius
        pub struct CornerRadiusVar: CornerRadius = CornerRadius::new_all(4);

        /// Button padding.
        ///
        /// Use the [`button::vis::padding`] property to set.
        ///
        /// [`button::vis::border`]: fn@border
        pub struct PaddingVar: SideOffsets = SideOffsets::new(7, 15, 7, 15);

        /// Button text color.
        ///
        /// Use the [`button::vis::text_color`] property to set.
        ///
        /// [`button::vis::text_color`]: fn@text_color
        pub struct TextColorVar: Rgba = colors::WHITE;

        /// Button content align.
        ///
        /// Use the [`button::vis::content_align`] property to set.
        ///
        /// [`button::vis::content_align`]: fn@content_align
        pub struct ContentAlignVar: Align = Align::CENTER;

        /// Button cursor icon.
        ///
        /// Use the [`button::vis::cursor`] property to set.
        ///
        /// Default is [`CursorIcon::Default`].
        pub struct CursorIconVar: Option<CursorIcon> = Some(CursorIcon::Default);
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

    /// Sets the [`ContentAlignVar`] that affects all texts inside buttons inside the widget.
    #[property(context, default(ContentAlignVar))]
    pub fn content_align(child: impl UiNode, align: impl IntoVar<Align>) -> impl UiNode {
        with_context_var(child, ContentAlignVar, align)
    }

    /// Sets the [`CursorIconVar`] that affects all buttons inside the widget.
    #[property(context, default(CursorIconVar))]
    pub fn cursor(child: impl UiNode, align: impl IntoVar<Option<CursorIcon>>) -> impl UiNode {
        with_context_var(child, CursorIconVar, align)
    }

    /// Pointer hovered values.
    pub mod hovered {
        use super::*;

        context_var! {
            /// Hovered button background color.
            ///
            /// Use the [`button::vis::hovered::background_color`] property to set.
            ///
            /// [`button::vis::hovered::background_color`]: fn@background_color
            pub struct BackgroundColorVar: Rgba = rgb(0.25, 0.25, 0.25);

            /// Hovered button border sides.
            ///
            /// Use the [`button::vis::hovered::border_sides`] property to set.
            ///
            /// [`button::vis::hovered::border_sides`]: fn@border_sides
            pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.4, 0.4, 0.4));

            /// Hovered button text color.
            ///
            /// Use the [`button::vis::hovered::text_color`] property to set.
            ///
            /// [`button::vis::hovered::text_color`]: fn@text_color
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
            /// Use the [`button::vis::pressed::background_color`] property to set.
            ///
            /// [`button::vis::pressed::background_color`]: fn@background_color
            pub struct BackgroundColorVar: Rgba = rgb(0.3, 0.3, 0.3);
            /// Pressed button border sides.
            ///
            /// Use the [`button::vis::pressed::border`] property to set.
            ///
            /// [`button::vis::pressed::border`]: fn@border
            pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.6, 0.6, 0.6));

            /// Pressed button text color.
            ///
            /// Use the [`button::vis::pressed::text_color`] property to set.
            ///
            /// [`button::vis::pressed::text_color`]: fn@text_color
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
            /// Use the [`button::vis::disabled::background_color`] property to set.
            ///
            /// [`button::vis::disabled::background_color`]: fn@background_color
            pub struct BackgroundColorVar: Rgba = rgb(0.2, 0.2, 0.2);
            /// Disabled button border sides.
            ///
            /// Use the [`button::vis::disabled::border`] property to set.
            ///
            /// [`button::vis::disabled::border`]: fn@border
            pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.2, 0.2, 0.2));

            /// Disabled button text color.
            ///
            /// Use the [`button::vis::disabled::text_color`] property to set.
            ///
            /// [`button::vis::disabled::text_color`]: fn@text_color
            pub struct TextColorVar: Rgba = colors::WHITE.darken(40.pct());

            /// Disabled button cursor icon.
            ///
            /// Use the [`button::vis::disabled::cursor`] property to set.
            ///
            /// Default is [`CursorIcon::NotAllowed`], meaning the parent cursor is used.
            pub struct CursorIconVar: Option<CursorIcon> = Some(CursorIcon::NotAllowed);
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

        /// Sets the disabled [`CursorIconVar`] that affects all buttons inside the widget.
        #[property(context, default(CursorIconVar))]
        pub fn cursor(child: impl UiNode, align: impl IntoVar<Option<CursorIcon>>) -> impl UiNode {
            with_context_var(child, CursorIconVar, align)
        }
    }
}
