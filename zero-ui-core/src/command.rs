//! Command events.
//!
//! Commands are [events](Event) that represent app actions.

/*!
<script>
// hide re-exported `self`. We need to `pub use crate::command;` to inline the macro
// but that the path to the `command` module too.
document.addEventListener('DOMContentLoaded', function() {
    var macros = document.getElementById('modules');
    macros.nextElementSibling.remove();
    macros.remove();

    var side_bar_anchor = document.querySelector("li a[href='#modules']").remove();
 })
</script>
 */

use std::{
    any::{type_name, Any, TypeId},
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt,
    rc::Rc,
    sync::atomic::{AtomicUsize, Ordering},
    thread::LocalKey,
};

use crate::{
    context::{InfoContext, OwnedStateMap, WidgetContext, WidgetContextMut, WindowContext},
    crate_util::{Handle, HandleOwner},
    event::{Event, Events, WithEvents},
    handler::WidgetHandler,
    impl_ui_node,
    state::{StateKey, StateMap},
    state_key,
    text::{Text, ToText},
    var::*,
    widget_info::{EventSlot, WidgetInfoBuilder, WidgetSubscriptions},
    window::WindowId,
    UiNode, WidgetId,
};

/// <span data-inline></span> Declares new [`Command`] types.
///
/// The macro generates a unit `struct` that implements [`Event`] with arguments type [`CommandArgs`] and implements [`Command`].
/// The most used methods of [`Event`] and [`Command`] are also *re-exported* as associated methods.
///
/// # Conventions
///
/// Command types have the `Command` suffix, for example a command for the clipboard *copy* action is called `CopyCommand`.
/// Public and user facing commands also set the [`CommandNameExt`] and [`CommandInfoExt`] with localized display text.
///
/// # Shortcuts
///
/// You can give commands one or more shortcuts using the [`CommandShortcutExt`], the [`GestureManager`] notifies commands
/// that match a pressed shortcut automatically.
///
/// # Examples
///
/// Declare two commands:
///
/// ```
/// use zero_ui_core::command::command;
///
/// command! {
///     /// Command docs.
///     pub FooCommand;
///
///     pub(crate) BarCommand;
/// }
/// ```
///
/// You can also initialize metadata:
///
/// ```
/// use zero_ui_core::{command::{command, CommandNameExt, CommandInfoExt}, gesture::{CommandShortcutExt, shortcut}};
///
/// command! {
///     /// Represents the **foo** action.
///     ///
///     /// # Metadata
///     ///
///     /// This command initializes with the following metadata:
///     ///
///     /// | metadata     | value                             |
///     /// |--------------|-----------------------------------|
///     /// | [`name`]     | "Foo!"                            |
///     /// | [`info`]     | "Does the foo! thing."            |
///     /// | [`shortcut`] | `CTRL+F`                          |
///     ///
///     /// [`name`]: CommandNameExt
///     /// [`info`]: CommandInfoExt
///     /// [`shortcut`]: CommandShortcutExt
///     pub FooCommand
///         .init_name("Foo!")
///         .init_info("Does the foo! thing.")
///         .init_shortcut(shortcut!(CTRL+F));
/// }
/// ```
///
/// The initialization uses the [command extensions] pattern.
///
/// [`Command`]: crate::command::Command
/// [`CommandArgs`]: crate::command::CommandArgs
/// [`CommandNameExt`]: crate::command::CommandNameExt
/// [`CommandInfoExt`]: crate::command::CommandInfoExt
/// [`CommandShortcutExt`]: crate::gesture::CommandShortcutExt
/// [`GestureManager`]: crate::gesture::GestureManager
/// [`Event`]: crate::event::Event
/// [command extensions]: crate::command::Command#extensions
#[macro_export]
macro_rules! command {
    ($(
        $(#[$outer:meta])*
        $vis:vis $Command:ident $(
                 .$init:ident( $($args:tt)* )
        )*;
    )+) => {$(

        $(#[$outer])*
        #[derive(Clone, Copy, Debug)]
        $vis struct $Command;
        impl $Command {
            std::thread_local! {
                static COMMAND: $crate::command::CommandValue = $crate::command::CommandValue::init($Command, ||{
                    #[allow(path_statements)] {
                        $Command $(
                        .$init( $($args)* )
                        )*;
                    }
                });
            }

            /// Gets the event arguments if the update is for this command type and scope.
            #[inline(always)]
            #[allow(unused)]
            pub fn update<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<$Command>> {
                if let Some(args) = args.args_for::<Self>() {
                    if args.scope == $crate::command::CommandScope::App {
                        Some(args)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }

            /// Gets the event arguments if the update is for this command type disregarding the scope.
            #[inline(always)]
            #[allow(unused)]
            pub fn update_any_scope<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<$Command>> {
                args.args_for::<Self>()
            }

            /// Schedule an event update if the command is enabled.
            ///
            /// The `parameter` is an optional value for the command handler.
            ///
            /// Returns `true` if notified, only notifies if the command is enabled.
            #[inline]
            #[allow(unused)]
            pub fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, parameter: Option<std::rc::Rc<dyn std::any::Any>>) -> bool {
                let scope = $crate::command::Command::scope(self);
                let enabled = Self::COMMAND.with(move |c| c.enabled_value(scope));
                if enabled {
                    events.with_events(|evs| {
                        evs.notify($Command, $crate::command::CommandArgs::now(parameter, $crate::command::Command::scope(self)))
                    });
                }
                enabled
            }

            /// Gets a read-only variable that indicates if the command has at least one enabled handler.
            ///
            /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
            /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
            /// visible but disabled.
            #[inline]
            #[allow(unused)]
            pub fn enabled(self) -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                <Self as $crate::command::Command>::enabled(self)
            }

            /// Gets a read-only variable that indicates if the command has at least one handler.
            ///
            /// When this is `false` the command can be considered *not relevant* in the current app state
            /// and associated command trigger widgets can be hidden.
            #[inline]
            #[allow(unused)]
            pub fn has_handlers(self) -> $crate::var::ReadOnlyVar<bool, $crate::var::RcVar<bool>> {
                <Self as $crate::command::Command>::has_handlers(self)
            }

            /// Create a new handle to this command.
            ///
            /// A handle indicates that there is an active *handler* for the event, the handle can also
            /// be used to set the [`enabled`](Self::enabled) state.
            #[inline]
            #[allow(unused)]
            pub fn new_handle<Evs: $crate::event::WithEvents>(self, events: &mut Evs, enabled: bool) -> $crate::command::CommandHandle {
                <Self as $crate::command::Command>::new_handle(self, events, enabled)
            }

            /// Get a scoped command derived from this command type.
            #[inline]
            #[allow(unused)]
            pub fn scoped<S: Into<$crate::command::CommandScope>>(self, scope: S) -> $crate::command::ScopedCommand<Self> {
                <Self as $crate::command::Command>::scoped(self, scope)
            }
        }
        impl $crate::event::Event for $Command {
            type Args = $crate::command::CommandArgs;

            #[inline(always)]
            fn notify<Evs: $crate::event::WithEvents>(self, events: &mut Evs, args: Self::Args) {
                let scope = $crate::command::Command::scope(self);
                if Self::COMMAND.with(move |c| c.enabled_value(scope)) {
                    events.with_events(|evs| evs.notify($Command, args));
                }
            }

            fn update<U: $crate::event::EventUpdateArgs>(self, args: &U) -> Option<&$crate::event::EventUpdate<Self>> {
                self.update(args)
            }

            fn slot(self) -> $crate::widget_info::EventSlot {
                Self::COMMAND.with(move |c| c.slot())
            }
        }
        impl $crate::command::Command for $Command {
            type AppScopeCommand = Self;

            #[inline]
            fn thread_local_value(self) -> &'static std::thread::LocalKey<$crate::command::CommandValue> {
                &Self::COMMAND
            }

            #[inline]
            fn scoped<S: Into<$crate::command::CommandScope>>(self, scope: S) ->  $crate::command::ScopedCommand<Self> {
                $crate::command::ScopedCommand{ command: self, scope: scope.into() }
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::command;

/// Identifies a command type.
///
/// Use the [`command!`] to declare command types, it declares command types with optional
/// [metadata](#metadata) initialization.
///
/// ```
/// # use zero_ui_core::command::*;
/// # pub trait CommandFooBarExt: Sized { fn init_foo(self, foo: bool) -> Self { self } fn init_bar(self, bar: bool) -> Self { self } }
/// # impl<C: Command> CommandFooBarExt for C { }
/// command! {
///     /// Foo-bar command.
///     pub FooBarCommand
///         .init_foo(false)
///         .init_bar(true);
/// }
/// ```
///
/// # Metadata
///
/// Commands can have metadata associated with then, this metadata is extendable and can be used to enable
/// command features such as command shortcuts. The metadata can be accessed using [`with_meta`], metadata
/// extensions are implemented using extension traits. See [`CommandMeta`] for more details.
///
/// # Handles
///
/// Unlike other events, commands only notify if there is at least one handler enabled, handlers
/// must call [`new_handle`] to indicate that the command is relevant to the current app state and
/// [set its enabled] flag to indicate that the handler can fulfill command requests.
///
/// Properties that setup a handler for a command event should do this automatically and are usually
/// paired with a *can_foo* context property that sets the enabled flag. You can use [`on_command`]
/// to declare command handler properties.
///
/// # Scopes
///
/// Commands are *global* by default, meaning an enabled handle anywhere in the app enables it everywhere.
/// You can call [`scoped`] to declare *sub-commands* that are new commands that represent a command type in a limited
/// scope only, See [`ScopedCommand<C>`] for details.
///
/// [`command!`]: macro@crate::command::command
/// [`new_handle`]: Command::new_handle
/// [set its enabled]: CommandHandle::set_enabled
/// [`with_meta`]: Command::with_meta
/// [`scoped`]: Command::scoped
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Command: Event<Args = CommandArgs> {
    /// The root command type.
    ///
    /// This should be `Self` by default, and will be once [this] is stable.
    ///
    /// [this]: https://github.com/rust-lang/rust/issues/29661
    #[doc(hidden)]
    type AppScopeCommand: Command;

    /// Thread-local storage for command.
    #[doc(hidden)]
    fn thread_local_value(self) -> &'static LocalKey<CommandValue>;

    /// Runs `f` with access to the metadata state-map. The first map is the root command map,
    /// the second optional map is the scoped command map.
    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.with_meta(f, scope))
    }

    /// Gets a read-only variable that indicates if the command has at least one enabled handler.
    ///
    /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
    /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
    /// visible but disabled.
    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.enabled(scope))
    }

    /// Gets if the command has at least one enabled handler.
    fn enabled_value(self) -> bool {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.enabled_value(scope))
    }

    /// Gets a read-only variable that indicates if the command has at least one handler.
    ///
    /// When this is `false` the command can be considered *not relevant* in the current app state
    /// and associated command trigger widgets can be hidden.
    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.has_handlers(scope))
    }

    /// Gets if the command has at least one handler.
    fn has_handlers_value(self) -> bool {
        let scope = self.scope();
        self.thread_local_value().with(move |c| c.has_handlers_value(scope))
    }

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`enabled`](Self::enabled) state.
    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        let tl = self.thread_local_value();
        let scope = self.scope();
        tl.with(move |c| c.new_handle(events, tl, scope, enabled))
    }

    /// Gets a [`AnyCommand`] that represents this command.
    fn as_any(self) -> AnyCommand {
        AnyCommand(self.thread_local_value(), self.scope())
    }

    /// The scope the command applies too.
    ///
    /// Scoped commands represent "a command in a scope" as a new command.
    ///
    /// The default value is [`CommandScope::App`].
    fn scope(self) -> CommandScope {
        CommandScope::App
    }

    /// Get a scoped command derived from this command type.
    ///
    /// Returns a new command that represents the command type in the `scope`.
    /// See [`ScopedCommand`] for details.
    fn scoped<S: Into<CommandScope>>(self, scope: S) -> ScopedCommand<Self::AppScopeCommand>;
}

