use proc_macro_hack::proc_macro_hack;

pub use zero_ui_proc_macros::{impl_ui_node, property, widget, widget_mixin, widget_stage2, widget_stage3};

#[proc_macro_hack(support_nested)]
pub use zero_ui_proc_macros::widget_new;

/// Declares new [`StateKey`](zero_ui::core::context::StateKey) types.
#[macro_export]
macro_rules! state_key {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty;)+) => {$(
        $(#[$outer])*
        /// # StateKey
        /// This `struct` is a [`StateKey`](zero_ui::core::context::StateKey).
        $vis struct $ident;

        impl zero_ui::core::context::StateKey for $ident {
            type Type = $type;
        }
    )+};
}

/// Declares new [`EventArgs`](crate::core::event::EventArgs) types.
///
/// # Example
/// ```
/// use zero_ui::core::render::WidgetPath;
///
/// event_args! {
///     /// My event arguments.
///     pub struct MyEventArgs {
///         /// My argument.
///         pub arg: String,
///         /// My event target.
///         pub target: WidgetPath,
///
///         ..
///
///         /// If `ctx.widget_id` is in the `self.target` path.
///         fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
///             self.target.contains(ctx.widget_id)
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui::core::render::WidgetPath;
/// #
/// /// My event arguments.
/// #[derive(Debug, Clone)]
/// pub struct MyEventArgs {
///     /// When the event happened.
///     pub timestamp: std::time::Instant,
///     /// My argument.
///     pub arg: String,
///     /// My event target.
///     pub target: WidgetPath,
/// }
///
/// impl MyEventArgs {
///     #[inline]
///     pub fn new(
///         timestamp: impl Into<std::time::Instant>,
///         arg: impl Into<String>,
///         target: impl Into<WidgetPath>,
///     ) -> Self {
///         MyEventArgs {
///             timestamp: timestamp.into(),
///             arg: arg.into(),
///             target: target.into(),
///         }
///     }
///
///     /// Arguments for event that happened now (`Instant::now`).
///     #[inline]
///     pub fn now(arg: impl Into<String>, target: impl Into<WidgetPath>) -> Self {
///         Self::new(std::time::Instant::now(), arg, target)
///     }
/// }
///
/// impl zero_ui::core::event::EventArgs for MyEventArgs {
///     #[inline]
///     fn timestamp(&self) -> std::time::Instant {
///         self.timestamp
///     }
///
///     #[inline]
///     /// If `ctx.widget_id` is in the `self.target` path.
///     fn concerns_widget(&self, ctx: &mut zero_ui::core::context::WidgetContext) -> bool {
///         self.target.contains(ctx.widget_id)
///     }
/// }
/// ```
#[macro_export]
macro_rules! event_args {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:ident: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {$(
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            /// When the event happened.
            pub timestamp: std::time::Instant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*
        }
        impl $Args {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }
        }
        impl zero_ui::core::event::EventArgs for $Args {
            #[inline]
            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }

            #[inline]
            $(#[$concerns_widget_outer])*
            fn concerns_widget(&$self, $ctx: &mut zero_ui::core::context::WidgetContext) -> bool {
                $($concerns_widget)+
            }
        }
    )+};

    // match discard WidgetContext in concerns_widget.
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, _: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {
        zero_ui::event_args! { $(

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, _ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }

        )+ }
    };
}

