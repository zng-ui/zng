use std::{cell::Cell, fmt, ops};

use crate::{
    impl_from_and_into_var, impl_ui_node, property, state_key,
    var::{context_var, ContextVarData, IntoVar, Var, Vars},
};

use super::*;

/// Represents a [`WidgetList`] that renders widgets sorted by [`z_index`].
///
/// Unlike the [`SortedWidgetVec`] this list does not change the index of the widgets only the
/// order they are rendered, the widgets are still updated and layout in their logical order.
///
/// Layout panel widget implementers should wrap their input in this type to enable support for the [`z_index`]
/// property, the sorting is very fast and only runs if one of the children sets the z-index.
///
/// Note that [`z_index`] can also be implemented manually using the [`WidgetListZIndexExt`] extension methods.
///
/// [`z_index`]: fn@z_index
/// [`render_filtered`]: WidgetList::render_filtered
pub struct ZSortedWidgetList<W: WidgetList> {
    list: W,

    lookup: Vec<u64>,
    has_non_default_zs: bool,
}
impl<W: WidgetList> ZSortedWidgetList<W> {
    /// Wrap the `list` adding support for the [`z_index`] property.
    ///
    /// Note that by convention only layout panel widget implementers should call this method.
    ///
    /// [`z_index`]: fn@z_index
    pub fn new(list: W) -> Self {
        ZSortedWidgetList {
            list,
            lookup: vec![],
            has_non_default_zs: false,
        }
    }

    fn sort(&mut self) {
        // We pack *z* and *i* as u32s in one u64 then create the sorted lookup table if
        // observed `[I].Z < [I-1].Z`, also records if any `Z != DEFAULT`:
        //
        // Advantages:
        //
        // - Makes `sort_unstable` stable.
        // - Only one alloc needed, just mask out Z after sorting.
        //
        // Disadvantages:
        //
        // - Only supports u32::MAX widgets.
        // - Uses 64-bit indexes in 32-bit builds.

        let len = self.len();
        assert!(len <= u32::MAX as usize);

        let mut prev_z = ZIndex::BACK;
        let mut need_lookup = false;
        let mut z_and_i = Vec::with_capacity(len);
        self.has_non_default_zs = false;

        for i in 0..len {
            let z = self.widget_z_index(i);
            z_and_i.push(((z.0 as u64) << 32) | i as u64);

            need_lookup |= z < prev_z;
            self.has_non_default_zs |= z != ZIndex::DEFAULT;
            prev_z = z;
        }

        if need_lookup {
            z_and_i.sort_unstable();

            for z in &mut z_and_i {
                *z &= u32::MAX as u64;
            }

            self.lookup = z_and_i;
        } else {
            self.lookup = vec![];
        }
    }

    /// Returns the widget Z-Index.
    pub fn widget_z_index(&self, index: usize) -> ZIndex {
        self.list.widget_z_index(index)
    }
}
impl<W: WidgetList> UiNodeList for ZSortedWidgetList<W> {
    fn is_fixed(&self) -> bool {
        self.list.is_fixed()
    }

    fn len(&self) -> usize {
        self.list.len()
    }

    fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    fn boxed_all(self) -> UiNodeVec {
        self.list.boxed_all()
    }

    fn info_all(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.list.info_all(ctx, info)
    }

    fn widget_info(&self, index: usize, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
        self.list.widget_info(index, ctx, info)
    }

    fn subscriptions_all(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.list.subscriptions_all(ctx, subscriptions)
    }

    fn widget_subscriptions(&self, index: usize, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
        self.list.widget_subscriptions(index, ctx, subscriptions)
    }

    fn init_all(&mut self, ctx: &mut WidgetContext) {
        let mut sort = false;
        self.list.init_all_z(ctx, &mut sort);
        if sort {
            self.sort();
        }
    }

    fn deinit_all(&mut self, ctx: &mut WidgetContext) {
        self.list.deinit_all(ctx)
    }

