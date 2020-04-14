mod boxed;
mod cloning;
#[macro_use]
mod context;
mod map;
#[macro_use]
mod merge;
mod owned;
mod read_only;
mod shared;
#[macro_use]
mod switch;
mod traits;

pub use boxed::*;
pub use cloning::*;
pub use context::*;
pub use map::*;
pub use merge::*;
pub use owned::*;
pub use read_only::*;
pub use shared::*;
pub use switch::*;
pub use traits::*;

pub mod test_cloning {
    use super::*;
    use crate::core::types::WidgetId;
    use crate::properties::*;

    pub fn test(arg0: impl IntoVar<bool> + Clone, arg1: impl Var<bool> + Clone, arg2: bool) {
        let w_arg0 = arg0.clone().into_var();
        let w_arg1 = arg1.clone().into_var();
        let w_arg2 = arg2.clone().into_var();

        //let _single_when1 = w_arg0;
        //let _single_when2 = w_arg0.map(|b| !(*b));
        let _multi_when = merge_var!(w_arg0, w_arg1, w_arg2, |a, b, c| (*a) && (*b) && (*c));

        call_property(arg0, arg1, arg2);
    }

    // WIDGET:
    // when self.is_pressed && check_id(self.id) { .. }
    pub fn w0(is_pressed: &impl is_pressed::Args, id: &impl id::Args, cursor: &impl cursor::Args) -> impl crate::core::var::Var<bool> {
        // convert args to ArgsWhen, it has all  the property args as vars.
        let self_is_pressed_0 = is_pressed::ArgsNumbered::arg0(is_pressed).clone().into_var();
        let self_id_0 = id::ArgsNumbered::arg0(id).clone().into_var();
        let self_cursor_0 = cursor::ArgsNumbered::arg0(cursor).clone().into_var();

        merge_var!(self_is_pressed_0, self_id_0, |self_is_pressed_0, self_id_0| (*self_is_pressed_0)
            && check_id(*self_id_0))
    }
    fn check_id(_: WidgetId) -> bool {
        true
    }
    pub fn call_property(_arg0: impl IntoVar<bool>, _arg1: impl Var<bool>, _arg2: bool) {}
}
