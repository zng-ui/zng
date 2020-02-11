use crate::core::UiNode;
use crate::core::gesture::ClickArgs;
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

macro_rules! widget2 {
    ($($tt:tt)*) => {};
}

widget2! {
    /// Docs of widget.
    pub button;

    // Uses inserted in the `button!` macro call.
    use crate::properties::{margin, align, Alignment, BorderStyle, on_click};
    use crate::core::types::{rgb, rgba};

    // Properties applied to the macro child.
    default(child) {
        // Property declaration without default value, if not set does not apply.
        // If set applies margin to child.
        padding -> margin;
        // Property declaration with default value, if not set still applies with
        // default value, only does not apply if set with `unset!`.
        content_align -> align: Alignment::CENTER;
        // Property declaration using that does not alias the property name.
        background_color: rgb(255, 255, 255);

        // to have a property apply to child and not `self` you can write:
        background_gradient -> background_gradient;
    }

    // Properties applied to the macro child properties.
    // Same sintax as `default(child)`.
    default(self) {
        border: 4., (rgba(0, 0, 0, 0.0), BorderStyle::Dashed);
        // When `required!` appears in the default values place the user
        // gets an error if the property is not set.
        on_click: required!;
    }

    // `when({bool expr})` blocks set properties given a condition. The
    // expression can contain `self.{property}` and `child.{property}` to reference
    // potentially live updating properties, every time this properties update the
    // expression is rechecked.
    when(self.is_mouse_over) {
        // Sets the properties when the expression is true.
        // the sintax in when blocks is like the sintax of properties
        // in the generated macro
        background_color: rgba(0, 0, 0, 0);
        background_gradient: {
            start: (0.0, 0.0),
            end: (1.0, 1.0),
            stops: vec![rgb(255, 0, 0), rgb(0, 255, 0), rgb(0, 0, 255)],
        };
    }

    /// Optionaly you can wrap the child into widgets, or do any custom code.
    ///
    /// This is evaluated after the `default(child)` and before the `default(self)`.
    => {
        let ct = container! {
            property: "";
            => child
        };
        println!("button created");
        ct
    }
}
