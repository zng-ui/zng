#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]

//! Zero-Ui is a pure Rust UI framework.
//!
//! # Example
//! ```no_run
//! use zero_ui::prelude::*;
//!
//! fn main() {
//!     App::default().run_window(|_| {
//!         let size = var_from((800., 600.));
//!         let title = size.map(|s: &Size| formatx!("Button Example - {}", s));
//!         window! {
//!             size;
//!             title;
//!             content: example();
//!         }
//!     })
//! }
//!
//! fn example() -> impl Widget {
//!     button! {
//!         on_click: |_,_| {
//!             println!("Button clicked!");
//!         };
//!         margin: 10.0;
//!         size: (300.0, 200.0);
//!         align: Alignment::CENTER;
//!         font_size: 28;
//!         content: text("Click Me!");
//!     }
//! }
//! ```
//!
//! # Architecture
//!
//! Zero-Ui apps are organized in a hierarchy of contexts that represents the lifetime of components.
//!
//! ```text
//! +----------------------------------------------------------------------+
//! | # Level 0 - App                                                      |
//! |                                                                      |
//! |  App, AppExtension, AppContext, AppServices, Events                  |
//! |                                                                      |
//! | +------------------------------------------------------------------+ |
//! | | # Level 1 - Window                                               | |
//! | |                                                                  | |
//! | |   Window, WindowContext, WindowServices, WindowId                | |
//! | |                                                                  | |
//! | | +--------------------------------------------------------------+ | |
//! | | | # Level 2 - Widget                                           | | |
//! | | |                                                              | | |
//! | | | UiNode, WidgetContext, widget!, #[property], WidgetId        | | |
//! | | |                                                              | | |
//! | | +--------------------------------------------------------------+ | |
//! | +------------------------------------------------------------------+ |
//! +----------------------------------------------------------------------+
//! ```
//!
//! ## Level 0 - App
//!
//! Components at this level live for the duration of the application, the root type is [`App`](crate::core::app::App).
//! An app is built from multiple extensions ([`AppExtension`](crate::core::app::AppExtension)) and then [`run`](crate::core::app::AppExtended::run).
//!
//! When the app is run, before the main loop starts, the extensions are [init](crate::core::app::AppExtension) with access to an especial context
//! [`AppInitContext`](crate::core::context::AppInitContext). [Services](#services) and [events](#services) can only be registered with
//! this context, they live for the duration of the application.
//!
//! After the app init, the main loop starts and the other extension methods are called with the [`AppContext`](crate::core::context::AppContext).
//!
//! ### Services
//!
//! Services are utilities that can be accessed by every component in every level, this includes [opening windows](crate::core::window::Windows)
//! and [shutting down](crate::core::app::AppProcess) the app it-self. All services implement [`AppService`](crate::core::service::AppService)
//! and can be requested from a [`AppServices`](crate::core::service::AppServices) that is provided by every context.
//!
//! ### Events
//!
//! Events are a list of [`EventArgs`](crate::core::event::EventArgs) that can be observed every update. New events can be generated from the
//! app extension methods or from other events. All events implement [`Event`](crate::core::event::Event) and a listener can be requested from
//! an [`Events`](crate::core::event::Events) that is provided by every context.
//!
//! Note that in [Level 2](#level-2-widget) events are abstracted further into a property that setups a listener and call a handler for every
//! event update.
//!
//! ## Level 1 - Window
//!
//! Components at this level live for the duration of a window instance. A window owns instances [window services](window-services)
//! and the root widget, it manages layout and rendering the widget tree.
//!
//! By default the [`WindowManager`](crate::core::window::WindowManager) extension sets-up the window contexts,
//! but that is not a requirement, you can implement your own *windows*.
//!
//! ### Window Services
//!
//! Services that require a [`WindowContext`](crate::core::context::WindowContext) to be instantiated. They live for the
//! duration of the window instance and every window has the same services. Window service builders must be registered only at
//! [Level 0](level-0-app) during the app initialization, the builders then are called during the window initialization to instantiate
//! the window services.
//!
//! These services can be requested from a [`WindowServices`](crate::core::service::WindowServices) that is provided by the window
//! and widget contexts.
//!
//! ## Level 2 - Widget
//!
//! The UI tree is composed of types that implement [`UiNode`](crate::core::UiNode), they can own one or more child nodes
//! that are also UI nodes, some special nodes introduce a new [`WidgetContext`](crate::core::context::WidgetContext) that
//! defines that sub-tree branch as a widget.
//!
//! The behavior and appearance of a widget is defined in these nodes, a widget
//! is usually composed of multiple nodes, one that defines the context, another that defines its unique behavior
//! plus more nodes introduced by [properties](#properties) that modify the widget.
//!
//! ### Properties
//!
//! A property is in essence a function that takes an UI node and some other arguments and returns a new UI node.
//! This is in fact the signature used for declaring one, see [`#[property]`](crate::core::property) for more details.
//!
//! Multiple properties are grouped together to form a widget.
//!
//! ### Widgets
//!
//! A widget is a bundle of preset properties plus two optional functions, see [`widget!`](crate::core::widget) for more details.
//!
//! During widget instantiation an UI node tree is build by feeding each property node to a subsequent property, in a debug build
//! inspector nodes are inserted also, to monitor the properties. You can press `CTRL+SHIT+I` to inspect a window.
//!
//! The widget root node introduces a new [`WidgetContext`](crate::core::context::WidgetContext) that can be accessed by all
//! widget properties.
//!
//! # State
//!
//! TODO how to keep state, and contextual states.
//!
//! ## Variables
//!
//! TODO
//!
//! # Updates
//!
//! TODO how the UI is updated.
//!
//! # Async
//!
//! TODO how to run async tasks that interact back with the UI.
//!
//! # Lifecycle Overview
//!
//!
//! ```text
//! +------------------------------------+
//! | # Setup                            |
//! |                  ↓↑                |
//! | App::default().extend(CustomExt)   |
//! |      ::empty()                     |
//! +------------------------------------+
//!    |
//!    | .run(|ctx: &mut AppContext| { .. })
//!    | .run_window(|ctx: &mut AppContext| { window! { .. } })
//!    ↓
//! +---------------------------------------+
//! | # Run                                 |
//! |                                       |
//! | services.register(AppProcess)         |
//! |    |                                  |
//! |    ↓            ↓↑                    |
//! | AppExtension::init(AppInitContext)    |
//! |    |                                  |
//! |    ↓     ↓OS  ↓timer  ↓UpdateNotifier |
//! | +---------------------------------------------+
//! | | # EventLoop                                 |
//! | |                                             |
//! | |  AppExtension ↓↑                            |
//! | |      ::on_window_event(..)                  |
//! | |      ::on_device_event(..)                  |
//! | |      ::on_new_frame_ready(..)               |
//! | |   |                                         |
//! | |   ↓      ↓update                            |
//! | | +-----------------------------------------------+
//! | | | # Updates                                     |
//! | | |                                               |
//! | | |   ↓↑ sync - pending assign, notify requests   |
//! | | |   ↓↑ vars - setup new values                  |
//! | | |   ↓↑ events - setup update arguments          |
//! | | |   ↓                                           |
//! | | |   if any -> AppExtension::update(..) ↑        |
//! | | |   |            UiNode::update(..)             |
//! | | |   |            UiNode::update_hp(..)          |
//! | | |   |               event handlers              |
//! | | |   ↓                                           |
//! | | +-----------------------------------------------+
//! | |     ↓                                        |
//! | | +-----------------------------------------------+
//! | | | # Layout/Render                               |
//! | | |                                               |
//! | | | AppExtension::update_display(..)              |
//! | | |           UiNode::measure(..)                 |
//! | | |           UiNode::arrange(..)                 |
//! | | |           UiNode::render(..)                  |
//! | | |           UiNode::render_update(..)           |
//! | | +-----------------------------------------------+
//! | |     ↓                                       |
//! | |   EventLoop                                 |
//! | +---------------------------------------------+
//! |   | AppProcess::shutdown()            |
//! |   ↓                                   |
//! |   0                                   |
//! +---------------------------------------+
//! ```

