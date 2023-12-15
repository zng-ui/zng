//! Dynamic widget instantiation from data.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use parking_lot::Mutex;
use std::{any::TypeId, sync::Arc};

use zero_ui_wgt::prelude::*;

mod vec;
pub use vec::{ObservableVec, VecChange};

/// Node that presents `data` using `update`.
///
/// The node's child is always the result of `update` for the `data` value, it is reinited every time
/// either `data` or `update` updates.
///
/// See also [`presenter_opt`] for a presenter that is nil with the data is `None` and [`View!`] for
/// avoiding a info tree rebuild for every data update.
///
/// Note that this node is not a full widget, it can be used as part of an widget without adding to the info tree.
///
/// [`View!`]: struct@View
pub fn presenter<D: VarValue>(data: impl IntoVar<D>, update: impl IntoVar<WidgetFn<D>>) -> impl UiNode {
    let data = data.into_var();
    let update = update.into_var();

    match_node(NilUiNode.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&data).sub_var(&update);
            *c.child() = update.get()(data.get());
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = NilUiNode.boxed();
        }
        UiNodeOp::Update { .. } => {
            if data.is_new() || update.is_new() {
                c.child().deinit();
                *c.child() = update.get()(data.get());
                c.child().init();
                c.delegated();
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

/// Node that presents `data` using `update` if data is available, otherwise presents nil.
///
/// This behaves like [`presenter`], but `update` is not called if `data` is `None`.
///
/// Note that this node is not a full widget, it can be used as part of an widget without adding to the info tree.
pub fn presenter_opt<D: VarValue>(data: impl IntoVar<Option<D>>, update: impl IntoVar<WidgetFn<D>>) -> impl UiNode {
    let data = data.into_var();
    let update = update.into_var();

    match_node(NilUiNode.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&data).sub_var(&update);
            if let Some(data) = data.get() {
                *c.child() = update.get()(data);
            }
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = NilUiNode.boxed();
        }
        UiNodeOp::Update { .. } => {
            if data.is_new() || update.is_new() {
                if let Some(data) = data.get() {
                    c.child().deinit();
                    *c.child() = update.get()(data);
                    c.child().init();
                    c.delegated();
                    WIDGET.update_info().layout().render();
                } else if c.child().actual_type_id() != TypeId::of::<NilUiNode>() {
                    c.child().deinit();
                    *c.child() = NilUiNode.boxed();
                    c.delegated();
                    WIDGET.update_info().layout().render();
                }
            }
        }
        _ => {}
    })
}

/// Arguments for the [`View!`] widget.
///
/// [`View!`]: struct@View
#[derive(Clone)]
pub struct ViewArgs<D: VarValue> {
    data: BoxedVar<D>,
    replace: Arc<Mutex<Option<BoxedUiNode>>>,
    is_nil: bool,
}
impl<D: VarValue> ViewArgs<D> {
    /// Reference the data variable.
    ///
    /// Can be cloned and used in the [`set_view`] to avoid rebuilding the info tree for every update.
    ///
    /// [`set_view`]: Self::set_view
    pub fn data(&self) -> &BoxedVar<D> {
        &self.data
    }

    /// If the current child is [`NilUiNode`];
    pub fn is_nil(&self) -> bool {
        self.is_nil
    }

    /// Get the current data value if [`is_nil`] or [`data`] is new.
    ///
    /// [`is_nil`]: Self::is_nil
    /// [`data`]: Self::data
    pub fn get_new(&self) -> Option<D> {
        if self.is_nil {
            Some(self.data.get())
        } else {
            self.data.get_new()
        }
    }

    /// Replace the child node.
    ///
    /// If set the current child node will be deinited and dropped.
    pub fn set_view(&self, new_child: impl UiNode) {
        *self.replace.lock() = Some(new_child.boxed());
    }

    /// Set the view to [`NilUiNode`].
    pub fn unset_view(&self) {
        self.set_view(NilUiNode)
    }
}

