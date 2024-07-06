use hashbrown::HashSet;
use zng_app_context::app_local;
use zng_time::INSTANT_APP;
use zng_txt::Txt;

use crate::update::{UpdatesTrace, UPDATES};

use super::*;

app_local! {
    pub(crate) static EVENTS_SV: EventsService = const { EventsService::new() };
}

pub(crate) struct EventsService {
    updates: Mutex<Vec<EventUpdate>>, // not locked, used to make service Sync.
    commands: CommandSet,
    register_commands: Vec<Command>,
    l10n: EventsL10n,
}
enum EventsL10n {
    Pending(Vec<([&'static str; 3], Command, &'static str, CommandMetaVar<Txt>)>),
    Init(Box<dyn Fn([&'static str; 3], Command, &'static str, CommandMetaVar<Txt>) + Send + Sync>),
}
impl EventsService {
    const fn new() -> Self {
        Self {
            updates: Mutex::new(vec![]),
            commands: HashSet::with_hasher(BuildFxHasher),
            register_commands: vec![],
            l10n: EventsL10n::Pending(vec![]),
        }
    }

    pub(super) fn register_command(&mut self, command: Command) {
        if self.register_commands.is_empty() {
            UPDATES.update(None);
        }
        self.register_commands.push(command);
    }

    pub(super) fn sender<A>(&mut self, event: Event<A>) -> EventSender<A>
    where
        A: EventArgs + Send,
    {
        EventSender {
            sender: UPDATES.sender(),
            event,
        }
    }

    pub(crate) fn has_pending_updates(&mut self) -> bool {
        !self.updates.get_mut().is_empty()
    }
}

/// Const rustc-hash hasher.
#[derive(Clone, Default)]
pub struct BuildFxHasher;
impl std::hash::BuildHasher for BuildFxHasher {
    type Hasher = rustc_hash::FxHasher;

    fn build_hasher(&self) -> Self::Hasher {
        rustc_hash::FxHasher::default()
    }
}

/// Registered commands set.
pub type CommandSet = HashSet<Command, BuildFxHasher>;

/// App events and commands service.
pub struct EVENTS;
impl EVENTS {
    /// Commands that had handles generated in this app.
    ///
    /// When [`Command::subscribe`] is called for the first time in an app, the command gets added
    /// to this list after the current update, if the command is app scoped it remains on the list for
    /// the lifetime of the app, if it is window or widget scoped it only remains while there are handles.
    ///
    /// [`Command::subscribe`]: crate::event::Command::subscribe
    pub fn commands(&self) -> CommandSet {
        EVENTS_SV.read().commands.clone()
    }

    /// Schedules the raw event update.
    pub fn notify(&self, update: EventUpdate) {
        UpdatesTrace::log_event(update.event);
        EVENTS_SV.write().updates.get_mut().push(update);
        UPDATES.send_awake();
    }

    #[must_use]
    pub(crate) fn apply_updates(&self) -> Vec<EventUpdate> {
        let _s = tracing::trace_span!("EVENTS").entered();

        let mut ev = EVENTS_SV.write();
        ev.commands.retain(|c| c.update_state());

        {
            let ev = &mut *ev;
            for cmd in ev.register_commands.drain(..) {
                if cmd.update_state() && !ev.commands.insert(cmd) {
                    tracing::error!("command `{cmd:?}` is already registered")
                }
            }
        }

        let mut updates: Vec<_> = ev.updates.get_mut().drain(..).collect();
        drop(ev);

        if !updates.is_empty() {
            let _t = INSTANT_APP.pause_for_update();

            for u in &mut updates {
                let ev = u.event;
                ev.on_update(u);
            }
        }
        updates
    }
}

/// EVENTS L10N integration.
#[allow(non_camel_case_types)]
pub struct EVENTS_L10N;
impl EVENTS_L10N {
    pub(crate) fn init_meta_l10n(&self, file: [&'static str; 3], cmd: Command, meta_name: &'static str, txt: CommandMetaVar<Txt>) {
        {
            let sv = EVENTS_SV.read();
            if let EventsL10n::Init(f) = &sv.l10n {
                f(file, cmd, meta_name, txt);
                return;
            }
        }

        let mut sv = EVENTS_SV.write();
        match &mut sv.l10n {
            EventsL10n::Pending(a) => a.push((file, cmd, meta_name, txt)),
            EventsL10n::Init(f) => f(file, cmd, meta_name, txt),
        }
    }

    /// Register a closure that is called to localize command metadata.
    ///
    /// The closure arguments are:
    ///
    /// * `file` is the crate package name, version and the file from command declaration `@l10n: "file"`
    ///    value or is empty if `@l10n` was set to something else.
    /// * `cmd` is the command, the command event name should be used as key.
    /// * `meta` is the metadata name, for example `"name"`, should be used as attribute.
    /// * `txt` is text variable that must be set with the translation.
    pub fn init_l10n(&self, localize: impl Fn([&'static str; 3], Command, &'static str, CommandMetaVar<Txt>) + Send + Sync + 'static) {
        let mut sv = EVENTS_SV.write();
        match &mut sv.l10n {
            EventsL10n::Pending(a) => {
                for (f, k, a, t) in a.drain(..) {
                    localize(f, k, a, t);
                }
            }
            EventsL10n::Init(_) => panic!("EVENTS_L10N already has a localizer"),
        }
        sv.l10n = EventsL10n::Init(Box::new(localize));
    }
}
