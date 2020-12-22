//! All zero-ui public macros. Declared in a separate crate
//! so that we can reexport then in there proper module scope.
//!
//! All macro documentation is done at the reexport place.

pub use zero_ui_proc_macros::*;

#[macro_export]
macro_rules! state_key {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty;)+) => {$(
        $(#[$outer])*
        /// # StateKey
        /// This `struct` is a [`StateKey`](zero_ui::core::context::StateKey).
        #[derive(Clone, Copy)]
        $vis struct $ident;

        impl zero_ui::core::context::StateKey for $ident {
            type Type = $type;
        }
    )+};
}

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

            stop_propagation: std::rc::Rc<std::cell::Cell<bool>>,
        }
        impl $Args {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    stop_propagation: std::rc::Rc::default(),
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }

            /// Requests that subsequent handlers skip this event.
            ///
            /// Cloned arguments signal stop for all clones.
            #[inline]
            pub fn stop_propagation(&self) {
                <Self as zero_ui::core::event::EventArgs>::stop_propagation(self)
            }

            /// If the handler must skip this event.
            ///
            /// Note that property level handlers don't need to check this, as those handlers are
            /// already not called when this is `true`. [`UiNode`](zero_ui::core::UiNode) and
            /// [`AppExtension`](zero_ui::core::app::AppExtension) implementers must check if this is `true`.
            #[inline]
            pub fn stop_propagation_requested(&self) -> bool {
                <Self as zero_ui::core::event::EventArgs>::stop_propagation_requested(self)
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

            #[inline]
            fn stop_propagation(&self) {
                self.stop_propagation.set(true);
            }

            #[inline]
            fn stop_propagation_requested(&self) -> bool {
                self.stop_propagation.get()
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
            cancel: std::rc::Rc<std::cell::Cell<bool>>,
            stop_propagation: std::rc::Rc<std::cell::Cell<bool>>,
        }
        impl $Args {
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn new(timestamp: impl Into<std::time::Instant>, $($arg : impl Into<$arg_ty>),*) -> Self {
                $Args {
                    timestamp: timestamp.into(),
                    $($arg: $arg.into(),)*
                    cancel: std::rc::Rc::default(),
                    stop_propagation: std::rc::Rc::default(),
                }
            }

            /// Arguments for event that happened now (`Instant::now`).
            #[inline]
            #[allow(clippy::too_many_arguments)]
            pub fn now($($arg : impl Into<$arg_ty>),*) -> Self {
                Self::new(std::time::Instant::now(), $($arg),*)
            }

            /// Requests that subsequent handlers skip this event.
            ///
            /// Cloned arguments signal stop for all clones.
            #[inline]
            pub fn stop_propagation(&self) {
                <Self as zero_ui::core::event::EventArgs>::stop_propagation(self)
            }

            /// If the handler must skip this event.
            ///
            /// Note that property level handlers don't need to check this, as those handlers are
            /// already not called when this is `true`. [`UiNode`](zero_ui::core::UiNode) and
            /// [`AppExtension`](zero_ui::core::app::AppExtension) implementers must check if this is `true`.
            #[inline]
            pub fn stop_propagation_requested(&self) -> bool {
                <Self as zero_ui::core::event::EventArgs>::stop_propagation_requested(self)
            }

            /// Cancel the originating action.
            ///
            /// Cloned arguments signal cancel for all clones.
            #[inline]
            pub fn cancel(&self) {
                <Self as zero_ui::core::event::CancelableEventArgs>::cancel(self)
            }

            /// If the originating action must be canceled.
            #[inline]
            pub fn cancel_requested(&self) -> bool {
                <Self as zero_ui::core::event::CancelableEventArgs>::cancel_requested(self)
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

            #[inline]
            fn stop_propagation(&self) {
                self.stop_propagation.set(true);
            }

            #[inline]
            fn stop_propagation_requested(&self) -> bool {
                self.stop_propagation.get()
            }
        }
        impl zero_ui::core::event::CancelableEventArgs for $Args {
            #[inline]
            fn cancel_requested(&self) -> bool {
                self.cancel.get()
            }

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

#[macro_export]
macro_rules! formatx {
    ($str:tt) => {
        zero_ui::core::text::Text::borrowed($str)
    };
    ($($tt:tt)*) => {
        zero_ui::core::text::Text::owned(format!($($tt)*))
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
            std::thread_local! {
                static THREAD_LOCAL_VALUE: zero_ui::core::var::ContextVarValue<$ident> = zero_ui::core::var::ContextVarValue::init();
            }

            /// [`Var`](zero_ui::core::var::Var) that represents this context var.
            #[inline]
            pub fn var() -> &'static zero_ui::core::var::ContextVarProxy<Self> {
                const VAR: zero_ui::core::var::ContextVarProxy<$ident> = zero_ui::core::var::ContextVarProxy(std::marker::PhantomData);
                &VAR
            }

            /// Default value, used when the variable is not set in a context.
            #[inline]
            pub fn default_value() -> &'static $type {
                $DEFAULT
            }
        }

        impl zero_ui::core::var::ContextVar for $ident {
            type Type = $type;

            #[inline]
            fn default_value() -> &'static Self::Type {
               Self::default_value()
            }

            #[inline]
            fn var() -> &'static zero_ui::core::var::ContextVarProxy<Self> {
               Self::var()
            }

            #[inline]
            fn thread_local_value() -> zero_ui::core::var::ContextVarLocalKey<Self> {
                zero_ui::core::var::ContextVarLocalKey::new(&Self::THREAD_LOCAL_VALUE)
            }
        }

        impl zero_ui::core::var::IntoVar<$type> for $ident {
            type Var = zero_ui::core::var::ContextVarProxy<Self>;
            #[inline]
            fn into_var(self) -> Self::Var {
                zero_ui::core::var::ContextVarProxy::default()
            }
        }
    };
}

