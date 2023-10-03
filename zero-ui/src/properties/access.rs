//! Properties that define accessibility metadata.
//!
//! The properties in this module should only be used by widget implementers, they only
//! define metadata for accessibility, this metadata signals the availability of behaviors
//! that are not implemented by these properties, for example an [`AccessRole::Button`] widget
//! must also be focusable and handle click events, an [`AccessRole::TabList`] must contain widgets
//! marked [`AccessRole::Tab`].

use std::num::NonZeroU32;

use zero_ui_core::widget_info::access::*;

use crate::core::widget_info::AccessRole;

use crate::prelude::new_property::*;

/// Sets the widget kind for accessibility services.
///
/// Note that the widget role must be implemented, this property only sets the metadata.
#[property(CONTEXT)]
pub fn access_role(child: impl UiNode, role: impl IntoVar<AccessRole>) -> impl UiNode {
    with_access_state(child, role, |b, v| b.set_role(*v))
}

/// Append supported access commands.
#[property(CONTEXT)]
pub fn access_commands(child: impl UiNode, commands: impl IntoVar<Vec<AccessCmdName>>) -> impl UiNode {
    with_access_state(child, commands, |b, v| {
        for cmd in v {
            b.push_command(*cmd);
        }
    })
}

/// Set how input text triggers display of one or more predictions of the user's intended
/// value for a [`ComboBox`], [`SearchBox`], or [`TextInput`].
///
/// [`ComboBox`]: AccessRole::ComboBox
/// [`SearchBox`]: AccessRole::SearchBox
/// [`TextInput`]: AccessRole::TextInput
#[property(CONTEXT)]
pub fn auto_complete(child: impl UiNode, auto_complete: impl IntoVar<AutoComplete>) -> impl UiNode {
    with_access_state(child, auto_complete, |b, v| b.set_auto_complete(*v))
}

/// If the widget is checked (`Some(true)`), unchecked (`Some(false)`), or if the checked status is indeterminate (`None`).
#[property(CONTEXT)]
pub fn checked(child: impl UiNode, checked: impl IntoVar<Option<bool>>) -> impl UiNode {
    with_access_state(child, checked, |b, v| b.set_checked(*v))
}

/// Indicates that the widget represents the current item of a [kind](CurrentKind).
#[property(CONTEXT)]
pub fn current(child: impl UiNode, kind: impl IntoVar<CurrentKind>) -> impl UiNode {
    with_access_state(child, kind, |b, v| b.set_current(*v))
}

/// Indicates that the widget is an error message for the `invalid_wgt`.
///
/// The other widget must [`invalid`].
///
/// [`invalid`]: fn@invalid
#[property(CONTEXT)]
pub fn error_message(child: impl UiNode, invalid_wgt: impl IntoVar<WidgetId>) -> impl UiNode {
    with_access_state(child, invalid_wgt, |b, v| b.set_error_message(*v))
}

/// Identifies the currently active widget when focus is on a composite widget.
#[property(CONTEXT)]
pub fn active_descendant(child: impl UiNode, descendant: impl IntoVar<WidgetId>) -> impl UiNode {
    with_access_state(child, descendant, |b, v| b.set_active_descendant(*v))
}

/// Indicate that the widget toggles the visibility of related widgets.
///
/// Use  [`controls`], or [`owns`] to indicate the widgets that change visibility based on
/// this value.
///
/// [`controls`]: fn@controls
/// [`owns`]: fn@owns
#[property(CONTEXT)]
pub fn expanded(child: impl UiNode, expanded: impl IntoVar<bool>) -> impl UiNode {
    with_access_state(child, expanded, |b, v| b.set_expanded(*v))
}

/// Indicates the availability and type of interactive popup widget.
#[property(CONTEXT)]
pub fn popup(child: impl UiNode, popup: impl IntoVar<Popup>) -> impl UiNode {
    with_access_state(child, popup, |b, v| b.set_popup(*v))
}

/// Sets a custom name for the widget in accessibility info.
///
/// Note that if this is not set the [`WidgetId::name`] of the widget is used.
#[property(CONTEXT)]
pub fn label(child: impl UiNode, label: impl IntoVar<Txt>) -> impl UiNode {
    with_access_state(child, label, |b, v| b.set_label(v.clone()))
}

/// Sets the hierarchical level of the widget within a parent scope.
#[property(CONTEXT)]
pub fn level(child: impl UiNode, hierarchical_level: impl IntoVar<NonZeroU32>) -> impl UiNode {
    with_access_state(child, hierarchical_level, |b, v| b.set_level(*v))
}

/// Indicates that the user may select more than one item from the current selectable descendants.
#[property(CONTEXT)]
pub fn multi_selectable(child: impl UiNode, multi_selectable: impl IntoVar<bool>) -> impl UiNode {
    with_access_state(child, multi_selectable, |b, v| {
        if *v {
            b.flag_multi_selectable()
        }
    })
}

