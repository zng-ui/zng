use crate::core::{
    context::WidgetContext,
    render::FrameBuilder,
    types::{LayoutPoint, LayoutRect, LayoutSize},
    ui_vec,
    var::{IntoVar, LocalVar},
    UiNode, UiVec, Widget, LAYOUT_ANY_SIZE,
};
use crate::core::{impl_ui_node, widget};
use crate::properties::{
    capture_only::{stack_spacing, widget_children},
    margin,
};
use std::marker::PhantomData;

trait StackDimension: 'static {
    fn length(size: LayoutSize) -> f32;
    /// Orthogonal length.
    fn ort_length(size: LayoutSize) -> f32;
    /// (length, ort_length).
    fn lengths_mut(size: &mut LayoutSize) -> (&mut f32, &mut f32);
    fn origin_mut(origin: &mut LayoutPoint) -> &mut f32;
}

struct Stack<S: LocalVar<f32>, D: StackDimension> {
    children: Box<[Box<dyn UiNode>]>,
    rectangles: Box<[LayoutRect]>,
    spacing: S,
    _d: PhantomData<D>,
}

#[impl_ui_node(children)]
impl<S: LocalVar<f32>, D: StackDimension> Stack<S, D> {
    fn new(children: UiVec, spacing: S, _dimension: D) -> Self {
        Stack {
            rectangles: vec![LayoutRect::zero(); children.len()].into_boxed_slice(),
            children: children.into_boxed_slice(),
            spacing,
            _d: PhantomData,
        }
    }

    #[UiNode]
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.spacing.init_local(ctx.vars);
        for child in self.children.iter_mut() {
            child.init(ctx);
        }
    }

    #[UiNode]
    fn update(&mut self, ctx: &mut WidgetContext) {
        if self.spacing.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
        }
        for child in self.children.iter_mut() {
            child.update(ctx);
        }
    }

    #[UiNode]
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        *D::lengths_mut(&mut available_size).0 = LAYOUT_ANY_SIZE;

        let mut total_size = LayoutSize::zero();
        let (total_len, max_ort_len) = D::lengths_mut(&mut total_size);
        let spacing = *self.spacing.get_local();
        let mut first = true;

        for (child, r) in self.children.iter_mut().zip(self.rectangles.iter_mut()) {
            r.size = child.measure(available_size);

            let origin = D::origin_mut(&mut r.origin);
            *origin = *total_len;
            *total_len += D::length(r.size);

            if first {
                first = false;
            } else {
                *origin += spacing;
                *total_len += spacing;
            }
            *max_ort_len = max_ort_len.max(D::ort_length(r.size));
        }

        total_size
    }

    #[UiNode]
    fn arrange(&mut self, final_size: LayoutSize) {
        let max_ort_len = D::ort_length(final_size);
        for (child, r) in self.children.iter_mut().zip(self.rectangles.iter_mut()) {
            let mut size = r.size;
            *D::lengths_mut(&mut size).1 = max_ort_len;
            child.arrange(size);
        }
    }

    #[UiNode]
    fn render(&self, frame: &mut FrameBuilder) {
        for (child, r) in self.children.iter().zip(self.rectangles.iter()) {
            frame.push_reference_frame(r.origin, |f| child.render(f));
        }
    }
}
struct VerticalD;
impl StackDimension for VerticalD {
    fn length(size: LayoutSize) -> f32 {
        size.height
    }
    fn ort_length(size: LayoutSize) -> f32 {
        size.width
    }
    fn lengths_mut(size: &mut LayoutSize) -> (&mut f32, &mut f32) {
        (&mut size.height, &mut size.width)
    }
    fn origin_mut(origin: &mut LayoutPoint) -> &mut f32 {
        &mut origin.y
    }
}
struct HorizontalD;
impl StackDimension for HorizontalD {
    fn length(size: LayoutSize) -> f32 {
        size.width
    }
    fn ort_length(size: LayoutSize) -> f32 {
        size.height
    }
    fn lengths_mut(size: &mut LayoutSize) -> (&mut f32, &mut f32) {
        (&mut size.width, &mut size.height)
    }
    fn origin_mut(origin: &mut LayoutPoint) -> &mut f32 {
        &mut origin.x
    }
}

