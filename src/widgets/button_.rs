use crate::core::types::{rgb, ColorF};
#[doc(hidden)]
pub use crate::properties::{align, background_color, is_hovered, is_pressed, on_click};
use crate::widget;
use crate::widgets::{container, focusable_mixin};

context_var! {
    /// Default background of [`button!`](crate::widgets::button) widgets.
    pub struct ButtonBackground: ColorF = rgb(0.2, 0.2, 0.2);
    pub struct ButtonBackgroundHovered: ColorF = rgb(0.25, 0.25, 0.25);
    pub struct ButtonBackgroundPressed: ColorF = rgb(0.3, 0.3, 0.3);
    pub struct ButtonBackgroundDisabled: ColorF = rgb(1.0, 1.0, 1.0);
}

widget! {
    /// A clickable container.
    pub button: container + focusable_mixin;

    default(self) {
        /// Button click event.
        on_click: required!;

        /// Set to [`ButtonBackground`](crate::widgets::ButtonBackground).
        background_color: ButtonBackground;
    }

    /// When the pointer device is over this button.
    when self.is_hovered {
        background_color: ButtonBackgroundHovered;
    }

    ///// When the mouse or touch pressed on this button and has not yet released.
    //when  self.is_pressed  {
    //    background_color: ButtonBackgroundPressed;
    //}

   ///// When the button is not enabled.
   //when {
   //    for i in 0..1000 {
   //        if i == 10 {
   //            return self.is_hovered.state
   //        } else if i %30 == 0 {
   //            return self.is_hovered.0
   //        }
   //    }
   //    self.is_pressed
   //} {
   //    background_color: ButtonBackgroundDisabled;
   //}
}

//TODO support properties with IntoVar parameters.
