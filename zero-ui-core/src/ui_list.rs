//! UI node and widget lists abstraction.

use crate::{
    context::{InfoContext, LayoutContext, RenderContext, StateMap, WidgetContext},
    event::EventUpdateArgs,
    impl_from_and_into_var,
    render::{FrameBuilder, FrameUpdate},
    units::{AvailableSize, PxSize},
    widget_info::{
        WidgetBorderInfo, WidgetInfoBuilder, WidgetLayout, WidgetLayoutInfo, WidgetRenderInfo, WidgetSubscriptions, WidgetTransformBuilder,
    },
    WidgetId,
};
#[allow(unused)] // used in docs.
use crate::{UiNode, Widget};

mod vec;
pub use vec::*;

mod sorted_vec;
pub use sorted_vec::*;

mod chain;
pub use chain::*;

mod tuples;
pub use tuples::*;

mod z_sorted;
pub use z_sorted::*;

/// A generic view over a list of [`UiNode`] items.
pub trait UiNodeList: 'static {
    /// Returns `true` if the list length and position of widgets does not change.
    fn is_fixed(&self) -> bool;

    /// Number of items in the list.
    fn len(&self) -> usize;

    /// If the list is empty.
    fn is_empty(&self) -> bool;

    /// Boxes all items.
    fn boxed_all(self) -> UiNodeVec;

    /// Creates a new list that consists of this list followed by the `other` list of nodes.
    fn chain_nodes<U>(self, other: U) -> UiNodeListChain<Self, U>
    where
        Self: Sized,
        U: UiNodeList,
    {
        UiNodeListChain(self, other)
    }

    /// Calls [`UiNode::info`] in all widgets in the list, sequentially.
    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);

    /// Calls [`UiNode::info`] in only the `index` widget.
    fn widget_info(&self, index: usize, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder);

    /// Calls [`UiNode::subscriptions`] in all widgets in the list, sequentially.
    fn subscriptions_all(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions);

    /// Calls [`UiNode::subscriptions`] in the `index` widget.
    fn widget_subscriptions(&self, index: usize, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions);

    /// Calls [`UiNode::init`] in all widgets in the list, sequentially.
    fn init_all(&mut self, ctx: &mut WidgetContext);

    /// Calls [`UiNode::deinit`] in all widgets in the list, sequentially.
    fn deinit_all(&mut self, ctx: &mut WidgetContext);

    /// Calls [`UiNode::update`] in all widgets in the list, sequentially.
    ///
    /// The `observer` can be used to monitor widget insertion/removal if this list is not [fixed]. Use `&mut ()` to ignore changes and
    /// `&mut bool` to simply get a flag that indicates any change has happened, see [`UiListObserver`] for more details. Note that
    /// an info and subscriptions rebuild is requested by the list implementer, the inserted/removed widgets are also (de)initialized by
    /// the list implementer, the observer is for updating custom state and requesting layout when required.
    ///
    /// [fixed]: UiNodeList::is_fixed
    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O);

    /// Calls [`UiNode::event`] in all widgets in the list, sequentially.
    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU);

    /// Calls [`UiNode::layout`] in all widgets in the list, sequentially.
    ///
    /// # `widget_config`
    ///
    /// The `widget_config` parameter is a function that must return customs layout context configs to apply for the call of layout for
    /// each child.
    ///
    /// # `final_size`
    ///
    /// The `final_size` parameter is a function is called with the widget measured size and outer transform builder.
    fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, widget_cfg: C, final_size: D)
    where
        C: FnMut(&mut LayoutContext, ConfigContextArgs) -> LayoutContextConfig,
        D: FnMut(&mut LayoutContext, FinalSizeArgs);

    /// Calls [`UiNode::layout`] in only the `index` item.
    fn widget_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize;

    /// Calls [`UiNode::render`] in all widgets in the list, sequentially.
    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Calls [`UiNode::render`] in only the `index` item.
    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Calls [`UiNode::render_update`] in all widgets in the list, sequentially.
    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    /// Calls [`UiNode::render_update`] in only the `index` item.
    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate);
}

