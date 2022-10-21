use std::{any::Any, cell::RefCell, mem, thread::LocalKey};

use crate::{
    context::{InfoContext, LayoutContext, MeasureContext, RenderContext, WidgetContext, WidgetUpdates},
    event::EventUpdate,
    render::{FrameBuilder, FrameUpdate},
    ui_node, units,
    widget_info::{WidgetInfoBuilder, WidgetLayout},
    widget_instance::UiNode,
};

///<span data-del-macro-root></span> Declares new thread local static that facilitates sharing *contextual* values.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::context_value;
/// context_value! {
///     /// A public documented value.
///     pub static FOO: u8 = 10u8;
///
///     // A private value.
///     static BAR: String = "Into!";
/// }
/// ```
///
/// # Default Value
///
/// All contextual values must have a fallback value that is used when no context is loaded.
///
/// The default value is instantiated once per thread, the expression can be any static value that converts [`Into<T>`].
///
/// # Usage
///
/// After you declare the contextual value you can use it by loading a context, calling a closure and inside it *visiting* the value.
///
/// ```
/// # use zero_ui_core::context_value;
/// context_value! { static FOO: String = "default"; }
///
/// fn print_value() {
///     FOO.with(|val| println!("value is {val}!"));
/// }
///
/// let mut value = Some(String::from("other"));
/// FOO.with_context(&mut value, || {
///     print!("in context, ");
///     print_value();
/// });
///
/// print!("out of context, ");
/// print_value();
/// ```
///
/// The example above prints:
///
/// ```text
/// in context, value is other!
/// out of context, value is default!
/// ```
#[macro_export]
macro_rules! context_value {
($(
    $(#[$attr:meta])*
    $vis:vis static $NAME:ident: $Type:ty = $default:expr;
)+) => {$(
    $crate::paste! {
        std::thread_local! {
            #[doc(hidden)]
            static [<$NAME _LOCAL>]: $crate::context::ContextValueData<$Type> = $crate::context::ContextValueData::init($default);
        }
    }

    $(#[$attr])*
    $vis static $NAME: $crate::context::ContextValue<$Type> = paste::paste! { $crate::context::ContextValue::new(&[<$NAME _LOCAL>]) };
)+}
}

#[doc(hidden)]
pub struct ContextValueData<T: Any> {
    value: RefCell<T>,
}
impl<T: Any> ContextValueData<T> {
    pub fn init(default: impl Into<T>) -> Self {
        Self {
            value: RefCell::new(default.into()),
        }
    }
}

/// Represents value that can only be read in a context.
///
/// See [`context_value!`] for more details.
pub struct ContextValue<T: Any> {
    local: &'static LocalKey<ContextValueData<T>>,
}

#[doc(hidden)]
pub mod option {
    pub trait Option<T> {
        fn get(&self) -> &std::option::Option<T>;
        fn get_mut(&mut self) -> &mut std::option::Option<T>;
    }

    impl<T> Option<T> for std::option::Option<T> {
        fn get(&self) -> &std::option::Option<T> {
            self
        }
        fn get_mut(&mut self) -> &mut std::option::Option<T> {
            self
        }
    }
}

impl<T: Any> ContextValue<T> {
    #[doc(hidden)]
    pub const fn new(local: &'static LocalKey<ContextValueData<T>>) -> Self {
        ContextValue { local }
    }

    /// Calls `f` with read-only access to the contextual value.
    ///
    /// Returns the result of `f`.
    pub fn with<R>(self, f: impl FnOnce(&T) -> R) -> R {
        self.local.with(|l| f(&*l.value.borrow()))
    }

    /// Like [`with`] but only calls `f` if the value is `Some(I)`.
    ///
    /// [`with`]: Self::with
    pub fn with_opt<I: Any, R>(self, f: impl FnOnce(&I) -> R) -> Option<R>
    where
        T: option::Option<I>,
    {
        self.with(|opt| opt.get().as_ref().map(f))
    }

    /// Calls `f` with exclusive access to the contextual value.
    ///
    /// Returns the result of `f`.
    pub fn with_mut<R>(self, f: impl FnOnce(&mut T) -> R) -> R {
        self.local.with(|l| f(&mut *l.value.borrow_mut()))
    }

    /// Like [`with_mut`] but only calls `f` if the value is `Some(I)`.
    ///
    /// [`with_mut`]: Self::with_mut
    pub fn with_mut_opt<I: Any, R>(self, f: impl FnOnce(&mut I) -> R) -> Option<R>
    where
        T: option::Option<I>,
    {
        self.with_mut(|opt| opt.get_mut().as_mut().map(f))
    }

    /// Runs `action` while the `value` is moved into context, restores the `value` if `action` does not panic.
    ///
    /// Returns the result of `action`, panics if `value` is `None`.
    pub fn with_context<R>(self, value: &mut Option<T>, action: impl FnOnce() -> R) -> R {
        let prev = self.set_context(value.take().expect("no contextual value to load"));
        let r = action();
        *value = Some(self.set_context(prev));
        r
    }

    /// Like [`with_context`], but for context values that are `T: Option<I>`.
    ///
    /// [`with_context`]: Self::with_context
    pub fn with_context_opt<I: Any, R>(self, value: &mut Option<I>, action: impl FnOnce() -> R) -> R
    where
        T: option::Option<I>,
    {
        let prev = self.set_context_opt(value.take());
        let r = action();
        *value = self.set_context_opt(prev);
        r
    }

    /// Get a clone of the current contextual value.
    pub fn get(self) -> T
    where
        T: Clone,
    {
        self.with(Clone::clone)
    }

    fn set_context(&self, val: T) -> T {
        self.local.with(|l| mem::replace(&mut *l.value.borrow_mut(), val))
    }

    fn set_context_opt<I>(&self, val: Option<I>) -> Option<I>
    where
        T: option::Option<I>,
    {
        self.local.with(|l| mem::replace(l.value.borrow_mut().get_mut(), val))
    }
}

impl<T: Any> Clone for ContextValue<T> {
    fn clone(&self) -> Self {
        Self { local: self.local }
    }
}
impl<T: Any> Copy for ContextValue<T> {}

/// Helper for declaring nodes that sets a context value.
pub fn with_context_value<T: Any>(child: impl UiNode, context: ContextValue<T>, value: impl Into<T>) -> impl UiNode {
    #[ui_node(struct WithContextValueNode<T: Any> {
        child: impl UiNode,
        context: ContextValue<T>,
        value: RefCell<Option<T>>,
    })]
    impl WithContextValueNode {
        fn with<R>(&self, mtd: impl FnOnce(&T_child) -> R) -> R {
            let mut value = self.value.borrow_mut();
            self.context.with_context(&mut value, move || mtd(&self.child))
        }

        fn with_mut<R>(&mut self, mtd: impl FnOnce(&mut T_child) -> R) -> R {
            let value = self.value.get_mut();
            self.context.with_context(value, || mtd(&mut self.child))
        }

        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.with_mut(|c| c.init(ctx))
        }

        #[UiNode]
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.with_mut(|c| c.deinit(ctx))
        }

        #[UiNode]
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            self.with(|c| c.info(ctx, info))
        }

        #[UiNode]
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            self.with_mut(|c| c.event(ctx, update))
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.with_mut(|c| c.update(ctx, updates))
        }

        #[UiNode]
        fn measure(&self, ctx: &mut MeasureContext) -> units::PxSize {
            self.with(|c| c.measure(ctx))
        }

        #[UiNode]
        fn layout(&mut self, ctx: &mut LayoutContext, wl: &mut WidgetLayout) -> units::PxSize {
            self.with_mut(|c| c.layout(ctx, wl))
        }

        #[UiNode]
        fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
            self.with(|c| c.render(ctx, frame))
        }

        #[UiNode]
        fn render_update(&self, ctx: &mut RenderContext, update: &mut FrameUpdate) {
            self.with(|c| c.render_update(ctx, update))
        }
    }
    WithContextValueNode {
        child,
        context,
        value: RefCell::new(Some(value.into())),
    }
}
