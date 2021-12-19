use crate::prelude::new_widget::*;

/// A clickable container.
#[widget($crate::widgets::button)]
pub mod button {
    use super::*;
    use crate::properties::capture_mouse;
    use crate::widgets::text::properties::{TextColorDisabledVar, TextColorVar};

    inherit!(focusable_mixin);
    inherit!(container);

    properties! {
        /// Button click event.
        ///
        /// # Example
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
        background_color = theme::BackgroundVar;

        /// Button border.
        border = {
            widths: theme::BorderWidthsVar,
            sides: theme::BorderSidesVar,
            radius: theme::BorderRadiusVar,
        };

        /// Color of text inside the button [`content`](#wp-content).
        text_color = TextColorVar;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the button is pressed.
        capture_mouse = true;

        child {
            /// Content padding.
            padding = theme::PaddingVar;
        }

        /// When the pointer device is over this button.
        when self.is_cap_hovered {
            background_color = theme::hovered::BackgroundVar;
            border = {
                widths: theme::hovered::BorderWidthsVar,
                sides: theme::hovered::BorderSidesVar,
                radius: theme::hovered::BorderRadiusVar,
            };
        }

        /// When the button is pressed in a way that press release will cause a button click.
        when self.is_pressed  {
            background_color = theme::pressed::BackgroundVar;
            border = {
                widths: theme::pressed::BorderWidthsVar,
                sides: theme::pressed::BorderSidesVar,
                radius: theme::pressed::BorderRadiusVar,
            };
        }

        /// When the button is disabled.
        when self.is_disabled {
            background_color = theme::disabled::BackgroundVar;
            border = {
                widths: theme::disabled::BorderWidthsVar,
                sides: theme::disabled::BorderSidesVar,
                radius: theme::disabled::BorderRadiusVar,
            };
            text_color = TextColorDisabledVar;
        }
    }

    /// Context variables and properties that affect the button appearance from parent widgets.
    pub mod theme {
        use super::*;

        context_var! {
            /// Button background color.
            ///
            /// Use the [`button::theme::background`] property to set.
            ///
            /// [`button::theme::background`]: fn@background
            pub struct BackgroundVar: Rgba = rgb(0.2, 0.2, 0.2);

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
            /// Button border radius.
            ///
            /// Use the [`button::theme::border`] property to set.
            ///
            /// [`button::theme::border`]: fn@border
            pub struct BorderRadiusVar: BorderRadius = BorderRadius::new_all(0.0);

            pub struct PaddingVar: SideOffsets = SideOffsets::new(7.0, 15.0, 7.0, 15.0);
        }

        /// Sets the [`BackgroundVar`] that affects all buttons inside the widget.
        #[property(context, default(BackgroundVar))]
        pub fn background(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, BackgroundVar, color)
        }

        /// Sets the [`BorderWidthsVar`], [`BorderSidesVar`] and [`BorderRadiusVar`] that affects all buttons inside the widget.
        #[property(context, default(BorderWidthsVar, BorderSidesVar, BorderRadiusVar))]
        pub fn border(
            child: impl UiNode,
            widths: impl IntoVar<SideOffsets>,
            sides: impl IntoVar<BorderSides>,
            radius: impl IntoVar<BorderRadius>,
        ) -> impl UiNode {
            let child = with_context_var(child, BorderWidthsVar, widths);
            let child = with_context_var(child, BorderSidesVar, sides);
            with_context_var(child, BorderRadiusVar, radius)
        }

        /// Pointer hovered values.
        pub mod hovered {
            use super::*;

