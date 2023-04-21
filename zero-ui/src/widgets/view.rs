use pretty_type_name::*;
use std::{fmt, ops, sync::Arc};

use zero_ui_core::widget_instance::{BoxedUiNode, NilUiNode};

use crate::{core::widget_instance::ArcNode, prelude::new_widget::*};

/// [`view`] update.
pub enum View<U: UiNode> {
    /// Changes the widget child.
    Update(U),
    /// Keep the same widget child.
    Same,
}
impl<U: UiNode> View<U> {
    /// Convert to `View<BoxedUiNode>`.
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
///
/// * `data`: Data variable that is presented by this view.
/// * `initial_ui`: UI shown before the first presenter call.
/// * `presenter`: A function that generates an UI from `data`.
///
/// # Usage
///
/// The `presenter` function is called on init and every time `data` changes, if it returns
/// [`View::Update(#new_view)`](View::Update) the view child is replaced by `#new_view`.
///
/// The view container must be able to hold all the possible child UIs, you can use
/// [`UiNode::boxed`](crate::core::widget_instance::UiNode::boxed) to unify the types.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::*;
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
///     Text! {
///         txt_color = rgba(0, 0, 0, 0.5);
///         txt = "starting..";
///     }.boxed(),
///
///     // presenter:
///     move |n| match state {
///         State::Starting => {
///             state = State::Counting;
///             View::Update(Text! {
///                 font_size = 28;
///                 txt = n.map(|n| n.to_text());
///             }.boxed())
///         }
///         State::Counting => {
///             if n.get() > 0 {
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
///                     Text! {
///                         txt_color = rgb(0, 128, 0);
///                         font_size = 18;
///                         txt = "Congratulations!";
///                     }
///                     .boxed(),
///                 )
///             }
///         }
///         State::End => View::Same,
///     })
/// }
/// ```
pub fn view<D, U, V, P>(data: V, initial_ui: U, presenter: P) -> impl UiNode
where
    D: VarValue,
    U: UiNode,
    V: Var<D>,
    P: FnMut(&V) -> View<U> + Send + 'static,
{
    use crate::core::widget_base::nodes;

    let node = nodes::widget_inner(view_node(data, initial_ui, presenter));
    nodes::widget(node, WidgetId::new_unique()).cfg_boxed()
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
    P: FnMut(&V) -> View<U> + Send + 'static,
{
    #[cfg(dyn_closure)]
    let presenter: Box<dyn FnMut(&V) -> View<U> + Send> = Box::new(presenter);

    #[ui_node(struct ViewNode<D: VarValue> {
        #[var] data: impl Var<D>,
        child: impl UiNode,
        presenter: impl FnMut(&T_data) -> View<T_child> + Send + 'static,
        _d: std::marker::PhantomData<D>,
    })]
    impl UiNode for ViewNode {
        fn init(&mut self) {
            self.auto_subs();

            if let View::Update(new_child) = (self.presenter)(&self.data) {
                self.child = new_child;
            }

            self.child.init();
        }

        fn update(&mut self, updates: &WidgetUpdates) {
            if self.data.is_new() {
                if let View::Update(new_child) = (self.presenter)(&self.data) {
                    self.child.deinit();
                    self.child = new_child;
                    self.child.init();
                    WIDGET.update_info().layout().render();
                }
            }
            self.child.update(updates);
        }
    }

    ViewNode {
        data,
        child: initial_ui,
        presenter,
        _d: std::marker::PhantomData,
    }
    .cfg_boxed()
}

type BoxedWgtFn<D> = Box<dyn Fn(D) -> BoxedUiNode + Send + Sync>;