    fn update_all<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O) {
        let mut resort = false;
        let mut items_changed = false;
        self.list.update_all_z(ctx, &mut (observer, &mut items_changed), &mut resort);

        if resort || (items_changed && self.has_non_default_zs) {
            // z_index changed or inserted

            self.sort();
            ctx.updates.render();
        }
    }

    fn event_all<EU: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &EU) {
        self.list.event_all(ctx, args)
    }

    fn measure_all<A, D>(&mut self, ctx: &mut LayoutContext, available_size: A, desired_size: D)
    where
        A: FnMut(&mut LayoutContext, AvailableSizeArgs) -> AvailableSize,
        D: FnMut(&mut LayoutContext, FinalSizeArgs),
    {
        self.list.measure_all(ctx, available_size, desired_size)
    }

    fn widget_measure(&mut self, index: usize, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        self.list.widget_measure(index, ctx, available_size)
    }

    fn arrange_all<F>(&mut self, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: F)
    where
        F: FnMut(&mut LayoutContext, &mut FinalSizeArgs) -> PxSize,
    {
        self.list.arrange_all(ctx, widget_layout, final_size)
    }

    fn widget_arrange(&mut self, index: usize, ctx: &mut LayoutContext, widget_layout: &mut WidgetLayout, final_size: PxSize) {
        self.list.widget_arrange(index, ctx, widget_layout, final_size)
    }

    fn render_all(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        if self.lookup.is_empty() {
            self.list.render_all(ctx, frame);
        } else {
            for &i in &self.lookup {
                self.widget_render(i as usize, ctx, frame);
            }
        }
    }

    fn widget_render(&self, index: usize, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        self.list.widget_render(index, ctx, frame)
    }

    fn render_update_all(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.list.render_update_all(ctx, update)
    }

    fn widget_render_update(&self, index: usize, ctx: &mut RenderContext, update: &mut FrameUpdate) {
        self.list.widget_render_update(index, ctx, update)
    }
}
impl<W: WidgetList> WidgetList for ZSortedWidgetList<W> {
    fn count<F>(&self, filter: F) -> usize
    where
        F: FnMut(WidgetFilterArgs) -> bool,
    {
        self.list.count(filter)
    }

    fn boxed_widget_all(self) -> WidgetVec {
        self.list.boxed_widget_all()
    }

    fn widget_id(&self, index: usize) -> WidgetId {
        self.list.widget_id(index)
    }

    fn widget_state(&self, index: usize) -> &StateMap {
        self.list.widget_state(index)
    }

    fn widget_state_mut(&mut self, index: usize) -> &mut StateMap {
        self.list.widget_state_mut(index)
    }

    fn widget_outer_info(&self, index: usize) -> &WidgetLayoutInfo {
        self.list.widget_outer_info(index)
    }

    fn widget_inner_info(&self, index: usize) -> &WidgetLayoutInfo {
        self.list.widget_inner_info(index)
    }

    fn widget_border_info(&self, index: usize) -> &WidgetBorderInfo {
        self.list.widget_border_info(index)
    }

    fn widget_render_info(&self, index: usize) -> &WidgetRenderInfo {
        self.list.widget_render_info(index)
    }

    fn render_filtered<F>(&self, mut filter: F, ctx: &mut RenderContext, frame: &mut FrameBuilder)
    where
        F: FnMut(WidgetFilterArgs) -> bool,
    {
        if self.lookup.is_empty() {
            self.list.render_filtered(filter, ctx, frame);
        } else {
            for &i in &self.lookup {
                let i = i as usize;
                let args = WidgetFilterArgs {
                    index: i,
                    outer_info: self.widget_outer_info(i),
                    inner_info: self.widget_inner_info(i),
                    border_info: self.widget_border_info(i),
                    render_info: self.widget_render_info(i),
                    state: self.widget_state(i),
                };
                if filter(args) {
                    self.widget_render(i, ctx, frame);
                }
            }
        }
    }
}

