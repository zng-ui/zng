/// Identifies a command type.
///
/// Use [`command!`](macro@crate::command::command) to declare.
#[cfg_attr(doc_nightly, doc(notable_trait))]
pub trait Command: Event<Args = CommandArgs> {
    /// Runs `f` with access to the metadata state-map.
    fn with_meta<F, R>(self, f: F) -> R
    where
        F: FnOnce(&mut StateMap) -> R;

    /// Gets a read-only variable that indicates if the command has at least one enabled handler.
    ///
    /// When this is `false` but [`has_handlers`](Self::has_handlers) is `true` the command can be considered
    /// *relevant* in the current app state but not enabled, associated command trigger widgets should be
    /// visible but disabled.
    fn enabled(self, target: CommandTarget) -> ReadOnlyVar<bool, RcVar<bool>>;

    /// Gets if the command has at least one enabled handler.
    fn enabled_value(self) -> bool;

    /// Gets a read-only variable that indicates if the command has at least one handler.
    ///
    /// When this is `false` the command can be considered *not relevant* in the current app state
    /// and associated command trigger widgets can be hidden.
    fn has_handlers(self) -> ReadOnlyVar<bool, RcVar<bool>>;

    /// Gets if the command has at least one handler.
    fn has_handlers_value(self) -> bool;

    /// Create a new handle to this command.
    ///
    /// A handle indicates that there is an active *handler* for the event, the handle can also
    /// be used to set the [`enabled`](Self::enabled) state.
    fn new_handle<Evs: WithEvents>(self, events: &mut Evs, enabled: bool) -> CommandHandle;

    /// Gets a [`AnyCommand`] that represents this command.
    fn as_any(self) -> AnyCommand;

    fn notify(self, parameter: Option<Rc<dyn Any>>, target: Option<WidgetPath>);

    /// Get a command that represents `self` + `id`.
    /// 
    // * Metadata set in the returned command does not set `self`.
    // * Requested metadata not set in the returned command uses the metadata from `self`.
    // * Handlers created for the returned command do not enable `self`.
    // * Notifying `self` notifies the returned command and `self`.
    // * Notifying the returned command does not notify `self`.
    fn sub_cmd(self, id: impl SubCommandId) -> impl Command {
        SubCommand(self, id.ctx_id())
    }
}

pub enum CommandTarget {
    Focused,
    Path(WidgetPath),
    Context
}

button! {
    command = (FooCommand, CommandTarget::Context, None);
    content = text(FooCommand::text());
    
    enabled = InspectCommand.sub_cmd(window_id).enabled();

    on_click = hn!(|ctx, _| {
        InspectCommand.sub_cmd(ctx.path.window_id()).notify(None);
    });
}

window! {
    on_inpect = hn!(|ctx, args| {
        if args.is_for_contextual(ctx.path.window_id()) {

        }
    })
}

trait SubCommandId {
    fn ctx_id(self) -> (SubCommandNamespace, usize) {

    }
}

#[derive(Clone, Copy)]
pub struct SubCommand<C: Command>(C, (SubCommandNamespace, usize));
impl<C: Command> Command for SubCommand<C> {

}

crate::event_args! {
    /// Event args for command events.
    pub struct CommandArgs {
        /// Optional parameter for the command handler.
        pub parameter: Option<Rc<dyn Any>>,

        /// Target allowed to execute the command, if `None` all handlers are allowed.
        pub target: Option<WidgetPath>,

        ..

        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            self.target.map(|p|p.contains(ctx.path.widget_id())).unwrap_or(true)
        }
    }
}