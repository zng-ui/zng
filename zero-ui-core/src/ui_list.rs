//! UI node and widget lists abstraction.

use crate::{
    context::{InfoContext, LayoutContext, MeasureContext, RenderContext, StateMap, WidgetContext},
    event::EventUpdateArgs,
    render::{FrameBuilder, FrameUpdate},
    units::{PxConstrains2d, PxSize},
    widget_info::{
        WidgetBorderInfo, WidgetBoundsInfo, WidgetInfoBuilder, WidgetLayout, WidgetLayoutTranslation, WidgetRenderInfo, WidgetSubscriptions,
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

    /// Calls [`UiNode::subscriptions`] in all widgets in the list, sequentially.
    fn subscriptions_all(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions);

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
    /// Note that you can also measure specific children with [`item_measure`].
    ///
    /// # Pre-Measure
    ///
    /// The `pre_measure` closure is called just before the measure call for each child, you can use the [`PreMeasureArgs`]
    /// to configure the constrains used to measure the child.
    ///
    /// # Pos-Measure
    ///
    /// The `pos_measure` closure is called after the measure call for each child, you can see the measured size in the [`PosMeasureArgs`].
    ///
    /// [`item_measure`]: Self::item_measure
    fn measure_all<C, D>(&self, ctx: &mut MeasureContext, pre_measure: C, pos_measure: D)
    where
        C: FnMut(&mut MeasureContext, &mut PreMeasureArgs),
        D: FnMut(&mut MeasureContext, PosMeasureArgs);

    /// Calls [`UiNode::measure`] in only the `index` node or widget.
    fn item_measure(&self, index: usize, ctx: &mut MeasureContext) -> PxSize;

    /// Calls [`UiNode::layout`] in all widgets in the list, sequentially.
    ///
    /// Note that you can also layout specific children with [`item_layout`], and if the list is a full [`WidgetList`]
    /// you can use the [`item_outer`] method to update each child transform without causing a second layout pass.
    ///
    /// # Pre-Layout
    ///
    /// The `pre_layout` closure is called just before the layout call for each child, inside it the [`WidgetLayout`] already
    /// affects the child, you can also use the [`PreLayoutArgs`] to configure the constrains used to layout the child.
    ///
    /// # Pos-Layout
    ///
    /// The `pos_layout` closure is called after the layout call for each child, inside it the [`WidgetLayout`] still affects the
    /// child, you can also see the new child size in [`PosLayoutArgs`].
    ///
    /// [`item_layout`]: UiNodeList::item_layout
    /// [`item_outer`]: WidgetList::item_outer
    fn layout_all<C, D>(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout, pre_layout: C, pos_layout: D)
    where
        C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
        D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs);

    /// Calls [`UiNode::layout`] in only the `index` node or widget.
    fn item_layout(&mut self, index: usize, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> PxSize;

    /// Calls [`UiNode::render`] in all widgets in the list, sequentially.
    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Calls [`UiNode::render`] in only the `index` node or widget.
    fn item_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder);

    /// Calls [`UiNode::render_update`] in all widgets in the list, sequentially.
    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate);

    /// Calls [`UiNode::render_update`] in only the `index` node or widget.
    fn item_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate);

    /// Gets the id of the widget at the `index` if the node is a full widget.
    ///
    /// The index is zero-based.
    fn try_item_id(&self, index: usize) -> Option<WidgetId>;

    /// Reference the state of the widget at the `index`.
    fn try_item_state(&self, index: usize) -> Option<&StateMap>;

    /// Exclusive reference the state of the widget at the `index`.
    fn try_item_state_mut(&mut self, index: usize) -> Option<&mut StateMap>;

    /// Gets the bounds layout info of the node at the `index` if it is a full widget.
    ///
    /// See [`Widget::bounds_info`] for more details.
    fn try_item_bounds_info(&self, index: usize) -> Option<&WidgetBoundsInfo>;

    /// Gets the border and corners info from the node at the `index` if it is a full widget.
    ///
    /// See [`Widget::border_info`] for more details.
    fn try_item_border_info(&self, index: usize) -> Option<&WidgetBorderInfo>;

    /// Gets the render info from the node at the `index` if it is a full widget.
    ///
    /// See [`Widget::render_info`] for more details.
    fn try_item_render_info(&self, index: usize) -> Option<&WidgetRenderInfo>;

    /// Calls [`UiNode::render`] in all nodes allowed by a `filter`, skips rendering the rest.
    fn render_node_filtered<F>(&self, filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(UiNodeFilterArgs) -> bool;

    /// Calls [`WidgetLayout::try_with_outer`] in only the `index` node. The `transform` closure only runs if the node
    /// is a full widget.
    fn try_item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> Option<R>
    where
        F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R;

    /// Calls [`WidgetLayout::try_with_outer`] in all nodes on the list. The `transform` closures only runs for the nodes
    /// that are full widgets.
    fn try_outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, transform: F)
    where
        F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs);

    /// Count nodes that pass the `filter`.
    fn count_nodes<F>(&self, filter: F) -> usize
    where
        F: FnMut(UiNodeFilterArgs) -> bool;
}

