use std::{any::type_name, fmt, rc::Rc};

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
/// # Examples
/// 
/// ```
/// use zero_ui::{
///     core::{color::{rgb, rgba}, text::ToText, var::Var, UiNode},
///     widgets::{text, text::{properties::{text_color, font_size}}, view, View},
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
                ctx.updates.info_layout_and_render();
            }
        }

        #[UiNode]
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.data);
            self.child.subscriptions(ctx, subscriptions);
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

type BoxedGenerator<D> = Box<dyn Fn(&mut WidgetContext, D) -> BoxedUiNode>;

/// Boxed shared closure that generates a view for a given data.
///
/// # Examples
///
/// Define the content that is shown when an image fails to load:
///
/// ```
/// # use zero_ui::{widgets::{ViewGenerator, image, image::properties::ImageErrorArgs, text}, core::color::colors};
/// # let _ =
/// image! {
///     source = "not_found.png";
///     error_view = ViewGenerator::new(|_ctx, e: ImageErrorArgs| text! {
///         text = e.error.clone();
///         color = colors::RED;
///     });
/// }
/// # ;
/// ```
///
/// You can also use the [`view_generator!`] macro, it has the advantage of being clone move.
pub struct ViewGenerator<D: ?Sized>(Option<Rc<BoxedGenerator<D>>>);
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
    pub fn new<U: UiNode>(generator: impl Fn(&mut WidgetContext, D) -> U + 'static) -> Self {
        ViewGenerator(Some(Rc::new(Box::new(move |ctx, data| generator(ctx, data).boxed()))))
    }

    /// Generator that always produces the [`NilUiNode`].
    ///
    /// No heap allocation happens in this function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui::{core::context_var, widgets::ViewGenerator};
    /// # pub struct Foo;
    /// context_var! {
    ///     /// View generator for `Foo` items.
    ///     pub struct FooViewVar: ViewGenerator<Foo> = ViewGenerator::nil();
    /// }
    /// ```
    pub fn nil() -> Self {
        // TODO make this const when rust#57563 is resolved.
        ViewGenerator(None)
    }

    /// If this is  the [`nil`] generator.
    ///
    /// [`nil`]: ViewGenerator::nil
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// Executes the generator for the given `data`.
    pub fn generate(&self, ctx: &mut WidgetContext, data: D) -> BoxedUiNode {
        if let Some(g) = &self.0 {
            g(ctx, data)
        } else {
            NilUiNode.boxed()
        }
    }

    /// Create a presenter node that delegates views generated by `generator`.
    ///
    /// The `update` closure is called every [`UiNode::init`] and [`UiNode::update`], it must return a [`DataUpdate`]
    /// that is used to [`generate`] a view, all other [`UiNode`] methods are delegated to this view. The `update` closure
    /// is also called every time the `generator` variable updates. The boolean parameter indicates if the generator variable has updated.
    ///
    /// The `subscribe` is called every time [`UiNode::subscriptions`] is called, it must register every update source that affects `update`.
    ///
    /// [`generate`]: ViewGenerator::generate
    pub fn presenter(
        generator: impl IntoVar<ViewGenerator<D>>,
        subscribe: impl Fn(&VarsRead, &mut WidgetSubscriptions) + 'static,
        update: impl FnMut(&mut WidgetContext, bool) -> DataUpdate<D> + 'static,
    ) -> impl UiNode
    where
        D: 'static,
    {
        Self::presenter_map(generator, subscribe, update, |v| v)
    }

    /// Create a presenter node that only updates when the `generator` updates using the [`Default`] data.
    pub fn presenter_default(generator: impl IntoVar<ViewGenerator<D>>) -> impl UiNode
    where
        D: Default + 'static,
    {
        Self::presenter(
            generator,
            |_, _| {},
            |_, new| if new { DataUpdate::Update(D::default()) } else { DataUpdate::Same },
        )
    }

    /// Like [`presenter`] but the generated view can be modified using the `map` closure.
    ///
    /// [`presenter`]: ViewGenerator::presenter
    pub fn presenter_map<V>(
        generator: impl IntoVar<ViewGenerator<D>>,
        subscribe: impl Fn(&VarsRead, &mut WidgetSubscriptions) + 'static,
        update: impl FnMut(&mut WidgetContext, bool) -> DataUpdate<D> + 'static,
        map: impl FnMut(BoxedUiNode) -> V + 'static,
    ) -> impl UiNode
    where
        D: 'static,
        V: UiNode,
    {
        struct ViewGenVarPresenter<G, S, U, M, V> {
            gen: G,
            subscribe: S,
            update: U,
            map: M,
            child: Option<V>,
        }
        #[impl_ui_node(child)]
        impl<D, G, S, U, M, V> UiNode for ViewGenVarPresenter<G, S, U, M, V>
        where
            D: 'static,
            G: Var<ViewGenerator<D>>,
            S: Fn(&VarsRead, &mut WidgetSubscriptions) + 'static,
            U: FnMut(&mut WidgetContext, bool) -> DataUpdate<D> + 'static,
            M: FnMut(BoxedUiNode) -> V + 'static,
            V: UiNode,
        {
            fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
                subscriptions.var(ctx, &self.gen);
                (self.subscribe)(ctx.vars, subscriptions);
                self.child.subscriptions(ctx, subscriptions);
            }

            fn init(&mut self, ctx: &mut WidgetContext) {
                let gen = self.gen.get(ctx.vars);

                if gen.is_nil() {
                    self.child = None;
                    return;
                }

                match (self.update)(ctx, true) {
                    DataUpdate::Update(data) => {
                        let mut child = (self.map)(gen.generate(ctx, data));
                        child.init(ctx);
                        self.child = Some(child);
                    }
                    DataUpdate::Same => self.child.init(ctx),
                    DataUpdate::None => self.child = None,
                }
            }
            fn deinit(&mut self, ctx: &mut WidgetContext) {
                self.child.deinit(ctx);
            }
            fn update(&mut self, ctx: &mut WidgetContext) {
                let gen = self.gen.get(ctx.vars);

                if gen.is_nil() {
                    if let Some(mut old) = self.child.take() {
                        old.deinit(ctx);
                        ctx.updates.info_layout_and_render();
                    }

                    return;
                }

                match (self.update)(ctx, self.gen.is_new(ctx.vars)) {
                    DataUpdate::Update(data) => {
                        if let Some(mut old) = self.child.take() {
                            old.deinit(ctx);
                        }
                        let mut child = (self.map)(gen.generate(ctx, data));
                        child.init(ctx);
                        self.child = Some(child);
                        ctx.updates.info_layout_and_render();
                    }
                    DataUpdate::Same => self.child.update(ctx),
                    DataUpdate::None => {
                        if let Some(mut old) = self.child.take() {
                            old.deinit(ctx);
                            ctx.updates.info_layout_and_render();
                        }
                    }
                }
            }
        }
        ViewGenVarPresenter {
            gen: generator.into_var(),
            subscribe,
            update,
            map,
            child: None,
        }
    }
}

/// An update for the [`ViewGenerator::presenter`].
#[derive(Debug, Clone, Copy)]
pub enum DataUpdate<D> {
    /// Generate a new view using the data.
    Update(D),
    /// Continue using the generated view, if there is any.
    Same,
    /// Discard the current view, does not present any view.
    None,
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
/// # use zero_ui::{widgets::{view_generator, image, image::properties::ImageErrorArgs, text}, core::{color::{Rgba, colors}, var::var, widget_base::Visibility}};
/// let img_error_vis = var(Visibility::Visible);
/// # let _ =
/// image! {
///     source = "not_found.png";
///     error_view = view_generator!(img_error_vis, |_ctx, e: ImageErrorArgs| text! {
///         text = e.error.clone();
///         color = colors::RED;
///         visibility = img_error_vis.clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_nil_static_unsafe() {
        // same size:
        let _: ViewGenerator<()> = unsafe { std::mem::transmute(None::<std::num::NonZeroUsize>) };

        // same value:
        let nil: usize = unsafe { std::mem::transmute(ViewGenerator::<()>::nil()) };
        let none: usize = unsafe { std::mem::transmute(None::<std::num::NonZeroUsize>) };
        assert_eq!(nil, none);
    }
}