#[macro_export]
macro_rules! context_var {
    ($($(#[$outer:meta])* $vis:vis struct $ident:ident: $type: ty = $mode:ident $default:expr;)+) => {$(
        $crate::__context_var_inner!($(#[$outer])* $vis struct $ident: $type = $mode $default;);
    )+};
}

#[macro_export]
macro_rules! merge_var {
    ($v0: expr, $v1: expr, $merge: expr) => {
        zero_ui::core::var::RcMerge2Var::new(($v0, $v1), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $merge: expr) => {
        zero_ui::core::var::RcMerge3Var::new(($v0, $v1, $v2), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $merge: expr) => {
        zero_ui::core::var::RcMerge4Var::new(($v0, $v1, $v2, $v3), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $merge: expr) => {
        zero_ui::core::var::RcMerge5Var::new(($v0, $v1, $v2, $v3, $v4), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $merge: expr) => {
        zero_ui::core::var::RcMerge6Var::new(($v0, $v1, $v2, $v3, $v4, $v5), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $merge: expr) => {
        zero_ui::core::var::RcMerge7Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $merge: expr) => {
        zero_ui::core::var::RcMerge8Var::new(($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7), $merge)
    };
    ($v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr, $v8: expr, $($more_args:tt)+) => {
        compile_error!("merge_var is only implemented to a maximum of 8 variables")
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (var0, var1, .., merge_fn")
    };
}

#[macro_export]
macro_rules! switch_var {
    ($index: expr, $v0: expr, $v1: expr) => {
        zero_ui::core::var::RcSwitch2Var::new($index, ($v0, $v1))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr) => {
        zero_ui::core::var::RcSwitch3Var::new($index, ($v0, $v1, $v2))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr) => {
        zero_ui::core::var::RcSwitch4Var::new($index, ($v0, $v1, $v2, $v3))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr) => {
        zero_ui::core::var::RcSwitch5Var::new($index, ($v0, $v1, $v2, $v3, $v4))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr) => {
        zero_ui::core::var::RcSwitch6Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr) => {
        zero_ui::core::var::RcSwitch7Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5, $v6))
    };
    ($index: expr, $v0: expr, $v1: expr, $v2: expr, $v3: expr, $v4: expr, $v5: expr, $v6: expr, $v7: expr) => {
        zero_ui::core::var::RcSwitch8Var::new($index, ($v0, $v1, $v2, $v3, $v4, $v5, $v6, $v7))
    };
    ($index: expr, $($v:expr),+) => {
        // we need a builder to have $v be IntoVar and work like the others.
        zero_ui::core::var::RcSwitchVarBuilder::new($index)
        $(.push($v))+
        .build()
    };
    ($($_:tt)*) => {
        compile_error!("this macro takes 3 or more parameters (initial_index, var0, var1, ..)")
    };
}

#[macro_export]
macro_rules! ui_vec {
    () => { zero_ui::core::WidgetVec::new() };
    ($($node:expr),+ $(,)?) => {
        vec![
            $(zero_ui::core::Widget::boxed_widget($node)),*
        ]
    };
}

#[macro_export]
macro_rules! event {
    ($($(#[$outer:meta])* $vis:vis $Event:ident : $Args:path;)+) => {$(
        $(#[$outer])*
        $vis struct $Event;
        impl zero_ui::core::event::Event for $Event {
            type Args = $Args;
        }
        impl $Event {
            /// New event emitter.
            #[inline]
            pub fn emitter() -> zero_ui::core::event::EventEmitter<$Args> {
                <Self as zero_ui::core::event::Event>::emitter()
            }

            /// New event listener that never updates.
            pub fn never() -> zero_ui::core::event::EventListener<$Args> {
                <Self as zero_ui::core::event::Event>::never()
            }
        }
    )+};
}

#[macro_export]
macro_rules! event_hp {
    ($($(#[$outer:meta])* $vis:vis $Event:ident : $Args:path;)+) => {$(
        $(#[$outer])*
        $vis struct $Event;
        impl zero_ui::core::event::Event for $Event {
            type Args = $Args;
            const IS_HIGH_PRESSURE: bool = true;
        }

        impl $Event {
            /// New event emitter.
            #[inline]
            pub fn emitter() -> zero_ui::core::event::EventEmitter<$Args> {
                <Self as zero_ui::core::event::Event>::emitter()
            }

            /// New event listener that never updates.
            pub fn never() -> zero_ui::core::event::EventListener<$Args> {
                <Self as zero_ui::core::event::Event>::never()
            }
        }
    )+};
}

#[doc(hidden)]
#[cfg(debug_assertions)]
#[macro_export]
macro_rules! source_location {
    () => {
        zero_ui::core::debug::SourceLocation {
            file: std::file!(),
            line: std::line!(),
            column: std::column!(),
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __shortcut {
    (-> + $Key:ident) => {
        zero_ui::core::gesture::KeyGesture {
            key: zero_ui::core::gesture::GestureKey::$Key,
            modifiers: zero_ui::core::keyboard::ModifiersState::empty(),
        }
    };

    (-> $($MODIFIER:ident)|+ + $Key:ident) => {
        zero_ui::core::gesture::KeyGesture {
            key: zero_ui::core::gesture::GestureKey::$Key,
            modifiers: $(zero_ui::core::keyboard::ModifiersState::$MODIFIER)|+,
        }
    };

    (=> $($STARTER_MODIFIER:ident)|* + $StarterKey:ident, $($COMPLEMENT_MODIFIER:ident)|* + $ComplementKey:ident) => {
        zero_ui::core::gesture::KeyChord {
            starter: $crate::__shortcut!(-> $($STARTER_MODIFIER)|* + $StarterKey),
            complement: $crate::__shortcut!(-> $($COMPLEMENT_MODIFIER)|* + $ComplementKey)
        }
    };
}

#[macro_export]
macro_rules! shortcut {
    (Logo) => {
        zero_ui::core::gesture::Shortcut::Modifier(zero_ui::core::gesture::ModifierGesture::Logo)
    };
    (Shift) => {
        zero_ui::core::gesture::Shortcut::Modifier(zero_ui::core::gesture::ModifierGesture::Shift)
    };
    (Ctrl) => {
        zero_ui::core::gesture::Shortcut::Modifier(zero_ui::core::gesture::ModifierGesture::Ctrl)
    };
    (Alt) => {
        zero_ui::core::gesture::Shortcut::Modifier(zero_ui::core::gesture::ModifierGesture::Alt)
    };

    ($Key:ident) => {
        zero_ui::core::gesture::Shortcut::Gesture($crate::__shortcut!(-> + $Key))
    };
    ($($MODIFIER:ident)|+ + $Key:ident) => {
        zero_ui::core::gesture::Shortcut::Gesture($crate::__shortcut!(-> $($MODIFIER)|+ + $Key))
    };

    ($StarterKey:ident, $ComplementKey:ident) => {
        zero_ui::core::gesture::Shortcut::Chord($crate::__shortcut!(=>
            + $StarterKey,
            + $ComplementKey
        ))
    };

    ($StarterKey:ident, $($COMPLEMENT_MODIFIER:ident)|+ + $ComplementKey:ident) => {
        zero_ui::core::gesture::Shortcut::Chord($crate::__shortcut!(=>
            + $StarterKey,
            $(COMPLEMENT_MODIFIER)|* + $ComplementKey
        ))
    };

    ($($STARTER_MODIFIER:ident)|+ + $StarterKey:ident, $ComplementKey:ident) => {
        zero_ui::core::gesture::Shortcut::Chord($crate::__shortcut!(=>
            $($STARTER_MODIFIER)|* + $StarterKey,
            + $ComplementKey
        ))
    };

    ($($STARTER_MODIFIER:ident)|+ + $StarterKey:ident, $($COMPLEMENT_MODIFIER:ident)|+ + $ComplementKey:ident) => {
        zero_ui::core::gesture::Shortcut::Chord($crate::__shortcut!(=>
            $($STARTER_MODIFIER)|* + $StarterKey,
            $($COMPLEMENT_MODIFIER)|* + $ComplementKey
        ))
    };
}

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

#[doc(hidden)]
#[macro_export]
macro_rules! __stops {
    // match single color stop at the $start, plus $color with 2 stops plus other stops, e.g.:
    // stops![colors::RED, (colors::GREEN, 14, 20), colors::BLUE]
    // OR
    // $next_middle that is a $color with 2 stops, plus other stops, e.g.:
    // .. (colors::GREEN, 14, 20), colors::BLUE]
    (
        start: $start:expr,
        middle: [$($middle:expr),*],
        tail: ($color:expr, $stop0:expr, $stop1:expr), $($stops:tt)+
    ) => {
        $crate::__stops! {
            start: $start,
            middle: [$($middle,)* ($color, $stop0), ($color, $stop1)],
            tail: $($stops)+
        }
    };
    // match single color stop at the $start, plus single color stop in the $next_middle, plus other stops, e.g.:
    // stops![colors::RED, colors::GREEN, colors::BLUE]
    // OR
    // $next_middle that is a single color stop, plus other stops, e.g.:
    // .. colors::GREEN, colors::BLUE]
    (
        start: $start:expr,
        middle: [$($middle:expr),*],
        tail: $next_middle:expr, $($stops:tt)+
    ) => {
        $crate::__stops! {
            start: $start,
            middle: [$($middle,)* $next_middle],
            tail: $($stops)+
        }
    };
    // match single color stop at the $start, plus single $color with 2 stops, e.g.:
    // stops![colors::RED, (colors::GREEN, 15, 30)]
    // OR
    // match last entry as single $color with 2 stops, e.g.:
    // .. (colors::BLUE, 20, 30)]
    (
        start: $start:expr,
        middle: [$($middle:expr),*],
        tail: ($color:expr, $stop0:expr, $stop1:expr) $(,)?
    ) => {
        $crate::__stops! {
            start: $start,
            middle: [$($middle,)* ($color, $stop0)],
            tail: ($color, $stop1)
        }
    };
    // match single color stop at the $start, plus single color stop at the $end, e.g.:
    // stops![colors::RED, colors::GREEN]
    // OR
    // match last entry as single color stop, at the $end, e.g.:
    // .. colors::GREEN]
    (
        start: $start:expr,
        middle: [$($middle:expr),*],
        tail: $end:expr $(,)?
    ) => {
        zero_ui::widgets::GradientStops {
            start: zero_ui::widgets::ColorStop::from($start),
            middle: std::vec![$(zero_ui::widgets::GradientStop::from($middle)),*],
            end: zero_ui::widgets::ColorStop::from($end),
        }
    };
}

#[macro_export]
macro_rules! stops {
    // match single entry that is a single color with 2 stops, e.g.:
    // stops![(colors::RED, 0, 20)]
    (($color:expr, $stop0:expr, $stop1:expr) $(,)?) => {
        $crate::__stops! {
            start: ($color, $stop0),
            middle: [],
            tail: ($color, $stop1)
        }
    };
    // match first entry as as single color with 2 stops, plus other stops, e.g:
    // stops![(colors::RED, 0, 20), colors::WHITE]
    (($color:expr, $stop0:expr, $stop1:expr), $($stops:tt)+) => {
        $crate::__stops! {
            start: ($color, $stop0),
            middle: [($color, $stop1)],
            tail: $($stops)+
        }
    };
    ($start:expr, $($stops:tt)+) => {
        $crate::__stops! {
            start: $start,
            middle: [],
            tail: $($stops)+
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __event_property {
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
            filter: |$ctx:ident, $args:ident|$filter:expr,
        }
    ) => { paste::paste! {
        $(#[$on_event_attrs])*
        ///
        /// # Preview Event
        ///
        #[doc = "You can preview this event using [`on_pre_" $event "`]."]
        #[zero_ui::core::property(event)]
        $vis fn [<on_ $event>](
            child: impl zero_ui::core::UiNode,
            handler: impl FnMut(&mut zero_ui::core::context::WidgetContext, &$Args) + 'static
        ) -> impl zero_ui::core::UiNode {
            zero_ui::properties::events::on_event_filtered(child, $Event, |$ctx, $args|$filter, handler)
        }

        #[doc = "Preview [on_" $event "] event."]
        ///
        /// # Preview Events
        ///
        /// Preview events are fired before the main event, if you stop the propagation of a preview event
        /// the main event does not run. See [`on_pre_event`](zero_ui::properties::events::on_pre_event) for more details.
        #[zero_ui::core::property(event)]
        $vis fn [<on_pre_ $event>](
            child: impl zero_ui::core::UiNode,
            handler: impl FnMut(&mut zero_ui::core::context::WidgetContext, &$Args) + 'static
        ) -> impl zero_ui::core::UiNode {
            zero_ui::properties::events::on_pre_event_filtered(child, $Event, |$ctx, $args|$filter, handler)
        }
    } };
    (
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
        }
    ) => {
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                filter: |ctx, args| zero_ui::core::event::EventArgs::concerns_widget(args, ctx),
            }
        }
    };
}

#[macro_export]
macro_rules! event_property {
    ($(
        $(#[$on_event_attrs:meta])*
        $vis:vis fn $event:ident {
            event: $Event:path,
            args: $Args:path,
            $(filter: |$ctx:ident, $args:ident|$filter:expr,)?
        }
    )+) => {$(
        $crate::__event_property! {
            $(#[$on_event_attrs])*
            $vis fn $event {
                event: $Event,
                args: $Args,
                $(filter: |$ctx, $args|$filter,)?
            }
        }
    )+};
}
