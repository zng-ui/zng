use std::{any::TypeId, mem};

use crate::{
    app::{App, AppId},
    context::*,
    crate_util::FxHashMap,
    gesture::CommandShortcutExt,
    handler::WidgetHandler,
    text::Txt,
    var::{types::ArcCowVar, *},
    widget_instance::{match_node, UiNode, UiNodeOp, WidgetId},
    window::WindowId,
};

use super::*;

/// <span data-del-macro-root></span> Declares new [`Command`] keys.
///
/// The macro generates an [`event!`] of args type [`CommandArgs`] and added capability to track the presence of listeners enabled
/// and disabled and any other custom attached metadata.
///
/// # Conventions
///
/// Command events have the `_CMD` suffix, for example a command for the clipboard *copy* action is called `COPY_CMD`.
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
/// use zero_ui_core::event::command;
///
/// command! {
///     static FOO_CMD;
///
///     /// Command docs.
///     pub(crate) static BAR_CMD;
/// }
/// ```
///
/// You can also initialize metadata:
///
/// ```
/// use zero_ui_core::{event::{command, CommandNameExt, CommandInfoExt}, gesture::{CommandShortcutExt, shortcut}};
///
/// command! {
///     /// Represents the **foo** action.
///     pub static FOO_CMD = {
///         name: "Foo!",
///         info: "Does the foo thing.",
///         shortcut: shortcut![CTRL+F],
///     };
/// }
/// ```
///
/// The initialization uses the [command extensions] pattern and runs once for each app.
///
/// Or you can use a custom closure to initialize the command:
///
/// ```
/// use zero_ui_core::{event::{command, CommandNameExt, CommandInfoExt}, gesture::{CommandShortcutExt, shortcut}};
///
/// command! {
///     /// Represents the **foo** action.
///     pub static FOO_CMD => |cmd| {
///         cmd.init_name("Foo!");
///         cmd.init_info("Does the foo thing.");
///         cmd.init_shortcut(shortcut![CTRL+F]);
///     };
/// }
/// ```
///
/// For the first kind of metadata initialization a documentation section is also generated with a table of metadata.
///
/// [`Command`]: crate::event::Command
/// [`CommandArgs`]: crate::event::CommandArgs
/// [`CommandNameExt`]: crate::event::CommandNameExt
/// [`CommandInfoExt`]: crate::event::CommandInfoExt
/// [`CommandShortcutExt`]: crate::gesture::CommandShortcutExt
/// [`GestureManager`]: crate::gesture::GestureManager
/// [`Event`]: crate::event::Event
/// [command extensions]: crate::event::Command#extensions
#[macro_export]
macro_rules! command {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident $(=> |$cmd:ident|$custom_meta_init:expr ;)? $(= { $($meta_ident:ident : $meta_init:expr),* $(,)? };)? $(;)?
    )+) => {
        $(
            $crate::__command! {
                $(#[$attr])*
                $vis static $COMMAND $(=> |$cmd|$custom_meta_init)? $(= {
                    $($meta_ident: $meta_init,)+
                })? ;
            }
        )+
    }
}
#[doc(inline)]
pub use command;