/// Represents the scope of a [`Command`].
///
/// See [`ScopedCommand<C>`] for more details.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandScope {
    /// Default scope, this is the scope of command types declared using [`command!`].
    App,
    /// Scope of a window.
    Window(WindowId),
    /// Scope of a widget.
    Widget(WidgetId),
    /// Custom scope. The first value is *namespace* type, the second value is an unique id within the namespace.
    Custom(TypeId, u64),
}
impl From<WidgetId> for CommandScope {
    fn from(id: WidgetId) -> Self {
        CommandScope::Widget(id)
    }
}
impl From<WindowId> for CommandScope {
    fn from(id: WindowId) -> CommandScope {
        CommandScope::Window(id)
    }
}
impl<'a> From<&'a WidgetContext<'a>> for CommandScope {
    /// Widget scope from the `ctx.path.widget_id()`.
    fn from(ctx: &'a WidgetContext<'a>) -> Self {
        CommandScope::Widget(ctx.path.widget_id())
    }
}
impl<'a> From<&'a WindowContext<'a>> for CommandScope {
    /// Window scope from the `ctx.window_id`.
    fn from(ctx: &'a WindowContext<'a>) -> CommandScope {
        CommandScope::Window(*ctx.window_id)
    }
}
impl<'a> From<&'a WidgetContextMut> for CommandScope {
    /// Widget scope from the `ctx.widget_id()`.
    fn from(ctx: &'a WidgetContextMut) -> Self {
        CommandScope::Widget(ctx.widget_id())
    }
}

