//! Accessibility metadata types.

use std::num::NonZeroU32;

use zero_ui_view_api::access::AccessState;
pub use zero_ui_view_api::access::{AccessRole, AutoComplete, CurrentKind, LiveChange, LiveIndicator, Orientation, Popup, SortDirection};

use crate::{context::StaticStateId, text::Txt, widget_instance::WidgetId};

use super::{WidgetInfo, WidgetInfoBuilder};

/// Accessibility metadata.
pub struct WidgetAccessInfoBuilder<'a> {
    pub(super) builder: &'a mut WidgetInfoBuilder,
}
impl<'a> WidgetAccessInfoBuilder<'a> {
    fn with_access(&mut self, f: impl FnOnce(&mut AccessInfo)) {
        self.builder.with_meta(move |mut m| f(m.entry(&ACCESS_INFO_ID).or_default()))
    }

    /// Set the accessibility role of the widget.
    pub fn set_role(&mut self, role: AccessRole) {
        self.with_access(|a| a.role = Some(role))
    }

    /// Set how input text triggers display of one or more predictions of the user's intended
    /// value for a [`ComboBox`], [`SearchBox`], or [`TextBox`].
    ///
    /// [`ComboBox`]: AccessRole::ComboBox
    /// [`SearchBox`]: AccessRole::SearchBox
    /// [`TextBox`]: AccessRole::TextBox
    pub fn set_auto_complete(&mut self, mode: AutoComplete) {
        self.with_access(|a| a.set_state(AccessState::AutoComplete(mode)))
    }

    /// If the widget is checked (`Some(true)`), unchecked (`Some(false)`), or if the checked status is indeterminate (`None`).
    pub fn set_checked(&mut self, checked: Option<bool>) {
        self.with_access(|a| a.set_state(AccessState::Checked(checked)))
    }

    /// Indicates that the widget represents the current item of a [kind](CurrentKind).
    pub fn set_current(&mut self, kind: CurrentKind) {
        self.with_access(|a| a.set_state(AccessState::Current(kind)))
    }

    /// Indicates that the widget is an error message for the `invalid_wgt`.
    ///
    /// The other widget must [`flag_invalid`].
    ///
    /// [`flag_invalid`]: fn@Self::flag_invalid
    pub fn set_error_message(&mut self, invalid_wgt: impl Into<WidgetId>) {
        let invalid_wgt = invalid_wgt.into();
        self.with_access(|a| a.set_state(AccessState::ErrorMessage(invalid_wgt.into())))
    }

    /// Indicate that the widget toggles the visibility of related widgets.
    ///
    /// Use  [`push_controls`], or [`push_owns`] to indicate the widgets that change visibility based on
    /// this value.
    ///
    /// [`push_controls`]: Self::push_controls
    /// [`push_owns`]: Self::push_owns
    pub fn set_expanded(&mut self, expanded: bool) {
        self.with_access(|a| a.set_state(AccessState::Expanded(expanded)))
    }

    /// Indicates the availability and type of interactive popup widget.
    pub fn set_has_popup(&mut self, popup: Popup) {
        self.with_access(|a| a.set_state(AccessState::HasPopup(popup)))
    }

    /// Indicates that the widget's data is invalid with optional kinds of errors.
    pub fn flag_invalid(&mut self, grammar: bool, spelling: bool) {
        if !grammar && !spelling {
            self.with_access(|a| a.set_state(AccessState::Invalid));
        }
        if grammar {
            self.with_access(|a| a.set_state(AccessState::InvalidGrammar));
        }
        if spelling {
            self.with_access(|a| a.set_state(AccessState::InvalidSpelling));
        }
    }

    /// Sets a custom name for the widget in accessibility info.
    ///
    /// Note that if this is not set a name is already derived from the text content or [`WidgetId::name`] of the widget.
    pub fn set_label(&mut self, name: impl Into<Txt>) {
        let name = name.into();
        self.builder.with_meta(move |mut m| m.set(&ACCESS_NAME_ID, name));
    }

    /// Sets the hierarchical level of the widget within a parent scope.
    pub fn set_level(&mut self, hierarchical_level: NonZeroU32) {
        self.with_access(|a| a.set_state(AccessState::Level(hierarchical_level)))
    }

    /// Indicates whether a [`TextBox`] accepts multiple lines of input.
    ///
    /// [`TextBox`]: AccessRole::TextBox
    pub fn flag_multi_line(&mut self) {
        self.with_access(|a| a.set_state(AccessState::MultiLine))
    }

    /// Indicates that the user may select more than one item from the current selectable descendants.
    pub fn flag_multi_selectable(&mut self) {
        self.with_access(|a| a.set_state(AccessState::MultiSelectable))
    }