#[doc(hidden)]
#[macro_export]
macro_rules! __command {
    (
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident => |$cmd:ident| $meta_init:expr;
    ) => {
        $(#[$attr])*
        $vis static $COMMAND: $crate::event::Command = {
            fn __meta_init__($cmd: $crate::event::Command) {
                $meta_init
            }
            $crate::context::app_local! {
                static EVENT: $crate::event::EventData = const { $crate::event::EventData::new(std::stringify!($COMMAND)) };
                static DATA: $crate::event::CommandData =  $crate::event::CommandData::new(__meta_init__);
            }
            $crate::event::Command::new(&EVENT, &DATA)
        };
    };
    (
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident = { $($meta_ident:ident : $meta_init:expr),* $(,)? };
    ) => {
        $crate::paste! {
            $crate::__command! {
                $(#[$attr])*
                ///
                /// # Metadata
                ///
                /// This command initializes with the following metadata:
                ///
                ///| metadata | value |
                ///|----------|-------|
                $(#[doc = concat!("|  `", stringify!($meta_ident), "`  |  `", stringify!($meta_init), "`  |")])+
                ///
                $vis static $COMMAND => |cmd| {
                    $(
                        cmd.[<init_ $meta_ident>]($meta_init);
                    )*
                };
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident;
    ) => {
        $crate::__command! {
            $(#[$attr])*
            $vis static $COMMAND => |_cmd|{};
        }
    };
}

/// Identifies a command event.
///
/// Use the [`command!`] to declare commands, it declares command keys with optional
/// [metadata](#metadata) initialization.
///
/// ```
/// # use zero_ui_core::event::*;
/// # pub trait CommandFooBarExt: Sized { fn init_foo(self, foo: bool) -> Self { self } fn init_bar(self, bar: bool) -> Self { self } }
/// # impl CommandFooBarExt for Command { }
/// command! {
///     /// Foo-bar command.
///     pub static FOO_BAR_CMD = {
///         foo: true,
///         bar: false,
///     };
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
/// Unlike other events, commands only notify if it has at least one handler, handlers
/// must call [`subscribe`] to indicate that the command is relevant to the current app state and
/// [set its enabled] flag to indicate that the handler can fulfill command requests.
///
/// Properties that setup a handler for a command event should do this automatically and are usually
/// paired with a *can_foo* context property that sets the enabled flag. You can use [`on_command`]
/// to declare command handler properties.
///
/// # Scopes
///
/// Commands are *global* by default, meaning an enabled handle anywhere in the app enables it everywhere.
/// You can call [`scoped`] to declare *sub-commands* that are the same command event, but filtered to a scope, metadata
/// of scoped commands inherit from the app scope metadata, but setting it overrides only for the scope.
///
/// [`command!`]: macro@crate::event::command
/// [`subscribe`]: Command::subscribe
/// [set its enabled]: CommandHandle::set_enabled
/// [`with_meta`]: Command::with_meta
/// [`scoped`]: Command::scoped
#[derive(Clone, Copy)]
pub struct Command {
    event: Event<CommandArgs>,
    local: &'static AppLocal<CommandData>,
    scope: CommandScope,
}
impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("Command")
                .field("event", &self.event)
                .field("scope", &self.scope)
                .finish_non_exhaustive()
        } else {
            write!(f, "{}", self.event.name())?;
            match self.scope {
                CommandScope::App => Ok(()),
                CommandScope::Window(id) => write!(f, "({id})"),
                CommandScope::Widget(id) => write!(f, "({id})"),
            }
        }
    }
}
impl Command {
    #[doc(hidden)]
    pub const fn new(event_local: &'static AppLocal<EventData>, command_local: &'static AppLocal<CommandData>) -> Self {
        Command {
            event: Event::new(event_local),
            local: command_local,
            scope: CommandScope::App,
        }
    }

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`is_enabled`](Self::is_enabled) state.
    ///
    /// If the handle is scoped on a widget it it is added to the command event subscribers.
    pub fn subscribe(&self, enabled: bool) -> CommandHandle {
        let mut evs = EVENTS_SV.write();
        self.local.write().subscribe(&mut evs, *self, enabled, None)
    }

    /// Create a new handle for this command for a handler in the `target` widget.
    ///
    /// The handle behaves like [`subscribe`], but include the `target` on the delivery list for window and app scoped commands.
    /// Note that for widget scoped commands only the scope can receive the event, so the `target` is ignored.
    ///
    /// [`subscribe`]: Command::subscribe
    pub fn subscribe_wgt(&self, enabled: bool, target: WidgetId) -> CommandHandle {
        let mut evs = EVENTS_SV.write();
        self.local.write().subscribe(&mut evs, *self, enabled, Some(target))
    }

    /// Raw command event.
    pub fn event(&self) -> Event<CommandArgs> {
        self.event
    }

    /// Command operating scope.
    pub fn scope(&self) -> CommandScope {
        self.scope
    }

    /// Gets the command in a new `scope`.
    pub fn scoped(mut self, scope: impl Into<CommandScope>) -> Command {
        self.scope = scope.into();
        self
    }

    /// Visit the command custom metadata of the current scope.
    pub fn with_meta<R>(&self, visit: impl FnOnce(&mut CommandMeta) -> R) -> R {
        {
            let mut write = self.local.write();
            if let Some(init) = write.meta_init.take() {
                drop(write);
                init(*self);
            }
        }

        match self.scope {
            CommandScope::App => visit(&mut CommandMeta {
                meta: self.local.read().meta.lock().borrow_mut(),
                scope: None,
            }),
            scope => {
                {
                    let mut write = self.local.write();
                    write.scopes.entry(scope).or_default();
                }

                let read = self.local.read();
                let scope = read.scopes.get(&scope).unwrap();
                let r = visit(&mut CommandMeta {
                    meta: read.meta.lock().borrow_mut(),
                    scope: Some(scope.meta.lock().borrow_mut()),
                });

                r
            }
        }
    }

    /// Returns `true` if the update is for this command and scope.
    pub fn has(&self, update: &EventUpdate) -> bool {
        self.on(update).is_some()
    }

    /// Get the command update args if the update is for this command and scope.
    pub fn on<'a>(&self, update: &'a EventUpdate) -> Option<&'a CommandArgs> {
        self.event.on(update).filter(|a| a.scope == self.scope)
    }

    /// Get the event update args if the update is for this event and propagation is not stopped.
    pub fn on_unhandled<'a>(&self, update: &'a EventUpdate) -> Option<&'a CommandArgs> {
        self.event
            .on(update)
            .filter(|a| a.scope == self.scope && !a.propagation().is_stopped())
    }

    /// Calls `handler` if the update is for this event and propagation is not stopped, after the handler is called propagation is stopped.
    pub fn handle<R>(&self, update: &EventUpdate, handler: impl FnOnce(&CommandArgs) -> R) -> Option<R> {
        if let Some(args) = self.on(update) {
            args.handle(handler)
        } else {
            None
        }
    }

    /// Gets a variable that tracks if this command has any live handlers.
    pub fn has_handlers(&self) -> ReadOnlyArcVar<bool> {
        let mut write = self.local.write();
        match self.scope {
            CommandScope::App => write.has_handlers.read_only(),
            scope => write.scopes.entry(scope).or_default().has_handlers.read_only(),
        }
    }

    /// Gets a variable that tracks if this command has any enabled live handlers.
    pub fn is_enabled(&self) -> ReadOnlyArcVar<bool> {
        let mut write = self.local.write();
        match self.scope {
            CommandScope::App => write.is_enabled.read_only(),
            scope => write.scopes.entry(scope).or_default().is_enabled.read_only(),
        }
    }

    #[cfg(test)]
    fn has_handlers_value(&self) -> bool {
        let read = self.local.read();
        match self.scope {
            CommandScope::App => read.handle_count > 0,
            scope => read.scopes.get(&scope).map(|l| l.handle_count > 0).unwrap_or(false),
        }
    }

    fn is_enabled_value(&self) -> bool {
        let read = self.local.read();
        match self.scope {
            CommandScope::App => read.enabled_count > 0,
            scope => read.scopes.get(&scope).map(|l| l.enabled_count > 0).unwrap_or(false),
        }
    }

    /// Schedule a command update without param.
    pub fn notify(&self) {
        self.event.notify(CommandArgs::now(None, self.scope, self.is_enabled_value()))
    }

    /// Schedule a command update with custom `param`.
    pub fn notify_param(&self, param: impl Any + Send + Sync) {
        self.event
            .notify(CommandArgs::now(CommandParam::new(param), self.scope, self.is_enabled_value()));
    }

    /// Schedule a command update linked with an external event `propagation`.
    pub fn notify_linked(&self, propagation: EventPropagationHandle, param: Option<CommandParam>) {
        self.event.notify(CommandArgs::new(
            Instant::now(),
            propagation,
            param,
            self.scope,
            self.is_enabled_value(),
        ))
    }

    pub(crate) fn update_state(&self) {
        let read = self.local.read();
        if let CommandScope::App = self.scope {
            let has_handlers = read.handle_count > 0;
            read.has_handlers.set(has_handlers);
            read.is_enabled.set(has_handlers && read.enabled_count > 0);
        } else if let Some(scope) = read.scopes.get(&self.scope) {
            let has_handlers = !scope.handle_count > 0;
            scope.has_handlers.set(has_handlers);
            scope.is_enabled.set(has_handlers && scope.enabled_count > 0);
        }
    }
}
impl Deref for Command {
    type Target = Event<CommandArgs>;

