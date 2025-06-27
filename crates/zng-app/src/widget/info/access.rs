//! Accessibility metadata types.

use std::num::NonZeroU32;

use parking_lot::Mutex;
use unic_langid::LanguageIdentifier;
use zng_layout::unit::{Factor, PxSize, PxTransform};
use zng_state_map::{StateId, static_id};
use zng_txt::Txt;
use zng_unique_id::IdMap;
use zng_var::{BoxedVar, IntoVar, Var};
pub use zng_view_api::access::{
    AccessCmdName, AccessRole, AutoComplete, CurrentKind, Invalid, LiveIndicator, Orientation, Popup, SortDirection,
};
use zng_view_api::access::{AccessNodeId, AccessState};

use crate::widget::WidgetId;

use super::{WidgetInfo, WidgetInfoBuilder, WidgetInfoTree, iter::TreeIterator};

impl WidgetInfoBuilder {
    /// Accessibility metadata builder.
    ///
    /// Only available if accessibility info is required for the window.
    pub fn access(&mut self) -> Option<WidgetAccessInfoBuilder<'_>> {
        if self.access_enabled.is_enabled() {
            Some(WidgetAccessInfoBuilder { builder: self })
        } else {
            None
        }
    }
}

/// Accessibility metadata.
pub struct WidgetAccessInfoBuilder<'a> {
    pub(super) builder: &'a mut WidgetInfoBuilder,
}
impl WidgetAccessInfoBuilder<'_> {
    fn with_access(&mut self, f: impl FnOnce(&mut AccessInfo)) {
        self.builder.with_meta(move |mut m| f(m.entry(*ACCESS_INFO_ID).or_default()))
    }

    /// Set the accessibility role of the widget.
    pub fn set_role(&mut self, role: AccessRole) {
        self.with_access(|a| a.role = Some(role))
    }

    /// Add a supported access command.
    pub fn push_command(&mut self, cmd: AccessCmdName) {
        self.with_access(|a| a.commands.push(cmd))
    }

    /// Set how input text triggers display of one or more predictions of the user's intended
    /// value for a [`ComboBox`], [`SearchBox`], or [`TextInput`].
    ///
    /// [`ComboBox`]: AccessRole::ComboBox
    /// [`SearchBox`]: AccessRole::SearchBox
    /// [`TextInput`]: AccessRole::TextInput
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
    /// The other widget must [`set_invalid`].
    ///
    /// [`set_invalid`]: fn@Self::set_invalid
    pub fn set_error_message(&mut self, invalid_wgt: impl Into<WidgetId>) {
        let invalid_wgt = invalid_wgt.into();
        self.with_access(|a| a.set_state(AccessState::ErrorMessage(invalid_wgt.into())))
    }

    /// Identifies the currently active widget when focus is on a composite widget.
    pub fn set_active_descendant(&mut self, descendant: impl Into<WidgetId>) {
        let descendant = descendant.into();
        self.with_access(|a| a.set_state(AccessState::ActiveDescendant(descendant.into())))
    }

    /// Indicate that the widget toggles the visibility of related widgets.
    ///
    /// Use [`push_controls`], or [`push_owns`] to indicate the widgets that change visibility based on
    /// this value.
    ///
    /// [`push_controls`]: Self::push_controls
    /// [`push_owns`]: Self::push_owns
    pub fn set_expanded(&mut self, expanded: bool) {
        self.with_access(|a| a.set_state(AccessState::Expanded(expanded)))
    }

    /// Indicates the availability and type of interactive popup widget.
    pub fn set_popup(&mut self, popup: Popup) {
        self.with_access(|a| a.set_state(AccessState::Popup(popup)))
    }

    /// Indicates that the widget's data is invalid with optional kinds of errors.
    pub fn set_invalid(&mut self, error: Invalid) {
        self.with_access(|a| a.set_state(AccessState::Invalid(error)));
    }

    /// Sets a custom name for the widget in accessibility info.
    ///
    /// Note that if this is not set the [`WidgetId::name`] of the widget is used.
    pub fn set_label(&mut self, name: impl Into<Txt>) {
        let name = name.into();
        self.with_access(|a| a.set_state_source(AccessStateSource::Label(name)))
    }

    /// Sets the hierarchical level of the widget within a parent scope.
    pub fn set_level(&mut self, hierarchical_level: NonZeroU32) {
        self.with_access(|a| a.set_state(AccessState::Level(hierarchical_level)))
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
        self.with_access(|a| a.set_state_source(AccessStateSource::Placeholder(placeholder)))
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
        let value = value.into();
        self.with_access(|a| a.set_state_source(AccessStateSource::ValueText(value)))
    }

    /// Indicate that the widget can change, how the change can be announced, if `atomic`
    /// the entire widget must be re-read, if `busy` the screen reader must wait until the change completes.
    pub fn set_live(&mut self, indicator: LiveIndicator, atomic: bool, busy: bool) {
        self.with_access(|a| a.set_state(AccessState::Live { indicator, atomic, busy }))
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

    /// Sets the number of columns spanned by the widget in the parent table or grid.
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

    /// Sets the number of rows spanned by the widget in the parent table or grid.
    pub fn set_row_span(&mut self, span: usize) {
        self.with_access(|a| a.set_state(AccessState::RowSpan(span)))
    }

    /// Sets the number of items in the current set of list items or tree items when not all items in the set are present in the tree.
    pub fn set_item_count(&mut self, count: usize) {
        self.with_access(|a| a.set_state(AccessState::ItemCount(count)))
    }

    /// Sets the widget's number or position in the current set of list items or tree items when not all items are present in the tree.
    pub fn set_item_index(&mut self, index: usize) {
        self.with_access(|a| a.set_state(AccessState::ItemIndex(index)))
    }

    /// Sets if the widget is modal when displayed.
    pub fn flag_modal(&mut self) {
        self.with_access(|a| a.set_state(AccessState::Modal))
    }

    /// Defines the language used by screen-readers to read text in this widget and descendants.
    pub fn set_lang(&mut self, lang: LanguageIdentifier) {
        self.with_access(|a| a.set_state(AccessState::Lang(lang)))
    }

    /// Sets the amount scrolled horizontally if allowed.
    ///
    /// The `normal_x` value can be a read-only variable, the variable can be updated without needing to rebuild
    /// info for every pixel scrolled, if the view-process requires access info the value is updated every render
    /// together with the widget bounds updates.
    ///
    /// The value must be normalized in the 0..=1 range, 0 is showing the content leftmost edge, 1 is showing
    /// the content the rightmost edge.
    pub fn set_scroll_horizontal(&mut self, normal_x: impl IntoVar<Factor>) {
        let normal_x = normal_x.into_var().boxed();
        self.with_access(|a| a.set_state_source(AccessStateSource::ScrollHorizontal(normal_x)))
    }

    /// Sets the amount scrolled vertically if allowed.
    ///
    /// The `normal_y` value can be a read-only variable, the variable can be updated without needing to rebuild
    /// info for every pixel scrolled, if the view-process requires access info the value is updated every render
    /// together with the widget bounds updates.
    ///
    /// The value must be normalized in the 0..=1 range, 0 is showing the content topmost edge, 1 is showing
    /// the content the bottommost edge.
    pub fn set_scroll_vertical(&mut self, normal_y: impl IntoVar<Factor>) {
        let normal_y = normal_y.into_var().boxed();
        self.with_access(|a| a.set_state_source(AccessStateSource::ScrollVertical(normal_y)))
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

    /// Set a widget that is described-by this widget.
    ///
    /// When access info for the view-process is build this is converted to a described-by entry. Note
    /// that only updated widgets are send to the view-process, so if this relation is dynamic you must
    /// request info rebuild for the previous and new `target_id` to ensure they update correctly.
    pub fn set_describes(&mut self, target_id: impl Into<WidgetId>) {
        let target_id = target_id.into();
        self.with_access(|a| {
            for state in &mut a.inverse_state {
                if let InverseAccessState::Describes(t) = state {
                    *t = target_id;
                    return;
                }
            }
            a.inverse_state.push(InverseAccessState::Describes(target_id));
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
    pub fn push_labelled_by(&mut self, label_id: impl Into<WidgetId>) {
        let label_id = label_id.into();
        self.with_access(|a| {
            for state in &mut a.state {
                if let AccessState::LabelledBy(c) = state {
                    c.push(label_id.into());
                    return;
                }
            }
            a.state.push(AccessState::LabelledBy(vec![label_id.into()]))
        })
    }

    /// Set a widget that is labelled-by this widget.
    ///
    /// When access info for the view-process is build this is converted to a labelled-by entry. Note
    /// that only updated widgets are send to the view-process, so if this relation is dynamic you must
    /// request info rebuild for the previous and new `target_id` to ensure they update correctly.
    pub fn set_labels(&mut self, target_id: impl Into<WidgetId>) {
        let target_id = target_id.into();
        self.with_access(|a| {
            for state in &mut a.inverse_state {
                if let InverseAccessState::Labels(t) = state {
                    *t = target_id;
                    return;
                }
            }
            a.inverse_state.push(InverseAccessState::Labels(target_id));
        })
    }

    /// Push a widget that is a *child* of this widget, but is not already a child in the info tree.
    pub fn push_owns(&mut self, owned_id: impl Into<WidgetId>) {
        let owned_id = owned_id.into();
        self.with_access(|a| {
            for state in &mut a.state {
                if let AccessState::Owns(c) = state {
                    c.push(owned_id.into());
                    return;
                }
            }
            a.state.push(AccessState::Owns(vec![owned_id.into()]))
        })
    }

    /// Push an option for next widget read that is not the next logical widget already.
    pub fn push_flows_to(&mut self, next_id: impl Into<WidgetId>) {
        let next_id = next_id.into();
        self.with_access(|a| {
            for state in &mut a.state {
                if let AccessState::FlowTo(c) = state {
                    c.push(next_id.into());
                    return;
                }
            }
            a.state.push(AccessState::FlowTo(vec![next_id.into()]))
        })
    }

    /// Uses the accessible children as [`labelled_by`].
    ///
    /// [`labelled_by`]: WidgetAccessInfo::labelled_by
    pub fn flag_labelled_by_child(&mut self) {
        self.with_access(|a| a.set_state(AccessState::LabelledByChild))
    }

    /// Exclude the widget and descendants from the view-process and screen readers.
    ///
    /// Note that the accessibility info for the widget and descendants is still collected and
    /// available in the app-process.
    pub fn flag_inaccessible(&mut self) {
        self.builder.flag_meta(*INACCESSIBLE_ID);
    }

    /// Register a `handler` that is called every time view-process access info is build from the current widget,
    /// the handler can modify the view info.
    pub fn on_access_build(&mut self, handler: impl Fn(AccessBuildArgs) + Send + Sync + 'static) {
        let handler = Box::new(handler);
        self.with_access(|a| a.build_handlers.push(handler));
    }
}

impl WidgetInfoTree {
    /// If this tree contains accessibility information.
    ///
    /// If accessibility is enabled for the window and will stay enabled for its lifetime.
    pub fn access_enabled(&self) -> AccessEnabled {
        self.0.access_enabled
    }

    /// Build an access tree from the info tree.
    ///
    /// If not [`access_enabled`] returns a placeholder tree with only the root node.
    ///
    /// [`access_enabled`]: Self::access_enabled
    pub fn to_access_tree(&self) -> zng_view_api::access::AccessTree {
        let mut builder = zng_view_api::access::AccessTreeBuilder::default();
        if self.0.access_enabled.is_enabled() {
            // no panic cause root role is always set by the builder.
            let inverse = self.collect_inverse_state();
            self.root().access().unwrap().to_access_info(&inverse, &mut builder);
        } else {
            builder.push(zng_view_api::access::AccessNode::new(
                self.root().id().into(),
                Some(AccessRole::Application),
            ));
        }
        builder.build()
    }

    /// Build partial or full access trees for updated widgets.
    ///
    /// Returns `None` if not [`access_enabled`] or no access info has changed. The [`focused`] value is always set
    /// to the root ID, it must be changed to the correct focused widget.
    ///
    /// This is usually called by window implementers just after the next frame after info rebuild. Note that these
    /// updates will also include [`to_access_updates_bounds`].
    ///
    /// [`access_enabled`]: Self::access_enabled
    /// [`focused`]: zng_view_api::access::AccessTreeUpdate::focused
    /// [`to_access_updates_bounds`]: Self::to_access_updates_bounds
    pub fn to_access_updates(&self, prev_tree: &Self) -> Option<zng_view_api::access::AccessTreeUpdate> {
        let is_enabled = self.access_enabled().is_enabled();
        let root_id = self.root().id().into();
        if is_enabled && !prev_tree.access_enabled().is_enabled() {
            // first update after access enabled
            return Some(zng_view_api::access::AccessTreeUpdate::new(
                vec![self.to_access_tree()],
                Some(root_id),
                root_id,
            ));
        }

        if is_enabled {
            let inverse = self.collect_inverse_state();
            let mut updates = vec![];
            self.root().access().unwrap().to_access_updates(prev_tree, &inverse, &mut updates);
            if !updates.is_empty() {
                return Some(zng_view_api::access::AccessTreeUpdate::new(updates, None, root_id));
            }
        }

        None
    }

    /// Build partial access trees for widgets that changed transform, size or visibility.
    ///
    /// Returns `None` if not [`access_enabled`] or no transform/visibility changed. The [`focused`] value is always set
    /// to the root ID, it must be changed to the correct focused widget.
    ///
    /// This is usually called by window implementers after each frame that is not [`to_access_updates`].
    ///
    /// [`access_enabled`]: Self::access_enabled
    /// [`focused`]: zng_view_api::access::AccessTreeUpdate::focused
    /// [`to_access_updates`]: Self::to_access_updates
    pub fn to_access_updates_bounds(&self) -> Option<zng_view_api::access::AccessTreeUpdate> {
        let is_enabled = self.access_enabled().is_enabled();
        let root_id = self.root().id().into();

        if is_enabled && {
            let frame = self.0.frame.read();
            frame.stats.bounds_updated_frame == frame.stats.last_frame || frame.stats.vis_updated_frame == frame.stats.last_frame
        } {
            let inverse = self.collect_inverse_state();
            let mut updates = vec![];
            self.root().access().unwrap().to_access_updates_bounds(&inverse, &mut updates);
            if !updates.is_empty() {
                return Some(zng_view_api::access::AccessTreeUpdate::new(updates, None, root_id));
            }
        }

        None
    }

    fn collect_inverse_state(&self) -> InverseAccess {
        let mut state = InverseAccess::default();
        for wgt in self.root().self_and_descendants() {
            if let Some(a) = wgt.access() {
                if let Some(t) = a.labels() {
                    state.labelled_by.entry(t.id()).or_default().push(wgt.id());
                }
                if let Some(t) = a.describes() {
                    state.described_by.entry(t.id()).or_default().push(wgt.id());
                }
            }
        }
        state
    }
}

impl WidgetInfo {
    /// Accessibility info, if the widget is accessible.
    ///
    /// The widget is accessible only if [`access_enabled`] and some accessibility metadata was set on the widget.
    ///
    /// [`access_enabled`]: crate::widget::info::WidgetInfoTree::access_enabled
    pub fn access(&self) -> Option<WidgetAccessInfo> {
        if self.tree.access_enabled().is_enabled() && self.meta().contains(*ACCESS_INFO_ID) {
            Some(WidgetAccessInfo { info: self.clone() })
        } else {
            None
        }
    }

    /// Descendant branches that have accessibility info.
    ///
    /// The iterator enters descendants only until it finds a node that has access info, these nodes are yielded.
    pub fn access_children(&self) -> impl Iterator<Item = WidgetAccessInfo> {
        self.descendants()
            .tree_filter(|w| {
                if w.access().is_some() {
                    super::TreeFilter::SkipDescendants
                } else {
                    super::TreeFilter::Skip
                }
            })
            .map(|w| w.access().unwrap())
    }

    fn access_children_ids(&self, is_prev: bool) -> Vec<zng_view_api::access::AccessNodeId> {
        self.access_children()
            .filter_map(|w| {
                if w.is_local_accessible() {
                    if is_prev && w.access().view_bounds.lock().is_none() {
                        // was collapsed
                        None
                    } else {
                        Some(w.info.id().into())
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// First ancestor that is accessible.
    pub fn access_parent(&self) -> Option<WidgetAccessInfo> {
        self.ancestors().find_map(|w| w.access())
    }
}

/// Accessibility info for a widget.
pub struct WidgetAccessInfo {
    info: WidgetInfo,
}
macro_rules! get_state {
    ($self:ident.$Discriminant:ident) => {
        get_state!($self, state, AccessState, $Discriminant)
    };
    ($self:ident.source.$Discriminant:ident) => {
        get_state!($self, state_source, AccessStateSource, $Discriminant)
    };
    ($self:ident.inverse.$Discriminant:ident) => {
        get_state!($self, inverse_state, InverseAccessState, $Discriminant)
    };
    ($self:ident, $state:ident, $State:ident, $Discriminant:ident) => {
        $self
            .access()
            .$state
            .iter()
            .find_map(|a| if let $State::$Discriminant(value) = a { Some(value) } else { None })
    };
}
macro_rules! has_state {
    ($self:ident.$Discriminant:ident) => {
        $self.access().state.iter().any(|a| matches!(a, AccessState::$Discriminant))
    };
}
macro_rules! get_widgets {
    ($self:ident.$Discriminant:ident) => {
        $self
            .access()
            .state
            .iter()
            .find_map(|a| {
                if let AccessState::$Discriminant(ids) = a {
                    Some(ids.iter().filter_map(|id| {
                        let id = WidgetId::from_raw(id.0);
                        $self.info.tree.get(id)
                    }))
                } else {
                    None
                }
            })
            .into_iter()
            .flatten()
    };
}
impl WidgetAccessInfo {
    /// Full widget info.
    pub fn info(&self) -> &WidgetInfo {
        &self.info
    }

    fn access(&self) -> &AccessInfo {
        self.info.meta().req(*ACCESS_INFO_ID)
    }

    /// Accessibility role of the widget.
    pub fn role(&self) -> Option<AccessRole> {
        self.access().role
    }

    /// Accessibility commands supported by the widget.
    pub fn commands(&self) -> &[AccessCmdName] {
        &self.access().commands
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

    /// Identifies the currently active widget when focus is on a composite widget.
    pub fn active_descendant(&self) -> Option<WidgetInfo> {
        let id = get_state!(self.ActiveDescendant)?;
        let id = WidgetId::from_raw(id.0);
        self.info.tree.get(id)
    }

    /// Gets visibility of related widgets.
    pub fn expanded(&self) -> Option<bool> {
        get_state!(self.Expanded).copied()
    }

    /// Indicates the availability and type of interactive popup widget.
    pub fn has_popup(&self) -> Option<Popup> {
        get_state!(self.Popup).copied()
    }

    /// If the widget data has errors.
    pub fn invalid(&self) -> Invalid {
        get_state!(self.Invalid).copied().unwrap_or_else(Invalid::empty)
    }

    /// Gets the accessibility name explicitly set on this widget.
    pub fn label(&self) -> Option<Txt> {
        get_state!(self.source.Label).cloned()
    }

    /// If the widget children must be used like [`labelled_by`].
    ///
    /// [`labelled_by`]: Self::labelled_by
    pub fn labelled_by_child(&self) -> bool {
        has_state!(self.LabelledByChild)
    }

    /// Gets the language of texts inside this widget and descendants.
    ///
    /// If not set it is the parents language.
    pub fn lang(&self) -> Option<LanguageIdentifier> {
        get_state!(self.Lang).cloned()
    }
    /// Normalized (0..1) horizontal scroll, 0 is showing the content leftmost edge, 1 is showing the content the rightmost edge.
    ///
    /// Also signals that the content is horizontally scrollable.
    pub fn scroll_horizontal(&self) -> Option<BoxedVar<Factor>> {
        get_state!(self.source.ScrollHorizontal).cloned()
    }
    /// Normalized (0..1) vertical scroll, 0 is showing the content topmost edge, 1 is showing the content the bottommost edge.
    ///
    /// Also signals that the content is vertically scrollable.
    pub fn scroll_vertical(&self) -> Option<BoxedVar<Factor>> {
        get_state!(self.source.ScrollVertical).cloned()
    }

    /// Indicates that the user may select more than one item from the current selectable descendants.
    pub fn is_multi_selectable(&self) -> bool {
        has_state!(self.MultiSelectable)
    }

    /// Indicates whether the widget's orientation is horizontal, vertical, or unknown/ambiguous.
    pub fn orientation(&self) -> Option<Orientation> {
        get_state!(self.Orientation).copied()
    }

    /// Short hint (a word or short phrase) intended to help the user with data entry when a form control has no value.
    pub fn placeholder(&self) -> Option<Txt> {
        get_state!(self.source.Placeholder).cloned()
    }

    /// Indicates that the widget is not editable, but is otherwise operable.
    pub fn is_read_only(&self) -> bool {
        has_state!(self.ReadOnly)
    }

    /// Indicates that user input is required on the widget before a form may be submitted.
    pub fn is_required(&self) -> bool {
        has_state!(self.Required)
    }

    /// Defines the hierarchical level of a widget within a structure.
    pub fn level(&self) -> Option<NonZeroU32> {
        get_state!(self.Level).copied()
    }

    /// Indicates that the widget is selected.
    pub fn is_selected(&self) -> bool {
        has_state!(self.Selected)
    }

    /// Indicates if items in a table or grid are sorted in ascending or descending order.
    pub fn sort(&self) -> Option<SortDirection> {
        get_state!(self.Sort).copied()
    }

    /// Maximum value (inclusive).
    pub fn value_max(&self) -> Option<f64> {
        get_state!(self.ValueMax).copied()
    }

    /// Minimum value (inclusive).
    pub fn value_min(&self) -> Option<f64> {
        get_state!(self.ValueMin).copied()
    }

    /// Current value.
    pub fn value(&self) -> Option<f64> {
        get_state!(self.Value).copied()
    }

    /// Current value in a readable format.
    ///
    /// Note that this returns `Some(_)` only when a value text was set, [`value`]
    /// may or may not be set also.
    ///
    /// [`value`]: Self::value
    pub fn value_text(&self) -> Option<Txt> {
        get_state!(self.source.ValueText).cloned()
    }

    /// Gets the live indicator, atomic and busy.
    ///
    /// See [`AccessState::Live`] for more details.
    ///
    /// [`AccessState::Live`]: zng_view_api::access::AccessState::Live
    pub fn live(&self) -> Option<(LiveIndicator, bool, bool)> {
        self.access().state.iter().find_map(|s| {
            if let AccessState::Live { indicator, atomic, busy } = s {
                Some((*indicator, *atomic, *busy))
            } else {
                None
            }
        })
    }

    /// Defines the total number of columns in a [`Table`], [`Grid`], or [`TreeGrid`] when not all columns are present in tree.
    ///
    /// The value `0` indicates that not all columns are in the widget and the application cannot determinate the exact number.
    ///
    /// [`Table`]: AccessRole::Table
    /// [`Grid`]: AccessRole::Grid
    /// [`TreeGrid`]: AccessRole::TreeGrid
    pub fn col_count(&self) -> Option<usize> {
        get_state!(self.ColCount).copied()
    }

    /// Defines a widget's column index in the parent table or grid.
    pub fn col_index(&self) -> Option<usize> {
        get_state!(self.ColIndex).copied()
    }

    /// Defines the number of columns spanned by the widget in the parent table or grid.
    pub fn col_span(&self) -> Option<usize> {
        get_state!(self.ColSpan).copied()
    }

    /// Defines the total number of rows in a [`Table`], [`Grid`], or [`TreeGrid`] when not all rows are present in tree.
    ///
    /// The value `0` indicates that not all rows are in the widget and the application cannot determinate the exact number.
    ///
    /// [`Table`]: AccessRole::Table
    /// [`Grid`]: AccessRole::Grid
    /// [`TreeGrid`]: AccessRole::TreeGrid
    pub fn row_count(&self) -> Option<usize> {
        get_state!(self.RowCount).copied()
    }

    /// Defines a widget's column index in the parent table or grid.
    pub fn row_index(&self) -> Option<usize> {
        get_state!(self.RowIndex).copied()
    }

    /// Defines the number of columns spanned by the widget in the parent table or grid.
    pub fn row_span(&self) -> Option<usize> {
        get_state!(self.RowSpan).copied()
    }

    /// Defines the number of items in the current set of list items or tree items when not all items in the set are present in the tree.
    pub fn item_count(&self) -> Option<usize> {
        get_state!(self.ItemCount).copied()
    }

    /// Defines the widget's number or position in the current set of list items or tree items when not all items are present in the tree.
    pub fn item_index(&self) -> Option<usize> {
        get_state!(self.ItemIndex).copied()
    }

    /// Indicates whether the widget is modal when displayed.
    pub fn modal(&self) -> bool {
        has_state!(self.Modal)
    }

    /// Widget(s) whose contents or presence are controlled by this widget.
    pub fn controls(&self) -> impl Iterator<Item = WidgetInfo> + '_ {
        get_widgets!(self.Controls)
    }

    /// Identifies the widget(s) that describes this widget.
    pub fn described_by(&self) -> impl Iterator<Item = WidgetInfo> + '_ {
        get_widgets!(self.DescribedBy)
    }

    /// Identifies the widget that is described by this widget.
    ///
    /// Note that this is not a query for all widgets that have this one in their [`described_by`] list, it is only
    /// set if it was set explicitly during info build.
    ///
    /// [`described_by`]: Self::described_by
    pub fn describes(&self) -> Option<WidgetInfo> {
        get_state!(self.inverse.Describes).copied().and_then(|id| self.info.tree().get(id))
    }

    /// Identifies the widget(s) that provide additional information related to this widget.
    pub fn details(&self) -> impl Iterator<Item = WidgetInfo> + '_ {
        get_widgets!(self.Details)
    }

    /// Identifies the widget(s) that labels the widget it is applied to.
    pub fn labelled_by(&self) -> impl Iterator<Item = WidgetInfo> + '_ {
        get_widgets!(self.LabelledBy)
    }

    /// Identifies the widget that is labelled by this widget.
    ///
    /// Note that this is not a query for all widgets that have this one in their [`labelled_by`] list, it is only
    /// set if it was set explicitly during info build.
    ///
    /// [`labelled_by`]: Self::labelled_by
    pub fn labels(&self) -> Option<WidgetInfo> {
        get_state!(self.inverse.Labels).copied().and_then(|id| self.info.tree().get(id))
    }

    /// Extra widgets that are *child* to this widget, but are not descendants on the info tree.
    pub fn owns(&self) -> impl Iterator<Item = WidgetInfo> + '_ {
        get_widgets!(self.Owns)
    }

    /// Options for next widget to read.
    pub fn flows_to(&self) -> impl Iterator<Item = WidgetInfo> + '_ {
        get_widgets!(self.FlowTo)
    }

    /// If the widget and descendants is *visible* in the view-process and screen readers.
    ///   
    /// Note that the accessibility info for the widget and descendants is still
    /// available in the app-process.
    pub fn is_accessible(&self) -> bool {
        for wgt in self.info.self_and_ancestors() {
            if wgt.meta().contains(*INACCESSIBLE_ID) || !self.info.visibility().is_visible() {
                return false;
            }
        }
        true
    }

    fn is_local_accessible(&self) -> bool {
        !self.info.meta().contains(*INACCESSIBLE_ID) && self.info.visibility().is_visible()
    }

    fn to_access_node_leaf(&self, inverse: &InverseAccess) -> zng_view_api::access::AccessNode {
        let mut node = zng_view_api::access::AccessNode::new(self.info.id().into(), None);
        let a = self.access();

        let bounds_info = self.bounds_info();
        node.transform = bounds_info.transform;
        node.size = bounds_info.size;
        *a.view_bounds.lock() = Some(bounds_info);

        node.role = a.role;
        node.state.clone_from(&a.state);
        node.state.extend(a.state_source.iter().map(From::from));

        if let Some(lb) = inverse.labelled_by.get(&self.info.id()) {
            let mut done = false;
            for state in node.state.iter_mut() {
                if let AccessState::LabelledBy(l) = state {
                    l.extend(lb.iter().map(|&id| AccessNodeId::from(id)));
                    done = true;
                    break;
                }
            }
            if !done {
                node.state.push(AccessState::LabelledBy(lb.iter().map(|&id| id.into()).collect()));
            }
        }
        if let Some(ds) = inverse.described_by.get(&self.info.id()) {
            let mut done = false;
            for state in node.state.iter_mut() {
                if let AccessState::DescribedBy(l) = state {
                    l.extend(ds.iter().map(|&id| AccessNodeId::from(id)));
                    done = true;
                    break;
                }
            }
            if !done {
                node.state.push(AccessState::DescribedBy(ds.iter().map(|&id| id.into()).collect()));
            }
        }

        node.commands.clone_from(&a.commands);

        for handler in &a.build_handlers {
            handler(AccessBuildArgs {
                widget: self,
                node: &mut node,
            });
        }

        node
    }

    fn bounds_info(&self) -> ViewBoundsInfo {
        let bounds = self.info.bounds_info();
        let undo_parent_transform = self
            .info
            .access_parent()
            .and_then(|w| w.info.inner_transform().inverse())
            .unwrap_or_default();
        let transform = bounds.inner_transform().then(&undo_parent_transform);
        let size = bounds.inner_size();

        let scroll_h = get_state!(self.source.ScrollHorizontal).map(|x| x.get());
        let scroll_v = get_state!(self.source.ScrollVertical).map(|x| x.get());

        ViewBoundsInfo {
            transform,
            size,
            scroll_h,
            scroll_v,
        }
    }

    fn to_access_info(&self, inverse: &InverseAccess, builder: &mut zng_view_api::access::AccessTreeBuilder) -> bool {
        if !self.is_local_accessible() {
            if self.info.parent().is_none() {
                // root node is required (but can be empty)
                builder.push(zng_view_api::access::AccessNode::new(self.info.id().into(), self.access().role));
            }
            *self.access().view_bounds.lock() = None;
            return false;
        }

        let node = builder.push(self.to_access_node_leaf(inverse));

        let mut children_len = 0;
        let len_before = builder.len();
        for child in self.info.access_children() {
            if child.to_access_info(inverse, builder) {
                children_len += 1;
            }
        }
        let descendants_len = (builder.len() - len_before) as u32;

        let node = builder.node(node);
        node.children_len = children_len;
        node.descendants_len = descendants_len;

        true
    }

    fn to_access_updates(&self, prev_tree: &WidgetInfoTree, inverse: &InverseAccess, updates: &mut Vec<zng_view_api::access::AccessTree>) {
        if !self.is_local_accessible() {
            // not accessible
            *self.access().view_bounds.lock() = None;
            return;
        }

        let mut bounds_changed = false;
        let mut vis_changed = false;
        if self.info.is_reused() {
            // no info change, check bounds that can change every render

            let bounds = Some(self.bounds_info());
            let a = self.access();
            let mut prev_bounds = a.view_bounds.lock();

            bounds_changed = *prev_bounds != bounds;

            if !bounds_changed {
                return;
            }

            vis_changed = prev_bounds.is_none() && bounds.is_some();

            *prev_bounds = bounds;
        }
        let bounds_changed = bounds_changed;
        let vis_changed = vis_changed;

        if let Some(prev) = prev_tree.get(self.info.id()) {
            let was_accessible = !vis_changed && prev.access().map(|w| w.is_local_accessible()).unwrap_or(false);
            if let (true, Some(prev)) = (was_accessible, prev.access()) {
                let mut children = None;
                if bounds_changed || !prev.access().info_eq(self.access()) || {
                    // check children and cache result
                    let c = self.info.access_children_ids(false);
                    let changed = c != prev.info.access_children_ids(true);
                    children = Some(c);
                    changed
                } {
                    // changed
                    let mut node = self.to_access_node_leaf(inverse);

                    for child in self.info.access_children() {
                        child.to_access_updates(prev_tree, inverse, updates);
                    }

                    node.children = children.unwrap_or_else(|| {
                        self.info
                            .access_children()
                            .filter_map(|a| if a.is_local_accessible() { Some(a.info.id().into()) } else { None })
                            .collect()
                    });

                    let mut builder = zng_view_api::access::AccessTreeBuilder::default();
                    builder.push(node);
                    updates.push(builder.build());

                    return;
                } else {
                    // no change in widget our children, may have change in descendants

                    for child in self.info.access_children() {
                        child.to_access_updates(prev_tree, inverse, updates);
                    }

                    return;
                }
            } else {
                // was not accessible
            }
        }

        // insert
        let mut builder = zng_view_api::access::AccessTreeBuilder::default();
        let insert = self.to_access_info(inverse, &mut builder);
        assert!(insert);
        updates.push(builder.build());
    }

    /// Returns `true` if access changed by visibility update.
    fn to_access_updates_bounds(&self, inverse: &InverseAccess, updates: &mut Vec<zng_view_api::access::AccessTree>) -> bool {
        if self.info.meta().contains(*INACCESSIBLE_ID) {
            // not accessible
            return false;
        }
        if !self.info.visibility().is_visible() {
            // not accessible because not visible
            return self.access().view_bounds.lock().take().is_some();
        }

        let a = self.access();

        let mut vis_changed = false;
        let mut update;

        let new_bounds = Some(self.bounds_info());
        {
            let mut bounds = a.view_bounds.lock();
            update = *bounds != new_bounds;
            if update {
                vis_changed = bounds.is_none();
                *bounds = new_bounds;
            }
        };

        if vis_changed {
            // branch now accessible
            let mut builder = zng_view_api::access::AccessTreeBuilder::default();
            let insert = self.to_access_info(inverse, &mut builder);
            assert!(insert);
            updates.push(builder.build());
        } else {
            // update if bounds info changed or a child changed visibility

            for child in self.info.access_children() {
                let child_vis_changed = child.to_access_updates_bounds(inverse, updates);
                update |= child_vis_changed;
            }

            if update {
                let mut node = self.to_access_node_leaf(inverse);
                node.children = self
                    .info
                    .access_children()
                    .filter_map(|a| if a.is_local_accessible() { Some(a.info.id().into()) } else { None })
                    .collect();

                let mut builder = zng_view_api::access::AccessTreeBuilder::default();
                builder.push(node);
                updates.push(builder.build());
            }
        }

        vis_changed
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
struct ViewBoundsInfo {
    transform: PxTransform,
    size: PxSize,
    scroll_h: Option<Factor>,
    scroll_v: Option<Factor>,
}

#[derive(Default)]
struct AccessInfo {
    role: Option<AccessRole>,
    commands: Vec<AccessCmdName>,
    state: Vec<AccessState>,
    state_source: Vec<AccessStateSource>,
    inverse_state: Vec<InverseAccessState>,

    view_bounds: Mutex<Option<ViewBoundsInfo>>,
    build_handlers: Vec<Box<dyn Fn(AccessBuildArgs) + Send + Sync>>,
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

    fn set_state_source(&mut self, state: AccessStateSource) {
        let discriminant = std::mem::discriminant(&state);
        if let Some(present) = self.state_source.iter_mut().find(|s| std::mem::discriminant(&**s) == discriminant) {
            *present = state;
        } else {
            self.state_source.push(state);
        }
    }

    fn info_eq(&self, other: &Self) -> bool {
        self.role == other.role && self.commands == other.commands && self.state == other.state && self.state_source == other.state_source
    }
}

enum AccessStateSource {
    Label(Txt),
    Placeholder(Txt),
    ValueText(Txt),
    ScrollHorizontal(BoxedVar<Factor>),
    ScrollVertical(BoxedVar<Factor>),
}
impl PartialEq for AccessStateSource {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Label(l0), Self::Label(r0)) => l0 == r0,
            (Self::Placeholder(l0), Self::Placeholder(r0)) => l0 == r0,
            (Self::ValueText(l0), Self::ValueText(r0)) => l0 == r0,
            // values equality not done here, see `ViewBoundsInfo` usage
            (Self::ScrollHorizontal(l0), Self::ScrollHorizontal(r0)) => l0.var_ptr() == r0.var_ptr(),
            (Self::ScrollVertical(l0), Self::ScrollVertical(r0)) => l0.var_ptr() == r0.var_ptr(),
            _ => false,
        }
    }
}
impl From<&AccessStateSource> for AccessState {
    fn from(value: &AccessStateSource) -> Self {
        match value {
            AccessStateSource::Label(l) => AccessState::Label(l.clone()),
            AccessStateSource::Placeholder(p) => AccessState::Placeholder(p.clone()),
            AccessStateSource::ValueText(v) => AccessState::ValueText(v.clone()),
            AccessStateSource::ScrollHorizontal(x) => AccessState::ScrollHorizontal(x.get().0),
            AccessStateSource::ScrollVertical(y) => AccessState::ScrollVertical(y.get().0),
        }
    }
}

enum InverseAccessState {
    Labels(WidgetId),
    Describes(WidgetId),
}

#[derive(Default)]
struct InverseAccess {
    labelled_by: IdMap<WidgetId, Vec<WidgetId>>,
    described_by: IdMap<WidgetId, Vec<WidgetId>>,
}

static_id! {
    static ref ACCESS_INFO_ID: StateId<AccessInfo>;
    static ref INACCESSIBLE_ID: StateId<()>;
}

bitflags::bitflags! {
    /// Defines how accessibility info is enabled.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct AccessEnabled: u8 {
        /// Access info is collected in the app-process and is available in ([`WidgetInfo::access`]).
        const APP = 0b01;
        /// Access info is send to the view-process because it was requested by an external tool, probably a screen reader.
        const VIEW = 0b11;
    }
}
impl AccessEnabled {
    /// Is enabled in app at least.
    pub fn is_enabled(self) -> bool {
        !self.is_empty()
    }

    /// Is not enabled in app nor view.
    pub fn is_disabled(self) -> bool {
        self.is_empty()
    }
}

/// Arguments for [`on_access_build`] handlers.
///
/// [`on_access_build`]: WidgetAccessInfoBuilder::on_access_build
#[non_exhaustive]
pub struct AccessBuildArgs<'a> {
    /// Widget that is converting to view info.
    pub widget: &'a WidgetAccessInfo,
    /// Partially build view info, does not include children info.
    pub node: &'a mut zng_view_api::access::AccessNode,
}