    /// Indicates whether the widget's orientation is horizontal, vertical, or unknown/ambiguous.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        self.with_access(|a| a.set_state(AccessState::Orientation(orientation)))
    }

    /// Short hint (a word or short phrase) intended to help the user with data entry when a form control has no value.
    pub fn set_placeholder(&mut self, placeholder: impl Into<Txt>) {
        let placeholder = placeholder.into();
        self.builder.with_meta(move |mut m| m.set(&ACCESS_PLACEHOLDER_ID, placeholder));
    }

    /// Indicates that the widget is not editable, but is otherwise operable.
    pub fn flag_read_only(&mut self) {
        self.with_access(|a| a.set_state(AccessState::ReadOnly))
    }

    /// Indicates that user input is required on the widget before a form may be submitted.
    pub fn flag_required(&mut self) {
        self.with_access(|a| a.set_state(AccessState::Required))
    }

    /// Indicates that the widget is selected.
    pub fn flag_selected(&mut self) {
        self.with_access(|a| a.set_state(AccessState::Selected))
    }

    /// Sets the sort direction for the table or grid items.
    pub fn set_sort(&mut self, direction: SortDirection) {
        self.with_access(|a| a.set_state(AccessState::Sort(direction)))
    }

    /// Set the maximum value (inclusive).
    pub fn set_value_max(&mut self, max: f64) {
        self.with_access(|a| a.set_state(AccessState::ValueMax(max)))
    }

    /// Set the minimum value (inclusive).
    pub fn set_value_min(&mut self, min: f64) {
        self.with_access(|a| a.set_state(AccessState::ValueMin(min)))
    }

    /// Set the current value.
    pub fn set_value(&mut self, value: f64) {
        self.with_access(|a| a.set_state(AccessState::Value(value)))
    }

    /// Set a text that is a readable version of the current value.
    pub fn set_value_text(&mut self, value: impl Into<Txt>) {
        let placeholder = value.into();
        self.builder.with_meta(move |mut m| m.set(&ACCESS_VALUE_ID, placeholder));
    }

    /// Flags that the widget will be updated, and describes the types of
    /// updates the user agents, assistive technologies, and user can expect from the live region.
    pub fn set_live(&mut self, indicator: LiveIndicator, changes: LiveChange, atomic: bool, busy: bool) {
        self.with_access(|a| {
            a.set_state(AccessState::Live {
                indicator,
                changes,
                atomic,
                busy,
            })
        })
    }

    /// Sets the total number of columns in a [`Table`], [`Grid`], or [`TreeGrid`] when not all columns are present in tree.
    ///
    /// The value `0` indicates that not all columns are in the widget and the application cannot determinate the exact number.
    ///
    /// [`Table`]: AccessRole::Table
    /// [`Grid`]: AccessRole::Grid
    /// [`TreeGrid`]: AccessRole::TreeGrid
    pub fn set_col_count(&mut self, count: usize) {
        self.with_access(|a| a.set_state(AccessState::ColCount(count)))
    }

    /// Sets the widget's column index in the parent table or grid.
    pub fn set_col_index(&mut self, index: usize) {
        self.with_access(|a| a.set_state(AccessState::ColIndex(index)))
    }

    /// sets the number of columns spanned by the widget in the parent table or grid.
    pub fn set_col_span(&mut self, span: usize) {
        self.with_access(|a| a.set_state(AccessState::ColSpan(span)))
    }

    /// Sets the total number of rows in a [`Table`], [`Grid`], or [`TreeGrid`] when not all rows are present in tree.
    ///
    /// The value `0` indicates that not all rows are in the widget and the application cannot determinate the exact number.
    ///
    /// [`Table`]: AccessRole::Table
    /// [`Grid`]: AccessRole::Grid
    /// [`TreeGrid`]: AccessRole::TreeGrid
    pub fn set_row_count(&mut self, count: usize) {
        self.with_access(|a| a.set_state(AccessState::RowCount(count)))
    }

    /// Sets the widget's row index in the parent table or grid.
    pub fn set_row_index(&mut self, index: usize) {
        self.with_access(|a| a.set_state(AccessState::RowIndex(index)))
    }

    /// sets the number of rows spanned by the widget in the parent table or grid.
    pub fn set_row_span(&mut self, span: usize) {
        self.with_access(|a| a.set_state(AccessState::RowSpan(span)))
    }

    /// Sets the widget's number or position in the current set of list items or tree items when not all items are present in the tree.
    pub fn set_item_index(&mut self, index: usize) {
        self.with_access(|a| a.set_state(AccessState::ItemIndex(index)))
    }

    /// Push a widget whose contents or presence are controlled by this widget.
    pub fn push_controls(&mut self, controlled_id: impl Into<WidgetId>) {
        let controlled_id = controlled_id.into();
        self.with_access(|a| {
            for state in &mut a.state {
                if let AccessState::Controls(c) = state {
                    c.push(controlled_id.into());
                    return;
                }
            }
            a.state.push(AccessState::Controls(vec![controlled_id.into()]))
        })
    }

    /// Push a widget that describes this widget.
    pub fn push_described_by(&mut self, descriptor_id: impl Into<WidgetId>) {
        let descriptor_id = descriptor_id.into();
        self.with_access(|a| {
            for state in &mut a.state {
                if let AccessState::DescribedBy(c) = state {
                    c.push(descriptor_id.into());
                    return;
                }
            }
            a.state.push(AccessState::DescribedBy(vec![descriptor_id.into()]))
        })
    }

    /// Push a widget that provide additional information related to this widget.
    pub fn push_details(&mut self, detail_id: impl Into<WidgetId>) {
        let detail_id = detail_id.into();
        self.with_access(|a| {
            for state in &mut a.state {
                if let AccessState::Details(c) = state {
                    c.push(detail_id.into());
                    return;
                }
            }
            a.state.push(AccessState::Details(vec![detail_id.into()]))
        })
    }

    /// Push a widget that provide additional information related to this widget.
    pub fn push_labeled_by(&mut self, label_id: impl Into<WidgetId>) {
        let detail_id = label_id.into();
        self.with_access(|a| {
            for state in &mut a.state {
                if let AccessState::LabelledBy(c) = state {
                    c.push(detail_id.into());
                    return;
                }
            }
            a.state.push(AccessState::LabelledBy(vec![detail_id.into()]))
        })
    }

    /// Push widget a widget that is a *child* of this widget, but is not already a child in the info tree.
    pub fn push_owns(&mut self, label_id: impl Into<WidgetId>) {
        let detail_id = label_id.into();
        self.with_access(|a| {
            for state in &mut a.state {
                if let AccessState::Owns(c) = state {
                    c.push(detail_id.into());
                    return;
                }
            }
            a.state.push(AccessState::Owns(vec![detail_id.into()]))
        })
    }
}