/// A command that is a command type in a scope.
///
/// Normal commands apply globally, if there is a handler enabled in any context the status
/// variables indicate its availability. You can use [`Command::scoped`] to change this by
/// creating a new *command* that represents a command type in a *scope* only. The scope can
/// be any of the [`CommandScope`] values.
///
/// # Examples
///
/// Get the a command type scoped to a window:
///
/// ```
/// # use zero_ui_core::{command::*, context::*};
/// # command! { pub FooCommand; }
/// # struct FooNode { cmd: ScopedCommand<FooCommand> }
/// # impl FooNode {
/// fn init(&mut self, ctx: &mut WindowContext) {
///     self.cmd = FooCommand.scoped(*ctx.window_id);
/// }
/// # }
/// ```
///
/// # Enabled & Has Handlers
///
/// The [`enabled`] and [`has_handlers`] variables are only `true` when there is
/// a handler created using the same scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*, handler::*, var::*, units::*};
/// # command! { pub FooCommand; }
/// # TestWidgetContext::doc_test((),
/// async_hn!(|mut ctx, _| {
///     let cmd = FooCommand;
///     let cmd_scoped = cmd.scoped(ctx.window_id());
///
///     let enabled = cmd.enabled();
///     let enabled_scoped = cmd_scoped.enabled();
///
///     let handle = cmd_scoped.new_handle(&mut ctx, true);
///     ctx.update().await;
///
///     assert!(!enabled.copy(&ctx));
///     assert!(enabled_scoped.copy(&ctx));
/// })
/// # );
/// ```
///
/// In the example above, only the `enabled_scoped` is `true` after only the `cmd_scoped` is enabled.
///
/// # Metadata
///
/// Metadata is *inherited* from the [not scoped] command type but can be overwritten for the scoped command
/// only, so you can rename or give a different shortcut for the command only in the scope.
///
/// ```
/// # use zero_ui_core::{var::*, command::*, handler::*, context::*};
/// # command! { pub FooCommand; }
/// # TestWidgetContext::doc_test((),
/// async_hn!(|ctx, _| {
///     let cmd = FooCommand;
///     let cmd_scoped = FooCommand.scoped(ctx.window_id());
///
///     // same initial value:
///     assert_eq!(cmd.name().get_clone(&ctx), cmd_scoped.name().get_clone(&ctx));
///     
///     // set a name for all commands, including scoped not overridden:
///     cmd.name().set(&ctx, "Foo!");
///     ctx.update().await;
///     assert_eq!("Foo!", cmd_scoped.name().get_clone(&ctx));
///
///     // name is overridden in the scoped command only:
///     cmd_scoped.name().set(&ctx, "Scoped Only!");
///     ctx.update().await;
///     assert_eq!("Scoped Only!", cmd_scoped.name().get_clone(&ctx));
///     assert_eq!("Foo!", cmd.name().get_clone(&ctx));
///
///     // scoped command no-longer affected:
///     cmd.name().set(&ctx, "F");
///     ctx.update().await;
///     assert_eq!("F", cmd.name().get_clone(&ctx));
///     assert_eq!("Scoped Only!", cmd_scoped.name().get_clone(&ctx));
/// })
/// # );
/// ```
///
/// See [`CommandMetaVar<T>`] for details of how this is implemented.
///
/// # Notify
///
/// Calling [`notify`] from a scoped command **notifies the base type** but sets the [`CommandArgs::scope`]
/// the event will be handled by handlers for the same scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*};
/// # command! { pub FooCommand; }
/// # fn init(ctx: &mut WindowContext) {
/// let notified = FooCommand.scoped(*ctx.window_id).notify(ctx, None);
/// # }  
/// ```
///
/// In the example above `notified` is `true` only if there are any enabled handlers for the same scope.
///
/// # Update
///
/// Calling [`update`] from a command detects updates for the same command type if the [`CommandArgs::scope`]
/// is equal to the command scope.
///
/// ```
/// # use zero_ui_core::{command::*, context::*, event::*};
/// # command! { pub FooCommand; }
/// # struct FooNode;
/// # impl FooNode {
/// fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
///     if let Some(args) = FooCommand.scoped(ctx.path.window_id()).update(args) {
///         println!("{:?}", args.scope);
///     }
/// }
/// # }
/// ```
///
/// The example will print only for commands on the scope of [`CommandScope::Window`] with the same id.
///
/// # App Scope
///
/// It is is possible to create a scoped command using the [`App`] scope. In this
/// case the scoped command struct behaves exactly like a default command type.
///
/// [`enabled`]: ScopedCommand::enabled
/// [`notify`]: ScopedCommand::notify
/// [`update`]: ScopedCommand::update
/// [`has_handlers`]: ScopedCommand::has_handlers
/// [`App`]: CommandScope::App
/// [`name`]: CommandNameExt::name
#[derive(Debug, Clone, Copy)]
pub struct ScopedCommand<C: Command> {
    /// Base command type.
    pub command: C,