    fn deref(&self) -> &Self::Target {
        &self.event
    }
}
impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event && self.scope == other.scope
    }
}
impl Eq for Command {}

/// Represents the scope of a [`Command`].
///
/// The command scope defines the targets of its event and the context of its metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CommandScope {
    /// Default scope, this is the scope of command types declared using [`command!`].
    App,
    /// Scope of a window.
    Window(WindowId),
    /// Scope of a widget.
    Widget(WidgetId),
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

event_args! {
    /// Event args for command events.
    pub struct CommandArgs {
        /// Optional parameter for the command handler.
        pub param: Option<CommandParam>,

        /// Scope of command that notified.
        pub scope: CommandScope,

        /// If the command handle was enabled when the command notified.
        ///
        /// If `false` the command primary action must not run, but a secondary "disabled interaction"
        /// that indicates what conditions enable the command is recommended.
        pub enabled: bool,

        ..

        /// Broadcast to all widgets for [`CommandScope::App`].
        ///
        /// Broadcast to all widgets in the window for [`CommandScope::Window`].
        ///
        /// Target ancestors and widget for [`CommandScope::Widget`], if it is found.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            match self.scope {
                CommandScope::Widget(id) => list.search_widget(id),
                _ => list.search_all(),
            }
        }
    }
}
impl CommandArgs {
    /// Returns a reference to a parameter of `T` if [`parameter`](#structfield.parameter) is set to a value of `T`.
    pub fn param<T: Any>(&self) -> Option<&T> {
        self.param.as_ref().and_then(|p| p.downcast_ref::<T>())
    }

    /// Returns [`param`] if is enabled interaction.
    ///
    /// [`param`]: Self::param()
    pub fn enabled_param<T: Any>(&self) -> Option<&T> {
        if self.enabled {
            self.param::<T>()
        } else {
            None
        }
    }

    /// Returns [`param`] if is disabled interaction.
    ///
    /// [`param`]: Self::param()
    pub fn disabled_param<T: Any>(&self) -> Option<&T> {
        if !self.enabled {
            self.param::<T>()
        } else {
            None
        }
    }

    /// Stops propagation and call `handler` if the command and local handler are enabled and was not handled.
    ///
    /// This is the default behavior of commands, when a command has a handler it is *relevant* in the context, and overwrites
    /// lower priority handlers, but if the handler is disabled the command primary action is not run.
    ///
    /// Returns the `handler` result if it was called.
    #[allow(unused)]
    pub fn handle_enabled<F, R>(&self, local_handle: &CommandHandle, handler: F) -> Option<R>
    where
        F: FnOnce(&Self) -> R,
    {
        let mut result = None;
        self.handle(|args| {
            if args.enabled && local_handle.is_enabled() {
                result = Some(handler(args));
            }
        });
        result
    }
}

/// A handle to a [`Command`].
///
/// Holding the command handle indicates that the command is relevant in the current app state.
/// The handle needs to be enabled to indicate that the command primary action can be executed.
///
/// You can use the [`Command::subscribe`] method in a command type to create a handle.
pub struct CommandHandle {
    command: Option<Command>,
    local_enabled: AtomicBool,
    app_id: Option<AppId>,
    _event_handle: EventHandle,
}
impl CommandHandle {
    /// The command.
    pub fn command(&self) -> Option<Command> {
        self.command
    }