/// Dynamically presents a data variable.
///
/// The `update` widget handler is used to generate the view UI from the `data`, it is called on init and
/// every time `data` or `update` are new. The view is set by calling [`ViewArgs::set_view`] in the widget function
/// args, note that the data variable is available in [`ViewArgs::data`], a good view will bind to the variable
/// to support some changes, only replacing the UI for major changes.
///
/// Note that this node is not a full widget, it can be used as part of an widget without adding to the info tree.
///
/// # Examples
///
/// View using the shorthand syntax:
///
/// ```
/// # macro_rules! _demo { () => {
/// use zero_ui::prelude::*;
///
/// fn countdown(n: impl IntoVar<usize>) -> impl UiNode {
///     View!(::<usize>, n, hn!(|a: &ViewArgs<usize>| {
///         // we generate a new view on the first call or when the data has changed to zero.
///         if a.is_nil() || a.data().get_new() == Some(0) {
///             a.set_view(if a.data().get() > 0 {
///                 // countdown view
///                 Text! {
///                     font_size = 28;
///                     // bind data, same view will be used for all n > 0 values.
///                     txt = a.data().map_to_text();
///                 }
///             } else {
///                 // finished view
///                 Text! {
///                     font_color = rgb(0, 128, 0);
///                     font_size = 18;
///                     txt = "Congratulations!";
///                 }
///             });
///         }
///     }))
/// }
/// # }}
/// ```
///
/// You can also use the normal widget syntax and set the `view` property.
///
/// ```
/// # macro_rules! _demo { () => {
/// use zero_ui::prelude::*;
///
/// fn countdown(n: impl IntoVar<usize>) -> impl UiNode {
///     View! {
///         view::<usize> = {
///             data: n,
///             update: hn!(|a: &ViewArgs<usize>| { }),
///         };
///         background_color = colors::GRAY;
///     }
/// }
/// # }}
/// ```
#[widget($crate::View {
    (::<$T:ty>, $data:expr, $update:expr $(,)?) => {
        view::<$T> = {
            data: $data,
            update: $update,
        };
    }
})]
pub struct View(WidgetBase);
impl View {
    widget_impl! {
        /// Spacing around content, inside the border.
        pub zero_ui_wgt_container::padding(padding: impl IntoVar<SideOffsets>);

        /// Content alignment.
        pub zero_ui_wgt_container::child_align(align: impl IntoVar<Align>);

        /// Content overflow clipping.
        pub zero_ui_wgt::clip_to_bounds(clip: impl IntoVar<bool>);
    }
}

/// The view generator.
///
/// See [`View!`] for more details.
///
/// [`View!`]: struct@View
#[property(CHILD, widget_impl(View))]
pub fn view<D: VarValue>(child: impl UiNode, data: impl IntoVar<D>, update: impl WidgetHandler<ViewArgs<D>>) -> impl UiNode {
    let data = data.into_var().boxed();
    let mut update = update.cfg_boxed();
    let replace = Arc::new(Mutex::new(None));

    match_node(child.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&data);
            update.event(&ViewArgs {
                data: data.clone(),
                replace: replace.clone(),
                is_nil: true,
            });
            if let Some(child) = replace.lock().take() {
                *c.child() = child;
            }
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = NilUiNode.boxed();
        }
        UiNodeOp::Update { .. } => {
            if data.is_new() {
                update.event(&ViewArgs {
                    data: data.clone(),
                    replace: replace.clone(),
                    is_nil: c.child().actual_type_id() == TypeId::of::<NilUiNode>(),
                });
            }

            update.update();

            if let Some(child) = replace.lock().take() {
                // skip update if nil -> nil, otherwise updates
                if c.child().actual_type_id() != TypeId::of::<NilUiNode>() || child.actual_type_id() != TypeId::of::<NilUiNode>() {
                    c.child().deinit();
                    *c.child() = child;
                    c.child().init();
                    c.delegated();
                    WIDGET.update_info().layout().render();
                }
            }
        }
        _ => {}
    })
}

/// Node that presents `list` using `element_fn` for each new element.
///
/// The node's children is always the result of `element_fn` called for each element in the `list`, removed
/// elements are deinited, inserted elements get a call to `element_fn` and are inserted in the same position
/// on the list.
pub fn list_presenter<D: VarValue>(list: impl IntoVar<ObservableVec<D>>, element_fn: impl IntoVar<WidgetFn<D>>) -> impl UiNodeList {
    ListPresenter {
        list: list.into_var(),
        element_fn: element_fn.into_var(),
        view: vec![],
        _e: std::marker::PhantomData,
    }
}

