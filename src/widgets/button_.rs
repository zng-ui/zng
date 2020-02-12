use crate::core::gesture::ClickArgs;
use crate::core::UiNode;
use crate::properties::OnEventArgs;
use crate::widget;

widget! {
    //! Button widget.
    //! # Arguments
    //! * `on_click`: Required button click event handler.
    //! * `padding`: Margin around the button content.
    //! * `background_color`:
    //! * `border`:
    //!
    //! # Examples
    //! ```
    //! use crate::widgets::text;
    //!
    //! button! {
    //!     on_click: |_, _| { println!("Button clicked!") };
    //!     => text("Click Me!")
    //! }
    //! ```

    use crate::properties::{margin, align, Alignment, BorderStyle, on_click};
    use crate::core::types::{rgb, rgba};

    // Properties applied to child before calling widget fn.
    child_properties {
        // Property declaration without default value, if not set does not apply.
        // If set applies margin to child.
        padding -> margin;
        // Same with default value.
        content_align -> align: Alignment::CENTER;
        // Default value of background_color property that is applied to child.
        background_color: rgb(255, 255, 255);
    }


    // Properties applied to return of widget fn. Same sintax as
    // child_properties.
    self_properties {
        border: 4., (rgba(0, 0, 0, 0.0), BorderStyle::Dashed);
        //on_click: required!;
    }

    // widget signature, must name the parameters after child,
    // they behave like required properties in the declared button! macro.

    pub fn button(child: impl UiNode, on_click: impl FnMut(OnEventArgs<ClickArgs>)) -> impl UiNode {
        todo!();
        child
        //container! {
        //    id: unset!;
        //    on_click: on_click;
        //    => child
        //}
    }
}