    /// Sets if the command event handler is active.
    ///
    /// When at least one [`CommandHandle`] is enabled the command is [`is_enabled`](Command::is_enabled).
    pub fn set_enabled(&self, enabled: bool) {
        if let Some(command) = self.command {
            if self.local_enabled.swap(enabled, Ordering::Relaxed) != enabled {
                if self.app_id != App::current_id() {
                    return;
                }

                UpdatesTrace::log_var::<bool>();

                let mut write = command.local.write();
                match command.scope {
                    CommandScope::App => {
                        if enabled {
                            write.enabled_count += 1;
                        } else {
                            write.enabled_count -= 1;
                        }
                    }
                    scope => {
                        if let Some(data) = write.scopes.get_mut(&scope) {
                            if enabled {
                                data.enabled_count += 1;
                            } else {
                                data.enabled_count -= 1;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Returns if this handle has enabled the command.
    pub fn is_enabled(&self) -> bool {
        self.local_enabled.load(Ordering::Relaxed)
    }

    /// New handle not connected to any command.
    pub fn dummy() -> Self {
        CommandHandle {
            command: None,
            app_id: None,
            local_enabled: AtomicBool::new(false),
            _event_handle: EventHandle::dummy(),
        }
    }

    /// If the handle is not connected to any command.
    pub fn is_dummy(&self) -> bool {
        self.command.is_none()
    }
}
impl fmt::Debug for CommandHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandHandle")
            .field("command", &self.command)
            .field("local_enabled", &self.local_enabled.load(Ordering::Relaxed))
            .finish()
    }
}
impl Drop for CommandHandle {
    fn drop(&mut self) {
        if let Some(command) = self.command {
            if self.app_id != App::current_id() {
                return;
            }

            let mut write = command.local.write();
            match command.scope {
                CommandScope::App => {
                    write.handle_count -= 1;
                    if self.local_enabled.load(Ordering::Relaxed) {
                        write.enabled_count -= 1;
                    }
                }
                scope => {
                    if let Some(data) = write.scopes.get_mut(&scope) {
                        data.handle_count -= 1;
                        if self.local_enabled.load(Ordering::Relaxed) {
                            data.enabled_count -= 1;
                        }
                    }
                }
            }
        }
    }
}
impl Default for CommandHandle {
    fn default() -> Self {
        Self::dummy()
    }
}

/// Represents a reference counted `dyn Any` object.
#[derive(Clone)]
pub struct CommandParam(pub Arc<dyn Any + Send + Sync>);
impl CommandParam {
    /// New param.
    pub fn new(param: impl Any + Send + Sync + 'static) -> Self {
        CommandParam(Arc::new(param))
    }

    /// Gets the [`TypeId`] of the parameter.
    pub fn type_id(&self) -> TypeId {
        self.0.type_id()
    }

    /// Gets a typed reference to the parameter if it is of type `T`.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }

    /// Returns `true` if the parameter type is `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.0.is::<T>()
    }
}
impl fmt::Debug for CommandParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("CommandParam").field(&self.0.type_id()).finish()
    }
}

unique_id_64! {
    /// Unique identifier of a command metadata state variable.
    ///
    /// This type is very similar to [`StateId`], but `T` is the value type of the metadata variable.
    pub struct CommandMetaVarId<T: (StateValue + VarValue)>: StateId;
}
impl<T: StateValue + VarValue> CommandMetaVarId<T> {
    fn app(self) -> StateId<ArcVar<T>> {
        let id = self.get();
        StateId::from_raw(id)
    }

    fn scope(self) -> StateId<ArcCowVar<T, ArcVar<T>>> {
        let id = self.get();
        StateId::from_raw(id)
    }
}

impl<T: StateValue + VarValue> fmt::Debug for CommandMetaVarId<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        #[cfg(debug_assertions)]
        let t = pretty_type_name::pretty_type_name::<T>();
        #[cfg(not(debug_assertions))]
        let t = "$T";

        if f.alternate() {
            writeln!(f, "CommandMetaVarId<{t} {{")?;
            writeln!(f, "   id: {},", self.get())?;
            writeln!(f, "   sequential: {}", self.sequential())?;
            writeln!(f, "}}")
        } else {
            write!(f, "CommandMetaVarId<{t}>({})", self.sequential())
        }
    }
}

/// Access to metadata of a command.
///
/// The metadata storage can be accessed using the [`Command::with_meta`]
/// method, implementers must declare and extension trait that adds methods that return [`CommandMetaVar`] or
/// [`ReadOnlyCommandMetaVar`] that are stored in the [`CommandMeta`]. An initialization builder method for
/// each value also must be provided to integrate with the [`command!`] macro.
///
/// # Examples
///
/// /// The [`command!`] initialization transforms `foo: true,` to `command.init_foo(true);`, to support that, the command extension trait
/// must has `foo` and `init_foo` methods.
///
/// ```
/// use zero_ui_core::{event::*, var::*};
///
/// static COMMAND_FOO_ID: StaticCommandMetaVarId<bool> = StaticCommandMetaVarId::new_unique();
/// static COMMAND_BAR_ID: StaticCommandMetaVarId<bool> = StaticCommandMetaVarId::new_unique();
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
/// impl CommandFooBarExt for Command {
///     fn foo(self) -> CommandMetaVar<bool> {
///         self.with_meta(|m| m.get_var_or_default(&COMMAND_FOO_ID))
///     }
///
///     fn bar(self) -> ReadOnlyCommandMetaVar<bool> {
///         self.with_meta(|m| m.get_var_or_insert(&COMMAND_BAR_ID, ||true)).read_only()
///     }
///
///     fn foo_and_bar(self) -> BoxedVar<bool> {
///         merge_var!(self.foo(), self.bar(), |f, b| *f && *b).boxed()
///     }
///
///     fn init_foo(self, foo: bool) -> Self {
///         self.with_meta(|m| m.init_var(&COMMAND_FOO_ID, foo));
///         self
///     }
///
///     fn init_bar(self, bar: bool) -> Self {
///         self.with_meta(|m| m.init_var(&COMMAND_BAR_ID, bar));
///         self
///     }
/// }
/// ```
///
/// [`command!`]: macro@crate::event::command
pub struct CommandMeta<'a> {
    meta: StateMapMut<'a, CommandMetaState>,
    scope: Option<StateMapMut<'a, CommandMetaState>>,
}
impl<'a> CommandMeta<'a> {
    /// Clone a meta value identified by a [`StateId`].
    ///
    /// If the key is not set in the app, insert it using `init` to produce a value.
    pub fn get_or_insert<T, F>(&mut self, id: impl Into<StateId<T>>, init: F) -> T
    where
        T: StateValue + Clone,
        F: FnOnce() -> T,
    {
        let id = id.into();
        if let Some(scope) = &mut self.scope {
            if let Some(value) = scope.get(id) {
                value.clone()
            } else if let Some(value) = self.meta.get(id) {
                value.clone()
            } else {
                let value = init();
                let r = value.clone();
                scope.set(id, value);
                r
            }
        } else {
            self.meta.entry(id).or_insert_with(init).clone()
        }
    }