/// Defines the render order of an widget in a layout panel.
///
/// When set the widget will still update and layout according to their *logical* position in the list but
/// they will render according to the order defined by the [`ZIndex`] value.
///
/// Layout panels that support this property should mention it in their documentation, implementers
/// see [`ZSortedWidgetList`] for more details.
#[property(context, default(ZIndex::DEFAULT))]
pub fn z_index(child: impl UiNode, index: impl IntoVar<ZIndex>) -> impl UiNode {
    struct ZIndexNode<C, I> {
        child: C,
        index: I,
        valid: bool,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode, I: Var<ZIndex>> UiNode for ZIndexNode<C, I> {
        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions.var(ctx, &self.index);

            self.child.subscriptions(ctx, subscriptions);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            let z_ctx = ZIndexContextVar::get(ctx.vars);
            if z_ctx.panel_id != ctx.path.ancestors().next() || z_ctx.panel_id.is_none() {
                tracing::error!(
                    "property `z_index` set for `{}` but it is not the direct child of a Z-sorting panel",
                    ctx.path.widget_id()
                );
                self.valid = false;
            } else {
                self.valid = true;

                let index = self.index.copy(ctx);
                if index != ZIndex::DEFAULT {
                    z_ctx.resort.set(true);
                    ctx.widget_state.set(ZIndexKey, self.index.copy(ctx));
                }
            }
            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if self.valid {
                if let Some(i) = self.index.copy_new(ctx) {
                    let z_ctx = ZIndexContextVar::get(ctx.vars);

                    debug_assert_eq!(z_ctx.panel_id, ctx.path.ancestors().next());

                    z_ctx.resort.set(true);
                    ctx.widget_state.set(ZIndexKey, i);
                }
            }

            self.child.update(ctx);
        }
    }
    ZIndexNode {
        child,
        index: index.into_var(),
        valid: false,
    }
}

/// Position of a widget inside a [`WidgetList`] render operation.
///
/// When two widgets have the same index their logical position defines the render order.
///
/// # Examples
///
/// Create a Z-index that causes the widget to render in front of all siblings that don't set Z-index.
///
/// ```
/// # use zero_ui_core::ui_list::ZIndex;
///
/// let highlight_z = ZIndex::DEFAULT + 1;
/// ```
///
/// See [`z_index`] for more details.
///
/// [`z_index`]: fn@z_index
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ZIndex(pub u32);
impl ZIndex {
    /// Widget is rendered first causing all overlapping siblings to render on top of it.
    ///
    /// The value is `0`.
    pub const BACK: ZIndex = ZIndex(0);

    /// Z-index of widgets that don't set the index.
    ///
    /// The value is `u32::MAX / 2`.
    pub const DEFAULT: ZIndex = ZIndex(u32::MAX / 2);

    /// Widget is rendered after all siblings causing it to render on top.
    pub const FRONT: ZIndex = ZIndex(u32::MAX);

    /// Computes `other` above `self`, caps at [`FRONT`].
    ///
    /// This is the default ZIndex addition, equivalent to `self + other`.
    ///
    /// [`FRONT`]: Self::FRONT
    pub fn saturating_add(self, other: impl Into<Self>) -> Self {
        ZIndex(self.0.saturating_add(other.into().0))
    }

    /// Computes `other` below `self`, stops at [`BACK`].
    ///
    /// This is the default ZIndex subtraction, equivalent to `self - other`.
    ///
    /// [`BACK`]: Self::BACK
    pub fn saturating_sub(self, other: impl Into<Self>) -> Self {
        ZIndex(self.0.saturating_sub(other.into().0))
    }

    /// Gets the index set on a widget.
    pub fn get(widget: &impl Widget) -> ZIndex {
        widget.state().copy(ZIndexKey).unwrap_or_default()
    }
}
impl Default for ZIndex {
    fn default() -> Self {
        ZIndex::DEFAULT
    }
}
impl<Z: Into<ZIndex>> ops::Add<Z> for ZIndex {
    type Output = Self;

    fn add(self, rhs: Z) -> Self::Output {
        self.saturating_add(rhs)
    }
}
impl<Z: Into<ZIndex>> ops::Sub<Z> for ZIndex {
    type Output = Self;