    /// Command scope.
    pub scope: CommandScope,
}
impl<C: Command> ScopedCommand<C> {
    /// Gets a read-only variable that indicates if the command has at least one enabled handler in the scope.
    ///
    /// You can use this in a notifier widget that *knows* the limited scope it applies too, unlike the general
    /// enabled, the widget will only enable if there is an active handler in the scope.
    #[inline]
    #[allow(unused)]
    pub fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        <Self as Command>::enabled(self)
    }

    /// Gets a read-only variable that indicates if the command has at least one handler in the scope.
    #[inline]
    #[allow(unused)]
    pub fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        <Self as Command>::has_handlers(self)
    }

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`enabled`](Self::enabled) state.
    #[inline]
    #[allow(unused)]
    pub fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        <Self as Command>::new_handle(self, events, enabled)
    }

    /// Schedule an event update if the command is enabled.
    ///
    /// The event type notified is the `C` type, not `Self`. The scope is passed in the [`CommandArgs`].
    ///
    /// The `parameter` is an optional value for the command handler.
    ///
    /// Returns `true` if notified, only notifies if the command is enabled.
    pub fn notify<Evs: WithEvents>(self, events: &mut Evs, parameter: Option<Rc<dyn Any>>) -> bool {
        let scope = self.scope();
        let enabled = self.thread_local_value().with(move |c| c.enabled_value(scope));
        if enabled {
            events.with_events(|evs| evs.notify(self.command, CommandArgs::now(parameter, self.scope)));
        }
        enabled
    }

    /// Gets the event arguments if the update is for this command type and scope.
    ///
    /// Returns `Some(args)` if the event type is the `C` type, and the [`CommandArgs::scope`] is equal.
    pub fn update<U: crate::event::EventUpdateArgs>(self, args: &U) -> Option<&crate::event::EventUpdate<C>> {
        if let Some(args) = args.args_for::<C>() {
            if args.scope == self.scope {
                Some(args)
            } else {
                None
            }
        } else {
            None
        }
    }
}
impl<C: Command> Event for ScopedCommand<C> {
    type Args = CommandArgs;

    fn notify<Evs: WithEvents>(self, events: &mut Evs, args: Self::Args) {
        if self.enabled_value() {
            events.with_events(|events| events.notify(self.command, args));
        }
    }

    fn update<U: crate::event::EventUpdateArgs>(self, args: &U) -> Option<&crate::event::EventUpdate<Self>> {
        self.update(args).map(|a| a.transmute_event::<Self>())
    }

    fn slot(self) -> EventSlot {
        self.thread_local_value().with(move |c| c.slot())
    }
}
impl<C: Command> Command for ScopedCommand<C> {
    type AppScopeCommand = C;

    fn thread_local_value(self) -> &'static LocalKey<CommandValue> {
        self.command.thread_local_value()
    }

    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.with_meta(f, scope))
    }

    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.enabled(scope))
    }

    fn enabled_value(self) -> bool {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.enabled_value(scope))
    }

    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.has_handlers(scope))
    }

    fn has_handlers_value(self) -> bool {
        let scope = self.scope;
        self.command.thread_local_value().with(move |c| c.has_handlers_value(scope))
    }

    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        let key = self.command.thread_local_value();
        let scope = self.scope;
        key.with(move |c| c.new_handle(events, key, scope, enabled))
    }

    fn scope(self) -> CommandScope {
        self.scope
    }

    fn scoped<S: Into<CommandScope>>(self, scope: S) -> ScopedCommand<C> {
        ScopedCommand {
            command: self.command,
            scope: scope.into(),
        }
    }

    fn as_any(self) -> AnyCommand {
        let mut any = self.command.as_any();
        any.1 = self.scope;
        any
    }
}

/// Represents a [`Command`] type.
#[derive(Clone, Copy)]
pub struct AnyCommand(&'static LocalKey<CommandValue>, CommandScope);
impl AnyCommand {
    #[inline]
    #[doc(hidden)]
    pub fn new(c: &'static LocalKey<CommandValue>, scope: CommandScope) -> Self {
        AnyCommand(c, scope)
    }

    pub(crate) fn update_state(&self, vars: &Vars) {
        let scope = self.1;
        self.0.with(|c| c.update_state(vars, scope))
    }

    /// Gets the [`TypeId`] of the command represented by `self`.
    #[inline]
    pub fn command_type_id(self) -> TypeId {
        self.0.with(|c| c.command_type_id)
    }

    /// Gets the scope of the command represented by `self`.
    #[inline]
    pub fn scope(self) -> CommandScope {
        self.1
    }

    /// Gets the [`type_name`] of the command represented by `self`.
    #[inline]
    pub fn command_type_name(self) -> &'static str {
        self.0.with(|c| c.command_type_name)
    }

    /// If the command `C` is represented by `self`.
    #[inline]
    pub fn is<C: Command>(self) -> bool {
        self.command_type_id() == TypeId::of::<C>()
    }

    /// Schedule an event update for the command represented by `self`.
    #[inline]
    pub fn notify(self, events: &mut Events, parameter: Option<Rc<dyn Any>>) {
        Event::notify(self, events, CommandArgs::now(parameter, self.1))
    }
}
impl fmt::Debug for AnyCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "any {}:{:?}", self.command_type_name(), self.scope())
    }
}
impl Event for AnyCommand {
    type Args = CommandArgs;

    fn notify<Evs: WithEvents>(self, events: &mut Evs, args: Self::Args) {
        let scope = self.1;
        self.0.with(move |c| {
            if c.enabled_value(scope) {
                events.with_events(|e| (c.notify)(e, args))
            }
        });
    }
    fn update<U: crate::event::EventUpdateArgs>(self, _: &U) -> Option<&crate::event::EventUpdate<Self>> {
        // TODO use a closure in the value and then transmute to Self?
        panic!("`AnyCommand` does not support `Event::update`");
    }

    fn slot(self) -> EventSlot {
        self.0.with(|c| c.slot)
    }
}

impl Command for AnyCommand {
    type AppScopeCommand = Self;

    fn thread_local_value(self) -> &'static LocalKey<CommandValue> {
        self.0
    }

    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        let scope = self.1;
        self.0.with(move |c| c.with_meta(f, scope))
    }

    fn enabled(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.1;
        self.0.with(move |c| c.enabled(scope))
    }

    fn enabled_value(self) -> bool {
        let scope = self.1;
        self.0.with(move |c| c.enabled_value(scope))
    }

    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>> {
        let scope = self.1;
        self.0.with(move |c| c.has_handlers(scope))
    }

    fn has_handlers_value(self) -> bool {
        let scope = self.1;
        self.0.with(move |c| c.has_handlers_value(scope))
    }

    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle {
        let key = self.0;
        let scope = self.1;
        key.with(move |c| c.new_handle(events, key, scope, enabled))
    }

    fn as_any(self) -> AnyCommand {
        self
    }

    fn scope(self) -> CommandScope {
        self.1
    }

    fn scoped<S: Into<CommandScope>>(self, scope: S) -> ScopedCommand<Self> {
        ScopedCommand {
            command: self,
            scope: scope.into(),
        }
    }
}

