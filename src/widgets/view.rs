use crate::core2::*;
use zero_ui_macros::impl_ui_node_crate;

/// [view] presenter function result.
pub enum View<U: UiNode> {
    /// Changes the view UiNode to the new UiNode associated.
    Update(U),
    /// Does not change the view UiNode.
    Same,
}

/// Dynamically presents a data variable.
struct DataView<D: VarValue, U: UiNode, V: Var<D>, P: FnMut(&V, &mut AppContext) -> View<U>> {
    data: V,
    child: U,
    presenter: P,
    _d: std::marker::PhantomData<D>,
}

#[impl_ui_node_crate(child)]
impl<D: VarValue, U: UiNode, V: Var<D>, P: FnMut(&V, &mut AppContext) -> View<U> + 'static> DataView<D, U, V, P> {
    fn refresh_child(&mut self, ctx: &mut AppContext) {
        if let View::Update(new_child) = (self.presenter)(&self.data, ctx) {
            self.child = new_child;
            ctx.push_layout();
        }
    }

    #[UiNode]
    fn init(&mut self, ctx: &mut AppContext) {
        self.refresh_child(ctx);
        self.child.init(ctx);
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut AppContext) {
        if self.data.is_new(ctx) {
            self.refresh_child(ctx);
        }
        self.child.update(ctx);
    }
}

/// Dynamically presents a data variable.
///
/// # Arguments
/// * `data`: Data variable that is presented by this view.
/// * `initial_ui`: UiNode shown before the first presenter call.
/// * `presenter`: A function that generates UiNode from `data`.
///
/// # Usage
/// `presenter` is called on init and every time `data` changes, the function returns a [View]
/// that is used by the view to present the data.
///
/// # Example
/// ```
/// # use zero_ui::{core::*, properties::*, *};
/// fn countdown(n: Var<usize>) -> impl UiNode {
///     enum State {
///         Blank,
///         Count,
///         End,
///     }
///
///     let mut state = State::Blank;
///
///     view(n, Ui3::A(text("starting..")), move |n, _| match state {
///         State::Blank => {
///             state = State::Count;
///             View::Update(Ui3::B(text(n.map(|n| format!("{}!", n).into()))))
///         }
///         State::Count => {
///             if **n > 0 {
///                 View::Same
///             } else {
///                 state = State::End;
///                 View::Update(Ui3::C(ui! {
///                     text_color: rgb(0, 0, 128);
///                     => text("Congratulations!!")
///                 }))
///             }
///         }
///         State::End => ViewUi::Same,
///     })
/// }
/// ```
pub fn view<D: VarValue, U: UiNode, V: Var<D>, P: FnMut(&V, &mut AppContext) -> View<U> + 'static>(
    data: V,
    initial_ui: U,
    presenter: P,
) -> impl UiNode {
    DataView {
        data,
        child: initial_ui,
        presenter,
        _d: std::marker::PhantomData,
    }
}