    /// Clone a meta value identified by a [`StateId`].
    ///
    /// If the key is not set, insert the default value and returns a clone of it.
    pub fn get_or_default<T>(&mut self, id: impl Into<StateId<T>>) -> T
    where
        T: StateValue + Clone + Default,
    {
        self.get_or_insert(id, Default::default)
    }

    /// Clone a meta value identified by a [`StateId`] if it is set.
    pub fn get<T>(&self, id: impl Into<StateId<T>>) -> Option<T>
    where
        T: StateValue + Clone,
    {
        let id = id.into();
        if let Some(scope) = &self.scope {
            scope.get(id).or_else(|| self.meta.get(id))
        } else {
            self.meta.get(id)
        }
        .cloned()
    }

    /// Set the meta value associated with the [`StateId`].
    pub fn set<T>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>)
    where
        T: StateValue + Clone,
    {
        if let Some(scope) = &mut self.scope {
            scope.set(id, value);
        } else {
            self.meta.set(id, value);
        }
    }

    /// Set the metadata value only if it was not set.
    ///
    /// This does not set the scoped override, only the command type metadata.
    pub fn init<T>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>)
    where
        T: StateValue + Clone,
    {
        self.meta.entry(id).or_insert(value);
    }

    /// Clone a meta variable identified by a [`CommandMetaVarId`].
    ///
    /// The variable is read-write and is clone-on-write if the command is scoped,
    /// call [`read_only`] to make it read-only.
    ///
    /// [`read_only`]: Var::read_only
    pub fn get_var_or_insert<T, F>(&mut self, id: impl Into<CommandMetaVarId<T>>, init: F) -> CommandMetaVar<T>
    where
        T: StateValue + VarValue,
        F: FnOnce() -> T,
    {
        let id = id.into();
        if let Some(scope) = &mut self.scope {
            let meta = &mut self.meta;
            scope
                .entry(id.scope())
                .or_insert_with(|| {
                    let var = meta.entry(id.app()).or_insert_with(|| var(init())).clone();
                    var.cow()
                })
                .clone()
                .boxed()
        } else {
            self.meta.entry(id.app()).or_insert_with(|| var(init())).clone().boxed()
        }
    }

    /// Clone a meta variable identified by a [`CommandMetaVarId`] it is was set.
    pub fn get_var<T>(&self, id: impl Into<CommandMetaVarId<T>>) -> Option<CommandMetaVar<T>>
    where
        T: StateValue + VarValue,
    {
        let id = id.into();
        if let Some(scope) = &self.scope {
            let meta = &self.meta;
            scope
                .get(id.scope())
                .map(|c| c.clone().boxed())
                .or_else(|| meta.get(id.app()).map(|c| c.clone().boxed()))
        } else {
            self.meta.get(id.app()).map(|c| c.clone().boxed())
        }
    }

    /// Clone a meta variable identified by a [`CommandMetaVarId`].
    ///
    /// Inserts a variable with the default value if no variable is in the metadata.
    pub fn get_var_or_default<T>(&mut self, id: impl Into<CommandMetaVarId<T>>) -> CommandMetaVar<T>
    where
        T: StateValue + VarValue + Default,
    {
        self.get_var_or_insert(id, Default::default)
    }

    /// Set the metadata variable if it was not set.
    ///
    /// This does not set the scoped override, only the command type metadata.
    pub fn init_var<T>(&mut self, id: impl Into<CommandMetaVarId<T>>, value: impl Into<T>)
    where
        T: StateValue + VarValue,
    {
        self.meta.entry(id.into().app()).or_insert_with(|| var(value.into()));
    }
}

/// Read-write command metadata variable.
///
/// If you get this variable from a not scoped command, setting it sets
/// the value for all scopes. If you get this variable using a scoped command,
/// setting it overrides only the value for the scope.
///
/// The boxed var is an [`ArcVar<T>`] for *app* scope, or [`ArcCowVar<T, ArcVar<T>>`] for scoped commands.
pub type CommandMetaVar<T> = BoxedVar<T>;

/// Read-only command metadata variable.
///
/// To convert a [`CommandMetaVar<T>`] into this var call [`read_only`].
///
/// [`read_only`]: Var::read_only
pub type ReadOnlyCommandMetaVar<T> = BoxedVar<T>;

/// Adds the [`name`](CommandNameExt) metadata.
pub trait CommandNameExt {
    /// Gets a read-write variable that is the display name for the command.
    fn name(self) -> CommandMetaVar<Txt>;

    /// Sets the initial name if it is not set.
    fn init_name(self, name: impl Into<Txt>) -> Self;

