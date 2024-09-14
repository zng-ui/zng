use std::{
    any::TypeId,
    collections::{hash_map, HashMap},
    mem, ops,
    thread::ThreadId,
};

use crate::{shortcut::CommandShortcutExt, update::UpdatesTrace, widget::info::WidgetInfo, window::WindowId, APP};

use super::*;

/// <span data-del-macro-root></span> Declares new [`Command`] static items.
///
/// Command static items represent widget or service actions. Command items are also events, that is they dereference
/// to [`Event<A>`] and *override* some event methods to enable communication from the command subscribers to the command
/// notifier. Command static items also host metadata about the command.
///
/// [`Event<A>`]: crate::event::Event
///
/// # Conventions
///
/// Command events have the `_CMD` suffix, for example a command for the clipboard *copy* action is called `COPY_CMD`.
/// Public and user facing commands also set the [`CommandNameExt`] and [`CommandInfoExt`] with localized display text.
///
/// # Shortcuts
///
/// You can give commands one or more shortcuts using the [`CommandShortcutExt`], the `GestureManager` notifies commands
/// that match a pressed shortcut automatically.
///
/// # Properties
///
/// If the command implementation is not specific you can use `command_property!` to declare properties that setup command handlers
/// for the command.
///
/// # Examples
///
/// Declare two commands:
///
/// ```
/// use zng_app::event::command;
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
/// use zng_app::{event::{command, CommandNameExt, CommandInfoExt}, shortcut::{CommandShortcutExt, shortcut}};
///
/// command! {
///     /// Represents the **foo** action.
///     pub static FOO_CMD = {
///         name: "Foo!",
///         info: "Does the foo thing",
///         shortcut: shortcut![CTRL+'F'],
///     };
/// }
/// ```
///
/// The initialization uses the [command extensions] pattern and runs once for each app.
///
/// Or you can use a custom closure to initialize the command:
///
/// ```
/// use zng_app::{event::{command, CommandNameExt, CommandInfoExt}, shortcut::{CommandShortcutExt, shortcut}};
///
/// command! {
///     /// Represents the **foo** action.
///     pub static FOO_CMD => |cmd| {
///         cmd.init_name("Foo!");
///         cmd.init_info("Does the foo thing.");
///         cmd.init_shortcut(shortcut![CTRL+'F']);
///     };
/// }
/// ```
///
/// For the first kind of metadata initialization a documentation section is also generated with a table of metadata.
///
/// # Localization
///
/// If the first metadata is `l10n!:` the command init will attempt to localize the other string metadata. The `cargo zng l10n`
/// command line tool scraps commands that set this special metadata.
///
/// ```
/// # use zng_app::{event::{command, CommandNameExt, CommandInfoExt}, shortcut::{CommandShortcutExt, shortcut}};
/// command! {
///     pub static FOO_CMD = {
///         l10n!: true,
///         name: "Foo!",
///         info: "Does the foo thing",
///     };
/// }
/// ```
///
/// The example above will be scrapped as:
///
/// ```ftl
/// FOO_CMD =
///     .name = Foo!
///     .info = Does the foo thing.
/// ```
///
/// The `l10n!:` meta can also be set to a localization file name:
///
/// ```
/// # use zng_app::{event::{command, CommandNameExt, CommandInfoExt}, shortcut::{CommandShortcutExt, shortcut}};
/// command! {
///     pub static FOO_CMD = {
///         l10n!: "file",
///         name: "Foo!",
///     };
/// }
/// ```
///
/// The example above is scrapped to `{l10n-dir}/{lang}/file.ftl` files.
///
/// ## Limitations
///
/// Interpolation is not supported in command localization strings.
///
/// The `l10n!:` value must be a *textual* literal, that is, it can be only a string literal or a `bool` literal, and it cannot be
/// inside a macro expansion.
///
/// [`Command`]: crate::event::Command
/// [`CommandArgs`]: crate::event::CommandArgs
/// [`CommandNameExt`]: crate::event::CommandNameExt
/// [`CommandInfoExt`]: crate::event::CommandInfoExt
/// [`Event`]: crate::event::Event
/// [command extensions]: crate::event::Command#extensions
/// [`CommandShortcutExt`]: crate::shortcut::CommandShortcutExt
#[macro_export]
macro_rules! command {
    ($(
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident $(=> |$cmd:ident|$custom_meta_init:expr ;)? $(= { $($meta_ident:ident $(!)? : $meta_init:expr),* $(,)? };)? $(;)?
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

use zng_app_context::AppId;
use zng_state_map::{OwnedStateMap, StateId, StateMapMut, StateValue};
use zng_txt::Txt;
use zng_unique_id::{static_id, unique_id_64};
use zng_var::{impl_from_and_into_var, types::ArcCowVar, var, AnyVar, ArcVar, BoxedVar, ReadOnlyArcVar, Var, VarValue};

#[doc(hidden)]
pub use zng_app_context::app_local;

#[doc(hidden)]
pub use paste::paste;

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
            $crate::event::app_local! {
                static EVENT: $crate::event::EventData = const { $crate::event::EventData::new(std::stringify!($COMMAND)) };
                static DATA: $crate::event::CommandData = $crate::event::CommandData::new(__meta_init__);
            }
            $crate::event::Command::new(&EVENT, &DATA)
        };
    };
    (
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident = { l10n: $l10n_arg:expr, $($meta_ident:ident : $meta_init:expr),* $(,)? };
    ) => {
        $crate::event::paste! {
            $crate::__command! {
                $(#[$attr])*
                ///
                /// # Metadata
                ///
                /// This command has the following default metadata:
                ///
                /// <table>
                /// <thead><tr><th>metadata</th><th>value</th></tr></thead>
                /// <tbody>
                $(#[doc = concat!("<tr> <td>", stringify!($meta_ident), "</td> <td>", stringify!($meta_init), "</td> </tr>")])+
                ///
                /// </tbody>
                /// </table>
                ///
                /// Text metadata is localized.
                $vis static $COMMAND => |cmd| {
                    let __l10n_arg = $l10n_arg;
                    $(
                        cmd.[<init_ $meta_ident>]($meta_init);
                        $crate::event::init_meta_l10n(std::env!("CARGO_PKG_NAME"), std::env!("CARGO_PKG_VERSION"), &__l10n_arg, cmd, stringify!($meta_ident), &cmd.$meta_ident());
                    )*
                };
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis static $COMMAND:ident = { $($meta_ident:ident : $meta_init:expr),* $(,)? };
    ) => {
        $crate::event::paste! {
            $crate::__command! {
                $(#[$attr])*
                ///
                /// # Metadata
                ///
                /// This command has the following default metadata:
                ///
                /// <table>
                /// <thead><tr><th>metadata</th><th>value</th></tr></thead>
                /// <tbody>
                $(#[doc = concat!("<tr> <td>", stringify!($meta_ident), "</td> <td>", stringify!($meta_init), "</td> </tr>")])+
                ///
                /// </tbody>
                /// </table>
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

#[doc(hidden)]
pub fn init_meta_l10n(
    pkg_name: &'static str,
    pkg_version: &'static str,
    l10n_arg: &dyn Any,
    cmd: Command,
    meta_name: &'static str,
    meta_value: &dyn Any,
) {
    if let Some(txt) = meta_value.downcast_ref::<CommandMetaVar<Txt>>() {
        let mut l10n_file = "";

        if let Some(&enabled) = l10n_arg.downcast_ref::<bool>() {
            if !enabled {
                return;
            }
        } else if let Some(&file) = l10n_arg.downcast_ref::<&'static str>() {
            l10n_file = file;
        } else {
            tracing::error!("unknown l10n value in {}", cmd.event().as_any().name());
            return;
        }

        EVENTS_L10N.init_meta_l10n([pkg_name, pkg_version, l10n_file], cmd, meta_name, txt.clone());
    }
}

/// Identifies a command event.
///
/// Use the [`command!`] to declare commands, it declares command static items with optional
/// [metadata](#metadata) initialization.
///
/// ```
/// # use zng_app::event::*;
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
/// Commands can have associated metadata, this metadata is extendable and can be used to enable
/// command features such as command shortcuts. The metadata can be accessed using [`with_meta`], metadata
/// extensions traits can use this metadata to store state. See [`CommandMeta`] for more details.
///
/// # Handles
///
/// Unlike other events, commands only notify if it has at least one handler, handlers
/// must call [`subscribe`] to indicate that the command is relevant to the current app state and
/// set the subscription handle [enabled] flag to indicate that the handler can fulfill command requests.
///
/// # Scopes
///
/// Commands are *global* by default, meaning an enabled handle anywhere in the app enables it everywhere.
/// You can use [`scoped`] to declare *sub-commands* that are the same command event, but filtered to a scope, metadata
/// of scoped commands inherit from the app scope metadata, but can be overridden just for the scope.
///
/// [`command!`]: macro@crate::event::command
/// [`subscribe`]: Command::subscribe
/// [enabled]: CommandHandle::set_enabled
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
    /// A handle indicates that command handlers are present in the current app, the `enabled` flag
    /// indicates the handler is ready to fulfill command requests.
    ///
    /// If the command is scoped on a window or widget if it is added to the command event subscribers.
    pub fn subscribe(&self, enabled: bool) -> CommandHandle {
        let mut evs = EVENTS_SV.write();
        self.local.write().subscribe(&mut evs, *self, enabled, None)
    }

    /// Create a new handle for this command for a handler in the `target` widget.
    ///
    /// The handle behaves like [`subscribe`], but include the `target` on the delivery list for app scoped commands.
    /// Note this only works for global commands (app scope), window and widget scoped commands only notify the scope
    /// so the `target` is ignored for scoped commands.
    ///
    /// [`subscribe`]: Command::subscribe
    pub fn subscribe_wgt(&self, enabled: bool, target: WidgetId) -> CommandHandle {
        let mut evs = EVENTS_SV.write();
        self.local.write().subscribe(&mut evs, *self, enabled, Some(target))
    }

    /// Underlying event that represents this command in any scope.
    pub fn event(&self) -> Event<CommandArgs> {
        self.event
    }

    /// Command scope.
    pub fn scope(&self) -> CommandScope {
        self.scope
    }

    /// Gets the command in a new `scope`.
    pub fn scoped(mut self, scope: impl Into<CommandScope>) -> Command {
        self.scope = scope.into();
        self
    }

    /// Visit the command custom metadata of the current scope.
    ///
    /// Metadata for [`CommandScope::App`] is retained for the duration of the app, metadata scoped
    /// on window or widgets is dropped after an update cycle with no handler and no strong references
    /// to [`has_handlers`] and [`is_enabled`].
    ///
    /// [`has_handlers`]: Self::has_handlers
    /// [`is_enabled`]: Self::is_enabled
    pub fn with_meta<R>(&self, visit: impl FnOnce(&mut CommandMeta) -> R) -> R {
        {
            let mut write = self.local.write();
            match write.meta_init.clone() {
                MetaInit::Init(init) => {
                    let lock = Arc::new((std::thread::current().id(), Mutex::new(())));
                    write.meta_init = MetaInit::Initing(lock.clone());
                    let _init_guard = lock.1.lock();
                    drop(write);
                    init(*self);
                    self.local.write().meta_init = MetaInit::Inited;
                }
                MetaInit::Initing(l) => {
                    drop(write);
                    if l.0 != std::thread::current().id() {
                        let _wait = l.1.lock();
                    }
                }
                MetaInit::Inited => {}
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

    /// Calls `handler` if the update is for this event and propagation is not stopped,
    /// after the handler is called propagation is stopped.
    pub fn handle<R>(&self, update: &EventUpdate, handler: impl FnOnce(&CommandArgs) -> R) -> Option<R> {
        if let Some(args) = self.on(update) {
            args.handle(handler)
        } else {
            None
        }
    }

    /// Gets a variable that tracks if this command has any handlers.
    pub fn has_handlers(&self) -> ReadOnlyArcVar<bool> {
        let mut write = self.local.write();
        match self.scope {
            CommandScope::App => write.has_handlers.read_only(),
            scope => write.scopes.entry(scope).or_default().has_handlers.read_only(),
        }
    }

    /// Gets a variable that tracks if this command has any enabled handlers.
    pub fn is_enabled(&self) -> ReadOnlyArcVar<bool> {
        let mut write = self.local.write();
        match self.scope {
            CommandScope::App => write.is_enabled.read_only(),
            scope => write.scopes.entry(scope).or_default().is_enabled.read_only(),
        }
    }

    /// Gets if the command has handlers without creating a tracking variable for the state.
    pub fn has_handlers_value(&self) -> bool {
        let read = self.local.read();
        match self.scope {
            CommandScope::App => read.handle_count > 0,
            scope => read.scopes.get(&scope).map(|l| l.handle_count > 0).unwrap_or(false),
        }
    }

    /// Gets if the command is enabled without creating a tracking variable for the state.
    pub fn is_enabled_value(&self) -> bool {
        let read = self.local.read();
        match self.scope {
            CommandScope::App => read.enabled_count > 0,
            scope => read.scopes.get(&scope).map(|l| l.enabled_count > 0).unwrap_or(false),
        }
    }

    /// Calls `visitor` for each scope of this command.
    ///
    /// Note that scoped commands are removed if unused, see [`with_meta`](Self::with_meta) for more details.
    pub fn visit_scopes(&self, mut visitor: impl FnMut(Command)) {
        let read = self.local.read();
        for &scope in read.scopes.keys() {
            visitor(self.scoped(scope));
        }
    }

    /// Schedule a command update without param.
    pub fn notify(&self) {
        self.event.notify(CommandArgs::now(None, self.scope, self.is_enabled_value()))
    }

    /// Schedule a command update without param for all scopes inside `parent`.
    pub fn notify_descendants(&self, parent: &WidgetInfo) {
        self.visit_scopes(|parse_cmd| {
            if let CommandScope::Widget(id) = parse_cmd.scope() {
                if let Some(scope) = parent.tree().get(id) {
                    if scope.is_descendant(parent) {
                        parse_cmd.notify();
                    }
                }
            }
        });
    }

    /// Schedule a command update with custom `param`.
    pub fn notify_param(&self, param: impl Any + Send + Sync) {
        self.event
            .notify(CommandArgs::now(CommandParam::new(param), self.scope, self.is_enabled_value()));
    }

    /// Schedule a command update linked with an external event `propagation`.
    pub fn notify_linked(&self, propagation: EventPropagationHandle, param: Option<CommandParam>) {
        self.event.notify(CommandArgs::new(
            crate::INSTANT.now(),
            propagation,
            param,
            self.scope,
            self.is_enabled_value(),
        ))
    }

    /// Creates a preview event handler for the command.
    ///
    /// This is similar to [`Event::on_pre_event`], but `handler` is only called if the command
    /// scope matches.
    ///
    /// The `enabled` parameter defines the initial state of the command subscription, the subscription
    /// handle is available in the handler args.
    pub fn on_pre_event<H>(&self, enabled: bool, handler: H) -> EventHandle
    where
        H: AppHandler<AppCommandArgs>,
    {
        self.event().on_pre_event(CmdAppHandler {
            handler,
            handle: Arc::new(self.subscribe(enabled)),
        })
    }

    /// Creates an event handler for the command.
    ///
    /// This is similar to [`Event::on_event`], but `handler` is only called if the command
    /// scope matches.
    ///
    /// The `enabled` parameter defines the initial state of the command subscription, the subscription
    /// handle is available in the handler args.
    pub fn on_event<H>(&self, enabled: bool, handler: H) -> EventHandle
    where
        H: AppHandler<AppCommandArgs>,
    {
        self.event().on_event(CmdAppHandler {
            handler,
            handle: Arc::new(self.subscribe(enabled)),
        })
    }

    /// Update state vars, returns if the command must be retained.
    #[must_use]
    pub(crate) fn update_state(&self) -> bool {
        let mut write = self.local.write();
        if let CommandScope::App = self.scope {
            let has_handlers = write.handle_count > 0;
            if has_handlers != write.has_handlers.get() {
                write.has_handlers.set(has_handlers);
            }
            let is_enabled = has_handlers && write.enabled_count > 0;
            if is_enabled != write.is_enabled.get() {
                write.is_enabled.set(is_enabled);
            }
            true
        } else if let hash_map::Entry::Occupied(entry) = write.scopes.entry(self.scope) {
            let scope = entry.get();

            if scope.handle_count == 0 && scope.has_handlers.strong_count() == 1 && scope.is_enabled.strong_count() == 1 {
                entry.remove();
                return false;
            }

            let has_handlers = scope.handle_count > 0;
            if has_handlers != scope.has_handlers.get() {
                scope.has_handlers.set(has_handlers);
            }
            let is_enabled = has_handlers && scope.enabled_count > 0;
            if is_enabled != scope.is_enabled.get() {
                scope.is_enabled.set(is_enabled);
            }
            true
        } else {
            false
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
impl std::hash::Hash for Command {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.event.as_any(), state);
        std::hash::Hash::hash(&self.scope, state);
    }
}

struct CmdAppHandler<H> {
    handler: H,
    handle: Arc<CommandHandle>,
}
impl<H: AppHandler<AppCommandArgs>> AppHandler<CommandArgs> for CmdAppHandler<H> {
    fn event(&mut self, args: &CommandArgs, handler_args: &AppHandlerArgs) {
        let args = AppCommandArgs {
            args: args.clone(),
            handle: self.handle.clone(),
        };
        self.handler.event(&args, handler_args);
    }
}

/// Represents the scope of a [`Command`].
///
/// The command scope defines the targets of its event and the context of its metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CommandScope {
    /// Default scope, this is the scope of command types declared using [`command!`].
    App,
    /// Scope of a window.
    ///
    /// Note that the window scope is different from the window root widget scope, the metadata store and command
    /// handles are different, but subscribers set on the window root should probably also subscribe to the window scope.
    Window(WindowId),
    /// Scope of a widget.
    Widget(WidgetId),
}
impl_from_and_into_var! {
    fn from(id: WidgetId) -> CommandScope {
        CommandScope::Widget(id)
    }
    fn from(id: WindowId) -> CommandScope {
        CommandScope::Window(id)
    }
    /// Widget scope.
    fn from(widget_name: &'static str) -> CommandScope {
        WidgetId::named(widget_name).into()
    }
    /// Widget scope.
    fn from(widget_name: Txt) -> CommandScope {
        WidgetId::named(widget_name).into()
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

        /// Broadcast to all widget subscribers for [`CommandScope::App`]. Targets the window root for
        /// [`CommandScope::Window`] if found. Target ancestors and widget for [`CommandScope::Widget`], if it is found.
        fn delivery_list(&self, list: &mut UpdateDeliveryList) {
            match self.scope {
                CommandScope::Widget(id) => list.search_widget(id),
                CommandScope::Window(id) => list.insert_window(id),
                CommandScope::App => list.search_all(),
            }
        }
    }
}
impl CommandArgs {
    /// Returns a reference to a parameter of `T` if [`parameter`](#structfield.parameter) is set to a value of `T`.
    pub fn param<T: Any>(&self) -> Option<&T> {
        self.param.as_ref().and_then(|p| p.downcast_ref::<T>())
    }

    /// Returns [`param`] if is [`enabled`].
    ///
    /// [`param`]: Self::param()
    /// [`enabled`]: Self::enabled
    pub fn enabled_param<T: Any>(&self) -> Option<&T> {
        if self.enabled {
            self.param::<T>()
        } else {
            None
        }
    }

    /// Returns [`param`] if is not [`enabled`].
    ///
    /// [`param`]: Self::param()
    /// [`enabled`]: Self::enabled
    pub fn disabled_param<T: Any>(&self) -> Option<&T> {
        if !self.enabled {
            self.param::<T>()
        } else {
            None
        }
    }

    /// Call `handler` if propagation is not stopped and the command and local handler are enabled. Stops propagation
    /// after `handler` is called.
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
        if self.propagation().is_stopped() || !self.enabled || !local_handle.is_enabled() {
            None
        } else {
            let r = handler(self);
            self.propagation().stop();
            Some(r)
        }
    }
}

/// Arguments for [`Command::on_event`].
#[derive(Debug, Clone)]
pub struct AppCommandArgs {
    /// The command args.
    pub args: CommandArgs,
    /// The command handle held by the event handler.
    pub handle: Arc<CommandHandle>,
}
impl ops::Deref for AppCommandArgs {
    type Target = CommandArgs;

    fn deref(&self) -> &Self::Target {
        &self.args
    }
}
impl AnyEventArgs for AppCommandArgs {
    fn clone_any(&self) -> Box<dyn AnyEventArgs> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn timestamp(&self) -> crate::DInstant {
        self.args.timestamp()
    }

    fn delivery_list(&self, list: &mut UpdateDeliveryList) {
        self.args.delivery_list(list)
    }

    fn propagation(&self) -> &EventPropagationHandle {
        self.args.propagation()
    }
}
impl EventArgs for AppCommandArgs {}

/// A handle to a [`Command`] subscription.
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
                if self.app_id != APP.id() {
                    return;
                }

                UpdatesTrace::log_var(std::any::type_name::<bool>());

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
            if self.app_id != APP.id() {
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

/// Represents a reference counted `dyn Any` object parameter for a command request.
#[derive(Clone)]
pub struct CommandParam(pub Arc<dyn Any + Send + Sync>);
impl PartialEq for CommandParam {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for CommandParam {}
impl CommandParam {
    /// New param.
    ///
    /// If `param` is already a [`CommandParam`] returns a clone.
    pub fn new(param: impl Any + Send + Sync + 'static) -> Self {
        let p: &dyn Any = &param;
        if let Some(p) = p.downcast_ref::<Self>() {
            p.clone()
        } else {
            CommandParam(Arc::new(param))
        }
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
zng_var::impl_from_and_into_var! {
    fn from(param: CommandParam) -> Option<CommandParam>;
}

unique_id_64! {
    /// Unique identifier of a command metadata state variable.
    ///
    /// This type is very similar to [`StateId`], but `T` is the value type of the metadata variable.
    ///
    /// [`StateId`]: zng_state_map::StateId
    pub struct CommandMetaVarId<T: (StateValue + VarValue)>: StateId;
}
zng_unique_id::impl_unique_id_bytemuck!(CommandMetaVarId<T: (StateValue + VarValue)>);
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
/// The [`command!`] initialization transforms `foo: true,` to `command.init_foo(true);`, to support that, the command extension trait
/// must have a `foo` and `init_foo` methods.
///
/// ```
/// use zng_app::{event::*, var::*, static_id};
///
/// static_id! {
///     static ref COMMAND_FOO_ID: CommandMetaVarId<bool>;
///     static ref COMMAND_BAR_ID: CommandMetaVarId<bool>;
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
/// impl CommandFooBarExt for Command {
///     fn foo(self) -> CommandMetaVar<bool> {
///         self.with_meta(|m| m.get_var_or_default(*COMMAND_FOO_ID))
///     }
///
///     fn bar(self) -> ReadOnlyCommandMetaVar<bool> {
///         self.with_meta(|m| m.get_var_or_insert(*COMMAND_BAR_ID, ||true)).read_only()
///     }
///
///     fn foo_and_bar(self) -> BoxedVar<bool> {
///         merge_var!(self.foo(), self.bar(), |f, b| *f && *b).boxed()
///     }
///
///     fn init_foo(self, foo: bool) -> Self {
///         self.with_meta(|m| m.init_var(*COMMAND_FOO_ID, foo));
///         self
///     }
///
///     fn init_bar(self, bar: bool) -> Self {
///         self.with_meta(|m| m.init_var(*COMMAND_BAR_ID, bar));
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
    ///
    /// [`StateId`]: zng_state_map::StateId
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
    ///
    /// [`StateId`]: zng_state_map::StateId
    pub fn get_or_default<T>(&mut self, id: impl Into<StateId<T>>) -> T
    where
        T: StateValue + Clone + Default,
    {
        self.get_or_insert(id, Default::default)
    }

    /// Clone a meta value identified by a [`StateId`] if it is set.
    ///
    /// [`StateId`]: zng_state_map::StateId
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
    ///
    /// [`StateId`]: zng_state_map::StateId
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

    /// Set the metadata value only if it is not set.
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
    /// The variable is read-write and is clone-on-write if the command is scoped.
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

    /// Clone a meta variable identified by a [`CommandMetaVarId`], if it is set.
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
/// The boxed var is an [`ArcVar<T>`] for *app* scope, or [`ArcCowVar<T, ArcVar<T>>`] for scoped commands.
/// If you get this variable from an app scoped command it sets
/// the value for all scopes. If you get this variable using a scoped command,
/// it is a clone-on-write variable that overrides only the value for the scope.
///
/// [`ArcVar<T>`]: zng_var::ArcVar
/// [`ArcCowVar<T, ArcVar<T>>`]: zng_var::types::ArcCowVar
pub type CommandMetaVar<T> = BoxedVar<T>;

/// Read-only command metadata variable.
///
/// To convert a [`CommandMetaVar<T>`] into this var call [`read_only`].
///
/// [`read_only`]: Var::read_only
pub type ReadOnlyCommandMetaVar<T> = BoxedVar<T>;

/// Adds the [`name`](CommandNameExt) command metadata.
pub trait CommandNameExt {
    /// Gets a read-write variable that is the display name for the command.
    fn name(self) -> CommandMetaVar<Txt>;

    /// Sets the initial name if it is not set.
    fn init_name(self, name: impl Into<Txt>) -> Self;

    /// Gets a read-only variable that formats the name and first shortcut in the following format: name (first_shortcut)
    /// Note: If no shortcuts are available this method returns the same as [`name`](Self::name)
    fn name_with_shortcut(self) -> BoxedVar<Txt>
    where
        Self: crate::shortcut::CommandShortcutExt;
}
static_id! {
    static ref COMMAND_NAME_ID: CommandMetaVarId<Txt>;
}
impl CommandNameExt for Command {
    fn name(self) -> CommandMetaVar<Txt> {
        self.with_meta(|m| {
            m.get_var_or_insert(*COMMAND_NAME_ID, || {
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
        self.with_meta(|m| m.init_var(*COMMAND_NAME_ID, name.into()));
        self
    }

    fn name_with_shortcut(self) -> BoxedVar<Txt>
    where
        Self: crate::shortcut::CommandShortcutExt,
    {
        crate::var::merge_var!(self.name(), self.shortcut(), |name, shortcut| {
            if shortcut.is_empty() {
                name.clone()
            } else {
                zng_txt::formatx!("{name} ({})", shortcut[0])
            }
        })
        .boxed()
    }
}

/// Adds the [`info`](CommandInfoExt) command metadata.
pub trait CommandInfoExt {
    /// Gets a read-write variable that is a short informational string about the command.
    fn info(self) -> CommandMetaVar<Txt>;

    /// Sets the initial info if it is not set.
    fn init_info(self, info: impl Into<Txt>) -> Self;
}
static_id! {
    static ref COMMAND_INFO_ID: CommandMetaVarId<Txt>;
}
impl CommandInfoExt for Command {
    fn info(self) -> CommandMetaVar<Txt> {
        self.with_meta(|m| m.get_var_or_insert(*COMMAND_INFO_ID, Txt::default))
    }

    fn init_info(self, info: impl Into<Txt>) -> Self {
        self.with_meta(|m| m.init_var(*COMMAND_INFO_ID, info.into()));
        self
    }
}

enum CommandMetaState {}

#[derive(Clone)]
enum MetaInit {
    Init(fn(Command)),
    /// Initing in a thread, lock is for other threads.
    Initing(Arc<(ThreadId, Mutex<()>)>),
    Inited,
}

#[doc(hidden)]
pub struct CommandData {
    meta_init: MetaInit,
    meta: Mutex<OwnedStateMap<CommandMetaState>>,

    handle_count: usize,
    enabled_count: usize,
    registered: bool,

    has_handlers: ArcVar<bool>,
    is_enabled: ArcVar<bool>,

    scopes: HashMap<CommandScope, ScopedValue>,
}
impl CommandData {
    pub fn new(meta_init: fn(Command)) -> Self {
        CommandData {
            meta_init: MetaInit::Init(meta_init),
            meta: Mutex::new(OwnedStateMap::new()),

            handle_count: 0,
            enabled_count: 0,
            registered: false,

            has_handlers: var(false),
            is_enabled: var(false),

            scopes: HashMap::default(),
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
            app_id: APP.id(),
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
        let _app = APP.minimal().run_headless(false);

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
        let _app = APP.minimal().run_headless(false);

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
        let _app = APP.minimal().run_headless(false);

        assert!(!FOO_CMD.has_handlers_value());

        let handle = FOO_CMD.subscribe(false);
        assert!(FOO_CMD.has_handlers_value());

        drop(handle);
        assert!(!FOO_CMD.has_handlers_value());
    }

    #[test]
    fn has_handlers_scoped() {
        let _app = APP.minimal().run_headless(false);

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