#[derive(Clone, Copy)]
struct AppCommandMetaKey<S>(S);
impl<S: StateKey> StateKey for AppCommandMetaKey<S>
where
    S::Type: VarValue,
{
    type Type = RcVar<S::Type>;
}

#[derive(Clone, Copy)]
struct ScopedCommandMetaKey<S>(S);
impl<S: StateKey> StateKey for ScopedCommandMetaKey<S>
where
    S::Type: VarValue,
{
    type Type = RcCowVar<S::Type, RcVar<S::Type>>;
}

/// Access to metadata of a command.
///
/// The metadata storage can be accessed using the [`Command::with_meta`]
/// method, you should declare and extension trait that adds methods that return [`CommandMetaVar`] or
/// [`ReadOnlyCommandMetaVar`] that are stored in the [`CommandMeta`]. An initialization builder method for
/// each value also must be provided to integrate with the [`command!`] macro.
///
/// # Examples
///
/// ```
/// use zero_ui_core::{command::*, context::state_key, var::*};
///
/// state_key! {
///     struct CommandFooKey: bool;
///     struct CommandBarKey: bool;
/// }
///
/// /// FooBar command values.
/// pub trait CommandFooBarExt {
///     /// Gets read/write *foo*.
///     fn foo(self) -> CommandMetaVar<bool>;
///
///     /// Gets read-only *bar*.
///     fn bar(self) -> ReadOnlyCommandMetaVar<bool>;
///
///     /// Gets a read-only var derived from other metadata.
///     fn foo_and_bar(self) -> BoxedVar<bool>;
///
///     /// Init *foo*.
///     fn init_foo(self, foo: bool) -> Self;
///
///     /// Init *bar*.
///     fn init_bar(self, bar: bool) -> Self;
/// }
///
/// impl<C: Command> CommandFooBarExt for C {
///     fn foo(self) -> CommandMetaVar<bool> {
///         self.with_meta(|m| m.get_var_or_default(CommandFooKey))
///     }
///
///     fn bar(self) -> ReadOnlyCommandMetaVar<bool> {
///         self.with_meta(|m| m.get_var_or_insert(CommandBarKey, ||true)).into_read_only()
///     }
///
///     fn foo_and_bar(self) -> BoxedVar<bool> {
///         merge_var!(self.foo(), self.bar(), |f, b| *f && *b).boxed()
///     }
///
///     fn init_foo(self, foo: bool) -> Self {
///         self.with_meta(|m| m.init_var(CommandFooKey, foo));
///         self
///     }
///
///     fn init_bar(self, bar: bool) -> Self {
///         self.with_meta(|m| m.init_var(CommandBarKey, bar));
///         self
///     }
/// }
/// ```
///
/// [`command!`]: macro@crate::command::command
pub struct CommandMeta<'a> {
    meta: &'a mut StateMap,
    scope: Option<&'a mut StateMap>,
}
impl<'a> CommandMeta<'a> {
    /// Clone a meta value identified by a [`StateKey`] type.
    ///
    /// If the key is not set in the app, insert it using `init` to produce a value.
    pub fn get_or_insert<S, F>(&mut self, key: S, init: F) -> S::Type
    where
        S: StateKey,
        F: FnOnce() -> S::Type,
        S::Type: Clone,
    {
        if let Some(scope) = &mut self.scope {
            if let Some(value) = scope.get(key) {
                value.clone()
            } else if let Some(value) = self.meta.get(key) {
                value.clone()
            } else {
                let value = init();
                let r = value.clone();
                scope.set(key, value);
                r
            }
        } else {
            self.meta.entry(key).or_insert_with(init).clone()
        }
    }

    /// Clone a meta value identified by a [`StateKey`] type.
    ///
    /// If the key is not set, insert the default value and returns a clone of it.
    pub fn get_or_default<S>(&mut self, key: S) -> S::Type
    where
        S: StateKey,
        S::Type: Clone + Default,
    {
        self.get_or_insert(key, Default::default)
    }

    /// Set the meta value associated with the [`StateKey`] type.
    ///
    /// Returns the previous value if any was set.
    pub fn set<S>(&mut self, key: S, value: S::Type)
    where
        S: StateKey,
        S::Type: Clone,
    {
        if let Some(scope) = &mut self.scope {
            scope.set(key, value);
        } else {
            self.meta.set(key, value);
        }
    }

    /// Set the metadata value only if it was not set.
    ///
    /// This does not set the scoped override, only the command type metadata.
    pub fn init<S>(&mut self, key: S, value: S::Type)
    where
        S: StateKey,
        S::Type: Clone,
    {
        self.meta.entry(key).or_insert(value);
    }

    /// Clone a meta variable identified by a [`StateKey`] type.
    ///
    /// The variable is read-write and is clone-on-write if the command is scoped,
    /// call [`into_read_only`] to make it read-only.
    ///
    /// Note that the the [`StateKey`] type is the variable value type, the variable
    /// type is [`CommandMetaVar<S::Type>`]. This is done to ensure that the associated
    /// metadata implements the *scoped inheritance* of values correctly.
    ///
    /// [`into_read_only`]: Var::into_read_only
    pub fn get_var_or_insert<S, F>(&mut self, key: S, init: F) -> CommandMetaVar<S::Type>
    where
        S: StateKey,
        F: FnOnce() -> S::Type,
        S::Type: VarValue,
    {
        if let Some(scope) = &mut self.scope {
            let meta = &mut self.meta;
            scope
                .entry(ScopedCommandMetaKey(key))
                .or_insert_with(|| {
                    let var = meta.entry(AppCommandMetaKey(key)).or_insert_with(|| var(init())).clone();
                    CommandMetaVar::new(var)
                })
                .clone()
        } else {
            let var = self.meta.entry(AppCommandMetaKey(key)).or_insert_with(|| var(init())).clone();
            CommandMetaVar::pass_through(var)
        }
    }

    /// Clone a meta variable identified by a [`StateKey`] type.
    ///
    /// Inserts a variable with the default value if no variable is in the metadata.
    pub fn get_var_or_default<S>(&mut self, key: S) -> CommandMetaVar<S::Type>
    where
        S: StateKey,
        S::Type: VarValue + Default,
    {
        self.get_var_or_insert(key, Default::default)
    }

    /// Set the metadata variable if it was not set.
    ///
    /// This does not set the scoped override, only the command type metadata.
    pub fn init_var<S>(&mut self, key: S, value: S::Type)
    where
        S: StateKey,
        S::Type: VarValue,
    {
        self.meta.entry(AppCommandMetaKey(key)).or_insert_with(|| var(value));
    }
}