/// Arguments for the closure in [`UiNodeList::measure_all`] that runs before each child is measured.
pub struct PreMeasureArgs<'a> {
    /// The widget/node index in the list.
    pub index: usize,

    /// Reference to the widget state.
    ///
    /// Can be `None` in [`UiNodeList`] for nodes that are not full widgets.
    pub state: Option<&'a StateMap>,

    /// Constrains overwrite just for this child.
    pub constrains: Option<PxConstrains2d>,
}
impl<'a> PreMeasureArgs<'a> {
    /// New args for item.
    pub fn new(index: usize, state: Option<&'a StateMap>) -> Self {
        PreMeasureArgs {
            index,
            state,
            constrains: None,
        }
    }
}

/// Arguments for the closure in [`UiNodeList::measure_all`] that runs after each child is measured.
pub struct PosMeasureArgs<'a> {
    /// The widget/node index in the list.
    pub index: usize,

    /// Reference to the widget state.
    ///
    /// Can be `None` in [`UiNodeList`] for nodes that are not full widgets.
    pub state: Option<&'a StateMap>,

    /// The measured size.
    pub size: PxSize,
}
impl<'a> PosMeasureArgs<'a> {
    /// New args for item.
    pub fn new(index: usize, state: Option<&'a StateMap>, size: PxSize) -> Self {
        PosMeasureArgs { index, state, size }
    }
}

/// Arguments for the closure in [`UiNodeList::layout_all`] that runs before each child is layout.
pub struct PreLayoutArgs<'a> {
    /// The widget/node index in the list.
    pub index: usize,

    /// Mutable reference to the widget state.
    ///
    /// Can be `None` in [`UiNodeList`] for nodes that are not full widgets.
    pub state: Option<&'a mut StateMap>,

    /// Constrains overwrite just for this child.
    pub constrains: Option<PxConstrains2d>,
}
impl<'a> PreLayoutArgs<'a> {
    /// New args for item.
    pub fn new(index: usize, state: Option<&'a mut StateMap>) -> Self {
        PreLayoutArgs {
            index,
            state,
            constrains: None,
        }
    }
}

/// Arguments for the closure in [`UiNodeList::layout_all`] that runs after each child is layout.
pub struct PosLayoutArgs<'a> {
    /// The widget/node index in the list.
    pub index: usize,

    /// Mutable reference to the widget state.
    ///
    /// Can be `None` in [`UiNodeList`] for nodes that are not full widgets.
    pub state: Option<&'a mut StateMap>,

    /// The updated size.
    pub size: PxSize,
}
impl<'a> PosLayoutArgs<'a> {
    /// New args for item.
    pub fn new(index: usize, state: Option<&'a mut StateMap>, size: PxSize) -> Self {
        PosLayoutArgs { index, state, size }
    }
}

fn default_widget_list_measure_all<W, C, D>(index: usize, widget: &W, ctx: &mut MeasureContext, mut pre_measure: C, mut pos_measure: D)
where
    W: Widget,
    C: FnMut(&mut MeasureContext, &mut PreMeasureArgs),
    D: FnMut(&mut MeasureContext, PosMeasureArgs),
{
    let mut args = PreMeasureArgs::new(index, Some(widget.state()));
    pre_measure(ctx, &mut args);
    let size = ctx.with_constrains(|c| args.constrains.take().unwrap_or(c), |ctx| widget.measure(ctx));
    pos_measure(ctx, PosMeasureArgs::new(index, Some(widget.state()), size));
}
fn default_ui_node_list_measure_all<N, C, D>(index: usize, node: &N, ctx: &mut MeasureContext, mut pre_measure: C, mut pos_measure: D)
where
    N: UiNode,
    C: FnMut(&mut MeasureContext, &mut PreMeasureArgs),
    D: FnMut(&mut MeasureContext, PosMeasureArgs),
{
    let mut args = PreMeasureArgs::new(index, node.try_state());
    pre_measure(ctx, &mut args);
    let size = ctx.with_constrains(|c| args.constrains.take().unwrap_or(c), |ctx| node.measure(ctx));
    pos_measure(ctx, PosMeasureArgs::new(index, node.try_state(), size));
}

fn default_widget_list_layout_all<W, C, D>(
    index: usize,
    widget: &mut W,
    ctx: &mut LayoutContext,
    wl: &mut WidgetLayout,
    mut pre_layout: C,
    mut pos_layout: D,
) where
    W: Widget,
    C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
    D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs),
{
    let (size, _) = wl.with_child(ctx, |ctx, wl| {
        let mut args = PreLayoutArgs::new(index, Some(widget.state_mut()));
        pre_layout(ctx, wl, &mut args);
        ctx.with_constrains(|c| args.constrains.take().unwrap_or(c), |ctx| widget.layout(ctx, wl))
    });
    pos_layout(ctx, wl, PosLayoutArgs::new(index, Some(widget.state_mut()), size));
}
fn default_ui_node_list_layout_all<N, C, D>(
    index: usize,
    node: &mut N,
    ctx: &mut LayoutContext,
    wl: &mut WidgetLayout,
    mut pre_layout: C,
    mut pos_layout: D,
) where
    N: UiNode,
    C: FnMut(&mut LayoutContext, &mut WidgetLayout, &mut PreLayoutArgs),
    D: FnMut(&mut LayoutContext, &mut WidgetLayout, PosLayoutArgs),
{
    let (size, _) = wl.with_child(ctx, |ctx, wl| {
        let mut args = PreLayoutArgs::new(index, node.try_state_mut());
        pre_layout(ctx, wl, &mut args);
        ctx.with_constrains(|c| args.constrains.take().unwrap_or(c), |ctx| node.layout(ctx, wl))
    });
    pos_layout(ctx, wl, PosLayoutArgs::new(index, node.try_state_mut(), size));
}

