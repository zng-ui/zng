use crate::prelude::new_widget::*;

/// A clickable container.
#[widget($crate::widgets::button)]
pub mod button {
    use super::*;
    use crate::properties::capture_mouse;
    use crate::properties::text_theme::{TextColorDisabledVar, TextColorVar};

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
        background_color = BackgroundVar;

        /// Button border.
        border = {
            widths: BorderWidthsVar,
            sides: BorderSidesVar,
            radius: BorderRadiusVar,
        };

        /// Color of text inside the button [`content`](#wp-content).
        text_color = TextColorVar;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the button is pressed.
        capture_mouse = true;

        child {
            /// Content padding.
            padding = PaddingVar;
        }

        /// When the pointer device is over this button.
        when self.is_cap_hovered {
            background_color = BackgroundHoveredVar;
            border = {
                widths: BorderWidthsHoveredVar,
                sides: BorderSidesHoveredVar,
                radius: BorderRadiusHoveredVar,
            };
        }

        /// When the button is pressed in a way that press release will cause a button click.
        when self.is_pressed  {
            background_color = BackgroundPressedVar;
            border = {
                widths: BorderWidthsPressedVar,
                sides: BorderSidesPressedVar,
                radius: BorderRadiusPressedVar,
            };
        }

        /// When the button is disabled.
        when self.is_disabled {
            background_color = BackgroundDisabledVar;
            border = {
                widths: BorderWidthsDisabledVar,
                sides: BorderSidesDisabledVar,
                radius: BorderRadiusDisabledVar,
            };
            text_color = TextColorDisabledVar;
        }
    }

    context_var! {
        pub struct BackgroundVar: Rgba = once rgb(0.2, 0.2, 0.2);
        pub struct BackgroundHoveredVar: Rgba = once rgb(0.25, 0.25, 0.25);
        pub struct BackgroundPressedVar: Rgba = once rgb(0.3, 0.3, 0.3);
        pub struct BackgroundDisabledVar: Rgba = once rgb(0.2, 0.2, 0.2);

        pub struct BorderWidthsVar: SideOffsets = once SideOffsets::new_all(1.0);
        pub struct BorderWidthsHoveredVar: SideOffsets = once SideOffsets::new_all(1.0);
        pub struct BorderWidthsPressedVar: SideOffsets = once SideOffsets::new_all(1.0);
        pub struct BorderWidthsDisabledVar: SideOffsets = once SideOffsets::new_all(1.0);

        pub struct BorderSidesVar: BorderSides = once BorderSides::solid(rgb(0.2, 0.2, 0.2));
        pub struct BorderSidesHoveredVar: BorderSides = once BorderSides::solid(rgb(0.4, 0.4, 0.4));
        pub struct BorderSidesPressedVar: BorderSides = once BorderSides::solid(rgb(0.6, 0.6, 0.6));
        pub struct BorderSidesDisabledVar: BorderSides = once BorderSides::solid(rgb(0.2, 0.2, 0.2));

        pub struct BorderRadiusVar: BorderRadius = once BorderRadius::new_all(0.0);
        pub struct BorderRadiusHoveredVar: BorderRadius = once BorderRadius::new_all(0.0);
        pub struct BorderRadiusPressedVar: BorderRadius = once BorderRadius::new_all(0.0);
        pub struct BorderRadiusDisabledVar: BorderRadius = once BorderRadius::new_all(0.0);

        pub struct PaddingVar: SideOffsets = once SideOffsets::new(7.0, 15.0, 7.0, 15.0);
    }
}