/// Read-write command metadata variable.
///
/// If you get this variable from a not scoped command, setting it sets
/// the value for all scopes. If you get this variable using a scoped command
/// setting it overrides only the value for the scope, see [`ScopedCommand`] for more details.
///
/// The aliased type is an [`RcVar`] wrapped in a [`RcCowVar`], for not scoped commands the
/// [`RcCowVar::pass_through`] is used so that the wrapped [`RcVar`] is set directly on assign
/// but the variable type matches that from a scoped command.
///
/// [`ScopedCommand`]: ScopedCommand#metadata
pub type CommandMetaVar<T> = RcCowVar<T, RcVar<T>>;

/// Read-only command metadata variable.
///
/// To convert a [`CommandMetaVar<T>`] into this var call [`into_read_only`].
///
/// [`into_read_only`]: Var::into_read_only
pub type ReadOnlyCommandMetaVar<T> = ReadOnlyVar<T, CommandMetaVar<T>>;

/// Adds the [`name`](CommandNameExt) metadata.
pub trait CommandNameExt: Command {
    /// Gets a read-write variable that is the display name for the command.
    fn name(self) -> CommandMetaVar<Text>;

    /// Sets the initial name if it is not set.
    fn init_name(self, name: impl Into<Text>) -> Self;

    /// Gets a read-only variable that formats the name and first shortcut in the following format: name (first_shortcut)
    /// Note: If no shortcuts are available this method returns the same as [`name`](Self::name)
    fn name_with_shortcut(self) -> BoxedVar<Text>
    where
        Self: crate::gesture::CommandShortcutExt;
}
state_key! {
    struct CommandNameKey: Text;
}
impl<C: Command> CommandNameExt for C {
    fn name(self) -> CommandMetaVar<Text> {
        self.with_meta(|m| {
            m.get_var_or_insert(CommandNameKey, || {
                let name = type_name::<C>();
                name.strip_suffix("Command").unwrap_or(name).to_text()
            })
        })
    }

    fn init_name(self, name: impl Into<Text>) -> Self {
        self.with_meta(|m| m.init_var(CommandNameKey, name.into()));
        self
    }

    fn name_with_shortcut(self) -> BoxedVar<Text>
    where
        Self: crate::gesture::CommandShortcutExt,
    {
        crate::merge_var!(self.name(), self.shortcut(), |name, shortcut| {
            if shortcut.is_empty() {
                name.clone()
            } else {
                crate::formatx!("{} ({})", name, shortcut[0])
            }
        })
        .boxed()
    }
}

/// Adds the [`info`](CommandInfoExt) metadata.
pub trait CommandInfoExt: Command {
    /// Gets a read-write variable that is a short informational string about the command.
    fn info(self) -> CommandMetaVar<Text>;

    /// Sets the initial info if it is not set.
    fn init_info(self, info: impl Into<Text>) -> Self;
}
state_key! {
    struct CommandInfoKey: Text;
}
impl<C: Command> CommandInfoExt for C {
    fn info(self) -> CommandMetaVar<Text> {
        self.with_meta(|m| m.get_var_or_insert(CommandInfoKey, || "".to_text()))
    }

    fn init_info(self, info: impl Into<Text>) -> Self {
        self.with_meta(|m| m.init_var(CommandNameKey, info.into()));
        self
    }
}

/// A handle to a [`Command`].
///
/// Holding the command handle indicates that the command is relevant in the current app state.
/// The handle needs to be enabled to indicate that the command can be issued.
///
/// You can use the [`Command::new_handle`] method in a command type to create a handle.
pub struct CommandHandle {
    handle: Handle<CommandHandleData>,
    local_enabled: Cell<bool>,
}
impl CommandHandle {
    /// Sets if the command event handler is active.
    ///
    /// When at least one [`CommandHandle`] is enabled the command is [`enabled`](Command::enabled).
    pub fn set_enabled(&self, enabled: bool) {
        if self.local_enabled.get() != enabled {
            self.local_enabled.set(enabled);
            let data = self.handle.data();

            if enabled {
                let check = data.enabled_count.fetch_add(1, Ordering::Relaxed);
                if check == usize::MAX {
                    data.enabled_count.store(usize::MAX, Ordering::Relaxed);
                    panic!("CommandHandle reached usize::MAX")
                }
            } else {
                data.enabled_count.fetch_sub(1, Ordering::Relaxed);
            };
        }
    }

    /// Returns a dummy [`CommandHandle`] that is not connected to any command.
    pub fn dummy() -> Self {
        CommandHandle {
            handle: Handle::dummy(CommandHandleData::default()),
            local_enabled: Cell::new(false),
        }
    }
}
impl Drop for CommandHandle {
    fn drop(&mut self) {
        if self.local_enabled.get() {
            self.handle.data().enabled_count.fetch_sub(1, Ordering::Relaxed);
        }
    }
}
#[derive(Default)]
struct CommandHandleData {
    enabled_count: AtomicUsize,
}

struct ScopedValue {
    handle: HandleOwner<CommandHandleData>,
    enabled: RcVar<bool>,
    has_handlers: RcVar<bool>,
    meta: OwnedStateMap,
    registered: bool,
}
impl Default for ScopedValue {
    fn default() -> Self {
        ScopedValue {
            enabled: var(false),
            has_handlers: var(false),
            handle: HandleOwner::dropped(CommandHandleData::default()),
            meta: OwnedStateMap::default(),
            registered: false,
        }
    }
}

#[doc(hidden)]
pub struct CommandValue {
    command_type_id: TypeId,
    command_type_name: &'static str,

    scopes: RefCell<HashMap<CommandScope, ScopedValue>>,
    slot: EventSlot,

    handle: HandleOwner<CommandHandleData>,

    enabled: RcVar<bool>,

    has_handlers: RcVar<bool>,

    meta: RefCell<OwnedStateMap>,

    meta_init: Cell<Option<Box<dyn FnOnce()>>>,
    registered: Cell<bool>,

