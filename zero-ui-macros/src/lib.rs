extern crate proc_macro;

#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use proc_macro_hack::proc_macro_hack;

mod impl_ui;
mod ui;

/// Generates default implementations of [Ui](zero_ui::core::Ui) methods.
///
/// # Usage
///
/// In a inherent impl, anotate the impl block with `#[impl_ui]` and custom `Ui` methods with `#[Ui]`.
///
/// ```rust
/// #[impl_ui]
/// impl<C: Value<ColorF>> FillColor<C> {
///     #[Ui]
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(LayoutRect::from_size(f.final_size()), *self.0, None);
///     }
/// }
/// ```
///
/// In a `Ui` trait impl block, anotate the impl block with `#[impl_ui]` and only implement custom methods.
///
/// ```rust
/// #[impl_ui]
/// impl<C: Value<ColorF>> Ui for FillColor<C> {
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(LayoutRect::from_size(f.final_size()), *self.0, None);
///     }
/// }
/// ```
///
/// The generated defaults can be configurated in the macro.
///
/// ## `#[impl_ui]`
///
/// Generates defaults for UI components without descendents.
///
/// ### Defaults
/// * Events: Does nothing, blank implementation.
/// * Layout: Normal fill behavior, fills finite spaces, collapses in infinite spaces.
/// * Render: Does nothing, blank implementation.
/// * Hit-test: Not hit-testable, `point_over` is always `None`.
///
/// ```rust
/// # use zero_ui::core::{Value, NextFrame, ColorF, LayoutSize, UiValues, NextUpdate};
/// # pub struct FillColor<C: Value<ColorF>>(C);
/// #
/// #[impl_ui]
/// impl<C: Value<ColorF>> FillColor<C> {
///     pub fn new(color: C) -> Self {
///         FillColor(color)
///     }
///     /// Custom impl
///     #[Ui]
///     fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
///         if self.0.changed() {
///             update.render_frame();
///         }
///     }
///     /// Custom impl
///     #[Ui]
///     fn render(&self, f: &mut NextFrame) {
///         f.push_color(LayoutRect::from_size(f.final_size()), *self.0, None);
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
/// impl<C: Value<ColorF>> zero_ui::core::Ui for FillColor<C> {
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
/// ## `#[impl_ui(child)]`
///
/// Shorthand for:
/// ```rust
/// #[impl_ui(
///     delegate: &self.child,
///     delegate_mut: &mut self.child
/// )]
/// ```
///
/// ## `#[impl_ui(children)]`
///
/// Shorthand for:
/// ```rust
/// #[impl_ui(
///     delegate_iter: self.children.iter(),
///     delegate_iter_mut: mut self.children.iter_mut()
/// )]
/// ```
///
/// ## `#[impl_ui(delegate: expr, delegate_mut: expr)]`
///
/// Generates defaults by delegating the method calls to
/// a reference of another Ui component.
///
/// Both arguments are required and in order.
///
/// ```rust
/// #[impl_ui(delegate: self.0.borrow(), delegate_mut: self.0.borrow_mut())]
/// // TODO
/// ```
///
/// ## `#[impl_ui(delegate_iter: expr, delegate_iter_mut: expr)]`
///
/// Generates defaults by delegating the method calls to
/// all Ui component references provided by the iterators.
///
/// ### Defaults
/// * Events: Calls the same event method for each `Ui` delegate provided by the iterator.
/// * Layout: Measure all delegates the largest size is returned. Arranges all delegates with the default top-left alignment.
/// * Render: Renders all delegates on top of each other in the iterator order.
/// * Hit-test: Returns the first delegate hit or `None` if none hit.
///
/// ```rust
/// #[impl_ui(delegate_iter: self.0.iter(), delegate_iter_mut: self.0.iter_mut())]
/// // TODO
/// ```
#[proc_macro_attribute]
pub fn impl_ui(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui::implementation(args, input, quote! {zero_ui})
}

/// Same as `impl_ui` but with type paths using the keyword `crate::` instead of `zero_ui::`.
#[doc(hidden)]
#[proc_macro_attribute]
pub fn impl_ui_crate(args: TokenStream, input: TokenStream) -> TokenStream {
    impl_ui::implementation(args, input, quote! {crate})
}

#[proc_macro_hack]
pub fn ui(input: TokenStream) -> TokenStream {
    ui::implementation(input)
}