impl WidgetInfo {
    /// Accessibility info, if the info tree was build with [`access_enabled`].
    ///
    /// [`access_enabled`]: WidgetInfoTree::access_enabled
    pub fn access(&self) -> Option<WidgetAccessInfo> {
        if self.tree.access_enabled() {
            Some(WidgetAccessInfo { info: self.clone() })
        } else {
            None
        }
    }
}

/// Accessibility info for a widget.
pub struct WidgetAccessInfo {
    info: WidgetInfo,
}
macro_rules! get_state {
    ($self:ident.$Discriminant:ident) => {
        $self.access()?.state.iter().find_map(|a| {
            if let AccessState::$Discriminant(value) = a {
                Some(value)
            } else {
                None
            }
        })
    };
}
impl WidgetAccessInfo {
    fn access(&self) -> Option<&AccessInfo> {
        self.info.meta().get(&ACCESS_INFO_ID)
    }

    /// Accessibility role of the widget.
    pub fn role(&self) -> Option<AccessRole> {
        self.access()?.role
    }

    /// How input text triggers display of one or more predictions of the user's intended value.
    pub fn auto_complete(&self) -> Option<AutoComplete> {
        get_state!(self.AutoComplete).copied()
    }

    /// If the widget is checked (`Some(true)`), unchecked (`Some(false)`), or if the checked status is indeterminate (`None`).
    ///
    /// Note that the value is wrapped in another `Option<_>` that indicates if it was set or not.
    pub fn checked(&self) -> Option<Option<bool>> {
        get_state!(self.Checked).copied()
    }

    /// Kind of current item the widget represents.
    pub fn current(&self) -> Option<CurrentKind> {
        get_state!(self.Current).copied()
    }

    /// Gets the invalid widget that this widget is an error message for.
    pub fn error_message(&self) -> Option<WidgetInfo> {
        let id = get_state!(self.ErrorMessage)?;
        let id = WidgetId::from_raw(id.0);
        self.info.tree.get(id)
    }

    /// Gets visibility of related widgets.
    pub fn expanded(&self) -> Option<bool> {
        get_state!(self.Expanded).copied()
    }
}

#[derive(Default)]
struct AccessInfo {
    role: Option<AccessRole>,
    state: Vec<AccessState>,
}
impl AccessInfo {
    fn set_state(&mut self, state: AccessState) {
        let discriminant = std::mem::discriminant(&state);
        if let Some(present) = self.state.iter_mut().find(|s| std::mem::discriminant(&**s) == discriminant) {
            *present = state;
        } else {
            self.state.push(state);
        }
    }
}

static ACCESS_INFO_ID: StaticStateId<AccessInfo> = StaticStateId::new_unique();
static ACCESS_NAME_ID: StaticStateId<Txt> = StaticStateId::new_unique();
static ACCESS_PLACEHOLDER_ID: StaticStateId<Txt> = StaticStateId::new_unique();
static ACCESS_VALUE_ID: StaticStateId<Txt> = StaticStateId::new_unique();

/*
!!: TODO state that is always derived?

* AccessState::Modal - Derived from interactivity.
* AccessState::ActiveDescendant - Derived from focused (we just use the normal focus nav for these widgets).
* AccessState::FlowTo - Derived from tab index.
* AccessState::SetSize - Derived from bounds.

*/