            context_var! {
                /// Hovered button background color.
                ///
                /// Use the [`button::theme::hovered::background`] property to set.
                ///
                /// [`button::theme::hovered::background`]: fn@background
                pub struct BackgroundVar: Rgba = rgb(0.25, 0.25, 0.25);
                /// Hovered button border widths.
                ///
                /// Use the [`button::theme::hovered::border`] property to set.
                ///
                /// [`button::theme::hovered::border`]: fn@border
                pub struct BorderWidthsVar: SideOffsets = SideOffsets::new_all(1.0);
                /// Hovered button border sides.
                ///
                /// Use the [`button::theme::hovered::border`] property to set.
                ///
                /// [`button::theme::hovered::border`]: fn@border
                pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.4, 0.4, 0.4));
                /// Hovered button border radius.
                ///
                /// Use the [`button::theme::hovered::border`] property to set.
                ///
                /// [`button::theme::hovered::border`]: fn@border
                pub struct BorderRadiusVar: BorderRadius = BorderRadius::new_all(0.0);
            }

            /// Sets the hovered [`BackgroundVar`] that affects all buttons inside the widget.
            #[property(context, default(BackgroundVar))]
            pub fn background(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, BackgroundVar, color)
            }

            /// Sets the hovered [`BorderWidthsVar`], [`BorderSidesVar`] and [`BorderRadiusVar`] that affects all buttons inside the widget.
            #[property(context, default(BorderWidthsVar, BorderSidesVar, BorderRadiusVar))]
            pub fn border(
                child: impl UiNode,
                widths: impl IntoVar<SideOffsets>,
                sides: impl IntoVar<BorderSides>,
                radius: impl IntoVar<BorderRadius>,
            ) -> impl UiNode {
                let child = with_context_var(child, BorderWidthsVar, widths);
                let child = with_context_var(child, BorderSidesVar, sides);
                with_context_var(child, BorderRadiusVar, radius)
            }
        }

        /// Button pressed values.
        pub mod pressed {
            use super::*;

            context_var! {
                /// Pressed button background color.
                ///
                /// Use the [`button::theme::pressed::background`] property to set.
                ///
                /// [`button::theme::pressed::background`]: fn@background
                pub struct BackgroundVar: Rgba = rgb(0.3, 0.3, 0.3);
                /// Pressed button border widths.
                ///
                /// Use the [`button::theme::pressed::border`] property to set.
                ///
                /// [`button::theme::pressed::border`]: fn@border
                pub struct BorderWidthsVar: SideOffsets = SideOffsets::new_all(1.0);
                /// Pressed button border sides.
                ///
                /// Use the [`button::theme::pressed::border`] property to set.
                ///
                /// [`button::theme::pressed::border`]: fn@border
                pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.6, 0.6, 0.6));
                /// Pressed button border radius.
                ///
                /// Use the [`button::theme::pressed::border`] property to set.
                ///
                /// [`button::theme::pressed::border`]: fn@border
                pub struct BorderRadiusVar: BorderRadius = BorderRadius::new_all(0.0);
            }

            /// Sets the pressed [`BackgroundVar`] that affects all buttons inside the widget.
            #[property(context, default(BackgroundVar))]
            pub fn background(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, BackgroundVar, color)
            }

            /// Sets the pressed [`BorderWidthsVar`], [`BorderSidesVar`] and [`BorderRadiusVar`] that affects all buttons inside the widget.
            #[property(context, default(BorderWidthsVar, BorderSidesVar, BorderRadiusVar))]
            pub fn border(
                child: impl UiNode,
                widths: impl IntoVar<SideOffsets>,
                sides: impl IntoVar<BorderSides>,
                radius: impl IntoVar<BorderRadius>,
            ) -> impl UiNode {
                let child = with_context_var(child, BorderWidthsVar, widths);
                let child = with_context_var(child, BorderSidesVar, sides);
                with_context_var(child, BorderRadiusVar, radius)
            }
        }

        /// Button disabled values.
        pub mod disabled {
            use super::*;

            context_var! {
                /// Disabled button background color.
                ///
                /// Use the [`button::theme::disabled::background`] property to set.
                ///
                /// [`button::theme::disabled::background`]: fn@background
                pub struct BackgroundVar: Rgba = rgb(0.2, 0.2, 0.2);
                /// Disabled button border widths.
                ///
                /// Use the [`button::theme::disabled::border`] property to set.
                ///
                /// [`button::theme::disabled::border`]: fn@border
                pub struct BorderWidthsVar: SideOffsets = SideOffsets::new_all(1.0);
                /// Disabled button border sides.
                ///
                /// Use the [`button::theme::disabled::border`] property to set.
                ///
                /// [`button::theme::disabled::border`]: fn@border
                pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.2, 0.2, 0.2));
                /// Disabled button border radius.
                ///
                /// Use the [`button::theme::disabled::border`] property to set.
                ///
                /// [`button::theme::disabled::border`]: fn@border
                pub struct BorderRadiusVar: BorderRadius = BorderRadius::new_all(0.0);
            }

            /// Sets the disabled [`BackgroundVar`] that affects all buttons inside the widget.
            #[property(context, default(BackgroundVar))]
            pub fn background(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, BackgroundVar, color)
            }

            /// Sets the disabled [`BorderWidthsVar`], [`BorderSidesVar`] and [`BorderRadiusVar`] that affects all buttons inside the widget.
            #[property(context, default(BorderWidthsVar, BorderSidesVar, BorderRadiusVar))]
            pub fn border(
                child: impl UiNode,
                widths: impl IntoVar<SideOffsets>,
                sides: impl IntoVar<BorderSides>,
                radius: impl IntoVar<BorderRadius>,
            ) -> impl UiNode {
                let child = with_context_var(child, BorderWidthsVar, widths);
                let child = with_context_var(child, BorderSidesVar, sides);
                with_context_var(child, BorderRadiusVar, radius)
            }
        }
    }
}