    notify: Box<dyn Fn(&mut Events, CommandArgs)>,
}
#[allow(missing_docs)] // this is all hidden
impl CommandValue {
    pub fn init<C: Command, I: FnOnce() + 'static>(command: C, meta_init: I) -> Self {
        CommandValue {
            command_type_id: TypeId::of::<C>(),
            command_type_name: type_name::<C>(),
            scopes: RefCell::default(),
            handle: HandleOwner::dropped(CommandHandleData::default()),
            enabled: var(false),
            has_handlers: var(false),
            meta: RefCell::default(),
            meta_init: Cell::new(Some(Box::new(meta_init))),
            registered: Cell::new(false),
            slot: EventSlot::next(),
            notify: Box::new(move |events, args| events.notify(command, args)),
        }
    }

    fn update_state(&self, vars: &Vars, scope: CommandScope) {
        if let CommandScope::App = scope {
            self.has_handlers.set_ne(vars, self.has_handlers_value(scope));
            self.enabled.set_ne(vars, self.enabled_value(scope));
        } else {
            let mut has_handlers = false;
            let mut enabled = false;
            if let Some(data) = self.scopes.borrow().get(&scope) {
                has_handlers = !data.handle.is_dropped();
                enabled = data.handle.data().enabled_count.load(Ordering::Relaxed) > 0;
            }

            let scopes = self.scopes.borrow_mut();
            let scope = scopes.get(&scope).unwrap();
            scope.has_handlers.set_ne(vars, has_handlers);
            scope.enabled.set_ne(vars, enabled);
        }
    }

    pub fn new_handle<Evs: WithEvents>(
        &self,
        events: &mut Evs,
        key: &'static LocalKey<CommandValue>,
        scope: CommandScope,
        enabled: bool,
    ) -> CommandHandle {
        if let CommandScope::App = scope {
            if !self.registered.get() {
                self.registered.set(true);
                events.with_events(|e| e.register_command(AnyCommand(key, CommandScope::App)));
            }
            let r = CommandHandle {
                handle: self.handle.reanimate(),
                local_enabled: Cell::new(false),
            };
            if enabled {
                r.set_enabled(true);
            }
            r
        } else {
            let mut scopes = self.scopes.borrow_mut();
            let value = scopes.entry(scope).or_insert_with(|| {
                // register scope first time and can create variables with the updated values already.
                events.with_events(|e| e.register_command(AnyCommand(key, scope)));
                ScopedValue {
                    enabled: var(enabled),
                    has_handlers: var(true),
                    handle: HandleOwner::dropped(CommandHandleData::default()),
                    meta: OwnedStateMap::new(),
                    registered: true,
                }
            });
            if !value.registered {
                // register scope first time.
                events.with_events(|e| e.register_command(AnyCommand(key, scope)));
                value.registered = true;
            }
            let r = CommandHandle {
                handle: value.handle.reanimate(),
                local_enabled: Cell::new(false),
            };
            if enabled {
                r.set_enabled(true);
            }
            r
        }
    }

    pub fn slot(&self) -> EventSlot {
        self.slot
    }

    pub fn enabled(&self, scope: CommandScope) -> ReadOnlyVar<bool, RcVar<bool>> {
        if let CommandScope::App = scope {
            ReadOnlyVar::new(self.enabled.clone())
        } else {
            let var = self.scopes.borrow_mut().entry(scope).or_default().enabled.clone();
            ReadOnlyVar::new(var)
        }
    }

    pub fn enabled_value(&self, scope: CommandScope) -> bool {
        if let CommandScope::App = scope {
            !self.handle.is_dropped() && self.handle.data().enabled_count.load(Ordering::Relaxed) > 0
        } else if let Some(value) = self.scopes.borrow().get(&scope) {
            !value.handle.is_dropped() && value.handle.data().enabled_count.load(Ordering::Relaxed) > 0
        } else {
            false
        }
    }

    pub fn has_handlers(&self, scope: CommandScope) -> ReadOnlyVar<bool, RcVar<bool>> {
        if let CommandScope::App = scope {
            ReadOnlyVar::new(self.has_handlers.clone())
        } else {
            let var = self.scopes.borrow_mut().entry(scope).or_default().has_handlers.clone();
            ReadOnlyVar::new(var)
        }
    }

    pub fn has_handlers_value(&self, scope: CommandScope) -> bool {
        if let CommandScope::App = scope {
            !self.handle.is_dropped()
        } else if let Some(value) = self.scopes.borrow().get(&scope) {
            !value.handle.is_dropped()
        } else {
            false
        }
    }

    pub fn with_meta<F, R>(&self, f: F, scope: CommandScope) -> R
    where
        F: FnOnce(&mut CommandMeta) -> R,
    {
        if let Some(init) = self.meta_init.take() {
            init()
        }

        if let CommandScope::App = scope {
            f(&mut CommandMeta {
                meta: &mut self.meta.borrow_mut().0,
                scope: None,
            })
        } else {
            let mut scopes = self.scopes.borrow_mut();
            let scope = scopes.entry(scope).or_default();
            f(&mut CommandMeta {
                meta: &mut self.meta.borrow_mut().0,
                scope: Some(&mut scope.meta.0),
            })
        }
    }
}

crate::event_args! {
    /// Event args for command events.
    pub struct CommandArgs {
        /// Optional parameter for the command handler.
        pub parameter: Option<Rc<dyn Any>>,

        /// Scope of command that notified.
        pub scope: CommandScope,

        ..

        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            match self.scope {
                CommandScope::App => true,
                CommandScope::Window(id) => ctx.path.window_id() == id,
                CommandScope::Widget(id) => ctx.path.contains(id),
                CommandScope::Custom(_, _) => true,
            }
        }
    }
}
impl CommandArgs {
    /// Returns a reference to a parameter of `T` if [`parameter`](#structfield.parameter) is set to a value of `T`.
    #[inline]
    pub fn parameter<T: Any>(&self) -> Option<&T> {
        self.parameter.as_ref().and_then(|p| p.downcast_ref::<T>())
    }
}