    fn sub(self, rhs: Z) -> Self::Output {
        self.saturating_sub(rhs)
    }
}
impl fmt::Debug for ZIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let z = *self;
        if f.alternate() {
            write!(f, "ZIndex::")?;
        }

        if z == Self::DEFAULT {
            write!(f, "DEFAULT")
        } else if z == Self::BACK {
            write!(f, "BACK")
        } else if z == Self::FRONT {
            write!(f, "FRONT")
        } else if z > Self::DEFAULT {
            if z > Self::FRONT - 10000 {
                write!(f, "FRONT-{}", Self::FRONT.0 - z.0)
            } else {
                write!(f, "DEFAULT+{}", z.0 - Self::DEFAULT.0)
            }
        } else if z < Self::BACK + 10000 {
            write!(f, "BACK+{}", z.0 - Self::BACK.0)
        } else {
            write!(f, "DEFAULT-{}", Self::DEFAULT.0 - z.0)
        }
    }
}
impl_from_and_into_var! {
    fn from(index: u32) -> ZIndex {
        ZIndex(index)
    }
}

/// Extension methods for [`Widget`] that read the [`ZIndex`] of the widget.
pub trait WidgetZIndexExt {
    /// Gets the widget z-index.
    fn z_index(&self) -> ZIndex;
}
impl<W: Widget> WidgetZIndexExt for W {
    fn z_index(&self) -> ZIndex {
        self.state().copy(ZIndexKey).unwrap_or_default()
    }
}

/// Extension methods for [`WidgetList`] that read the [`ZIndex`] of the widget and does updates monitoring
/// z-sort update requests.
///
/// These trait methods are intended for layout panel implementers that cannot use the default [`ZSortedWidgetList`], but
/// want to support the [`z_index`] property with a custom implementation.
///
/// [`z_index`]: fn@z_index
pub trait WidgetListZIndexExt {
    /// Returns the widget Z-Index.
    fn widget_z_index(&self, index: usize) -> ZIndex;

    /// Does an [`init_all`], sets `sort_z` if any of the widgets sets a non-default z-index.
    ///
    /// [`init_all`]: UiNodeList::init_all
    fn init_all_z(&mut self, ctx: &mut WidgetContext, sort_z: &mut bool);

    /// Does an [`update_all`], sets `resort_z` if the z-index changed for any widget or an widget was inited (inserted) with
    /// a non-default index.
    ///
    /// Note that if the list is already sorting or has observed a non-default index it must also resort for any change
    /// reported to the `observer`.
    ///
    /// [`update_all`]: UiNodeList::update_all
    fn update_all_z<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O, resort_z: &mut bool);
}
impl<L: WidgetList> WidgetListZIndexExt for L {
    fn widget_z_index(&self, index: usize) -> ZIndex {
        self.widget_state(index).copy(ZIndexKey).unwrap_or_default()
    }

    fn init_all_z(&mut self, ctx: &mut WidgetContext, sort_z: &mut bool) {
        *sort_z = ZIndexContext::with(ctx.vars, ctx.path.widget_id(), || self.init_all(ctx));
    }

    fn update_all_z<O: UiListObserver>(&mut self, ctx: &mut WidgetContext, observer: &mut O, resort_z: &mut bool) {
        *resort_z = ZIndexContext::with(ctx.vars, ctx.path.widget_id(), || self.update_all(ctx, observer));
    }
}

state_key! {
    struct ZIndexKey: ZIndex;
}

#[derive(Default, Clone, Debug)]
struct ZIndexContext {
    // used in `z_index` to validate that it will have an effect.
    panel_id: Option<WidgetId>,
    // set by `z_index` to signal a z-resort is needed.
    resort: Cell<bool>,
}
impl ZIndexContext {
    fn with(vars: &Vars, panel_id: WidgetId, action: impl FnOnce()) -> bool {
        let ctx = ZIndexContext {
            panel_id: Some(panel_id),
            resort: Cell::new(false),
        };
        vars.with_context_var(ZIndexContextVar, ContextVarData::fixed(&ctx), action);
        ctx.resort.get()
    }
}
context_var! {
    struct ZIndexContextVar: ZIndexContext = ZIndexContext::default();
}