/// Arguments for the closure in [`UiNodeList::layout_all`] that provides the available size for an widget.
pub struct ConfigContextArgs<'a> {
    /// The widget/node index in the list.
    pub index: usize,

    /// Mutable reference to the widget state.
    ///
    /// Is `None` for arrange in UI node lists.
    pub state: Option<&'a mut StateMap>,
}

/// Parameters to set on a layout context for calling layout in a widget item in a [`UiNode::layout_all`] operation.
#[derive(Default, Debug, Clone)]
pub struct LayoutContextConfig {
    /// Available size.
    pub available_size: Option<AvailableSize>,
}
impl LayoutContextConfig {
    /// New default.
    pub fn none() -> Self {
        Self::default()
    }

    /// Call `f` with the layout context configured.
    pub fn with<R>(&self, ctx: &mut LayoutContext, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        if let Some(av) = self.available_size {
            ctx.with_available_size(av, f)
        } else {
            f(ctx)
        }
    }
}
impl_from_and_into_var! {
    fn from(available_size: AvailableSize) -> LayoutContextConfig {
        LayoutContextConfig {
            available_size: Some(available_size)
        }
    }
}

/// Arguments for the closure in [`UiNodeList::layout_all`] that received the widget desired size.
pub struct FinalSizeArgs<'a> {
    /// The widget/node index in the list.
    pub index: usize,

    /// Mutable reference to the widget state.
    ///
    /// Is `None` for layout in UI node lists.
    pub state: Option<&'a mut StateMap>,

    /// The widget outer size.
    pub size: PxSize,

    /// The widget outer transform builder.
    ///
    /// Is `None` for layout in UI node lists.
    pub transform: Option<&'a mut WidgetTransformBuilder>,
}

/// All [`Widget`] accessible *info*.
pub struct WidgetFilterArgs<'a> {
    /// The widget index in the list.
    pub index: usize,

    /// The [`Widget::outer_info`].
    pub outer_info: &'a WidgetLayoutInfo,
    /// The [`Widget::inner_info`].
    pub inner_info: &'a WidgetLayoutInfo,
    /// The [`Widget::border_info`].
    pub border_info: &'a WidgetBorderInfo,
    /// The [`Widget::render_info`].
    pub render_info: &'a WidgetRenderInfo,
    /// The [`Widget::state`].
    pub state: &'a StateMap,
}
impl<'a> WidgetFilterArgs<'a> {
    /// Copy or borrow all info from a widget list and index.
    pub fn get(list: &'a impl WidgetList, index: usize) -> Self {
        WidgetFilterArgs {
            index,
            outer_info: list.widget_outer_info(index),
            inner_info: list.widget_inner_info(index),
            border_info: list.widget_border_info(index),
            render_info: list.widget_render_info(index),
            state: list.widget_state(index),
        }
    }

    /// Copy or borrow all info from a widget reference.
    pub fn new(index: usize, widget: &'a impl Widget) -> Self {
        WidgetFilterArgs {
            index,
            outer_info: widget.outer_info(),
            inner_info: widget.inner_info(),
            border_info: widget.border_info(),
            render_info: widget.render_info(),
            state: widget.state(),
        }
    }
}

/// A generic view over a list of [`Widget`] UI nodes.
///
/// Layout widgets should use this to abstract the children list type if possible.
pub trait WidgetList: UiNodeList {
    /// Count widgets that pass filter using the widget state.
    fn count<F>(&self, filter: F) -> usize
    where
        F: FnMut(WidgetFilterArgs) -> bool;

    /// Boxes all widgets and moved then to a [`WidgetVec`].
    fn boxed_widget_all(self) -> WidgetVec;

    /// Creates a new list that consists of this list followed by the `other` list.
    fn chain<U>(self, other: U) -> WidgetListChain<Self, U>
    where
        Self: Sized,
        U: WidgetList,
    {
        WidgetListChain(self, other)
    }

    /// Gets the id of the widget at the `index`.
    ///
    /// The index is zero-based.
    fn widget_id(&self, index: usize) -> WidgetId;