/// Boxed shared closure that generates an widget for a given data.
///
/// # Examples
///
/// Define the content that is shown when an image fails to load:
///
/// ```
/// # use zero_ui::prelude::*;
/// # let _ =
/// Image! {
///     source = "not_found.png";
///     img_error_fn = WidgetFn::new(|e: image::ImgErrorArgs| Text! {
///         txt = e.error.clone();
///         txt_color = colors::RED;
///     });
/// }
/// # ;
/// ```
///
/// You can also use the [`wgt_fn!`] macro, it has the advantage of being clone move.
pub struct WidgetFn<D: ?Sized>(Option<Arc<BoxedWgtFn<D>>>);
impl<D> Clone for WidgetFn<D> {
    fn clone(&self) -> Self {
        WidgetFn(self.0.clone())
    }
}
impl<D> fmt::Debug for WidgetFn<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WidgetFn<{}>", pretty_type_name::<D>())
    }
}
impl<D> WidgetFn<D> {
    /// New from a closure that generates a [`View`] update from data.
    pub fn new<U: UiNode>(func: impl Fn(D) -> U + Send + Sync + 'static) -> Self {
        WidgetFn(Some(Arc::new(Box::new(move |data| func(data).boxed()))))
    }

    /// Function that always produces the [`NilUiNode`].
    ///
    /// No heap allocation happens in this function.
    ///
    /// # Examples
    ///
    /// ```
    /// # use zero_ui::{core::context_var, widgets::WidgetFn};
    /// # pub struct Foo;
    /// context_var! {
    ///     /// Widget function for `Foo` items.
    ///     pub static FOO_GEN_VAR: WidgetFn<Foo> = WidgetFn::nil();
    /// }
    /// ```
    pub fn nil() -> Self {
        // TODO make this const when rust#100136 is resolved.
        WidgetFn(None)
    }

    /// If this is  the [`nil`] function.
    ///
    /// [`nil`]: WidgetFn::nil
    pub fn is_nil(&self) -> bool {
        self.0.is_none()
    }

    /// Calls the function with `data` argument.
    ///
    /// Note that you can call the widget function directly where `D: 'static`:
    ///
    /// ```
    /// use zero_ui::prelude::*;
    ///
    /// fn foo(func: &WidgetFn<bool>) {
    ///     let a = func.call(true);
    ///     let b = func(true);
    /// }
    /// ```
    ///
    /// In the example above `a` and `b` are both calls to the widget function.
    pub fn call(&self, data: D) -> BoxedUiNode {
        if let Some(g) = &self.0 {
            g(data)
        } else {
            NilUiNode.boxed()
        }
    }