    /// Gets a read-only variable that formats the name and first shortcut in the following format: name (first_shortcut)
    /// Note: If no shortcuts are available this method returns the same as [`name`](Self::name)
    fn name_with_shortcut(self) -> BoxedVar<Txt>
    where
        Self: crate::gesture::CommandShortcutExt;
}
static COMMAND_NAME_ID: StaticCommandMetaVarId<Txt> = StaticCommandMetaVarId::new_unique();
impl CommandNameExt for Command {
    fn name(self) -> CommandMetaVar<Txt> {
        self.with_meta(|m| {
            m.get_var_or_insert(&COMMAND_NAME_ID, || {
                let name = self.event.name();
                let name = name.strip_suffix("_CMD").unwrap_or(name);
                let mut title = String::with_capacity(name.len());
                let mut lower = false;
                for c in name.chars() {
                    if c == '_' {
                        if !title.ends_with(' ') {
                            title.push(' ');
                        }
                        lower = false;
                    } else if lower {
                        for l in c.to_lowercase() {
                            title.push(l);
                        }
                    } else {
                        title.push(c);
                        lower = true;
                    }
                }
                Txt::from(title)
            })
        })
    }

    fn init_name(self, name: impl Into<Txt>) -> Self {
        self.with_meta(|m| m.init_var(&COMMAND_NAME_ID, name.into()));
        self
    }

    fn name_with_shortcut(self) -> BoxedVar<Txt>
    where
        Self: crate::gesture::CommandShortcutExt,
    {
        crate::var::merge_var!(self.name(), self.shortcut(), |name, shortcut| {
            if shortcut.is_empty() {
                name.clone()
            } else {
                crate::formatx!("{name} ({})", shortcut[0])
            }
        })
        .boxed()
    }
}

/// Adds the [`info`](CommandInfoExt) metadata.
pub trait CommandInfoExt {
    /// Gets a read-write variable that is a short informational string about the command.
    fn info(self) -> CommandMetaVar<Txt>;

    /// Sets the initial info if it is not set.
    fn init_info(self, info: impl Into<Txt>) -> Self;
}
static COMMAND_INFO_ID: StaticCommandMetaVarId<Txt> = StaticCommandMetaVarId::new_unique();
impl CommandInfoExt for Command {
    fn info(self) -> CommandMetaVar<Txt> {
        self.with_meta(|m| m.get_var_or_insert(&COMMAND_INFO_ID, Txt::default))
    }

    fn init_info(self, info: impl Into<Txt>) -> Self {
        self.with_meta(|m| m.init_var(&COMMAND_INFO_ID, info.into()));
        self
    }
}

enum CommandMetaState {}

#[doc(hidden)]
pub struct CommandData {
    meta_init: Option<fn(Command)>,
    meta: Mutex<OwnedStateMap<CommandMetaState>>,

    handle_count: usize,
    enabled_count: usize,
    registered: bool,

    has_handlers: ArcVar<bool>,
    is_enabled: ArcVar<bool>,

    scopes: FxHashMap<CommandScope, ScopedValue>,
}
impl CommandData {
    pub fn new(meta_init: fn(Command)) -> Self {
        CommandData {
            meta_init: Some(meta_init),
            meta: Mutex::new(OwnedStateMap::new()),

            handle_count: 0,
            enabled_count: 0,
            registered: false,

            has_handlers: var(false),
            is_enabled: var(false),

            scopes: FxHashMap::default(),
        }
    }

    fn subscribe(&mut self, events: &mut EventsService, command: Command, enabled: bool, mut target: Option<WidgetId>) -> CommandHandle {
        match command.scope {
            CommandScope::App => {
                if !mem::replace(&mut self.registered, true) {
                    events.register_command(command);
                }

                self.handle_count += 1;
                if enabled {
                    self.enabled_count += 1;
                }
            }
            scope => {
                let data = self.scopes.entry(scope).or_default();

                if !mem::replace(&mut data.registered, true) {
                    events.register_command(command);
                }

                data.handle_count += 1;
                if enabled {
                    data.enabled_count += 1;
                }

                if let CommandScope::Widget(id) = scope {
                    target = Some(id);
                }
            }
        };

        CommandHandle {
            command: Some(command),
            app_id: App::current_id(),
            local_enabled: AtomicBool::new(enabled),
            _event_handle: target.map(|t| command.event.subscribe(t)).unwrap_or_else(EventHandle::dummy),
        }
    }
}

struct ScopedValue {
    handle_count: usize,
    enabled_count: usize,
    is_enabled: ArcVar<bool>,
    has_handlers: ArcVar<bool>,
    meta: Mutex<OwnedStateMap<CommandMetaState>>,
    registered: bool,
}
impl Default for ScopedValue {
    fn default() -> Self {
        ScopedValue {
            is_enabled: var(false),
            has_handlers: var(false),
            handle_count: 0,
            enabled_count: 0,
            meta: Mutex::new(OwnedStateMap::default()),
            registered: false,
        }
    }
}