struct ListPresenter<D: VarValue, L: Var<ObservableVec<D>>, E: Var<WidgetFn<D>>> {
    list: L,
    element_fn: E,
    view: Vec<BoxedUiNode>,
    _e: std::marker::PhantomData<D>,
}

impl<D, L, E> UiNodeList for ListPresenter<D, L, E>
where
    D: VarValue,
    L: Var<ObservableVec<D>>,
    E: Var<WidgetFn<D>>,
{
    fn with_node<R, F>(&mut self, index: usize, f: F) -> R
    where
        F: FnOnce(&mut BoxedUiNode) -> R,
    {
        self.view.with_node(index, f)
    }

    fn for_each<F>(&mut self, f: F)
    where
        F: FnMut(usize, &mut BoxedUiNode),
    {
        self.view.for_each(f)
    }

    fn par_each<F>(&mut self, f: F)
    where
        F: Fn(usize, &mut BoxedUiNode) + Send + Sync,
    {
        self.view.par_each(f)
    }

    fn par_fold_reduce<T, I, F, R>(&mut self, identity: I, fold: F, reduce: R) -> T
    where
        T: Send + 'static,
        I: Fn() -> T + Send + Sync,
        F: Fn(T, usize, &mut BoxedUiNode) -> T + Send + Sync,
        R: Fn(T, T) -> T + Send + Sync,
    {
        self.view.par_fold_reduce(identity, fold, reduce)
    }

    fn len(&self) -> usize {
        self.view.len()
    }

    fn boxed(self) -> BoxedUiNodeList {
        Box::new(self)
    }

    fn drain_into(&mut self, vec: &mut Vec<BoxedUiNode>) {
        self.view.drain_into(vec);
        tracing::warn!("drained `list_presenter`, now out of sync with data");
    }

    fn init_all(&mut self) {
        debug_assert!(self.view.is_empty());
        self.view.clear();

        WIDGET.sub_var(&self.list).sub_var(&self.element_fn);

        let e_fn = self.element_fn.get();
        self.list.with(|l| {
            for el in l.iter() {
                let child = e_fn(el.clone());
                self.view.push(child);
            }
        });

        self.view.init_all();
    }

    fn deinit_all(&mut self) {
        self.view.deinit_all();
        self.view.clear();
    }

    fn update_all(&mut self, updates: &WidgetUpdates, observer: &mut dyn UiNodeListObserver) {
        let mut need_reset = self.element_fn.is_new();

        let is_new = self
            .list
            .with_new(|l| {
                need_reset |= l.changes().is_empty() || l.changes() == [VecChange::Clear];

                if need_reset {
                    return;
                }

                // update before new items to avoid update before init.
                self.view.update_all(updates, observer);

                let e_fn = self.element_fn.get();

                for change in l.changes() {
                    match change {
                        VecChange::Insert { index, count } => {
                            for i in *index..(*index + count) {
                                let mut el = e_fn(l[i].clone());
                                el.init();
                                self.view.insert(i, el);
                                observer.inserted(i);
                            }
                        }
                        VecChange::Remove { index, count } => {
                            let mut count = *count;
                            let index = *index;
                            while count > 0 {
                                count -= 1;

                                let mut el = self.view.remove(index);
                                el.deinit();
                                observer.removed(index);
                            }
                        }
                        VecChange::Move { from_index, to_index } => {
                            let el = self.view.remove(*from_index);
                            self.view.insert(*to_index, el);
                            observer.moved(*from_index, *to_index);
                        }
                        VecChange::Clear => unreachable!(),
                    }
                }
            })
            .is_some();

        if !need_reset && !is_new && self.list.with(|l| l.len() != self.view.len()) {
            need_reset = true;
        }

        if need_reset {
            self.view.deinit_all();
            self.view.clear();

            let e_fn = self.element_fn.get();
            self.list.with(|l| {
                for el in l.iter() {
                    let child = e_fn(el.clone());
                    self.view.push(child);
                }
            });

            self.view.init_all();
        } else if !is_new {
            self.view.update_all(updates, observer);
        }
    }
}