/// Helper for declaring command handlers.
#[inline]
pub fn on_command<U, C, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    C: Command,
    CB: FnMut(&mut WidgetContext) -> C + 'static,
    E: Var<bool>,
    EB: FnMut(&mut WidgetContext) -> E + 'static,
    H: WidgetHandler<CommandArgs>,
{
    struct OnCommandNode<U, C, CB, E, EB, H> {
        child: U,
        command: Option<C>,
        command_builder: CB,
        enabled: Option<E>,
        enabled_builder: EB,
        handler: H,
        handle: Option<CommandHandle>,
    }
    #[impl_ui_node(child)]
    impl<U, C, CB, E, EB, H> UiNode for OnCommandNode<U, C, CB, E, EB, H>
    where
        U: UiNode,
        C: Command,
        CB: FnMut(&mut WidgetContext) -> C + 'static,
        E: Var<bool>,
        EB: FnMut(&mut WidgetContext) -> E + 'static,
        H: WidgetHandler<CommandArgs>,
    {
        fn info(&self, ctx: &mut InfoContext, widget_builder: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget_builder);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .event(self.command.expect("OnCommandNode not initialized"))
                .var(ctx, self.enabled.as_ref().unwrap())
                .handler(&self.handler);

            self.child.subscriptions(ctx, subscriptions);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            let enabled = (self.enabled_builder)(ctx);
            let is_enabled = enabled.copy(ctx);
            self.enabled = Some(enabled);

            let command = (self.command_builder)(ctx);
            self.command = Some(command);

            self.handle = Some(command.new_handle(ctx, is_enabled));
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.command.expect("OnCommandNode not initialized").update(args) {
                self.child.event(ctx, args);

                if !args.stop_propagation_requested() && self.enabled.as_ref().unwrap().copy(ctx) {
                    self.handler.event(ctx, args);
                }
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.child.update(ctx);

            self.handler.update(ctx);

            if let Some(enabled) = self.enabled.as_ref().expect("OnCommandNode not initialized").copy_new(ctx) {
                self.handle.as_ref().unwrap().set_enabled(enabled);
            }
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.handle = None;
            self.command = None;
            self.enabled = None;
        }
    }
    OnCommandNode {
        child,
        command: None,
        command_builder,
        enabled: None,
        enabled_builder,
        handler,
        handle: None,
    }
}

/// Helper for declaring command preview handlers.
#[inline]
pub fn on_pre_command<U, C, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    C: Command,
    CB: FnMut(&mut WidgetContext) -> C + 'static,
    E: Var<bool>,
    EB: FnMut(&mut WidgetContext) -> E + 'static,
    H: WidgetHandler<CommandArgs>,
{
    struct OnPreCommandNode<U, C, CB, E, EB, H> {
        child: U,
        command: Option<C>,
        command_builder: CB,
        enabled: Option<E>,
        enabled_builder: EB,
        handler: H,
        handle: Option<CommandHandle>,
    }
    #[impl_ui_node(child)]
    impl<U, C, CB, E, EB, H> UiNode for OnPreCommandNode<U, C, CB, E, EB, H>
    where
        U: UiNode,
        C: Command,
        CB: FnMut(&mut WidgetContext) -> C + 'static,
        E: Var<bool>,
        EB: FnMut(&mut WidgetContext) -> E + 'static,
        H: WidgetHandler<CommandArgs>,
    {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            let enabled = (self.enabled_builder)(ctx);
            let is_enabled = enabled.copy(ctx);
            self.enabled = Some(enabled);

            let command = (self.command_builder)(ctx);
            self.command = Some(command);

            self.handle = Some(command.new_handle(ctx, is_enabled));
        }

        fn info(&self, ctx: &mut InfoContext, widget_builder: &mut WidgetInfoBuilder) {
            self.child.info(ctx, widget_builder);
        }

        fn subscriptions(&self, ctx: &mut InfoContext, subscriptions: &mut WidgetSubscriptions) {
            subscriptions
                .event(self.command.expect("OnPreCommandNode not initialized"))
                .var(ctx, self.enabled.as_ref().unwrap())
                .handler(&self.handler);

            self.child.subscriptions(ctx, subscriptions);
        }

        fn event<A: crate::event::EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = self.command.expect("OnPreCommandNode not initialized").update(args) {
                if !args.stop_propagation_requested() && self.enabled.as_ref().unwrap().copy(ctx) {
                    self.handler.event(ctx, args);
                }

                self.child.event(ctx, args);
            } else {
                self.child.event(ctx, args);
            }
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            self.handler.update(ctx);

            if let Some(enabled) = self.enabled.as_ref().expect("OnPreCommandNode not initialized").copy_new(ctx) {
                self.handle.as_ref().unwrap().set_enabled(enabled);
            }

            self.child.update(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.handle = None;
            self.command = None;
            self.enabled = None;
        }
    }
    OnPreCommandNode {
        child,
        command: None,
        command_builder,
        enabled: None,
        enabled_builder,
        handler,
        handle: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::context::TestWidgetContext;

    use super::*;

    command! {
        FooCommand;
        BarCommand;
    }

    #[test]
    fn parameter_none() {
        let _ = CommandArgs::now(None, CommandScope::App);
    }

    #[test]
    fn enabled() {
        let mut ctx = TestWidgetContext::new();
        assert!(!FooCommand.enabled_value());

        let handle = FooCommand.new_handle(&mut ctx, true);
        assert!(FooCommand.enabled_value());

        handle.set_enabled(false);
        assert!(!FooCommand.enabled_value());

        handle.set_enabled(true);
        assert!(FooCommand.enabled_value());

        drop(handle);
        assert!(!FooCommand.enabled_value());
    }

    #[test]
    fn enabled_scoped() {
        let mut ctx = TestWidgetContext::new();

        let cmd = FooCommand;
        let cmd_scoped = FooCommand.scoped(ctx.window_id);
        assert!(!cmd.enabled_value());
        assert!(!cmd_scoped.enabled_value());

        let handle_scoped = cmd_scoped.new_handle(&mut ctx, true);
        assert!(!cmd.enabled_value());
        assert!(cmd_scoped.enabled_value());

        handle_scoped.set_enabled(false);
        assert!(!cmd.enabled_value());
        assert!(!cmd_scoped.enabled_value());

        handle_scoped.set_enabled(true);
        assert!(!cmd.enabled_value());
        assert!(cmd_scoped.enabled_value());

        drop(handle_scoped);
        assert!(!cmd.enabled_value());
        assert!(!cmd_scoped.enabled_value());
    }

    #[test]
    fn has_handlers() {
        let mut ctx = TestWidgetContext::new();
        assert!(!FooCommand.has_handlers_value());

        let handle = FooCommand.new_handle(&mut ctx, false);
        assert!(FooCommand.has_handlers_value());

        drop(handle);
        assert!(!FooCommand.has_handlers_value());
    }

    #[test]
    fn has_handlers_scoped() {
        let mut ctx = TestWidgetContext::new();

        let cmd = FooCommand;
        let cmd_scoped = FooCommand.scoped(ctx.window_id);

        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.has_handlers_value());

        let handle = cmd_scoped.new_handle(&mut ctx, false);

        assert!(!cmd.has_handlers_value());
        assert!(cmd_scoped.has_handlers_value());

        drop(handle);

        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.has_handlers_value());
    }

    // there are also integration tests in tests/command.rs
}