/// All [`Widget`] accessible info.
pub struct WidgetFilterArgs<'a> {
    /// The widget index in the list.
    pub index: usize,

    /// The [`Widget::id`].
    pub id: WidgetId,
    /// The [`Widget::bounds_info`].
    pub bounds_info: &'a WidgetBoundsInfo,
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
            id: list.item_id(index),
            bounds_info: list.item_bounds_info(index),
            border_info: list.item_border_info(index),
            render_info: list.item_render_info(index),
            state: list.item_state(index),
        }
    }

    /// Copy or borrow all info from a widget reference.
    pub fn new(index: usize, widget: &'a impl Widget) -> Self {
        WidgetFilterArgs {
            index,
            id: widget.id(),
            bounds_info: widget.bounds_info(),
            border_info: widget.border_info(),
            render_info: widget.render_info(),
            state: widget.state(),
        }
    }
}

/// All [`UiNode`] accessible widget info.
pub struct UiNodeFilterArgs<'a> {
    /// The node index in the list.
    pub index: usize,

    /// The [`UiNode::try_id`].
    pub id: Option<WidgetId>,
    /// The [`UiNode::try_bounds_info`].
    pub bounds_info: Option<&'a WidgetBoundsInfo>,
    /// The [`UiNode::try_border_info`].
    pub border_info: Option<&'a WidgetBorderInfo>,
    /// The [`UiNode::try_render_info`].
    pub render_info: Option<&'a WidgetRenderInfo>,
    /// The [`UiNode::try_state`].
    pub state: Option<&'a StateMap>,
}
impl<'a> UiNodeFilterArgs<'a> {
    /// Copy or borrow all info from a node list and index.
    pub fn get(list: &'a impl UiNodeList, index: usize) -> Self {
        UiNodeFilterArgs {
            index,
            id: list.try_item_id(index),
            bounds_info: list.try_item_bounds_info(index),
            border_info: list.try_item_border_info(index),
            render_info: list.try_item_render_info(index),
            state: list.try_item_state(index),
        }
    }

    /// Copy or borrow all info from a node reference.
    pub fn new(index: usize, node: &'a impl UiNode) -> Self {
        UiNodeFilterArgs {
            index,
            id: node.try_id(),
            bounds_info: node.try_bounds_info(),
            border_info: node.try_border_info(),
            render_info: node.try_render_info(),
            state: node.try_state(),
        }
    }
}

/// A generic view over a list of [`Widget`] UI nodes.
///
/// Layout widgets should use this to abstract the children list type if possible.
pub trait WidgetList: UiNodeList {
    /// Count widgets that pass the `filter`.
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
    fn item_id(&self, index: usize) -> WidgetId;

    /// Reference the state of the widget at the `index`.
    fn item_state(&self, index: usize) -> &StateMap;

    /// Exclusive reference the state of the widget at the `index`.
    fn item_state_mut(&mut self, index: usize) -> &mut StateMap;

    /// Gets the bounds layout info of the widget at the `index`.
    ///
    /// See [`Widget::bounds_info`] for more details.
    fn item_bounds_info(&self, index: usize) -> &WidgetBoundsInfo;

    /// Gets the border and corners info of the widget at the `index`.
    ///
    /// See [`Widget::border_info`] for more details.
    fn item_border_info(&self, index: usize) -> &WidgetBorderInfo;

    /// Gets the render info the widget at the `index`.
    ///
    /// See [`Widget::render_info`] for more details.
    fn item_render_info(&self, index: usize) -> &WidgetRenderInfo;

    /// Calls [`UiNode::render`] in all widgets allowed by a `filter`, skips rendering the rest.
    fn render_filtered<F>(&self, filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(WidgetFilterArgs) -> bool;

    /// Calls [`WidgetLayout::with_outer`] in only the `index` widget.
    fn item_outer<F, R>(&mut self, index: usize, wl: &mut WidgetLayout, keep_previous: bool, transform: F) -> R
    where
        F: FnOnce(&mut WidgetLayoutTranslation, PosLayoutArgs) -> R;

    /// Calls [`WidgetLayout::with_outer`] in all widgets on the list.
    fn outer_all<F>(&mut self, wl: &mut WidgetLayout, keep_previous: bool, transform: F)
    where
        F: FnMut(&mut WidgetLayoutTranslation, PosLayoutArgs);
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