/// Helper for declaring command event properties.
///
/// This function is used by the [`command_property!`] macro.
///
/// # Command
///
/// The `cmd` closure is called on init to generate the command, it is a closure to allow
/// creation of widget scoped commands. The event handler will receive events for the command
/// and scope that target the widget where it is set.
///
/// # Enabled
///
/// The `enabled` closure is called on init to generate a boolean variable that defines
/// if the command handle is enabled. Command event handlers track both their existence and
/// the enabled flag, see [`Command::subscribe`] for details.
///
/// Note that the command handler can be enabled even when the widget is disabled, the widget
/// will receive the event while disabled in this case, you can use this to show feedback explaining
/// why the command cannot run.
///
/// # Route
///
/// The event `handler` is called after the [`on_pre_command`] equivalent at the same context level. If the command
/// event targets more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///  
/// [`propagation`]: AnyEventArgs::propagation
pub fn on_command<U, CB, E, EB, H>(child: U, command_builder: CB, enabled_builder: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    CB: FnMut() -> Command + Send + 'static,
    E: Var<bool>,
    EB: FnMut() -> E + Send + 'static,
    H: WidgetHandler<CommandArgs>,
{
    let mut enabled = None;
    let mut handle = None;
    let mut command = None;

    #[cfg(dyn_closure)]
    let mut command_builder: Box<dyn FnMut() -> Command + Send> = Box::new(command_builder);
    #[cfg(not(dyn_closure))]
    let mut command_builder = command_builder;

    #[cfg(dyn_closure)]
    let mut enabled_builder: Box<dyn FnMut() -> E + Send> = Box::new(enabled_builder);
    #[cfg(not(dyn_closure))]
    let mut enabled_builder = enabled_builder;

    let mut handler = handler.cfg_boxed();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            child.init();

            let e = enabled_builder();
            WIDGET.sub_var(&e);
            let is_enabled = e.get();
            enabled = Some(e);

            let c = command_builder();
            handle = Some(c.subscribe_wgt(is_enabled, WIDGET.id()));
            command = Some(c);
        }
        UiNodeOp::Deinit => {
            child.deinit();

            enabled = None;
            handle = None;
            command = None;
        }

        UiNodeOp::Event { update } => {
            child.event(update);

            if let Some(args) = command.expect("node not inited").on_unhandled(update) {
                handler.event(args);
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            handler.update();

            if let Some(enabled) = enabled.as_ref().expect("node not inited").get_new() {
                handle.as_ref().unwrap().set_enabled(enabled);
            }
        }

        _ => {}
    })
}

/// Helper for declaring command preview handlers.
///
/// # Command
///
/// The `cmd` closure is called on init to generate the command, it is a closure to allow
/// creation of widget scoped commands. The event handler will receive events for the command
/// and scope that target the widget where it is set.
///
/// # Enabled
///
/// The `enabled` closure is called on init to generate a boolean variable that defines
/// if the command handle is enabled. Command event handlers track both their existence and
/// the enabled flag, see [`Command::subscribe`] for details.
///
/// Note that the command handler can be enabled even when the widget is disabled, the widget
/// will receive the event while disabled in this case, you can use this to show feedback explaining
/// why the command cannot run.
///
/// # Route
///
/// The event `handler` is called before the [`on_command`] equivalent at the same context level. If the command event
/// targets more then one widget and one widget contains the other, the `handler` is called on the inner widget first.
///
/// # Async
///
/// Async event handlers are called like normal, but code after the first `.await` only runs in subsequent updates. This means
/// that [`propagation`] must be stopped before the first `.await`, otherwise you are only signaling
/// other async tasks handling the same event, if they are monitoring the propagation handle.
///  
/// [`propagation`]: AnyEventArgs::propagation
pub fn on_pre_command<U, CB, E, EB, H>(child: U, cmd: CB, enabled: EB, handler: H) -> impl UiNode
where
    U: UiNode,
    CB: FnMut() -> Command + Send + 'static,
    E: Var<bool>,
    EB: FnMut() -> E + Send + 'static,
    H: WidgetHandler<CommandArgs>,
{
    #[cfg(dyn_closure)]
    let mut command_builder: Box<dyn FnMut() -> Command + Send> = Box::new(cmd);
    #[cfg(not(dyn_closure))]
    let mut command_builder = cmd;

    #[cfg(dyn_closure)]
    let mut enabled_builder: Box<dyn FnMut() -> E + Send> = Box::new(enabled);
    #[cfg(not(dyn_closure))]
    let mut enabled_builder = enabled;

    let mut handler = handler.cfg_boxed();

    let mut enabled = None;
    let mut handle = None;
    let mut command = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            child.init();

            let e = enabled_builder();
            WIDGET.sub_var(&e);
            let is_enabled = e.get();
            enabled = Some(e);

            let c = command_builder();
            handle = Some(c.subscribe_wgt(is_enabled, WIDGET.id()));
            command = Some(c);
        }
        UiNodeOp::Deinit => {
            child.deinit();

            enabled = None;
            handle = None;
            command = None;
        }

        UiNodeOp::Event { update } => {
            if let Some(args) = command.expect("on_pre_command not initialized").on_unhandled(update) {
                handler.event(args);
            }
        }
        UiNodeOp::Update { .. } => {
            handler.update();

            if let Some(enabled) = enabled.as_ref().expect("on_pre_command not initialized").get_new() {
                handle.as_ref().unwrap().set_enabled(enabled);
            }
        }

        _ => {}
    })
}