/*!
<script>
// hide macros from doc root
document.addEventListener('DOMContentLoaded', function() {
    var macros = document.getElementById('macros');
    macros.nextElementSibling.remove();
    macros.remove();

    var side_bar_anchor = document.querySelector("li a[href='#macros']").remove();
 })
</script>
*/

// to make the proc-macro $crate substitute work in doc-tests.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui;

/// Calls `eprintln!("error: {}", format_args!($))` with `error` colored bright red and bold.
#[allow(unused)]
macro_rules! error_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "error".bright_red().bold(), format_args!($($tt)*))
    }}
}

/// Calls `eprintln!("warning: {}", format_args!($))` with `warning` colored bright yellow and bold.
#[allow(unused)]
macro_rules! warn_println {
    ($($tt:tt)*) => {{
        use colored::*;
        eprintln!("{}: {}", "warning".bright_yellow().bold(), format_args!($($tt)*))
    }}
}

#[allow(unused)]
#[cfg(debug_assertions)]
macro_rules! print_backtrace {
    () => {
        eprintln!("\n\n\n=========BACKTRACE=========\n{:?}", backtrace::Backtrace::new())
    };
}

/// Implements From and IntoVar without boilerplate.
macro_rules! impl_from_and_into_var {
    ($(
        $(#[$docs:meta])*
        fn from $(< $($T:ident  $(: $TConstrain:path)?),+ $(,)?>)? (
            $($name:ident)? // single ident OR
            $( ( // tuple deconstruct of
                $(
                    $($tuple_names:ident)? // single idents OR
                    $( ( // another tuple deconstruct of
                        $($tuple_inner_names:ident ),+ // inner idents
                    ) )?
                ),+
            ) )?
            : $From:ty) -> $To:ty
            $convert_block:block
    )+) => {
        $(
            impl $(< $($T $(: $TConstrain)?),+ >)? From<$From> for $To {
                $(#[$docs])*
                #[inline]
                fn from(
                    $($name)?
                    $( (
                        $(
                            $($tuple_names)?
                            $( (
                                $($tuple_inner_names),+
                            ) )?
                        ),+
                    ) )?
                    : $From) -> Self
                    $convert_block

            }

            impl $(< $($T $(: $TConstrain + Clone)?),+ >)? $crate::core::var::IntoVar<$To> for $From {
                type Var = $crate::core::var::OwnedVar<$To>;

                $(#[$docs])*
                fn into_var(self) -> Self::Var {
                    $crate::core::var::OwnedVar(self.into())
                }
            }
        )+
    };
}

/// Core infrastructure required for creating components and running an app.
pub mod core {
    /// Generates default implementations of [`UiNode`](crate::core::UiNode) methods.
    ///
    /// # Arguments
    ///
    /// The macro attribute takes arguments that indicate how the missing methods will be generated.
    ///
    /// ## Single Node Delegate
    ///
    /// Set this two arguments to delegate to a single node:
    ///
    /// * `delegate: &impl UiNode` - Expression that borrows the node, you can use `self` in the expression.
    /// * `delegate_mut: &mut impl UiNode` - Exclusive borrow the node.
    ///
    /// ## Multiple Nodes Delegate
    ///
    /// Set this two arguments to delegate to a widget list:
    ///
    /// * `delegate_list: & impl WidgetList` - Expression that borrows the list.
    /// * `delegate_list_mut: &mut impl WidgetList` - Exclusive borrow the list.
    ///
    /// Or, set this two arguments to delegate to a node iterator sequence:
    ///
    /// * `delegate_iter: impl Iterator<& impl UiNode>` - Expression that creates a borrowing iterator.
    /// * `delegate_iter_mut: impl Iterator<&mut impl UiNode>` - Exclusive borrowing iterator.
    ///
    /// ## Shorthand
    ///
    /// You can also use shorthand for common delegation:
    ///
    /// * `child` is the same as `delegate: &self.child, delegate_mut: &mut self.child`.
    /// * `children` is the same as `delegate_list: &self.children, delegate_list_mut: &mut self.children`.
    /// * `children_iter` is the same as `delegate_iter: self.children.iter(), delegate_iter_mut: self.children.iter_mut()`.
    ///
    /// ## None
    ///
    /// And for nodes without descendants you can use:
    /// * `none`
    ///
    /// # Validation
    /// If delegation is configured but no delegation occurs in the manually implemented methods
    /// you get the error ``"auto impl delegates call to `{}` but this manual impl does not"``.
    ///
    /// To disable this error use `#[allow_(zero_ui::missing_delegate)]` in the method or in the `impl` block.
    ///
    /// # Usage Examples
    ///
    /// Given an UI node `struct`:
    /// ```
    /// # use zero_ui::core::units::LayoutSize;
    /// struct FillColorNode<C> {
    ///     color: C,
    ///     final_size: LayoutSize,
    /// }
    /// ```
    ///
    /// In an `UiNode` trait impl block, annotate the impl block with `#[impl_ui_node(..)]` and only implement custom methods.
    ///
    /// ```
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::impl_ui_node;
    /// # use zero_ui::core::render::FrameBuilder;
    /// # use zero_ui::core::var::*;
    /// # use zero_ui::core::color::*;
    /// # use zero_ui::core::UiNode;
    /// # struct FillColorNode<C> { color: C, final_size: LayoutSize, }
    /// #[impl_ui_node(none)]
    /// impl<C: VarLocal<Rgba>> UiNode for FillColorNode<C> {
    ///     fn render(&self, frame: &mut FrameBuilder) {
    ///         let area = LayoutRect::from_size(self.final_size);
    ///         frame.push_color(area, (*self.color.get_local()).into());
    ///     }
    /// }
    /// ```
    ///
    /// Or, in a inherent impl, annotate the impl block with `#[impl_ui_node(..)]` and custom `UiNode` methods with `#[UiNode]`.
    ///
    /// ```
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::impl_ui_node;
    /// # use zero_ui::core::render::FrameBuilder;
    /// # use zero_ui::core::var::*;
    /// # use zero_ui::core::color::*;
    /// # struct FillColorNode<C> { color: C, final_size: LayoutSize, }
    /// #[impl_ui_node(none)]
    /// impl<C: VarLocal<Rgba>> FillColorNode<C> {
    ///     pub fn new(color: C) -> Self {
    ///         FillColorNode { color, final_size: LayoutSize::zero() }
    ///     }
    ///
    ///     #[UiNode]
    ///     fn render(&self, frame: &mut FrameBuilder) {
    ///         let area = LayoutRect::from_size(self.final_size);
    ///         frame.push_color(area, (*self.color.get_local()).into());
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
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::impl_ui_node;
    /// # use zero_ui::core::render::FrameBuilder;
    /// # use zero_ui::core::var::*;
    /// # use zero_ui::core::color::*;
    /// # struct FillColorNode<C> { color: C, final_size: LayoutSize }
    /// #[impl_ui_node(none)]
    /// impl<C: VarLocal<Rgba>> FillColorNode<C> {
    ///     pub fn new(color: C) -> Self {
    ///          FillColorNode { color, final_size: LayoutSize::zero() }
    ///     }
    ///
    ///     #[UiNode]
    ///     fn render(&self, frame: &mut FrameBuilder) {
    ///         let area = LayoutRect::from_size(self.final_size);
    ///         frame.push_color(area, (*self.color.get_local()).into())
    ///     }
    /// }
    /// ```
    /// Expands to:
    ///
    /// ```
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::impl_ui_node;
    /// # use zero_ui::core::var::*;
    /// # use zero_ui::core::color::*;
    /// # use zero_ui::core::context::*;
    /// # struct FillColorNode<C> { color: C, final_size: LayoutSize }
    /// impl<C: VarLocal<Rgba>> FillColorNode<C> {
    ///     pub fn new(color: C) -> Self {
    ///          FillColorNode { color, final_size: LayoutSize::zero() }
    ///     }
    /// }
    ///
    /// impl<C: VarLocal<Rgba>> zero_ui::core::UiNode for FillColorNode<C> {
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
    ///     fn measure(&mut self, available_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) -> zero_ui::core::units::LayoutSize {
    ///         let mut size = available_size;
    ///         if zero_ui::core::is_layout_any_size(size.width) {
    ///             size.width = 0.0;
    ///         }
    ///         if zero_ui::core::is_layout_any_size(size.height) {
    ///             size.height = 0.0;
    ///         }
    ///         size
    ///     }
    ///
    ///     #[inline]
    ///     fn arrange(&mut self, final_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) { }
    ///
    ///     #[inline]
    ///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
    ///         // empty here when you don't implement render.
    ///
    ///         let area = LayoutRect::from_size(self.final_size);
    ///         frame.push_color(area, (*self.color.get_local()).into())
    ///     }
    ///
    ///     #[inline]
    ///     fn render_update(&self, update: &mut zero_ui::core::render::FrameUpdate) { }
    ///
    ///     #[inline]
    ///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) { }
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
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::{impl_ui_node, UiNode};
    /// # use zero_ui::core::var::*;
    /// struct DelegateChildNode<C: UiNode> { child: C }
    ///
    /// #[impl_ui_node(child)]
    /// impl<C: UiNode> UiNode for DelegateChildNode<C> { }
    /// ```
    ///
    /// Expands to:
    ///
    /// ```
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::{impl_ui_node, UiNode};
    /// # use zero_ui::core::var::*;
    /// # struct DelegateChildNode<C: UiNode> { child: C }
    /// impl<C: UiNode> UiNode for DelegateChildNode<C> {
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
    ///     fn measure(&mut self, available_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) -> zero_ui::core::units::LayoutSize {
    ///         let child = { &mut self.child };
    ///         child.measure(available_size, ctx)
    ///     }
    ///
    ///     #[inline]
    ///     fn arrange(&mut self, final_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) {
    ///         let child = { &mut self.child };
    ///         child.arrange(final_size, ctx)
    ///     }
    ///
    ///     #[inline]
    ///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
    ///         let child = { &self.child };
    ///         child.render(frame)
    ///     }
    ///
    ///     #[inline]
    ///     fn render_update(&self, update: &mut zero_ui::core::render::FrameUpdate) {
    ///         let child = { &self.child };
    ///         child.render_update(update)
    ///     }
    ///
    ///     #[inline]
    ///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         let child = { &mut self.child };
    ///         child.deinit(ctx)
    ///     }
    /// }
    /// ```
    ///
    /// ## Delegate to many (`children` or `delegate_list, delegate_list_mut`)
    ///
    /// Generates defaults for UI components with a multiple children widgets. This is used mostly by
    /// layout panels.
    ///
    /// ### Defaults
    ///
    /// * Init, Updates: Calls the [`WidgetList`](crate::core::WidgetList) equivalent method.
    /// * Layout: Is the same size as the largest child.
    /// * Render: Z-stacks the children. Last child on top.
    ///
    /// ```
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::{impl_ui_node, UiNode, WidgetList};
    /// struct DelegateChildrenNode<C: WidgetList> {
    ///     children: C,
    /// }
    /// #[impl_ui_node(children)]
    /// impl<C: WidgetList> UiNode for DelegateChildrenNode<C> { }
    /// ```
    ///
    /// Expands to:
    ///
    /// ```
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::{impl_ui_node, UiNode, WidgetList};
    /// struct DelegateChildrenNode<C: WidgetList> {
    ///     children: C,
    /// }
    /// impl<C: WidgetList> UiNode for DelegateChildrenNode<C> {
    ///     #[inline]
    ///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         let children = { &mut self.children };
    ///         children.init_all(ctx);
    ///     }
    ///
    ///     #[inline]
    ///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         let children = { &mut self.children };
    ///         children.update_all(ctx);
    ///     }
    ///
    ///     #[inline]
    ///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         let children = { &mut self.children };
    ///         children.update_hp_all(ctx);
    ///     }
    ///
    ///     #[inline]
    ///     fn measure(&mut self, available_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) -> zero_ui::core::units::LayoutSize {
    ///         let children = { &mut self.children };
    ///         let mut size = zero_ui::core::units::LayoutSize::zero();
    ///         children.measure_all(|_, _|available_size, |_, desired_size, _| {
    ///             size = size.max(desired_size);
    ///         }, ctx);
    ///         size
    ///     }
    ///
    ///     #[inline]
    ///     fn arrange(&mut self, final_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) {
    ///         let children = { &mut self.children };
    ///         children.arrange_all(|_, _|final_size, ctx);
    ///     }
    ///
    ///     #[inline]
    ///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
    ///         let children = { &self.children };
    ///         children.render_all(|_|zero_ui::core::units::LayoutPoint::zero(), frame);
    ///     }
    ///
    ///     #[inline]
    ///     fn render_update(&self, update: &mut zero_ui::core::render::FrameUpdate) {
    ///         let children = { &self.children };
    ///         children.render_update_all(update);
    ///     }
    ///
    ///     #[inline]
    ///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         let children = { &mut self.children };
    ///         children.deinit_all(ctx);
    ///     }
    /// }
    /// ```
    ///
    ///
    /// ## Delegate to many nodes (`children_iter` or `delegate_iter, delegate_iter_mut`)
    ///
    /// Generates defaults for UI components with a multiple children nodes. This must be used only
    /// when a widget list cannot be used.
    ///
    /// ### Defaults
    ///
    /// * Init, Updates: Delegates to same method in each child.
    /// * Layout: Is the same size as the largest child.
    /// * Render: Z-stacks the children. Last child on top.
    ///
    /// ```
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::{impl_ui_node, UiNode, WidgetVec};
    /// # use zero_ui::core::var::*;
    /// # use zero_ui::core::context::*;
    /// struct DelegateChildrenNode {
    ///     children: WidgetVec,
    /// }
    /// #[impl_ui_node(children_iter)]
    /// impl UiNode for DelegateChildrenNode { }
    /// ```
    ///
    /// Expands to:
    ///
    /// ```
    /// # use zero_ui::core::units::*;
    /// # use zero_ui::core::{impl_ui_node, UiNode, WidgetVec};
    /// # use zero_ui::core::var::*;
    /// # use zero_ui::core::context::*;
    /// # struct DelegateChildrenNode { children: WidgetVec }
    /// impl UiNode for DelegateChildrenNode {
    ///     #[inline]
    ///     fn init(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         for child in { self.children.iter_mut() } {
    ///             child.init(ctx)
    ///         }
    ///     }
    ///
    ///     #[inline]
    ///     fn update(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         for child in { self.children.iter_mut() } {
    ///             child.update(ctx)
    ///         }
    ///     }
    ///
    ///     #[inline]
    ///     fn update_hp(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         for child in { self.children.iter_mut() } {
    ///             child.update_hp(ctx)
    ///         }
    ///     }
    ///
    ///     #[inline]
    ///     fn measure(&mut self, available_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) -> zero_ui::core::units::LayoutSize {
    ///         let mut size = zero_ui::core::units::LayoutSize::zero();
    ///         for child in { self.children.iter_mut() } {
    ///            size = child.measure(available_size, ctx).max(size);
    ///         }
    ///         size
    ///     }
    ///
    ///     #[inline]
    ///     fn arrange(&mut self, final_size: zero_ui::core::units::LayoutSize, ctx: &mut zero_ui::core::context::LayoutContext) {
    ///         for child in { self.children.iter_mut() } {
    ///             child.arrange(final_size, ctx)
    ///         }
    ///     }
    ///
    ///     #[inline]
    ///     fn render(&self, frame: &mut zero_ui::core::render::FrameBuilder) {
    ///         for child in { self.children.iter() } {
    ///             child.render(frame)
    ///         }
    ///     }
    ///
    ///     #[inline]
    ///     fn render_update(&self, update: &mut zero_ui::core::render::FrameUpdate) {
    ///         for child in { self.children.iter() } {
    ///             child.render_update(update)
    ///         }
    ///     }
    ///
    ///     #[inline]
    ///     fn deinit(&mut self, ctx: &mut zero_ui::core::context::WidgetContext) {
    ///         for child in { self.children.iter_mut() } {
    ///             child.deinit(ctx)
    ///         }
    ///     }
    /// }
    /// ```
    pub use zero_ui_core::impl_ui_node;

    /// Expands a function to a widget property module.
    ///
    /// # Arguments
    ///
    /// The macro attribute takes arguments that configure how the property can be used in widgets.
    ///
    /// **Required**
    ///
    /// The first argument is required and indicates when the property is set in relation to the other properties in a widget.
    /// The valid values are: [`context`](#context), [`event`](#event), [`outer`](#outer), [`size`](#size), [`inner`](#inner) or
    /// [`capture_only`](#capture_only).
    ///
    /// **Optional**
    ///
    /// Optional arguments can be set after the required, they use the `name: value` syntax. Currently there is only one
    /// [`allowed_in_when`](#when-conditions).
    ///
    /// # Function
    ///
    /// The macro attribute must be set in a stand-alone function that sets the property by modifying the UI node tree.
    ///
    /// ## Arguments and Output
    ///
    /// The function argument and return type requirements are the same for normal properties (not `capture_only`).
    ///
    /// ### Normal Properties
    ///
    /// Normal properties must take at least two arguments, the first argument is the child [`UiNode`](crate::core::UiNode), the other argument(s)
    /// are the property values. The function must return a type that implements `UiNode`. The first argument must support any type that implements
    /// `UiNode`. All of these requirements are validated at compile time.
    ///
    /// ```
    /// # fn main() { }
    /// use zero_ui::core::{property, UiNode, impl_ui_node, var::{Var, IntoVar}, context::WidgetContext};
    ///
    /// struct MyNode<C, V> { child: C, value: V }
    /// #[impl_ui_node(child)]
    /// impl<C: UiNode, V: Var<&'static str>> UiNode for MyNode<C, V> {
    ///     fn init(&mut self, ctx: &mut WidgetContext) {
    ///         self.child.init(ctx);
    ///         println!("{}", self.value.get(ctx.vars));
    ///     }
    /// }
    ///
    /// /// Property docs.
    /// #[property(context)]
    /// pub fn my_property(child: impl UiNode, value: impl IntoVar<&'static str>) -> impl UiNode {
    ///     MyNode { child, value: value.into_var() }
    /// }
    /// ```
    ///
    /// ### `capture_only`
    ///
    /// Capture-only properties do not modify the UI node tree, they exist only as a named bundle of arguments that widgets capture to use internally.
    /// At least one argument is required. The return type must be never (`!`) and the property body must be empty.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{property, var::IntoVar, text::Text};
    /// /// Property docs.
    /// #[property(capture_only)]
    /// pub fn my_property(value: impl IntoVar<Text>) -> ! { }
    /// ```
    /// ## Limitations
    ///
    /// There are some limitations to what kind of function can be used:
    ///
    /// * Only standalone safe functions are supported, type methods, `extern` functions and `unsafe` are not supported.
    /// * Only sized 'static types are supported.
    /// * All stable generics are supported, generic bounds, impl trait and where clauses, const generics are not supported.
    /// * Const functions are not supported. You need generics to support any type of UI node but generic const functions are unstable.
    /// * Async functions are not supported.
    /// * Only the simple argument pattern `name: T` are supported. Destructuring arguments or discard (_) are not supported.
    ///
    /// ## Name
    ///
    /// The property name follows some conventions that are enforced at compile time.
    ///
    /// * `on_` prefix: Can only be used for `event` or `capture_only` properties and must take only a single event handler value.
    /// * `is_` prefix: Can only take a single [`StateVar`](crate::core::var::StateVar) value.
    ///
    /// # Priority
    ///
    /// Except for `capture_only` the other configurations indicate the priority that the property must be applied to form a widget.
    ///
    /// ## `context`
    ///
    /// The property is applied after all other so that they can setup information associated with the widget that the other properties
    /// can use. Context variables and widget state use this priority.
    ///
    /// You can easily implement this properties using [`with_context_var`](crate::properties::with_context_var)
    /// and [`set_widget_state`](crate::properties::set_widget_state).
    ///
    /// ## `event`
    ///
    /// Event properties are the next priority, they are set after all others except `context`, this way events can be configured by the
    /// widget context properties but also have access to the widget visual they contain.
    ///
    /// It is strongly encouraged that the event handler signature matches the one from [`on_event`](crate::core::event::on_event).
    ///
    /// ## `outer`
    ///
    /// Properties that shape the visual outside of the widget, the [`margin`](fn@crate::properties::margin) property is an example.
    ///
    /// ## `size`
    ///
    /// Properties that set the widget visual size. Most widgets are sized automatically by their content, if the size is configured by a user value
    /// the property has this priority.
    ///
    /// ## `inner`
    ///
    /// Properties that are set first, so they end-up inside of all other widget properties. Most of the properties that render use this priority.
    ///
    /// # When Conditions
    ///
    /// Most properties can be used in widget when condition expressions, by default all properties that don't have the `on_` prefix are allowed.
    /// This can be overridden by setting the optional argument `allowed_in_when`.
    ///
    /// ## State Probing
    ///
    /// Properties with the `is_` prefix are special, they output information about the widget instead of shaping it. They are automatically set
    /// to a new probing variable when used in an widget when condition expression.
    pub use zero_ui_core::property;

    /// Declares a new widget macro and module.
    ///
    /// Widgets are a bundle of [property blocks](#property-blocks), [when blocks](#when-blocks) and [initialization functions](#initialization-functions).
    ///
    /// # Header
    ///
    /// The widget header declares the widget name, [documentation](#attributes), [visibility](#visibility) and what other widgets and mix-ins
    /// are [inherited](#inheritance) into the new one.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # // use zero_ui::core::widgets::{container, mixins::focusable_mixin};
    /// widget! {
    ///     /// Widget documentation.
    ///     pub button;//: container + focusable_mixin;
    /// }
    /// ```
    ///
    /// ## Attributes
    ///
    /// All attributes are transferred to the generated module. Conditional compilation (`#[cfg]`) attributes are also applied to the generated macro.
    /// Extra documentation about the widget properties is auto-generated and added to the module as well.
    ///
    /// ```
    /// # use zero_ui::core::widget;
    /// widget! {
    ///     /// Widget documentation.
    ///     #[cfg(debug_assertions)]
    ///     widget_name;
    /// }
    /// ```
    ///
    /// ## Visibility
    ///
    /// The visibility is transferred to the widget module and macro and supports all visibility configurations.
    ///
    /// ```
    /// # use zero_ui::core::widget;
    /// widget! {
    ///     pub(crate) widget_name;
    /// }
    /// ```
    ///
    /// ## Inheritance
    ///
    /// Widgets can optionally 'inherit' from other widgets and widget mix-ins.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// widget! {
    ///     pub foo;
    /// }
    /// ```
    ///
    /// Widgets inheritance works by 'importing' all properties, when blocks and init functions into the new widget.
    /// All widgets automatically inherit from [`implicit_mixin`](mod@crate::core::widget_base::implicit_mixin) (after all other inherits).
    ///
    /// ### Conflict Resolution
    ///
    /// Properties and functions of the same name are overwritten by the left-most import or by the new widget declaration.
    ///
    /// When blocks with conditions that are no longer valid are removed.
    ///
    /// # Property Blocks
    ///
    /// Property blocks contains a list of [property declarations](#property-declarations) grouped by the [target](#target) of the properties.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// widget! {
    ///     pub foo;
    ///
    ///     default {
    ///         enabled: false;
    ///     }
    /// }
    /// ```
    ///
    /// # Target
    ///
    /// The property targets are selected by the keyword used to open a property block, `default` properties are applied
    /// to the widget normally, `default_child` properties are applied first so that they affect the widget child node before
    /// all other properties.
    ///
    /// ## Property Declarations
    ///
    /// Properties are declared by their [name](#name-resolution) follow by optional [remapping](#remapping), default or
    /// special value and terminated by semi-colon (`;`). They can also have documentation attributes.
    ///
    /// ### Name Resolution
    ///
    /// If a property with the same name is inherited that is the property, if not then is is assumed that a
    /// [`property`](crate::core::property) module is with the same name is imported.
    ///
    /// You can only use single names, module paths are not allowed. You can only declare a property with the same name once,
    ///
    /// ### Remapping
    ///
    /// New properties can map to other properties, meaning the other property is applied when the new property is used. This is also
    /// the only way to apply the same property twice.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{widget, property, UiNode, var::IntoVar};
    /// # #[property(context)]
    /// # fn other_property(child: impl UiNode, value: impl IntoVar<bool>) -> impl UiNode { child }
    /// widget! {
    /// # widget_name;
    ///     //..
    ///     
    ///     default {
    ///         new_property -> other_property;
    ///     }
    /// }
    /// ```
    ///
    /// ### Default Value
    ///
    /// Properties can have a default value. If they do the property is applied automatically during widget
    /// instantiation using the default value if the user does not set the property.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{widget, property, UiNode, var::IntoVar, text::Text};
    /// # #[property(context)]
    /// # pub fn my_property(child: impl UiNode, value: impl IntoVar<Text>) -> impl UiNode { child }
    /// widget! {
    /// # widget_name;
    ///     //..
    ///     
    ///     default {
    ///         my_property: "value";
    ///         foo -> my_property: "value";
    ///     }
    /// }
    /// ```
    ///
    /// Properties without a default value are only applied if the user sets then.
    ///
    /// ### `required!`
    ///
    /// Properties declared with the `required!` special value must be set by the user during widget initialization.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::widget;
    /// # use zero_ui::core::widget_base::enabled as on_click;
    /// widget! {
    /// # widget_name;
    ///     //..
    ///     
    ///     default {
    ///         on_click: required!;
    ///     }
    /// }
    /// ```
    ///
    /// [Captured](#initialization-functions) properties are also required.
    ///
    /// ### `unset!`
    ///
    /// Removes an inherited property by redeclaring then with the `unset!` special value.
    ///
    /// # When Blocks
    ///
    /// When blocks assign properties when a condition is true, the condition references properties and is always updated
    /// if the referenced values are [vars](crate::core::var::Var).
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{widget, UiNode, property, color::{rgb, Rgba}, var::{IntoVar, StateVar}};
    /// # #[property(inner)] pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode { child }
    /// # #[property(context)] pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode { child }
    /// widget! {
    /// # widget_name;
    /// # default { background_color: rgb(0, 0, 0); }
    ///     //..
    ///     
    ///     when self.is_pressed {
    ///         background_color: rgb(0.3, 0.3, 0.3);
    ///     }
    /// }
    /// ```
    ///
    /// ## Condition
    ///
    /// The `when` condition is an expression similar to the `if` condition. In it you can reference properties by using the `self.` prefix, at least one
    /// property reference is required.
    ///
    /// If the first property argument is referenced by `self.property`, to reference other arguments you can use `self.property.1` or `self.property.arg_name`.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{widget, UiNode, property, color::{rgb, Rgba}, text::Text, var::{IntoVar, StateVar}};
    /// # #[property(inner)] pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode { child }
    /// # #[property(inner)] pub fn title(child: impl UiNode, title: impl IntoVar<Text>) -> impl UiNode { child }
    /// # #[property(context)] pub fn is_pressed(child: impl UiNode, state: StateVar) -> impl UiNode { child }
    /// widget! {
    /// # widget_name;
    /// # default { title: "value"; background_color: rgb(0, 0, 0); }
    ///     //..
    ///     
    ///     when self.title == "value" && self.is_pressed {
    ///         background_color: rgb(255, 0, 255);
    ///     }
    /// }
    /// ```
    ///
    /// If the property arguments are [vars](crate::core::var::Var) the when condition is reevaluated after any variable changes.
    ///
    /// The referenced properties must have a default value, be [`required`](#required) or be a [state property](crate::core::property#state-probing).
    /// If the user [unsets](#unset) a referenced property the whole when block is not instantiated.
    ///
    /// ## Assigns
    ///
    /// Inside the when block you can assign properties using `property_name: "value";`.  
    /// The assigned property must have a default value or be [`required`](#required).
    /// If the user [unsets](#unset) the property it is removed from the when block.
    ///
    /// # Initialization Functions
    ///
    /// Every widget has two initialization functions, [`new_child`](#new_child) and [`new`](#new). They are like other rust standalone
    /// functions except the input arguments have no explicit type.
    ///
    /// ## `new_child`
    ///
    /// Initializes the inner most node of the widget.
    ///
    /// ```
    /// # fn main() { }
    /// # use zero_ui::core::{widget, UiNode};
    /// // widget! {
    /// //     pub container;
    /// //     
    /// //     default_child {
    /// //         content -> widget_child: required!;
    /// //     }
    /// //     
    /// //     fn new_child(content) -> impl UiNode {
    /// //         content.unwrap()
    /// //     }
    /// // }
    /// ```
    ///
    /// The function must return a type that implements [`UiNode`](crate::core::UiNode). It has no required arguments but
    /// can [capture](#property-capturing) property arguments.
    ///
    /// If omitted the left-most inherited widget `new_child` is used, if the widget only inherits from mix-ins
    /// [`default_widget_new_child`](crate::core::widget_base::default_widget_new_child) is used.
    ///
    /// ## `new`
    ///
    /// Initializes the outer wrapper of the widget.
    ///
    /// ```
    /// # fn main() { }
    /// # // use zero_ui::core::{widget, color::rgb, var::IntoVar, WidgetId, text::Text, color::Rgba};
    /// # // use zero_ui::core::properties::title;
    /// # // use zero_ui::core::properties::background::background_color;
    /// # // use zero_ui::core::widgets::container;
    /// # // pub struct Window { } impl Window { pub fn new(child: impl crate::core::UiNode, id: impl IntoVar<WidgetId>, title: impl IntoVar<Text>, background_color: impl IntoVar<Rgba>) -> Self { todo!() } }
    /// // widget! {
    /// //     pub window: container;
    /// //     
    /// //     default {
    /// //         title: "New Window";
    /// //         background_color: rgb(1.0, 1.0, 1.0);
    /// //     }
    /// //     
    /// //     fn new(child, id, title, background_color) -> Window {
    /// //         Window::new(child, id.unwrap(), title.unwrap(), background_color.unwrap())
    /// //     }
    /// // }
    /// ```
    ///
    /// The function can return any type, but if the type does not implement [`Widget`](crate::core::Widget)
    /// it cannot be the content of most other container widgets.
    ///
    /// The first argument is required, it can have any name but the type is `impl UiNode`,
    /// it contains the UI node tree formed by the widget properties and `new_child`.
    /// After the first argument it can [capture](#property-capturing) property arguments.
    ///
    /// If omitted the left-most inherited widget `new` is used, if the widget only inherits from mix-ins
    /// [`default_widget_new`](crate::core::widget_base::default_widget_new) is used.
    ///
    /// ## Property Capturing
    ///
    /// The initialization functions can capture properties by listing then in the function input. The argument type is an `impl property_name::Args`.
    ///
    /// Captured properties are not applied during widget instantiation, the arguments are moved to the function that captured then.
    /// Because they are required for calling the initialization functions they are automatically marked 'required'.
    ///
    /// # Internals
    ///
    /// TODO details of internal code generated.
    pub use zero_ui_core::widget;

    /// Declares a new widget mix-in module.
    ///
    /// Widget mix-ins can be inherited by other mix-ins and widgets, but cannot be instantiated.
    ///
    /// # Syntax
    ///
    /// The syntax is the same as in [`widget!`](macro.widget.html), except
    /// you cannot write the `new` and `new_child` functions.
    ///
    /// ```
    /// # // fn main() { }
    /// # // use zero_ui::core::prelude::new_widget::{widget_mixin, focusable, border, is_focused_hgl, foreground_highlight, SideOffsets};
    /// # // use zero_ui::core::widgets::mixins::{FocusHighlightDetailsVar, FocusHighlightWidthsVar, FocusHighlightOffsetsVar};
    /// // widget_mixin! {
    /// //     /// Focusable widget mix-in. Enables keyboard focusing on the widget and adds a focused
    /// //     /// highlight border.
    /// //     pub focusable_mixin;
    /// //
    /// //     default {
    /// //
    /// //         /// Enables keyboard focusing in the widget.
    /// //         focusable: true;
    /// //
    /// //         /// A border overlay that is visible when the widget is focused.
    /// //         focus_highlight -> foreground_highlight: {
    /// //             widths: SideOffsets::new_all(0.0),
    /// //             offsets: SideOffsets::new_all(0.0),
    /// //             details: FocusHighlightDetailsVar
    /// //         };
    /// //     }
    /// //
    /// //     when self.is_focused_hgl {
    /// //         focus_highlight: {
    /// //             widths: FocusHighlightWidthsVar,
    /// //             offsets: FocusHighlightOffsetsVar,
    /// //             details: FocusHighlightDetailsVar
    /// //         };
    /// //     }
    /// // }
    /// ```
    ///
    /// # Expands to
    ///
    /// The macro expands to a module declaration with the same name and visibility.
    ///
    /// All documentation is incorporated into specially formatted HTML that uses the
    /// rust-doc stylesheets to present the widget mix-in as a first class item. See
    /// [`focusable_mixin`](mod@crate::widgets::mixins::focusable_mixin) for an example.
    ///
    /// ## Internals
    ///
    /// In the generated module some public but doc-hidden items are generated, this items
    /// are used during widget instantiation.
    pub use zero_ui_core::widget_mixin;

    pub use zero_ui_core::widget2;
    pub use zero_ui_core::widget_mixin2;

    pub use zero_ui_core::*;
}

pub mod properties;
pub mod widgets;

/// All the types you need to build an app.
///
/// Use glob import (`*`) to quickly start implementing an application.
///
/// ```no_run
/// use zero_ui::prelude::*;
///
/// App::default().run_window(|_| {
///     todo!()
/// })
/// ```
///
/// # Other Preludes
///
/// There are prelude modules for other contexts, [`new_property`](crate::prelude::new_property) for
/// new properties, [`new_widget`](crate::prelude::new_widget) for creating widgets.
pub mod prelude {
    pub use crate::core::{
        app::{App, ElementState},
        color::{
            self, blur, brightness, colors, contrast, drop_shadow, grayscale, hex, hsl, hsla, hue_rotate, opacity, rgb, rgba, saturate,
            sepia, Rgba,
        },
        context::WidgetContext,
        focus::{DirectionalNav, Focus, TabIndex, TabNav},
        gesture::{shortcut, GestureKey, Shortcut, Shortcuts},
        gradient::{stops, ExtendMode, GradientStop, GradientStops},
        keyboard::{Key, ModifiersState},
        mouse::MouseButton,
        render::WidgetPath,
        service::{AppServices, WindowServices},
        sync::Sync,
        text::{
            font_features::{
                CapsVariant, CharVariant, CnVariant, EastAsianWidth, FontPosition, FontStyleSet, JpVariant, NumFraction, NumSpacing,
                NumVariant,
            },
            formatx, FontFeatures, FontName, FontNames, FontStretch, FontStyle, FontWeight, Fonts, Hyphens, LineBreak, Text, TextAlign,
            TextTransformFn, ToText, WhiteSpace, WordBreak,
        },
        ui_vec,
        units::{
            rotate, skew, translate, Alignment, AngleUnits, FactorUnits, Length, LengthUnits, Line, LineFromTuplesBuilder, LineHeight,
            Point, Rect, RectFromTuplesBuilder, SideOffsets, Size, TimeUnits,
        },
        var::{merge_var, state_var, switch_var, var, var_from, RcVar, Var, VarObj, Vars},
        window::{AppRunWindow, CursorIcon, StartPosition, Window, Windows},
        UiNode, Widget, WidgetId, WidgetList, WidgetVec,
    };

    pub use crate::properties::*;
    pub use crate::widgets::*;

    pub use crate::properties::background::{background, *};
    pub use crate::properties::border::*;
    pub use crate::properties::events::{focus::*, gesture::*, keyboard::*};
    pub use crate::properties::filters::*;
    pub use crate::properties::focus::*;
    pub use crate::properties::foreground::{foreground, *};
    pub use crate::properties::size::{size, *};
    pub use crate::properties::states::*;
    pub use crate::properties::text_theme::{
        font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, text_align, text_color,
        text_transform, word_spacing,
    };
    pub use crate::properties::transform::{transform, *};

    pub use crate::widgets::layouts::*;
    pub use crate::widgets::text::{text, *};

    /// All the types you need to declare a new property.
    ///
    /// Use glob import (`*`) to quickly start implementing properties.
    pub mod new_property {
        pub use crate::core::app::ElementState;
        pub use crate::core::color::{self, *};
        pub use crate::core::context::*;
        pub use crate::core::event::*;
        pub use crate::core::gesture::*;
        pub use crate::core::render::*;
        pub use crate::core::text::Text;
        pub use crate::core::units::{self, *};
        pub use crate::core::var::*;
        pub use crate::core::widget_base::{IsEnabled, WidgetEnabledExt};
        pub use crate::core::{
            impl_ui_node, is_layout_any_size, property, ui_vec, FillUiNode, UiNode, UiNodeList, Widget, WidgetId, WidgetList, WidgetVec,
            LAYOUT_ANY_SIZE,
        };
        pub use crate::properties::{set_widget_state, with_context_var};
    }

    /// All the types you need to declare a new widget or widget mix-in.
    ///
    /// Use glob import (`*`) to quickly start implementing widgets.
    pub mod new_widget {
        pub use crate::core::color::*;
        pub use crate::core::context::*;
        pub use crate::core::render::*;
        pub use crate::core::text::*;
        pub use crate::core::units::*;
        pub use crate::core::var::*;
        pub use crate::core::{
            impl_ui_node, is_layout_any_size, ui_vec, widget, widget2, widget_mixin, FillUiNode, UiNode, UiNodeList, Widget, WidgetId,
            WidgetList, WidgetVec, LAYOUT_ANY_SIZE,
        };
        pub use crate::properties::background::{background, *};
        pub use crate::properties::border::{border, *};
        pub use crate::properties::capture_only::*;
        pub use crate::properties::events::{self, gesture::*, keyboard::*};
        pub use crate::properties::filters::*;
        pub use crate::properties::focus::focusable;
        pub use crate::properties::focus::*;
        pub use crate::properties::foreground::{foreground, *};
        pub use crate::properties::size::{size, *};
        pub use crate::properties::states::*;
        pub use crate::properties::text_theme::{
            font_family, font_size, font_stretch, font_style, font_weight, letter_spacing, line_height, tab_length, text_align, text_color,
            text_transform, word_spacing,
        };
        pub use crate::properties::transform::{transform, *};
        pub use crate::properties::*;
        pub use crate::widgets::container;
        pub use crate::widgets::mixins::*;
    }
}

// see test-crates/no-direct-deps
#[doc(hidden)]
pub fn crate_reference_called() -> bool {
    true
}
#[doc(hidden)]
#[macro_export]
macro_rules! crate_reference_call {
    () => {
        $crate::crate_reference_called()
    };
}