/// Indicates whether the widget's orientation is horizontal, vertical, or unknown/ambiguous.
#[property(CONTEXT)]
pub fn orientation(child: impl UiNode, orientation: impl IntoVar<Orientation>) -> impl UiNode {
    with_access_state(child, orientation, |b, v| b.set_orientation(*v))
}

/// Short hint (a word or short phrase) intended to help the user with data entry when a form control has no value.
#[property(CONTEXT)]
pub fn placeholder(child: impl UiNode, placeholder: impl IntoVar<Txt>) -> impl UiNode {
    with_access_state(child, placeholder, |b, v| b.set_placeholder(v.clone()))
}

/// Indicates that the widget is not editable, but is otherwise operable.
#[property(CONTEXT)]
pub fn read_only(child: impl UiNode, read_only: impl IntoVar<bool>) -> impl UiNode {
    with_access_state(child, read_only, |b, v| {
        if *v {
            b.flag_read_only()
        }
    })
}

/// Indicates that user input is required on the widget before a form may be submitted.
#[property(CONTEXT)]
pub fn required(child: impl UiNode, required: impl IntoVar<bool>) -> impl UiNode {
    with_access_state(child, required, |b, v| {
        if *v {
            b.flag_required()
        }
    })
}

/// Indicates that the widget is selected.
#[property(CONTEXT)]
pub fn selected(child: impl UiNode, selected: impl IntoVar<bool>) -> impl UiNode {
    with_access_state(child, selected, |b, v| {
        if *v {
            b.flag_selected()
        }
    })
}

/// Sets the sort direction for the table or grid items.
#[property(CONTEXT)]
pub fn sort(child: impl UiNode, direction: impl IntoVar<SortDirection>) -> impl UiNode {
    with_access_state(child, direction, |b, v| b.set_sort(*v))
}

/// Set the maximum value (inclusive).
#[property(CONTEXT)]
pub fn value_max(child: impl UiNode, max: impl IntoVar<f64>) -> impl UiNode {
    with_access_state(child, max, |b, v| b.set_value_max(*v))
}

/// Set the minimum value (inclusive).
#[property(CONTEXT)]
pub fn value_min(child: impl UiNode, min: impl IntoVar<f64>) -> impl UiNode {
    with_access_state(child, min, |b, v| b.set_value_min(*v))
}

/// Set the current value.
#[property(CONTEXT)]
pub fn value(child: impl UiNode, value: impl IntoVar<f64>) -> impl UiNode {
    with_access_state(child, value, |b, v| b.set_value(*v))
}

/// Set a text that is a readable version of the current value.
#[property(CONTEXT)]
pub fn value_text(child: impl UiNode, value: impl IntoVar<Txt>) -> impl UiNode {
    with_access_state(child, value, |b, v| b.set_value_text(v.clone()))
}

/// Sets the total number of columns in a [`Table`], [`Grid`], or [`TreeGrid`] when not all columns are present in tree.
///
/// The value `0` indicates that not all columns are in the widget and the application cannot determinate the exact number.
///
/// [`Table`]: AccessRole::Table
/// [`Grid`]: AccessRole::Grid
/// [`TreeGrid`]: AccessRole::TreeGrid
#[property(CONTEXT)]
pub fn col_count(child: impl UiNode, count: impl IntoVar<usize>) -> impl UiNode {
    with_access_state(child, count, |b, v| b.set_col_count(*v))
}

/// Sets the widget's column index in the parent table or grid.
#[property(CONTEXT)]
pub fn col_index(child: impl UiNode, index: impl IntoVar<usize>) -> impl UiNode {
    with_access_state(child, index, |b, v| b.set_col_index(*v))
}

/// Sets the number of columns spanned by the widget in the parent table or grid.
#[property(CONTEXT)]
pub fn col_span(child: impl UiNode, span: impl IntoVar<usize>) -> impl UiNode {
    with_access_state(child, span, |b, v| b.set_col_span(*v))
}

/// Sets the total number of rows in a [`Table`], [`Grid`], or [`TreeGrid`] when not all rows are present in tree.
///
/// The value `0` indicates that not all rows are in the widget and the application cannot determinate the exact number.
///
/// [`Table`]: AccessRole::Table
/// [`Grid`]: AccessRole::Grid
/// [`TreeGrid`]: AccessRole::TreeGrid
#[property(CONTEXT)]
pub fn row_count(child: impl UiNode, count: impl IntoVar<usize>) -> impl UiNode {
    with_access_state(child, count, |b, v| b.set_row_count(*v))
}

/// Sets the widget's row index in the parent table or grid.
#[property(CONTEXT)]
pub fn row_index(child: impl UiNode, index: impl IntoVar<usize>) -> impl UiNode {
    with_access_state(child, index, |b, v| b.set_row_index(*v))
}

/// Sets the number of rows spanned by the widget in the parent table or grid.
#[property(CONTEXT)]
pub fn row_span(child: impl UiNode, span: impl IntoVar<usize>) -> impl UiNode {
    with_access_state(child, span, |b, v| b.set_row_span(*v))
}

