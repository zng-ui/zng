extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;

#[macro_use]
mod util;

mod impl_ui_node;
mod property;
pub(crate) mod widget;
pub(crate) mod widget_new;

use widget::CallKind;

/// Generates default implementations of [`UiNode`](zero_ui::core::UiNode) methods.
///
/// # Arguments
///
/// The macro attribute takes arguments that indicate how the missing methods will be generated.
///
/// * `delegate: {&}, delegate_mut: {&mut}`.
///
/// * `delegate_iter: {Iterator<&>}, delegate_iter_mut: {Iterator<&mut>}`.
///
/// You can also use shorthand for common delegation:
///
/// * `child` is the same as `delegate: &self.child, delegate_mut: &mut self.child`.
///
/// * `children` is the same as `delegate_iter: self.children.iter(), delegate_iter_mut: self.children.iter_mut()`.
///
/// And for nodes without descendants you can use:
/// * `none`
///
/// # Validation
/// If delegation is configured but no delegation occurs in the manually implemented methods
/// you get the error ``"auto impl delegates call to `{}` but this manual impl does not"``.
///
/// To disable this error use `#[allow_missing_delegate]` in the method or in the `impl` block.
///
/// # Usage Examples
///
/// Given an UI node `struct`:
/// ```
/// struct FillColor<C> {
///     color: C,
///     final_size: LayoutSize,
/// }
/// ```
///
/// In an `UiNode` trait impl block, annotate the impl block with `#[impl_ui_node(..)]` and only implement custom methods.
///
/// ```
/// #[impl_ui_node(none)]
/// impl<C: Var<ColorF>> UiNode for FillColor<C> {
///     fn render(&self, f: &mut FrameBuilder) {
///         frame.push_color(LayoutRect::from_size(self.final_size), *self.color.get_local());
///     }
/// }
/// ```
///
/// Or, in a inherent impl, annotate the impl block with `#[impl_ui_node(..)]` and custom `UiNode` methods with `#[UiNode]`.
///
/// ```
/// #[impl_ui_node(none)]
/// impl<C: LocalVar<ColorF>> FillColor<C> {
///     pub fn new(color: C) -> Self {
///         FillColor { color: final_size: LayoutSize::zero() }
///     }
///
///     #[UiNode]
///     fn render(&self, frame: &mut FrameBuilder) {
///         frame.push_color(LayoutRect::from_size(self.final_size), *self.color.get_local());
///     }
/// }
/// ```
///
/// In both cases a full `UiNode` implement is generated for the node `struct`, but in the second case the inherent methods
/// are also kept, you can use this to reduce verbosity for nodes with multiple generics.
///
/// ## Delegate to `none`
///
/// Generates defaults for UI components without descendants.
///
/// ### Defaults
///
/// * Init, Updates: Does nothing, blank implementation.
/// * Layout: Fills finite spaces, collapses in infinite spaces.
/// * Render: Does nothing, blank implementation.
///
/// ```
/// #[impl_ui_node(none)]
/// impl<C: LocalVar<ColorF>> NoneDelegateSample<C> {
///     pub fn new(color: C) -> Self {
///          FillColor { color: final_size: LayoutSize::zero() }
///     }
/// }
/// ```
///
/// ### Expands to
///
/// ```
/// impl<C: LocalVar<ColorF>> NoneDelegateSample<C> {
///     pub fn new(color: C) -> Self {
///          FillColor { color: final_size: LayoutSize::zero() }
///     }
/// }
///
/// impl<C: LocalVar<ColorF>> zero_ui::core::UiNode for NoneDelegateSample<C> {
///
///     #[inline]
///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
///
///     #[inline]
///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
///
///     #[inline]
///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::types::LayoutSize) -> zero_ui::core::types::LayoutSize {
///         let mut size = available_size;
///         if size.width.is_infinite() {
///             size.width = 0.0;
///         }
///         if size.height.is_infinite() {
///             size.height = 0.0;
///         }
///         size
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::types::LayoutSize) { }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) { }
///
///     #[inline]
///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
///
/// }
/// ```
///
/// ## Delegate to one (`child` or `delegate, delegate_mut`)
///
/// Generates defaults for UI components with a single child node. This is the most common mode,
/// used by property nodes.
///
/// ### Defaults
///
/// * Init, Updates: Delegates to same method in child.
/// * Layout: Is the same size as child.
/// * Render: Delegates to child render.
///
/// ```
/// #[impl_ui_node(child)]
/// impl<C: UiNode> UiNode for ChildDelegateSample<C> { }
/// ```
///
/// ### Expands to
///
/// ```
/// impl<C: UiNode> UiNode ChildDelegateSample<C> {
///
///     #[inline]
///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let child = { &mut self.child };
///         child.init(ctx)
///     }
///
///     #[inline]
///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let child = { &mut self.child };
///         child.update(ctx)
///     }
///
///     #[inline]
///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let child = { &mut self.child };
///         child.update_hp(ctx)
///     }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::types::LayoutSize) -> zero_ui::core::types::LayoutSize {
///         let child = { &mut self.child };
///         child.measure(available_size)
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::types::LayoutSize) {
///         let child = { &mut self.child };
///         child.arrange(final_size)
///     }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
///         let child = { &self.child };
///         child.render(frame)
///     }
///
///     #[inline]
///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         let child = { &mut self.child };
///         child.deinit(ctx)
///     }
///
/// }
/// ```
///
/// ## Delegate to many (`children` or `delegate_iter, delegate_iter_mut`)
///
/// Generates defaults for UI components with a multiple children nodes. This is used mostly by
/// layout panels.
///
/// ### Defaults
///
/// * Init, Updates: Delegates to same method in each child.
/// * Layout: Is the same size as the largest child.
/// * Render: Z-stacks the children. Last child on top.
///
/// ```
/// #[impl_ui_node(children)]
/// impl<C: UiNode> UiNode for ChildrenDelegateSample<C> { }
/// ```
///
/// ### Expands to
///
/// ```
/// impl<C: UiNode> UiNode ChildrenDelegateSample<C> {
///
///     #[inline]
///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.child.iter_mut() } {
///             child.init(ctx)
///         }
///     }
///
///     #[inline]
///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.child.iter_mut() } {
///             child.update(ctx)
///         }
///     }
///
///     #[inline]
///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.child.iter_mut() } {
///             child.update_hp(ctx)
///         }
///     }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::types::LayoutSize) -> zero_ui::core::types::LayoutSize {
///         let mut size = Default::default();
///         for child in { self.child.iter_mut() } {
///            size = child.measure(available_size).max(size);
///         }
///         size
///     }
///
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::types::LayoutSize) {
///         for child in { self.child.iter_mut() } {
///             child.arrange(final_size)
///         }
///     }
///
///     #[inline]
///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
///         for child in { self.child.iter() } {
///             child.render(frame)
///         }
///     }
///
///     #[inline]
///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
///         for child in { self.child.iter_mut() } {
///             child.deinit(ctx)
///         }
///     }
///
/// }
/// ```
#[proc_macro_attribute]
pub fn impl_ui_node(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_node::gen_impl_ui_node(args, input)
}

/// Declares a new widget property.
///
///
///
/// # Usage
///
/// Annotate
#[proc_macro_attribute]
pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
    property::expand_property(args, input)
}

/// Declares a new widget macro.
#[proc_macro]
pub fn widget(input: TokenStream) -> TokenStream {
    widget::expand_widget(CallKind::Widget, input)
}

/// Declares a new widget mix-in macro.
#[proc_macro]
pub fn widget_mixin(input: TokenStream) -> TokenStream {
    widget::expand_widget(CallKind::Mixin, input)
}

/// Declares a new widget inherit macro.
#[proc_macro]
pub fn widget_inherit(input: TokenStream) -> TokenStream {
    widget::expand_widget(CallKind::Inherit, input)
}

/// Declares a new widget mix-in inherit macro.
#[proc_macro]
pub fn widget_mixin_inherit(input: TokenStream) -> TokenStream {
    widget::expand_widget(CallKind::MixinInherit, input)
}

/// Used internally by macros generated by `[widget]` to
/// initialize the widget.
#[doc(hidden)]
#[proc_macro_hack]
pub fn widget_new(input: TokenStream) -> TokenStream {
    widget_new::expand_widget_new(input)
}