widget! {
    /// Horizontal stack layout.
    ///
    /// # Example
    ///
    /// ```
    /// let text = v_stack! {
    ///     spacing: 5.0;
    ///     items: ui_vec![
    ///         text("1. Hello"),
    ///         text("2. World"),
    ///     ];
    /// };
    /// ```
    ///
    /// ## `h_stack()`
    ///
    /// If you only want to set the `items` property you can use the [`h_stack`](function@h_stack) shortcut function.
    pub h_stack;

    default_child {
        /// Space in-between items.
        spacing -> stack_spacing: 0.0;
        /// Widget items.
        items -> widget_children: ui_vec![];
        /// Items margin.
        padding -> margin;
    }

    /// New stack layout.
    #[inline]
    fn new_child(items, spacing) -> impl UiNode {
        Stack::new(items.unwrap(), spacing.unwrap().into_local(), HorizontalD)
    }
}

widget! {
    /// Vertical stack layout.
    ///
    /// # Example
    ///
    /// ```
    /// let text = h_stack! {
    ///     spacing: 5.0;
    ///     items: ui_vec![
    ///         text("Hello"),
    ///         text("World"),
    ///     ];
    /// };
    /// ```
    /// ## `v_stack()`
    ///
    /// If you only want to set the `items` property you can use the [`v_stack`](function@v_stack) shortcut function.
    pub v_stack;

    default_child {
        /// Space in-between items.
        spacing -> stack_spacing: 0.0;
        /// Widget items.
        items -> widget_children: ui_vec![];
        /// Items margin.
        padding -> margin;
    }

    /// New stack layout.
    #[inline]
    fn new_child(items, spacing) -> impl UiNode {
        Stack::new(items.unwrap(), spacing.unwrap().into_local(), VerticalD)
    }
}

/// Basic horizontal stack layout.
///
/// # Example
///
/// ```
/// let text = h_stack(ui_vec![
///     text("Hello "),
///     text("World"),
/// ]);
/// ```
///
/// # `h_stack!`
///
/// This function is just a shortcut for [`h_stack!`](module@v_stack). Use the full widget
/// to better configure the horizontal stack widget.
pub fn h_stack(items: UiVec) -> impl Widget {
    h_stack! {
        items;
    }
}

/// Basic vertical stack layout.
///
/// # Example
///
/// ```
/// let text = v_stack(ui_vec![
///     text("1. Hello"),
///     text("2. World"),
/// ]);
/// ```
///
/// # `v_stack!`
///
/// This function is just a shortcut for [`v_stack!`](module@v_stack). Use the full widget
/// to better configure the vertical stack widget.
pub fn v_stack(items: UiVec) -> impl Widget {
    v_stack! {
        items;
    }
}

struct ZStack {
    children: Box<[Box<dyn UiNode>]>,
}

#[impl_ui_node(children)]
impl UiNode for ZStack {}

widget! {
    /// Layering stack layout.
    ///
    /// # Example
    ///
    /// ```
    /// let text = z_stack! {
    ///     padding: 5.0;
    ///     items: ui_vec![
    ///         text("under"),
    ///         text("over"),
    ///     ];
    /// }
    /// ```
    ///
    /// ## `z_stack()`
    ///
    /// If you only want to set the `items` property you can use the [`z_stack`](function@z_stack) shortcut function.
    pub z_stack;

    default_child {
        /// Widget items.
        items -> widget_children: ui_vec![];
        /// Items margin.
        padding -> margin;
    }

    fn new_child(items) -> impl UiNode {
        ZStack {
            children: items.unwrap().into_boxed_slice(),
        }
    }
}

/// Basic layering stack layout.
///
/// # Example
///
/// ```
/// let text = z_stack(ui_vec![
///     text("under"),
///     text("over"),
/// ]);
/// ```
///
/// # `z_stack!`
///
/// This function is just a shortcut for [`z_stack!`](module@z_stack). Use the full widget
/// to better configure the layering stack widget.
pub fn z_stack(items: UiVec) -> impl Widget {
    z_stack! { items; }
}
