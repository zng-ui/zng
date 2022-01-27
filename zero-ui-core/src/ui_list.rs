use std::cell::Cell;

use crate::{
    context::{InfoContext, LayoutContext, RenderContext, StateMap, WidgetContext},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate, SpatialFrameId},
    units::{AvailableSize, PxPoint, PxRect, PxSize},
    widget_base::Visibility,
    widget_info::{BoundsInfo, WidgetInfoBuilder, WidgetLayout, WidgetSubscriptions},
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

    /// Calls [`UiNode::measure`] in all widgets in the list, sequentially.
    ///
    /// # `available_size`
    ///
    /// The `available_size` parameter is a function that takes a widget index and the `ctx` and returns
    /// the available size for the widget.
    ///
    /// The index is zero-based, `0` is the first widget, `len() - 1` is the last.
    ///
    /// # `desired_size`
    ///
    /// The `desired_size` parameter is a function is called with the widget index, the widget measured size and the `ctx`.
    ///
    /// This is how you get the widget desired size.
    fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, available_size: A, desired_size: D)
    where
        A: FnMut(usize, &mut LayoutContext) -> AvailableSize,
        D: FnMut(usize, PxSize, &mut LayoutContext);

    /// Calls [`UiNode::measure`] in only the `index` widget.
    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize;

    /// Calls [`UiNode::arrange`] in all widgets in the list, sequentially.
    ///
    /// # `final_rect`
    ///
    /// The `final_rect` parameter is a function that takes a widget index and the `ctx` and returns the
    /// final size and widget offset for the indexed widget.
    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_rect: F)
    where
        F: FnMut(usize, &mut LayoutContext) -> PxRect;

    /// Calls [`UiNode::arrange`] in only the `index` widget.
    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize);

    /// Calls [`UiNode::render`] in all widgets in the list, sequentially. Uses a reference frame
    /// to offset each widget.
    ///
    /// # `origin`
    ///
    /// The `origin` parameter is a function that takes a widget index and returns the offset that must
    /// be used to render it.
    fn render_all<O>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize) -> PxPoint;

    /// Calls [`UiNode::render`] in only the `index` widget.
    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Calls [`UiNode::render_update`] in all widgets in the list, sequentially.
    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    /// Calls [`UiNode::render_update`] in only the `index` widget.
    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate);
}

/// All [`Widget`] accessible *info*.
pub struct WidgetFilterArgs<'a> {
    /// The [`Widget::outer_bounds`].
    pub outer_bounds: &'a BoundsInfo,
    /// The [`Widget::inner_bounds`].
    pub inner_bounds: &'a BoundsInfo,
    /// The [`Widget::visibility`].
    pub visibility: Visibility,
    /// The [`Widget::state`].
    pub state: &'a StateMap,
}
impl<'a> WidgetFilterArgs<'a> {
    /// Copy or borrow all info.
    pub fn get(list: &'a impl WidgetList, index: usize) -> Self {
        WidgetFilterArgs {
            outer_bounds: list.widget_outer_bounds(index),
            inner_bounds: list.widget_inner_bounds(index),
            visibility: list.widget_visibility(index),
            state: list.widget_state(index),
        }
    }
}

/// A generic view over a list of [`Widget`] UI nodes.
///
/// Layout widgets should use this to abstract the children list type if possible.
pub trait WidgetList: UiNodeList {
    /// Count widgets that pass filter using the widget state.
    fn count<F>(&self, mut filter: F) -> usize
    where
        F: FnMut(usize, WidgetFilterArgs) -> bool,
        Self: Sized,
    {
        let mut count = 0;
        for i in 0..self.len() {
            if filter(i, WidgetFilterArgs::get(self, i)) {
                count += 1;
            }
        }
        count
    }

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

    /// Gets the last arranged outer bounds of the widget at the `index`.
    ///
    /// See [`Widget::outer_bounds`] for more details.
    fn widget_outer_bounds(&self, index: usize) -> &BoundsInfo;
    /// Gets the last arranged inner bounds of the widget at the `index`.
    ///
    /// See [`Widget::inner_bounds`] for more details.
    fn widget_inner_bounds(&self, index: usize) -> &BoundsInfo;

    /// Gets the last rendered visibility of the widget at the `index`.
    ///
    /// See [`Widget::visibility`] for more details.
    fn widget_visibility(&self, index: usize) -> Visibility;

    /// Calls [`UiNode::render`] in all widgets in the list that have an origin, sequentially. Uses a reference frame
    /// to offset each widget.
    ///
    /// # `origin`
    ///
    /// The `origin` parameter is a function that takes a widget index, size and state and returns the offset that must
    /// be used to render it, if it must be rendered.
    fn render_filtered<O>(&self, origin: O, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        O: FnMut(usize, WidgetFilterArgs) -> Option<PxPoint>;
}

/// Initialize an optimized [`WidgetList`].
///
/// The list type is opaque (`impl WidgetList`), and it changes depending on if the build is release or debug.
/// In both cases the list cannot be modified and the only methods available are provided by [`WidgetList`].
///
/// This is the recommended way to declare the contents of layout panel.
///
/// # Example
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
        $crate::opaque_widgets($crate::WidgetList0)
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
/// # Example
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
        $crate::opaque_nodes($crate::UiNodeList0)
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
        $crate::opaque_nodes($crate::node_vec![
            $($node),+
        ])
    };
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! __widgets {
    ($($widget:expr),+ $(,)?) => {
        $crate::opaque_widgets($crate::widget_vec![
            $($widget),+
        ])
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __nodes {
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr, $w7:expr, $($w_rest:expr),+ $(,)?) => {
        $crate::opaque_nodes({
            let w8 = $crate::__nodes!($w0, $w1, $w2, $w3, $w4, $w5, $w6, $w7);
            $crate::UiNodeList::chain_nodes(w8, $crate::__nodes!($($w_rest),+))
        })
    };
    ($($tt:tt)*) => {
        $crate::opaque_nodes($crate::static_list!($crate::UiNodeList0; $($tt)*))
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! __widgets {
    ($w0:expr, $w1:expr, $w2:expr, $w3:expr, $w4:expr, $w5:expr, $w6:expr, $w7:expr, $($w_rest:expr),+ $(,)?) => {
        $crate::opaque_widgets({
            let w8 = $crate::__widgets!($w0, $w1, $w2, $w3, $w4, $w5, $w6, $w7);
            $crate::WidgetList::chain(w8, $crate::__widgets!($($w_rest),+))
        })
    };
    ($($tt:tt)*) => {
        $crate::opaque_widgets($crate::static_list!($crate::WidgetList0; $($tt)*))
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

#[derive(Default)]
struct SpatialIdGen(Cell<Option<SpatialFrameId>>);
impl SpatialIdGen {
    pub fn get(&self) -> SpatialFrameId {
        if let Some(id) = self.0.get() {
            id
        } else {
            let id = SpatialFrameId::new_unique();
            self.0.set(Some(id));
            id
        }
    }
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
