//! Accessibility service, events and properties.
//!
//! # Full API
//!
//! See [`zero_ui_app::access`] and [`zero_ui_wgt_access`] for the full API.

pub use zero_ui_app::access::{
    AccessClickArgs, AccessExpanderArgs, AccessIncrementArgs, AccessInitedArgs, AccessNumberArgs, AccessScrollArgs, AccessSelectionArgs,
    AccessTextArgs, AccessToolTipArgs, ScrollCmd, ACCESS, ACCESS_CLICK_EVENT, ACCESS_EXPANDER_EVENT, ACCESS_INCREMENT_EVENT,
    ACCESS_INITED_EVENT, ACCESS_NUMBER_EVENT, ACCESS_SCROLL_EVENT, ACCESS_SELECTION_EVENT, ACCESS_TEXT_EVENT, ACCESS_TOOLTIP_EVENT,
};
pub use zero_ui_wgt_access::{
    access_commands, access_role, accessible, active_descendant, auto_complete, checked, col_count, col_index, col_span, controls, current,
    described_by, details, error_message, expanded, flows_to, invalid, item_count, item_index, label, labelled_by, labelled_by_child,
    level, live, modal, multi_selectable, on_access_click, on_access_expander, on_access_increment, on_access_number, on_access_scroll,
    on_access_selection, on_access_text, on_access_tooltip, on_pre_access_click, on_pre_access_expander, on_pre_access_increment,
    on_pre_access_number, on_pre_access_scroll, on_pre_access_selection, on_pre_access_text, on_pre_access_tooltip, orientation, owns,
    placeholder, popup, read_only, required, row_count, row_index, row_span, scroll_horizontal, scroll_vertical, selected, sort, value,
    value_max, value_min, AccessCmdName, AccessRole, AutoComplete, CurrentKind, Invalid, LiveIndicator, Orientation, Popup, SortDirection,
};
