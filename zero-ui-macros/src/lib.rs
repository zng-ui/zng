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

/// Generates default implementations of [UiNode](zero_ui::core::UiNode) methods.
///
/// # Usage
///
/// In a inherent impl, annotate the impl block with `#[impl_ui_node(..)]` and custom `UiNode` methods with `#[UiNode]`.
///
/// ```rust
/// #[impl_ui_node(none)]
/// impl<C: Var<ColorF>> FillColor<C> {
///     #[UiNode]
///     fn render(&self, f: &mut FrameBuilder) {
///         f.push_fill_color(&LayoutRect::from_size(f.final_size()), *self.color.get_local());
///     }
/// }
/// ```
///
/// In a `UiNode` trait impl block, annotate the impl block with `#[impl_ui_node(..)]` and only implement custom methods.
///
/// ```rust
/// #[impl_ui_node(none)]
/// impl<C: Var<ColorF>> UiNode for FillColor<C> {
///     fn render(&self, f: &mut FrameBuilder) {
///         f.push_fill_color(&LayoutRect::from_size(f.final_size()), *self.color.get_local());
///     }
/// }
/// ```
///
/// The generated defaults can be configurated in the macro.
///
/// ## `#[impl_ui_node(none)]`
///
/// Generates defaults for UI components without descendants.
///
/// ### Defaults
/// * Init, Updates: Does nothing, blank implementation.
/// * Layout: Fills finite spaces, collapses in infinite spaces.
/// * Render: Does nothing, blank implementation.
///
/// ```rust
/// # use zero_ui::core::{Var, FrameBuilder, ColorF, LayoutSize, UiValues, NextUpdate};
/// # pub struct FillColor<C: Var<ColorF>>(C);
/// #
/// #[impl_ui_node(none)]
/// impl<C: Var<ColorF>> FillColor<C> {
///     pub fn new(color: C) -> Self {
///         FillColor(color)
///     }
///     /// Custom impl
///     #[UiNode]
///     fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
///         if self.0.changed() {
///             update.render_frame();
///         }
///     }
///     /// Custom impl
///     #[UiNode]
///     fn render(&self, f: &mut FrameBuilder) {
///         f.push_fill_color(&LayoutRect::from_size(f.final_size()), *self.color.get_local());
///     }
/// }
/// ```
/// ### Expands to
///
/// ```rust
/// impl<C: Value<ColorF>> FillColor<C> {
///     pub fn new(color: C) -> Self {
///         FillColor(color)
///     }
/// }
///
/// impl<C: Value<ColorF>> zero_ui::core::UiNode for FillColor<C> {
///     /// Custom impl
///     #[inline]
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(LayoutRect::from_size(f.final_size()), *self.0, None);
///     }
///     /// Custom impl
///     #[inline]
///     fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
///         if self.0.changed() {
///             update.render_frame();
///         }
///     }
///
///     #[inline]
///     fn measure(&mut self, available_size: zero_ui::core::LayoutSize) -> zero_ui::core::LayoutSize {
///         let mut size = available_size;
///         if size.width.is_infinite() {
///             size.width = 0.0;
///         }
///         if size.height.is_infinite() {
///             size.height = 0.0;
///         }
///         size
///     }
///     #[inline]
///     fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
///         None
///     }
///     #[inline]
///     fn init(&mut self, values: &mut zero_ui::core::UiValues, update: &mut zero_ui::core::NextUpdate) {}
///     #[inline]
///     fn arrange(&mut self, final_size: zero_ui::core::LayoutSize) {}
///     #[inline]
///     fn keyboard_input(
///         &mut self,
///         input: &zero_ui::core::KeyboardInput,
///         values: &mut zero_ui::core::UiValues,
///         update: &mut zero_ui::core::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn window_focused(
///         &mut self,
///         focused: bool,
///         values: &mut zero_ui::core::UiValues,
///         update: &mut zero_ui::core::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn mouse_input(
///         &mut self,
///         input: &zero_ui::core::MouseInput,
///         hits: &zero_ui::core::Hits,
///         values: &mut zero_ui::core::UiValues,
///         update: &mut zero_ui::core::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn mouse_move(
///         &mut self,
///         input: &zero_ui::core::UiMouseMove,
///         hits: &zero_ui::core::Hits,
///         values: &mut zero_ui::core::UiValues,
///         update: &mut zero_ui::core::NextUpdate,
///     ) {
///     }
///     #[inline]
///     fn mouse_entered(&mut self, values: &mut zero_ui::core::UiValues, update: &mut zero_ui::core::NextUpdate) {}
///     #[inline]
///     fn mouse_left(&mut self, values: &mut zero_ui::core::UiValues, update: &mut zero_ui::core::NextUpdate) {}
///     #[inline]
///     fn close_request(&mut self, values: &mut zero_ui::core::UiValues, update: &mut zero_ui::core::NextUpdate) {}
///     #[inline]
///     fn parent_value_changed(
///         &mut self,
///         values: &mut zero_ui::core::UiValues,
///         update: &mut zero_ui::core::NextUpdate,
///     ) {
///     }
/// }
/// ```
///
/// ## `#[impl_ui_node(child)]`
///
/// Shorthand for:
/// ```rust
/// #[impl_ui_node(
///     delegate: &self.child,
///     delegate_mut: &mut self.child
/// )]
/// ```
///
/// ## `#[impl_ui_node(children)]`
///
/// Shorthand for:
/// ```rust
/// #[impl_ui_node(
///     delegate_iter: self.children.iter(),
///     delegate_iter_mut: mut self.children.iter_mut()
/// )]
/// ```
///
/// ## `#[impl_ui_node(delegate: expr, delegate_mut: expr)]`
///
/// Generates defaults by delegating the method calls to
/// a reference of another UiNode component.
///
/// Both arguments are required and in order.
///
/// ```rust
/// #[impl_ui_node(delegate: self.0.borrow(), delegate_mut: self.0.borrow_mut())]
/// // TODO
/// ```
///
/// ## `#[impl_ui_node(delegate_iter: expr, delegate_iter_mut: expr)]`
///
/// Generates defaults by delegating the method calls to
/// all UiNode component references provided by the iterators.
///
/// ### Defaults
/// * Events: Calls the same event method for each `UiNode` delegate provided by the iterator.
/// * Layout: Measure all delegates the largest size is returned. Arranges all delegates with the default top-left alignment.
/// * Render: Renders all delegates on top of each other in the iterator order.
/// * Hit-test: Returns the first delegate hit or `None` if none hit.
///
/// ```rust
/// #[impl_ui_node(delegate_iter: self.0.iter(), delegate_iter_mut: self.0.iter_mut())]
/// // TODO
/// ```
///
/// ## Delegate Validation
/// If delegation is configured but no delegation occurs in the manually implemented methods
/// you get the error `"auto impl delegates call to `{}` but this manual impl does not"`.
///
/// To disable this error use `#[allow_missing_delegate]` in the method or in the `impl` block.
#[proc_macro_attribute]
pub fn impl_ui_node(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui_node::gen_impl_ui_node(args, input)
}

/// Declares a new widget property.
#[proc_macro_attribute]
pub fn property(args: TokenStream, input: TokenStream) -> TokenStream {
    property::expand_property(args, input)
}

/// Declares a new widget macro.
#[proc_macro]
pub fn widget(input: TokenStream) -> TokenStream {
    widget::expand_widget(input)
}

/// Used internally by macros generated by `[widget]` to
/// initialize the widget.
#[doc(hidden)]
#[proc_macro_hack]
pub fn widget_new(input: TokenStream) -> TokenStream {
    widget_new::expand_widget_new(input)
}