#[doc(hidden)]
#[macro_export]
macro_rules! __command_property {
    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd: $cmd_init:expr,
            enabled: $enabled_var:expr,
        }
    ) => { $crate::paste! {
        $(#[$on_cmd_attrs])*
        ///
        /// # Preview
        ///
        #[doc = "You can preview this command event using [`on_pre_"$command "`](fn.on_pre_"$command ".html)."]
        /// Otherwise the handler is only called after the widget content has a chance of handling the event by stopping propagation.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property.
        #[$crate::property(EVENT, default( $crate::handler::hn!(|_|{}) ))]
        $vis fn [<on_ $command>](
            child: impl $crate::widget_instance::UiNode,
            handler: impl $crate::handler::WidgetHandler<$crate::event::CommandArgs>,
        ) -> impl $crate::widget_instance::UiNode {
            $crate::event::on_command(child, || $cmd_init, || $enabled_var, handler)
        }

        #[doc = "Preview [`on_"$command "`](fn.on_"$command ".html) event."]
        ///
        /// # Preview
        ///
        /// Preview event properties call the handler before the main event property and before the widget content, if you stop
        /// the propagation of a preview event the main event handler is not called.
        ///
        /// # Async
        ///
        /// You can use async event handlers with this property, note that only the code before the fist `.await` is *preview*,
        /// subsequent code runs in widget updates.
        #[$crate::property(EVENT, default( $crate::handler::hn!(|_|{}) ))]
        $vis fn [<on_pre_ $command>](
            child: impl $crate::widget_instance::UiNode,
            handler: impl $crate::handler::WidgetHandler<$crate::event::CommandArgs>,
        ) -> impl $crate::widget_instance::UiNode {
            $crate::event::on_pre_command(child, || $cmd_init, || $enabled_var, handler)
        }
    } };

    (
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd: $cmd_init:expr,
        }
    ) => {
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd: $cmd_init,
                enabled: $crate::var::LocalVar(true),
            }
        }
    };
}

///<span data-del-macro-root></span> Declare one or more command event properties.
///
/// Each declaration expands to two properties `on_$command`, `on_pre_$command`.
/// The preview properties call [`on_pre_command`], the main event properties call [`on_command`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// # use zero_ui_core::event::*;
/// # use zero_ui_core::context::*;
/// # use zero_ui_core::var::*;
/// # command! {
/// #   pub static PASTE_CMD;
/// # }
/// command_property! {
///     /// Paste command property docs.
///     pub fn paste {
///         cmd: PASTE_CMD.scoped(WIDGET.id()),
///         // enabled: LocalVar(true), // default enabled
///     }
/// }
/// ```
///
/// # Command
///
/// The `cmd` closure is called on init to generate the command, it is a closure to allow
/// creation of widget scoped commands. The event handler will receive events for the command
/// and scope that target the widget where it is set.
///
/// # Enabled
///
/// The `enabled` closure is called on init to generate a boolean variable that defines
/// if the command handle is enabled. Command event handlers track both their existence and
/// the enabled flag, see [`Command::subscribe`] for details.
///
/// If not provided the command is always enabled.
///
/// # Async
///
/// Async event handlers are supported by properties generated by this macro, but only the code before the first `.await` executes
/// in the event track, subsequent code runs in widget updates.
#[macro_export]
macro_rules! command_property {
    ($(
        $(#[$on_cmd_attrs:meta])*
        $vis:vis fn $command:ident {
            cmd: $cmd_init:expr$(,
            enabled: $enabled_var:expr)? $(,)?
        }
    )+) => {$(
        $crate::__command_property! {
            $(#[$on_cmd_attrs])*
            $vis fn $command {
                cmd: $cmd_init,
                $(enabled: $enabled_var,)?
            }
        }
    )+};
}
#[doc(inline)]
pub use crate::command_property;

#[cfg(test)]
mod tests {
    use super::*;

    command! {
        static FOO_CMD;
    }

    #[test]
    fn parameter_none() {
        let _ = CommandArgs::now(None, CommandScope::App, true);
    }

    #[test]
    fn enabled() {
        let _app = App::minimal().run_headless(false);

        assert!(!FOO_CMD.has_handlers_value());

        let handle = FOO_CMD.subscribe(true);
        assert!(FOO_CMD.is_enabled_value());

        handle.set_enabled(false);
        assert!(FOO_CMD.has_handlers_value());
        assert!(!FOO_CMD.is_enabled_value());

        handle.set_enabled(true);
        assert!(FOO_CMD.is_enabled_value());

        drop(handle);
        assert!(!FOO_CMD.has_handlers_value());
    }

    #[test]
    fn enabled_scoped() {
        let _app = App::minimal().run_headless(false);

        let cmd = FOO_CMD;
        let cmd_scoped = FOO_CMD.scoped(WindowId::named("enabled_scoped"));
        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.has_handlers_value());

        let handle_scoped = cmd_scoped.subscribe(true);
        assert!(!cmd.has_handlers_value());
        assert!(cmd_scoped.is_enabled_value());

        handle_scoped.set_enabled(false);
        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.is_enabled_value());
        assert!(cmd_scoped.has_handlers_value());

        handle_scoped.set_enabled(true);
        assert!(!cmd.has_handlers_value());
        assert!(cmd_scoped.is_enabled_value());

        drop(handle_scoped);
        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.has_handlers_value());
    }

    #[test]
    fn has_handlers() {
        let _app = App::minimal().run_headless(false);

        assert!(!FOO_CMD.has_handlers_value());

        let handle = FOO_CMD.subscribe(false);
        assert!(FOO_CMD.has_handlers_value());

        drop(handle);
        assert!(!FOO_CMD.has_handlers_value());
    }

    #[test]
    fn has_handlers_scoped() {
        let _app = App::minimal().run_headless(false);

        let cmd = FOO_CMD;
        let cmd_scoped = FOO_CMD.scoped(WindowId::named("has_handlers_scoped"));

        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.has_handlers_value());

        let handle = cmd_scoped.subscribe(false);

        assert!(!cmd.has_handlers_value());
        assert!(cmd_scoped.has_handlers_value());

        drop(handle);

        assert!(!cmd.has_handlers_value());
        assert!(!cmd_scoped.has_handlers_value());
    }

    // there are also integration tests in tests/command.rs
}