    /// Reference the state of the widget at the `index`.
    fn widget_state(&self, index: usize) -> &StateMap;

    /// Exclusive reference the state of the widget at the `index`.
    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap;

    /// Gets the outer bounds layout info of the widget at the `index`.
    ///
    /// See [`Widget::outer_info`] for more details.
    fn widget_outer_info(&self, index: usize) -> &WidgetLayoutInfo;
    /// Gets the inner bounds layout info of the widget at the `index`.
    ///
    /// See [`Widget::inner_info`] for more details.
    fn widget_inner_info(&self, index: usize) -> &WidgetLayoutInfo;

    /// Gets the border and corners info of the widget at the `index`.
    ///
    /// See [`Widget::border_info`] for more details.
    fn widget_border_info(&self, index: usize) -> &WidgetBorderInfo;

    /// Gets the render info the widget at the `index`.
    ///
    /// See [`Widget::render_info`] for more details.
    fn widget_render_info(&self, index: usize) -> &WidgetRenderInfo;

    /// Calls [`UiNode::render`] in all widgets allowed by a `filter`, skips rendering the rest.
    fn render_filtered<F>(&self, filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(WidgetFilterArgs) -> bool;

    /// Calls [`WidgetLayout::with_outer`] in only the `index` widget.
    fn widget_outer<F>(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout, f: F)
    where
        F: FnOnce(&mut LayoutContext, FinalSizeArgs);
}

/// Initialize an optimized [`WidgetList`].
///
/// The list type is opaque (`impl WidgetList`), and it changes depending on if the build is release or debug.
/// In both cases the list cannot be modified and the only methods available are provided by [`WidgetList`].
///
/// This is the recommended way to declare the contents of layout panel.
///
/// # Examples
///
/// ```todo
/// # use zero_ui_core::{widgets, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { implicit_base::new(NilUiNode, WidgetId::new_unique())  };
/// # use text as foo;
/// # use text as bar;
/// let items = widgets![
///     foo("Hello "),
///     bar("World!")
/// ];
/// ```
#[macro_export]
macro_rules! widgets {
    () => {
        $crate::ui_list::opaque_widgets($crate::ui_list::WidgetList0)
    };
    ($($widget:expr),+ $(,)?) => {
        $crate::__widgets!{ $($widget),+ }
    }
}
#[doc(inline)]
pub use crate::widgets;

/// Initialize an optimized [`UiNodeList`].
///
/// The list type is opaque (`impl UiNodeList`), and it changes depending on if the build is release or debug.
/// In both cases the list cannot be modified and the only methods available are provided by [`UiNodeList`].
///
/// This is the recommended way to declare the contents of a property that takes multiple [`UiNode`](crate::UiNode) implementers.
///
/// # Examples
///
/// ```todo
/// # use zero_ui_core::{nodes, UiNode, Widget, WidgetId, NilUiNode};
/// # use zero_ui_core::widget_base::*;
/// # fn text(fake: &str) -> impl Widget { implicit_base::new(NilUiNode, WidgetId::new_unique())  };
/// # use text as foo;
/// # use text as bar;
/// let items = widgets![
///     foo("Hello "),
///     bar("World!")
/// ];
/// ```
#[macro_export]
macro_rules! nodes {
    () => {
        $crate::ui_list::opaque_nodes($crate::ui_list::UiNodeList0)
    };
    ($($node:expr),+ $(,)?) => {
        $crate::__nodes!{ $($node),+ }
    }
}
#[doc(inline)]
pub use crate::nodes;

#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! __nodes {
    ($($node:expr),+ $(,)?) => {
        $crate::ui_list::opaque_nodes($crate::node_vec![
            $($node),+
        ])
    };
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! __widgets {
    ($($widget:expr),+ $(,)?) => {
        $crate::ui_list::opaque_widgets($crate::widget_vec![
            $($widget),+
        ])
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __nodes {
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr, $w7:expr, $($w_rest:expr),+ $(,)?) => {
        $crate::ui_list::opaque_nodes({
            let w8 = $crate::__nodes!($w0, $w1, $w2, $w3, $w4, $w5, $w6, $w7);
            $crate::UiNodeList::chain_nodes(w8, $crate::__nodes!($($w_rest),+))
        })
    };
    ($($tt:tt)*) => {
        $crate::ui_list::opaque_nodes($crate::static_list!($crate::ui_list::UiNodeList0; $($tt)*))
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __widgets {
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr, $w7:expr, $($w_rest:expr),+ $(,)?) => {
        $crate::ui_list::opaque_widgets({
            let w8 = $crate::__widgets!($w0, $w1, $w2, $w3, $w4, $w5, $w6, $w7);
            $crate::WidgetList::chain(w8, $crate::__widgets!($($w_rest),+))
        })
    };
    ($($tt:tt)*) => {
        $crate::ui_list::opaque_widgets($crate::static_list!($crate::ui_list::WidgetList0; $($tt)*))
    };
}

#[doc(hidden)]
pub fn opaque_widgets(widgets: impl WidgetList) -> impl WidgetList {
    widgets
}

#[doc(hidden)]
pub fn opaque_nodes(nodes: impl UiNodeList) -> impl UiNodeList {
    nodes
}

/// Represents an [`UiNodeList::update_all`] observer that can be used to monitor widget insertion, removal and re-order.
///
/// All indexes are in the context of the previous changes, if you are maintaining a *mirror* vector simply using the
/// [`Vec::insert`] and [`Vec::remove`] commands in the same order as they are received should keep the vector in sync.
///
/// This trait is implemented for `()`, to **not** observe simply pass on a `&mut ()`.
///
/// This trait is implemented for [`bool`], if any change happens the flag is set to `true`.
pub trait UiListObserver {
    /// Large changes made to the list.
    fn reseted(&mut self);
    /// Widget inserted at the `index`.
    fn inserted(&mut self, index: usize);
    /// Widget removed from the `index`.
    fn removed(&mut self, index: usize);
    /// Widget removed and re-inserted.
    fn moved(&mut self, removed_index: usize, inserted_index: usize);
}
/// Does nothing.
impl UiListObserver for () {
    fn reseted(&mut self) {}

    fn inserted(&mut self, _: usize) {}

    fn removed(&mut self, _: usize) {}

    fn moved(&mut self, _: usize, _: usize) {}
}
/// Sets to `true` for any change.
impl UiListObserver for bool {
    fn reseted(&mut self) {
        *self = true;
    }

    fn inserted(&mut self, _: usize) {
        *self = true;
    }

    fn removed(&mut self, _: usize) {
        *self = true;
    }

    fn moved(&mut self, _: usize, _: usize) {
        *self = true;
    }
}

/// Represents an [`UiListObserver`] that applies an offset to all indexes.
///
/// This type is useful for implementing [`UiNodeList`] that are composed of other lists.
pub struct OffsetUiListObserver<'o, O: UiListObserver>(pub usize, pub &'o mut O);
impl<'o, O: UiListObserver> UiListObserver for OffsetUiListObserver<'o, O> {
    fn reseted(&mut self) {
        self.1.reseted()
    }

    fn inserted(&mut self, index: usize) {
        self.1.inserted(index + self.0)
    }

    fn removed(&mut self, index: usize) {
        self.1.removed(index + self.0)
    }

    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
        self.1.moved(removed_index + self.0, inserted_index + self.0)
    }
}

impl<'o, 't, O: UiListObserver, T: UiListObserver> UiListObserver for (&mut O, &mut T) {
    fn reseted(&mut self) {
        self.0.reseted();
        self.1.reseted();
    }

    fn inserted(&mut self, index: usize) {
        self.0.inserted(index);
        self.1.inserted(index);
    }

    fn removed(&mut self, index: usize) {
        self.0.removed(index);
        self.1.removed(index);
    }

    fn moved(&mut self, removed_index: usize, inserted_index: usize) {
        self.0.moved(removed_index, inserted_index);
        self.1.moved(removed_index, inserted_index);
    }
}