/// Declares new [`CancelableEventArgs`](crate::core::event::CancelableEventArgs) types.
///
/// Same syntax as [`event_args!`](macro.event_args.html) but the generated args is also cancelable.
///
/// # Example
/// ```
/// use zero_ui::core::render::WidgetPath;
///
/// cancelable_event_args! {
///     /// My event arguments.
///     pub struct MyEventArgs {
///         /// My argument.
///         pub arg: String,
///         /// My event target.
///         pub target: WidgetPath,
///
///         ..
///
///         /// If `ctx.widget_id` is in the `self.target` path.
///         fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
///             self.target.contains(ctx.widget_id)
///         }
///     }
///
///     // multiple structs can be declared in the same call.
///     // pub struct MyOtherEventArgs { /**/ }
/// }
/// ```
///
/// Expands to:
///
/// ```
/// # use zero_ui::core::render::WidgetPath;
/// #
/// /// My event arguments.
/// #[derive(Debug, Clone)]
/// pub struct MyEventArgs {
///     /// When the event happened.
///     pub timestamp: std::time::Instant,
///     /// My argument.
///     pub arg: String,
///     /// My event target.
///     pub target: WidgetPath,
///
///     cancel: std::rc::Rc<std::cell::Cell<bool>>
/// }
///
/// impl MyEventArgs {
///     #[inline]
///     pub fn new(
///         timestamp: impl Into<std::time::Instant>,
///         arg: impl Into<String>,
///         target: impl Into<WidgetPath>,
///     ) -> Self {
///         MyEventArgs {
///             timestamp: timestamp.into(),
///             arg: arg.into(),
///             target: target.into(),
///             cancel: std::rc::Rc::default()
///         }
///     }
///
///     /// Arguments for event that happened now (`Instant::now`).
///     #[inline]
///     pub fn now(arg: impl Into<String>, target: impl Into<WidgetPath>) -> Self {
///         Self::new(std::time::Instant::now(), arg, target)
///     }
/// }
///
/// impl zero_ui::core::event::EventArgs for MyEventArgs {
///     #[inline]
///     fn timestamp(&self) -> std::time::Instant {
///         self.timestamp
///     }
///
///     #[inline]
///     /// If `ctx.widget_id` is in the `self.target` path.
///     fn concerns_widget(&self, ctx: &mut zero_ui::core::context::WidgetContext) -> bool {
///         self.target.contains(ctx.widget_id)
///     }
/// }
///
/// impl zero_ui::core::event::CancelableEventArgs for MyEventArgs {
///     /// If a listener canceled the action.
///     #[inline]
///     fn cancel_requested(&self) -> bool {
///         self.cancel.get()
///     }
///
///     /// Cancel the action.
///     ///
///     /// Cloned args are still linked, canceling one will cancel the others.
///     #[inline]
///     fn cancel(&self) {
///         self.cancel.set(true);
///     }
/// }
/// ```
#[macro_export]
macro_rules! cancelable_event_args {

    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, $ctx:ident: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {$(
        $(#[$outer])*
        #[derive(Debug, Clone)]
        $vis struct $Args {
            /// When the event happened.
            pub timestamp: std::time::Instant,
            $($(#[$arg_outer])* $arg_vis $arg : $arg_ty,)*
            cancel: std::rc::Rc<std::cell::Cell<bool>>
        }
        impl $Args {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    cancel: std::rc::Rc::default()
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }
        }
        impl zero_ui::core::event::EventArgs for $Args {
            #[inline]
            fn timestamp(&self) -> std::time::Instant {
                self.timestamp
            }

            #[inline]
            $(#[$concerns_widget_outer])*
            fn concerns_widget(&$self, $ctx: &mut zero_ui::core::context::WidgetContext) -> bool {
                $($concerns_widget)+
            }
        }
        impl zero_ui::core::event::CancelableEventArgs for $Args {
            /// If a listener canceled the action.
            #[inline]
            fn cancel_requested(&self) -> bool {
                self.cancel.get()
            }

            /// Cancel the action.
            ///
            /// Cloned args are still linked, canceling one will cancel the others.
            #[inline]
            fn cancel(&self) {
                self.cancel.set(true);
            }
        }
    )+};

    // match discard WidgetContext in concerns_widget.
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Args:ident {
            $($(#[$arg_outer:meta])* $arg_vis:vis $arg:ident : $arg_ty:ty,)*
            ..
            $(#[$concerns_widget_outer:meta])*
            fn concerns_widget(&$self:ident, _: &mut WidgetContext) -> bool { $($concerns_widget:tt)+ }
        }
    )+) => {
        $crate::cancelable_event_args! { $(

            $(#[$outer])*
            $vis struct $Args {
                $($(#[$arg_outer])* $arg_vis $arg: $arg_ty,)*
                ..
                $(#[$concerns_widget_outer])*
                fn concerns_widget(&$self, _ctx: &mut WidgetContext) -> bool { $($concerns_widget)+ }
            }

        )+ }
    };
}

/// Declares a [`ProfileScope`](crate::core::profiler::ProfileScope) variable if
/// the `app_profiler` feature is active.
///
/// # Example
///
/// If compiled with the `app_profiler` feature, this will register a "do-things" scope
/// that starts when the macro was called and has the duration of the block.
/// ```
/// # fn main()
/// {
/// # fn do_thing() { }
/// # fn do_another_thing() { }
///     profile_scope!("do-things");
///
///     do_thing();
///     do_another_thing();
/// }
/// ```
///
/// You can also format strings:
/// ```
/// # fn main() {
/// # let thing = "";
/// profile_scope!("do-{}", thing);
/// # }
/// ```
#[macro_export]
macro_rules! profile_scope {
    ($name:expr) => {
        #[cfg(feature = "app_profiler")]
        let _profile_scope =
            zero_ui::core::profiler::ProfileScope::new($name);
    };
    ($($args:tt)+) => {
        #[cfg(feature = "app_profiler")]
        let _profile_scope =
            zero_ui::core::profiler::ProfileScope::new(format!($($args)+));
    };
}

/// Creates a [`Text`](crate::core::types::Text) by calling the `format!` macro and
/// wrapping the result in a `Cow::Owned`.
///
/// # Example
/// ```
/// # fn main() {
/// use zero_ui::core::types::Text;
///
/// let text: Text = formatx!("Hello {}", "World!");
/// # }
/// ```
#[macro_export]
macro_rules! formatx {
    ($($tt:tt)*) => {
        std::borrow::Cow::Owned(format!($($tt)*))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __context_var_inner {
    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = const $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {

            static DEFAULT: $type = $default;
            &DEFAULT

        };);
    };

    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = once $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {

            static DEFAULT: once_cell::sync::OnceCell<$type> = once_cell::sync::OnceCell::new();
            DEFAULT.get_or_init(||{
                $default
            })

        };);
    };

    ($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = return $default:expr;) => {
        $crate::__context_var_inner!(gen => $(#[$outer])* $vis struct $ident: $type = {
            $default
        };);
    };


    (gen => $(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $DEFAULT:expr;) => {
        $(#[$outer])*
        /// # ContextVar
        /// This `struct` is a [`ContextVar`](zero_ui::core::var::ContextVar).
        #[derive(Debug, Clone, Copy)]
        $vis struct $ident;

        impl $ident {
            /// Context var as [`Var`](zero_ui::core::var::Var).
            #[inline]
            pub fn as_var(self) -> zero_ui::core::var::ContextVarImpl<Self> {
                zero_ui::core::var::ContextVarImpl::<Self>::default()
            }

            /// [`Var`](zero_ui::core::var::Var) that represents this context var.
            #[inline]
            pub fn var() -> zero_ui::core::var::ContextVarImpl<Self> {
                Self.as_var()
            }
        }

        impl zero_ui::core::var::ContextVar for $ident {
            type Type = $type;

            fn default() -> &'static Self::Type {
               $DEFAULT
            }
        }

        impl zero_ui::core::var::IntoVar<$type> for $ident {
            type Var = zero_ui::core::var::ContextVarImpl<Self>;
            #[inline]
            fn into_var(self) -> Self::Var {
                self.as_var()
            }
        }
    };
}

/// Declares new [`ContextVar`](crate::core::context::ContextVar) types.
///
/// # Examples
/// ```
/// # fn main() {
/// # #[derive(Debug, Clone)]
/// # struct NotConst(u8);
/// # fn init_val() -> NotConst { NotConst(10) }
/// #
/// context_var! {
///     /// A public documented property with default value initialization that is `const`.
///     /// Will use a static variable.
///     pub struct Property1: u8 = const 10;
///
///     // A private property with default value that is not `const`. Will evaluate
///     // and cache the default on the first usage.
///     struct Property2: NotConst = once init_val();
/// }
/// # }
/// ```
#[macro_export]
macro_rules! context_var {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $mode:ident $default:expr;)+) => {$(
        $crate::__context_var_inner!($(#[$outer])* $vis struct $ident: $type = $mode $default;);
    )+};
}

/// Initializes a new [`Var`](crate::core::var::Var) with value made
/// by merging multiple other variables.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `var0..N`: A list of [vars](crate::core::var::Var), minimal 2.
/// * `merge`: A function that produces a new value from references to all variable values. `FnMut(&var0_T, ..) -> merge_T`
///
/// # Example
/// ```
/// # use zero_ui::prelude::{var, text, Text};
/// # use zero_ui::core::var::SharedVar;
/// # fn main() {
/// let var0: SharedVar<Text> = var("Hello");
/// let var1: SharedVar<Text> = var("World");
///
/// let greeting_text = text(merge_var!(var0, var1, |a, b|formatx!("{} {}!", a, b)));
/// # }
/// ```
#[macro_export]
macro_rules! merge_var {
    ($v0: expr, $v1: expr, $merge: expr) => {
        zero_ui::core::var::MergeVar2::new($v0, $v1, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $merge: expr) => {
        zero_ui::core::var::MergeVar3::new($v0, $v1, $v2, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $merge: expr) => {
        zero_ui::core::var::MergeVar4::new($v0, $v1, $v2, $v3, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $merge: expr) => {
        zero_ui::core::var::MergeVar5::new($v0, $v1, $v2, $v3, $v4, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $merge: expr) => {
        zero_ui::core::var::MergeVar6::new($v0, $v1, $v2, $v3, $v4, $v5, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $merge: expr) => {
        zero_ui::core::var::MergeVar7::new($v0, $v1, $v2, $v3, $v4, $v5, $v6, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $merge: expr) => {
        zero_ui::core::var::MergeVar8::new($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7, $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $($more_args:tt)+) => {
        compile_error!("merge_var is only implemented to a maximum of 8 variables")
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (var0, var1, .., merge_fn")
    };
}

/// Initializes a new switch var.
///
/// # Arguments
///
/// All arguments are separated by comma like a function call.
///
/// * `$index`: A positive integer that is the initial switch index.
/// * `$v0..$vn`: A list of [vars](crate::core::var::ObjVar), minimal 2,
/// [`SwitchVarDyn`](crate::core::var::SwitchVarDyn) is used for more then 8 variables.
///
/// # Example
/// ```
/// # use zero_ui::prelude::{var, text};
/// # fn main() {
/// let var0 = var("Read-write");
/// let var1 = "Read-only";
///
/// let t = text(switch_var!(0, var0, var1));
/// # }
/// ```
#[macro_export]
macro_rules! switch_var {
    ($index: expr, $v0: expr, $v1: expr) => {
        zero_ui::core::var::SwitchVar2::new($index, $v0, $v1)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr) => {
        zero_ui::core::var::SwitchVar3::new($index, $v0, $v1, $v2)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr) => {
        zero_ui::core::var::SwitchVar4::new($index, $v0, $v1, $v2, $v3)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr) => {
        zero_ui::core::var::SwitchVar5::new($index, $v0, $v1, $v2, $v3, $v4)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr) => {
        zero_ui::core::var::SwitchVar6::new($index, $v0, $v1, $v2, $v3, $v4, $v5)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr) => {
        zero_ui::core::var::SwitchVar7::new($index, $v0, $v1, $v2, $v3, $v4, $v5, $v6)
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr) => {
        zero_ui::core::var::SwitchVar8::new($index, $v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7)
    };
    ($index: expr, $($v:expr),+) => {
        zero_ui::core::var::SwitchVarDyn::new($index, vec![$($v.boxed()),+])
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (initial_index, var0, var1, ..)")
    };
}

/// Creates a [`UiVec`](zero_ui::core::UiVec) containing the arguments.
///
/// # Example
///
/// ```
/// let widgets = ui_vec![
///     text("Hello"),
///     text("World!")
/// ];
/// ```
/// `ui_vec!` automatically boxes each widget.
#[macro_export]
macro_rules! ui_vec {
    () => { zero_ui::core::UiVec::new() };
    ($($node:expr),+ $(,)?) => {
        vec![
            $(zero_ui::core::UiNode::boxed($node)),*
        ]
    };
}
