use crate::prelude::new_widget::*;

/// [`view`] update.
pub enum View<U: UiNode> {
    /// Changes the view child.
    Update(U),
    /// Keep the same view child.
    Same,
}

/// Dynamically presents a data variable.
///
/// # Arguments
/// * `data`: Data variable that is presented by this view.
/// * `initial_ui`: UI shown before the first presenter call.
/// * `presenter`: A function that generates an UI from `data`.
///
/// # Usage
/// The `presenter` function is called on init and every time `data` changes, if it returns
/// [`View::Update(#new_view)`](View::Update) the view child is replaced by `#new_view`.
///
/// The the view container must be able to hold all the possible child UIs, you can use
/// [`UiNode::boxed`](crate::core::UiNode::boxed) to unify the types.
///
/// # Example
/// ```
/// use zero_ui::{
///     core::{color::{rgb, rgba}, text::ToText, var::Var, UiNode},
///     properties::text_theme::{text_color, font_size},
///     widgets::{text::text, view, View},
/// };
///
/// fn countdown(n: impl Var<usize>) -> impl UiNode {
///     enum State {
///         Starting,
///         Counting,
///         End,
///     }
///
///     let mut state = State::Starting;
///
///     view(n,
///
///     // initial_ui:
///     text! {
///         color = rgba(0, 0, 0, 0.5);
///         text = "starting..";
///     }.boxed(),
///
///     // presenter:
///     move |ctx, n| match state {
///         State::Starting => {
///             state = State::Counting;
///             View::Update(text! {
///                 font_size = 28;
///                 text = n.map(|n| n.to_text());
///             }.boxed())
///         }
///         State::Counting => {
///             if *n.get(ctx.vars) > 0 {
///                 // text updates automatically when `n` updates
///                 // se we can continue using the same UI.
///
///                 View::Same
///             } else {
///                 state = State::End;
///
///                 // we want a different style for the end text
///                 // so we need to update the UI.
///
///                 View::Update(
///                     text! {
///                         color = rgb(0, 128, 0);
///                         font_size = 18;
///                         text = "Congratulations!!";
///                     }
///                     .boxed(),
///                 )
///             }
///         }
///         State::End => View::Same,
///     })
/// }
/// ```
pub fn view<D, U, V, P>(data: V, initial_ui: U, presenter: P) -> impl Widget
where
    D: VarValue,
    U: UiNode,
    V: Var<D>,
    P: FnMut(&mut WidgetContext, &V) -> View<U> + 'static,
{
    crate::core::widget_base::implicit_base::new(view_node(data, initial_ui, presenter), WidgetId::new_unique())
}

/// Node only [`view`].
///
/// This is the raw [`UiNode`] that implements the core `view` functionality
/// without defining a full widget.
pub fn view_node<D, U, V, P>(data: V, initial_ui: U, presenter: P) -> impl UiNode
where
    D: VarValue,
    U: UiNode,
    V: Var<D>,
    P: FnMut(&mut WidgetContext, &V) -> View<U> + 'static,
{
    struct ViewNode<D, U, V, P> {
        data: V,
        child: U,
        presenter: P,
        _d: std::marker::PhantomData<D>,
    }
    #[impl_ui_node(child)]
    impl<D, U, V, P> ViewNode<D, U, V, P>
    where
        D: VarValue,
        U: UiNode,
        V: Var<D>,
        P: FnMut(&mut WidgetContext, &V) -> View<U> + 'static,
    {
        fn refresh_child(&mut self, ctx: &mut WidgetContext) {
            if let View::Update(new_child) = (self.presenter)(ctx, &self.data) {
                self.child = new_child;
                ctx.updates.layout();
            }
        }

        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.refresh_child(ctx);
            self.child.init(ctx);
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.data.is_new(ctx.vars) {
                self.refresh_child(ctx);
            }
            self.child.update(ctx);
        }
    }

    ViewNode {
        data,
        child: initial_ui,
        presenter,
        _d: std::marker::PhantomData,
    }
}