/// Sets the number of items in the current set of list items or tree items when not all items in the set are present in the tree.
#[property(CONTEXT)]
pub fn item_count(child: impl UiNode, count: impl IntoVar<usize>) -> impl UiNode {
    with_access_state(child, count, |b, v| b.set_item_count(*v))
}

/// Sets the widget's number or position in the current set of list items or tree items when not all items are present in the tree.
#[property(CONTEXT)]
pub fn item_index(child: impl UiNode, index: impl IntoVar<usize>) -> impl UiNode {
    with_access_state(child, index, |b, v| b.set_item_index(*v))
}

/// Sets if the widget is modal when displayed.
#[property(CONTEXT)]
pub fn modal(child: impl UiNode, modal: impl IntoVar<bool>) -> impl UiNode {
    with_access_state(child, modal, |b, v| {
        if *v {
            b.flag_modal()
        }
    })
}

/// Append widgets whose contents or presence are controlled by this widget to the controlled list.
#[property(CONTEXT)]
pub fn controls(child: impl UiNode, controlled: impl IntoVar<Vec<WidgetId>>) -> impl UiNode {
    with_access_state(child, controlled, |b, v| {
        for id in v {
            b.push_controls(*id);
        }
    })
}

/// Append widgets that describes this widget to the descriptors list.
#[property(CONTEXT)]
pub fn described_by(child: impl UiNode, descriptors: impl IntoVar<Vec<WidgetId>>) -> impl UiNode {
    with_access_state(child, descriptors, |b, v| {
        for id in v {
            b.push_described_by(*id);
        }
    })
}

/// Append widgets that provide additional information related to this widget to the details list.
#[property(CONTEXT)]
pub fn details(child: impl UiNode, details: impl IntoVar<Vec<WidgetId>>) -> impl UiNode {
    with_access_state(child, details, |b, v| {
        for id in v {
            b.push_details(*id);
        }
    })
}

/// Append widgets that provide additional information related to this widget.
#[property(CONTEXT)]
pub fn labeled_by(child: impl UiNode, labels: impl IntoVar<Vec<WidgetId>>) -> impl UiNode {
    with_access_state(child, labels, |b, v| {
        for id in v {
            b.push_labeled_by(*id);
        }
    })
}

/// Append widgets that are a *child* of this widget, but is not already a child in the info tree.
#[property(CONTEXT)]
pub fn owns(child: impl UiNode, owned: impl IntoVar<Vec<WidgetId>>) -> impl UiNode {
    with_access_state(child, owned, |b, v| {
        for id in v {
            b.push_owns(*id);
        }
    })
}

/// Append options for next widget to be read by screen readers.
#[property(CONTEXT)]
pub fn flows_to(child: impl UiNode, next_options: impl IntoVar<Vec<WidgetId>>) -> impl UiNode {
    with_access_state(child, next_options, |b, v| {
        for id in v {
            b.push_flows_to(*id);
        }
    })
}

/// Indicates that the widget's data is invalid with optional kinds of errors.
#[property(CONTEXT)]
pub fn invalid(child: impl UiNode, error: impl IntoVar<Invalid>) -> impl UiNode {
    with_access_state(child, error, |b, v| b.set_invalid(*v))
}

/// Indicate that the widget can change, how the change can be announced, if `atomic`
/// the entire widget must be re-read, if `busy` the screen reader must wait until the change completes.
#[property(CONTEXT)]
pub fn live(
    child: impl UiNode,
    indicator: impl IntoVar<LiveIndicator>,
    atomic: impl IntoVar<bool>,
    busy: impl IntoVar<bool>,
) -> impl UiNode {
    let indicator = indicator.into_var();
    let atomic = atomic.into_var();
    let busy = busy.into_var();
    let mut handles = VarHandles::dummy();
    match_node(child, move |c, op| match op {
        UiNodeOp::Deinit => {
            handles.clear();
        }
        UiNodeOp::Info { info } => {
            c.info(info);
            if let Some(mut builder) = info.access() {
                if handles.is_dummy() {
                    handles.push(indicator.subscribe(UpdateOp::Info, WIDGET.id()));
                    handles.push(atomic.subscribe(UpdateOp::Info, WIDGET.id()));
                    handles.push(busy.subscribe(UpdateOp::Info, WIDGET.id()));
                }
                builder.set_live(indicator.get(), atomic.get(), busy.get());
            }
        }
        _ => {}
    })
}

fn with_access_state<T: VarValue>(
    child: impl UiNode,
    state: impl IntoVar<T>,
    set_info: impl Fn(&mut WidgetAccessInfoBuilder, &T) + Send + 'static,
) -> impl UiNode {
    let state = state.into_var();
    let mut handle = VarHandle::dummy();
    match_node(child, move |c, op| match op {
        UiNodeOp::Deinit => {
            handle = VarHandle::dummy();
        }
        UiNodeOp::Info { info } => {
            c.info(info);
            if let Some(mut builder) = info.access() {
                if handle.is_dummy() {
                    handle = state.subscribe(UpdateOp::Info, WIDGET.id());
                }
                state.with(|val| set_info(&mut builder, val));
            }
        }
        _ => {}
    })
}
