//! Widget info, builder and base, UI node and list.
//!
//! # Full API
//!
//! See [`zero_ui_app::widget`] for the full API.

pub use zero_ui_app::widget::base::{HitTestMode, Parallel, WidgetBase, WidgetExt, WidgetImpl, PARALLEL_VAR};

pub use zero_ui_app::widget::{
    easing, property, ui_node, widget, widget_mixin, widget_set, StaticWidgetId, WidgetId, WidgetUpdateMode, WIDGET,
};

pub use zero_ui_app::widget::border::{
    BorderSide, BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle, BORDER,
};
pub use zero_ui_app::widget::info::Visibility;
pub use zero_ui_app::widget::node::ZIndex;

pub use zero_ui_wgt::{
    border, border_align, border_over, can_auto_hide, clip_to_bounds, corner_radius, corner_radius_fit, enabled, hit_test_mode, inline,
    interactive, is_collapsed, is_disabled, is_enabled, is_hidden, is_hit_testable, is_inited, is_visible, modal, modal_included,
    modal_includes, on_block, on_blocked_changed, on_deinit, on_disable, on_enable, on_enabled_changed, on_info_init, on_init,
    on_interactivity_changed, on_move, on_node_op, on_pre_block, on_pre_blocked_changed, on_pre_deinit, on_pre_disable, on_pre_enable,
    on_pre_enabled_changed, on_pre_init, on_pre_interactivity_changed, on_pre_move, on_pre_node_op, on_pre_transform_changed,
    on_pre_unblock, on_pre_update, on_pre_vis_disable, on_pre_vis_enable, on_pre_vis_enabled_changed, on_transform_changed, on_unblock,
    on_update, on_vis_disable, on_vis_enable, on_vis_enabled_changed, parallel, visibility, wgt_fn, z_index, OnDeinitArgs, OnNodeOpArgs,
    Wgt, WidgetFn,
};

pub use zero_ui_wgt_fill::{
    background, background_color, background_conic, background_fn, background_gradient, background_radial, foreground, foreground_color,
    foreground_fn, foreground_gradient, foreground_highlight,
};

/// Widget and property builder types.
///
/// See [`zero_ui_app::widget::builder`] for the full API.
pub mod builder {
    pub use zero_ui_app::widget::builder::{
        property_args, property_id, property_info, property_input_types, source_location, widget_type, AnyWhenArcWidgetHandlerBuilder,
        ArcWidgetHandler, BuilderProperty, BuilderPropertyMut, BuilderPropertyRef, Importance, InputKind, NestGroup, NestPosition,
        PropertyArgs, PropertyBuildAction, PropertyBuildActionArgs, PropertyBuildActions, PropertyBuildActionsWhenData, PropertyId,
        PropertyInfo, PropertyInput, PropertyInputTypes, PropertyNewArgs, SourceLocation, WhenBuildAction, WhenInfo, WhenInput,
        WhenInputMember, WhenInputVar, WidgetBuilder, WidgetBuilderProperties, WidgetBuilding, WidgetType,
    };
}

/// Widget info tree and info builder.
pub mod info {
    pub use zero_ui_app::widget::info::{
        iter, HitInfo, HitTestInfo, InlineSegmentInfo, InteractionPath, Interactivity, InteractivityChangedArgs, InteractivityFilterArgs,
        ParallelBuilder, RelativeHitZ, TransformChangedArgs, TreeFilter, VisibilityChangedArgs, WidgetBorderInfo, WidgetBoundsInfo,
        WidgetDescendantsRange, WidgetInfo, WidgetInfoBuilder, WidgetInfoChangedArgs, WidgetInfoMeta, WidgetInfoTree, WidgetInfoTreeStats,
        WidgetInlineInfo, WidgetInlineMeasure, WidgetPath, INTERACTIVITY_CHANGED_EVENT, TRANSFORM_CHANGED_EVENT, VISIBILITY_CHANGED_EVENT,
        WIDGET_INFO_CHANGED_EVENT,
    };

    /// Accessibility metadata types.
    pub mod access {
        pub use zero_ui_app::widget::info::access::{AccessBuildArgs, WidgetAccessInfo, WidgetAccessInfoBuilder};
    }

    /// Helper types for inspecting an UI tree.
    pub mod inspector {
        pub use zero_ui_app::widget::inspector::{
            InspectPropertyPattern, InspectWidgetPattern, InspectorContext, InspectorInfo, InstanceItem, WidgetInfoInspectorExt,
        };
    }
}

/// Widget node types, [`UiNode`], [`UiNodeList`] and others.
///
/// [`UiNode`]: crate::prelude::UiNode
/// [`UiNodeList`]: crate::prelude::UiNodeList
pub mod node {
    pub use zero_ui_app::widget::node::{
        extend_widget, match_node, match_node_leaf, match_node_list, match_node_typed, match_widget, ui_vec, AdoptiveChildNode,
        AdoptiveNode, ArcNode, ArcNodeList, BoxedUiNode, BoxedUiNodeList, DefaultPanelListData, EditableUiNodeList, EditableUiNodeListRef,
        FillUiNode, MatchNodeChild, MatchNodeChildren, MatchWidgetChild, NilUiNode, OffsetUiListObserver, PanelList, PanelListData,
        PanelListRange, PanelListRangeHandle, SortingList, UiNode, UiNodeList, UiNodeListChain, UiNodeListChainImpl, UiNodeListObserver,
        UiNodeOp, UiNodeOpMethod, UiNodeVec, WeakNode, WeakNodeList, WhenUiNodeBuilder, WhenUiNodeListBuilder, SORTING_LIST, Z_INDEX,
    };

    pub use zero_ui_wgt::node::{
        bind_state, border_node, event_is_state, event_is_state2, event_is_state3, event_is_state4, fill_node, interactive_node,
        list_presenter, presenter, presenter_opt, validate_getter_var, widget_state_get_state, widget_state_is_state, with_context_blend,
        with_context_local, with_context_local_init, with_context_var, with_context_var_init, with_index_len_node, with_index_node,
        with_rev_index_node, with_widget_state, with_widget_state_modify,
    };
}
