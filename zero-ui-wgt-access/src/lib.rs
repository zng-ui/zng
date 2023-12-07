//! Properties that define accessibility metadata.
//!
//! The properties in this crate should only be used by widget implementers, they only
//! define metadata for accessibility, this metadata signals the availability of behaviors
//! that are not implemented by these properties, for example an [`AccessRole::Button`] widget
//! must also be focusable and handle click events, an [`AccessRole::TabList`] must contain widgets
//! marked [`AccessRole::Tab`].
//!
//! [`AccessRole::Button`]: zero_ui_app::widget::info::access::AccessRole::Button
//! [`AccessRole::TabList`]: zero_ui_app::widget::info::access::AccessRole::TabList
//! [`AccessRole::Tab`]: zero_ui_app::widget::info::access::AccessRole::Tab

mod events;
mod meta;
pub use events::*;
pub use meta::*;
