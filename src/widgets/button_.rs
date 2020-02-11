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
    use crate::widgets::container;

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

/*

widget! {
    //! Button widget.
    //! # Arguments
    //! * `on_click`: Required button click event handler.
    //! * `padding`: Margin around the button content.
    //! * `background_color`:
    //! * `border`:

    use $crate::properties::*;

     // Properties applied to child before calling widget fn.
     properties(child) {
         // Property declaration without default value, if not set does not apply.
         // If set applies margin to child.
         padding -> margin;
         // Same with default value.
         content_align: CENTER -> align;
         // Default value of background_color property that is applied to child.
         background_color: rgb(0, 0, 0);
     },


     // Properties applied to return of widget fn. Same sintax as
     // child_properties.
     properties(self) {
         border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
     },

     on(hover) {
        background_color: rgb(10, 10, 10);
        border: 4., (Var::clone(&text_border), BorderStyle::Dashed);
     }

     on(pressed) {

     }

     on(disabled) {

     }

     // widget signature, must name the parameters after child,
     // they behave like required properties in the declared button! macro.

     pub fn button(child: impl Ui, on_click: impl FnMut(ButtonInput, &mut NextUpdate) + 'static, state: State) -> impl Ui {
         let on_click = Rc::new(RefCell::new(on_click));
         ui_part! {
             focusable: default;
             on_mouse_enter: |_, _| {
                state.hover(true);
             };
             on_mouse_leave: |_, _| {
                state.hover(false);
             };
             on_mouse_down: |_, _| {
                state.pressed(true);
             };
             on_mouse_up: |_, _| {
                state.pressed(false);
             };
             on_click: enclose! ((on_click) move |ci, n|{
                 if ci.button == MouseButton::Left {
                     (&mut *on_click.borrow_mut())(ButtonInput::Mouse(ci), n);
                 }
             });
             on_key_tap: move |kt, n|{
                 if kt.key == VirtualKeyCode::Return || kt.key == VirtualKeyCode::Space {
                     (&mut *on_click.borrow_mut())(ButtonInput::Keyboard(kt), n);
                 }
             };
             => child
         }
     }
}
*/
