//! Event and command API.
//!
//! Events are represented by a static instance of [`Event<A>`] with name suffix `_EVENT`. Events have
//! custom argument types that implement [`EventArgs`], this means that all event arg types have a timestamp, propagation
//! handle and can define their own delivery list.
//!
//! # Notify
//!
//! An event update is requested using [`Event::notify`] the notification is pending until the end of the current update,
//! at that moment the pending notifications apply, in the order they where requested. Each event notifies in this order:
//!
//! 1 - All [`AppExtension::event_preview`](crate::app::AppExtension::event_preview).
//! 2 - All [`Event::on_pre_event`] handlers.
//! 3 - All [`AppExtension::event_ui`](crate::app::AppExtension::event_ui).
//! 3.1 - Preview route from window root to each target widget.
//! 3.2 - Main route from target widget to window root.
//! 4 - All [`AppExtension::event`](crate::app::AppExtension::event).
//! 5 - All [``Event::on_event`] handlers.
//!
//! Each event args has an [`EventPropagationHandle`] that can be used to signal later handlers that the event
//! is already handled. The event notification always makes the full route, low level handlers must check if propagation
//! is stopped or can deliberately ignore it. Event properties automatically check propagation.
//!
//! The two event routes in widgets are an emergent property of nested nodes. There is only a method for events, [`UiNode::event`],
//! if a node handles the event before propagating to the child node it handled it in the preview route (also called tunnel route),
//! if it handles the event after it propagated it to the child node it handled it in the main route (also called bubble route).
//!
//! [`UiNode::event`]: crate::widget::node::UiNode::event
//!
//! # Subscribe
//!
//! The high-level way to subscribe to an event is by using an event property. These are properties named with prefix
//! `on_` and `on_pre_`, these properties handle subscription for the widget, filter out propagation stopped events and
//! also filter into specific aspects of an underlying event.
//!
//! ```
//! use zng::prelude::*;
//!
//! # let _scope = APP.defaults();
//! # let _ =
//! Button! {
//!     child = Text!("Button");
//!
//!     gesture::on_pre_single_click = hn!(|args: &gesture::ClickArgs| {
//!         assert!(args.is_single());
//!         println!("single click");
//!         args.propagation().stop();
//!     });
//!     on_click = hn!(|args: &gesture::ClickArgs| {
//!         assert!(!args.is_single());
//!         println!("click {:?}", args.click_count.get());
//!     });
//! }
//! # ;
//! ```
//!
//! In the example above the [`gesture::on_pre_single_click`] and [`gesture::on_click`] are handled, both properties
//! operate on the same underlying [`gesture::CLICK_EVENT`]. The `on_pre_single_click` property only accepts clicks
//! with the primary button that are not double-clicks (or triple, etc.), the `on_click` only accepts clicks with
//! the primary button. In the example `on_click` is never called for single clicks because the `on_pre_single_click` handler
//! stopped propagation for those events in the preview route, before the click handler.
//!
//! ## Subscribe in Nodes
//!
//! Widget and properties can subscribe to events directly. When the event [`UpdateDeliveryList`] is build only widgets
//! selected by the event arguments that are also subscribers to the event are added to the list.
//!
//! The [`WIDGET.sub_event`] method can be used to subscribe for the lifetime of the widget, the [`Event::subscribe`]
//! method can be used to subscribe for an arbitrary lifetime. The [`Event::on`] or [`Event::on_unhandled`] can be
//! used to match and receive the event.
//!
//! [`WIDGET.sub_event`]: crate::widget::WIDGET::sub_event
//! [`UpdateDeliveryList`]: crate::update::UpdateDeliveryList
//!
//! ```
//! # fn main() { }
//! use zng::prelude::*;
//! use zng::prelude_wgt::*;
//!
//! #[property(EVENT)]
//! pub fn print_click(child: impl IntoUiNode, preview: impl IntoVar<bool>) -> UiNode {
//!     let preview = preview.into_var();
//!     match_node(child, move |child, op| match op {
//!         UiNodeOp::Init => {
//!             WIDGET.sub_event(&gesture::CLICK_EVENT);
//!         }
//!         UiNodeOp::Event { update } => {
//!             if let Some(args) = gesture::CLICK_EVENT.on(update) {
//!                 if preview.get() {
//!                     println!("preview click {:?}", args.propagation().is_stopped());
//!                     child.event(update);
//!                 } else {
//!                     child.event(update);
//!                     println!("click {:?}", args.propagation().is_stopped());
//!                 }
//!             }
//!         }
//!         _ => {}
//!     })
//! }
//! ```
//!
//! The example above declares a property that prints the `CLICK_EVENT` propagation status, the preview/main
//! routes are defined merely by the position of `child.event(update)` in relation with the handling code.
//!
//! ## App Extensions
//!
//! App extensions don't need to subscribe to events, they all receive all events.
//!
//! ```
//! use zng::{app::AppExtension, update::EventUpdate, gesture::CLICK_EVENT};
//!
//! #[derive(Default)]
//! struct PrintClickManager { }
//!
//! impl AppExtension for PrintClickManager {
//!     fn event_preview(&mut self, update: &mut EventUpdate) {
//!         if let Some(args) = CLICK_EVENT.on(update) {
//!             println!("click, before all UI handlers");
//!         }
//!     }
//!
//!     fn event(&mut self, update: &mut EventUpdate) {
//!         if let Some(args) = CLICK_EVENT.on(update) {
//!             println!("click, after all UI handlers");
//!         }
//!     }
//! }
//! ```
//!
//! ## Direct Handlers
//!
//! Event handlers can be set directly on the events using [`Event::on_event`] and [`Event::on_pre_event`].
//! The handlers run in the app scope (same as app extensions). These event handlers are only called if
//! propagation is not stopped.
//!
//! ```
//! use zng::prelude::*;
//! # let _scope = APP.defaults();
//!
//! gesture::CLICK_EVENT.on_pre_event(app_hn!(|_, _| {
//!     println!("click, before all UI handlers");
//! })).perm();
//!
//!
//! gesture::CLICK_EVENT.on_event(app_hn!(|_, _| {
//!     println!("click, after all UI handlers");
//! })).perm();
//! ```
//!
//! [`gesture::on_pre_single_click`]: fn@crate::gesture::on_pre_single_click
//! [`gesture::on_click`]: fn@crate::gesture::on_click
//! [`gesture::CLICK_EVENT`]: crate::gesture::CLICK_EVENT
//!
//! # Event Macros
//!
//! Events can be declared using the [`event!`] macro, event arguments using the [`event_args!`]. Event properties
//! can be declared using [`event_property!`].
//!
//! ```
//! # fn main() { }
//! use zng::prelude_wgt::*;
//!
//! event_args! {
//!     pub struct FooArgs {
//!         pub target: WidgetPath,
//!         ..
//!         fn delivery_list(&self, list: &mut UpdateDeliveryList) {
//!             list.insert_wgt(&self.target);
//!         }         
//!     }
//! }
//!
//! event! {
//!     pub static FOO_EVENT: FooArgs;
//! }
//!
//! event_property! {
//!     pub fn foo {
//!         event: FOO_EVENT,
//!         args: FooArgs,
//!     }
//! }
//!
//! # fn usage() -> UiNode {
//! zng::widget::Wgt! {
//!     zng::widget::on_info_init = hn!(|_| {
//!         let this_wgt = WIDGET.info().path();
//!         FOO_EVENT.notify(FooArgs::now(this_wgt));
//!     });
//!
//!     on_pre_foo = hn!(|_| {
//!         println!("on_pre_foo!");
//!     });
//!     on_foo = hn!(|_| {
//!         println!("on_foo!");
//!     });
//! }
//! # }
//! ```
//!
//! The example above declares `FooArgs`, `FOO_EVENT`, `on_pre_foo` and `on_foo`. The example then declares
//! a widget that sends the `FOO_EVENT` to itself on init and receives it using the event properties.
//!
//! # Commands
//!
//! Command events are represented by a static instance of [`Command`] with name suffix `_CMD`. Commands have
//! custom argument type [`CommandArgs`]. Every command event is also an `Event<CommandArgs>`, commands extend
//! the event type to provide associated metadata, scope and *enabled* control.
//!
//! ## Command Macros
//!
//! Commands can be declared using the [`command!`] macro. Command properties can be declared using [`command_property!`].
//!
//! ```
//! # fn main() { }
//! use zng::prelude_wgt::*;
//!
//! command! {
//!     /// Foo docs.
//!     pub static FOO_CMD = {
//!         l10n!: true,
//!         name: "Foo",
//!         info: "foo bar",
//!         shortcut: shortcut![CTRL+'F'],
//!     };
//! }
//!
//! command_property! {
//!     pub fn foo {
//!         cmd: FOO_CMD.scoped(WIDGET.id()),
//!     }
//! }
//!
//! # fn usage() -> UiNode {
//! zng::widget::Wgt! {
//!     zng::widget::on_info_init = hn!(|_| {
//!         FOO_CMD.scoped(WIDGET.id()).notify();
//!     });
//!
//!     on_pre_foo = hn!(|_| {
//!         println!("on_pre_foo!");
//!     });
//!     on_foo = hn!(|_| {
//!         println!("on_foo!");
//!     });
//! }
//! # }
//! ```
//!
//! The example above declares `FOO_CMD`, `on_pre_foo`, `on_foo`, `can_foo` and `CAN_FOO_VAR`. The example then declares
//! a widget that sends the `FOO_CMD` to itself on init and receives it using the event properties.
//!
//! ## Metadata
//!
//! All commands provide an [`Command::with_meta`] access point for reading and writing arbitrary metadata. Usually
//! metadata is declared following the [command extensions] pattern. In the example above the `name`, `info` and `shortcut`
//! are actually command extensions declared as [`CommandNameExt`], [`CommandInfoExt`] and [`CommandShortcutExt`].
//!
//! [command extensions]: Command#extensions
//! [`CommandShortcutExt`]: crate::gesture::CommandShortcutExt
//!
//! ### Localization
//!
//! The special `l10n!:` metadata enables localization for the other text metadata of the command. It must be the first
//! metadata assign and the value must be a literal `bool` or string `""`, the string defines the localization file.
//!
//! See the [`l10n`](crate::zng::l10n#commands) module docs om commands for more details.
//!
//! ## Scopes
//!
//! Commands can be scoped to a window or widget, a scoped command is a different instance of [`Command`], it
//! inherits metadata from the main command (app scoped), but metadata can be set for a specific scope.
//!
//! ```
//! use zng::prelude::*;
//! use zng::{clipboard, event::CommandArgs};
//!
//! # let _scope = APP.defaults();
//! # let _ =
//! Stack!(
//!     top_to_bottom,
//!     5,
//!     ui_vec![
//!         SelectableText! {
//!             id = "print-copy";
//!             txt = "Print Copy";
//!
//!             widget::on_init = hn!(|_| {
//!                 let cmd = clipboard::COPY_CMD.scoped(WIDGET.id());
//!                 cmd.name().set(r#"Print "copy!""#);
//!                 cmd.info().set("");
//!             });
//!             clipboard::on_pre_copy = hn!(|args: &CommandArgs| {
//!                 args.propagation().stop();
//!                 println!("copy!");
//!             });
//!         },
//!         SelectableText! {
//!             id = "default-copy";
//!             txt = "Default Copy";
//!         },
//!         Button!(clipboard::COPY_CMD.scoped(WidgetId::named("print-copy"))),
//!         Button!(clipboard::COPY_CMD.scoped(WidgetId::named("default-copy"))),
//!         Button! {
//!             cmd = clipboard::COPY_CMD.focus_scoped();
//!             zng::focus::alt_focus_scope = true;
//!         },
//!     ]
//! )
//! # ;
//! ```
//!
//! The example above overrides the metadata and implementation of the copy command for the "print-copy" widget, buttons
//! targeting that widget show the new metadata.
//!
//! Widgets should prefer subscribing only to the command scoped to the widget. App scoped commands target all subscribers,
//! widget scoped commands target the widget only.
//!
//! # Full API
//!
//! See [`zng_app::event`] for the full event API.

pub use zng_app::event::{
    AnyEvent, AnyEventArgs, AppCommandArgs, Command, CommandArgs, CommandHandle, CommandInfoExt, CommandMeta, CommandMetaVar,
    CommandMetaVarId, CommandNameExt, CommandParam, CommandScope, EVENTS, Event, EventArgs, EventHandle, EventHandles,
    EventPropagationHandle, EventReceiver, command, event, event_args,
};
pub use zng_wgt::node::{command_property, event_property, on_command, on_event, on_pre_command, on_pre_event};