    /// Create a presenter node that delegates widgets generated by `func`.
    ///
    /// The `update` closure is called every [`UiNode::init`] and [`UiNode::update`], it must return a [`DataUpdate`]
    /// that is used to instantiate a widget, all other [`UiNode`] methods are delegated to this widget. The `update` closure
    /// is also called every time the `func` variable updates. The boolean parameter indicates if the function variable has updated or
    /// is init.
    pub fn presenter(func: impl IntoVar<WidgetFn<D>>, update: impl FnMut(bool) -> DataUpdate<D> + Send + 'static) -> impl UiNode
    where
        D: 'static,
    {
        Self::presenter_map(func, update, |v| v)
    }

    /// Create a presenter node that only updates when the `func` updates using the [`Default`] data.
    pub fn presenter_default(func: impl IntoVar<WidgetFn<D>>) -> impl UiNode
    where
        D: Default + 'static,
    {
        Self::presenter(func, |new| if new { DataUpdate::Update(D::default()) } else { DataUpdate::Same })
    }

    /// Like [`presenter`] but the generated widget can be modified using the `map` closure.
    ///
    /// [`presenter`]: WidgetFn::presenter
    pub fn presenter_map<V>(
        func: impl IntoVar<WidgetFn<D>>,
        update: impl FnMut(bool) -> DataUpdate<D> + Send + 'static,
        map: impl FnMut(BoxedUiNode) -> V + Send + 'static,
    ) -> impl UiNode
    where
        D: 'static,
        V: UiNode,
    {
        let func = func.into_var();

        #[cfg(dyn_closure)]
        let update: Box<dyn FnMut(bool) -> DataUpdate<D> + Send> = Box::new(update);

        #[cfg(dyn_closure)]
        let map: Box<dyn FnMut(BoxedUiNode) -> V + Send> = Box::new(map);

        #[ui_node(struct ViewFnVarPresenter<D: 'static, V: UiNode> {
            #[var] func: impl Var<WidgetFn<D>>,
            update: impl FnMut(bool) -> DataUpdate<D> + Send + 'static,
            map: impl FnMut(BoxedUiNode) -> V + Send + 'static,
            child: Option<V>,
        })]
        impl UiNode for ViewFnVarPresenter {
            fn init(&mut self) {
                self.auto_subs();

                let func = self.func.get();

                if func.is_nil() {
                    self.child = None;
                    return;
                }

                match (self.update)(true) {
                    DataUpdate::Update(data) => {
                        let mut child = (self.map)(func(data));
                        child.init();
                        self.child = Some(child);
                    }
                    DataUpdate::Same => self.child.init(),
                    DataUpdate::None => self.child = None,
                }
            }
            fn deinit(&mut self) {
                self.child.deinit();
            }
            fn update(&mut self, updates: &WidgetUpdates) {
                let func = self.func.get();

                if func.is_nil() {
                    if let Some(mut old) = self.child.take() {
                        old.deinit();
                        WIDGET.update_info().layout().render();
                    }

                    return;
                }

                match (self.update)(self.func.is_new()) {
                    DataUpdate::Update(data) => {
                        if let Some(mut old) = self.child.take() {
                            old.deinit();
                        }
                        let mut child = (self.map)(func(data));
                        child.init();
                        self.child = Some(child);
                        WIDGET.update_info().layout().render();
                    }
                    DataUpdate::Same => self.child.update(updates),
                    DataUpdate::None => {
                        if let Some(mut old) = self.child.take() {
                            old.deinit();
                            WIDGET.update_info().layout().render();
                        }
                    }
                }
            }
        }
        ViewFnVarPresenter {
            func,
            update,
            map,
            child: None,
        }
        .cfg_boxed()
    }

    /// New widget function that returns the same `widget` for every call.
    ///
    /// The `widget` is wrapped in an [`ArcNode`] and every function call returns an [`ArcNode::take_on_init`] node.
    pub fn singleton(widget: impl UiNode) -> Self {
        let widget = ArcNode::new(widget);
        Self::new(move |_| widget.take_on_init())
    }
}
impl<D: 'static> ops::Deref for WidgetFn<D> {
    type Target = dyn Fn(D) -> BoxedUiNode;

    fn deref(&self) -> &Self::Target {
        match self.0.as_ref() {
            Some(f) => &**f,
            None => &nil_call::<D>,
        }
    }
}
fn nil_call<D>(_: D) -> BoxedUiNode {
    NilUiNode.boxed()
}

/// An update for the [`WidgetFn::presenter`].
#[derive(Debug, Clone, Copy)]
pub enum DataUpdate<D> {
    /// Generate a new widget using the data.
    Update(D),
    /// Continue using the generated widget, if there is any.
    Same,
    /// Discard the current widget, does not present any widget.
    None,
}

/// <span data-del-macro-root></span> Declares a widget function closure.
///
/// The output type is a [`WidgetFn`], the closure is [`clmv!`].
///
/// # Examples
///
/// Define the content that is shown when an image fails to load, capturing another variable too.
///
/// ```
/// # use zero_ui::prelude::*;
/// let img_error_vis = var(Visibility::Visible);
/// # let _ =
/// Image! {
///     source = "not_found.png";
///     img_error_fn = wgt_fn!(img_error_vis, |e: image::ImgErrorArgs| Text! {
///         txt = e.error.clone();
///         txt_color = colors::RED;
///         visibility = img_error_vis.clone();
///     });
/// }
/// # ;
/// ```
///
/// [`WidgetFn`]: crate::widgets::WidgetFn
/// [`clmv!`]: crate::core::clmv
#[macro_export]
macro_rules! wgt_fn {
    ($($tt:tt)+) => {
        $crate::widgets::WidgetFn::new($crate::core::clmv! {
            $($tt)+
        })
    }
}
#[doc(inline)]
pub use crate::wgt_fn;
