use std::{any::type_name, cell::RefCell, fmt, rc::Rc};

use zero_ui_core::{BoxedUiNode, NilUiNode};

use crate::prelude::new_widget::*;

/// [`view`] update.
pub enum View<U: UiNode> {
    /// Changes the view child.
    Update(U),
    /// Keep the same view child.
    Same,
}
impl<U: UiNode> View<U> {
    /// Convert to `View<BoxedUiNode>`.
    #[inline]
    pub fn boxed(self) -> View<BoxedUiNode> {
        match self {
            View::Update(ui) => View::Update(ui.boxed()),
            View::Same => View::Same,
        }
    }
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
///             if n.copy(ctx) > 0 {
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
            if self.data.is_new(ctx) {
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

type BoxedGenerator<D> = Box<dyn FnMut(&mut WidgetContext, &D) -> BoxedUiNode>;

/// Boxed shared closure that generates a view for a given data.
///
/// # Examples
///
/// Define the content that is shown when an image fails to load:
///
/// ```
/// # use zero_ui::{widgets::{ViewGenerator, image, text}, core::color::colors};
/// # let _ =
/// image! {
///     source = "not_found.png";
///     error_view = ViewGenerator::new(|_ctx, error| text! {
///         text = error;
///         color = colors::RED;
///     });
/// }
/// # ;
/// ```
///
/// You can also use the [`view_generator!`] macro, it has the advantage of being clone move.
pub struct ViewGenerator<D>(Option<Rc<RefCell<BoxedGenerator<D>>>>);
impl<D> Clone for ViewGenerator<D> {
    fn clone(&self) -> Self {
        ViewGenerator(self.0.clone())
    }
}
impl<D> fmt::Debug for ViewGenerator<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ViewGenerator<{}>", type_name::<D>())
    }
}
impl<D> ViewGenerator<D> {
    /// New from a closure that generates a [`View`] update from data.
    pub fn new<U: UiNode>(mut presenter: impl FnMut(&mut WidgetContext, &D) -> U + 'static) -> Self {
        ViewGenerator(Some(Rc::new(RefCell::new(Box::new(move |ctx, data| presenter(ctx, data).boxed())))))
    }

    /// Generator that always produces the [`NilUiNode`].
    ///
    /// No heap allocation happens in this function. See [`nil_static`] for creating context variables.
    pub fn nil() -> Self {
        // TODO make this const when rust#57563 is resolved.
        ViewGenerator(None)
    }

    /// [`nil`] as a static reference.
    ///
    /// Note that this function is `const` allowing it to be used as the default value of a [`context_var!`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui::{core::context_var, widgets::ViewGenerator};
    /// # pub struct Foo;
    /// context_var! {
    ///     /// View generator for `Foo` items.
    ///     pub struct FooViewVar: ViewGenerator<Foo> = return ViewGenerator::nil_static();
    /// }
    pub fn nil_static() -> &'static Self {
        #[cfg(debug_assertions)]
        fn _assert_size() {
            let _: ViewGenerator<()> = unsafe { std::mem::transmute(None::<std::num::NonZeroUsize>) };
        }
        
        // SAFETY: TODO check if None is zero in both cases.
        static NIL: Option<std::num::NonZeroUsize> = None;
        unsafe { std::mem::transmute(&NIL) }
    }

    /// If this is  the [`nil`] generator.
    ///
    /// [`nil`]: ViewGenerator::nil
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// Executes the generator for the given `data`.
    pub fn present(&self, ctx: &mut WidgetContext, data: &D) -> BoxedUiNode {
        if let Some(p) = &self.0 {
            let mut presenter = p.borrow_mut();
            presenter(ctx, data)
        } else {
            NilUiNode.boxed()
        }
    }
}

/// <span data-inline></span> Declares a view generator closure.
///
/// The output type is a [`ViewGenerator`], the closure is [`clone_move!`].
///
/// # Examples
///
/// Define the content that is shown when an image fails to load, capturing another variable too.
///
/// ```
/// # use zero_ui::{widgets::{view_generator, image, text}, core::{color::{Rgba, colors}, var::var, widget_base::Visibility}};
/// let img_error_vis = var(Visibility::Visible);
/// # let _ =
/// image! {
///     source = "not_found.png";
///     error_view = view_generator!(img_error_vis, |_ctx, error| text! {
///         text = error;
///         color = colors::RED;
///         visibility = img_error_vis;
///     });
/// }
/// # ;
/// ```
///
/// [`ViewGenerator`]: crate::widgets::ViewGenerator
/// [`clone_move!`]: crate::core::clone_move
#[macro_export]
macro_rules! view_generator {
    ($($tt:tt)+) => {
        $crate::widgets::ViewGenerator::new($crate::core::clone_move! {
            $($tt)+
        })
    }
}
#[doc(inline)]
pub use crate::view_generator;
