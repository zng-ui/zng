//! Event and command API.
//!
//! Events are represented by a static instance of [`Event<A>`] with name suffix `_EVENT`, they are declared.
//!
//! # Commands
//!
//! TODO !!:
//!
//! # Full API
//!
//! See [`zero_ui_app::event`] for the full event API.

pub use zero_ui_app::event::{
    command, event, event_args, AnyEvent, AnyEventArgs, Command, CommandArgs, CommandHandle, CommandInfoExt, CommandMeta, CommandMetaVar,
    CommandMetaVarId, CommandNameExt, CommandParam, CommandScope, Event, EventArgs, EventHandle, EventHandles, EventPropagationHandle,
    EventReceiver, EVENTS,
};
pub use zero_ui_wgt::node::{command_property, event_property, on_command, on_event, on_pre_command, on_pre_event};
