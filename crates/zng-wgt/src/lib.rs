#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Basic widget properties and helpers for declaring widgets and properties.
//!
//! # Widget Instantiation
//!
//! See [`enable_widget_macros!`] if you want to instantiate widgets without depending on the `zng` crate.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
// suppress nag about very simple boxed closure signatures.
#![allow(clippy::type_complexity)]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

pub use zng_app::enable_widget_macros;
enable_widget_macros!();

#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zng_wgt; // for doc-tests

/// Prelude for declaring properties and widgets.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::__prelude::*;
}
mod __prelude {
    pub use zng_app::{
        event::{
            command, event, event_args, AnyEventArgs as _, Command, CommandHandle, CommandInfoExt as _, CommandNameExt as _, Event,
            EventArgs as _, EventHandle, EventHandles, EventPropagationHandle,
        },
        handler::{app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once, AppHandler, WidgetHandler},
        render::{FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, SpatialFrameId, TransformStyle},
        shortcut::{shortcut, CommandShortcutExt as _, Shortcut, ShortcutFilter, Shortcuts},
        timer::{DeadlineHandle, DeadlineVar, TimerHandle, TimerVar, TIMERS},
        update::{EventUpdate, UpdateDeliveryList, UpdateOp, WidgetUpdates, UPDATES},
        widget::{
            base::{WidgetBase, WidgetImpl},
            border::{BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle, BORDER},
            builder::{property_id, NestGroup, WidgetBuilder, WidgetBuilding},
            easing,
            info::{
                InteractionPath, Interactivity, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder,
                WidgetLayout, WidgetMeasure, WidgetPath,
            },
            node::{
                match_node, match_node_leaf, match_node_list, match_node_typed, match_widget, ui_vec, ArcNode, ArcNodeList, BoxedUiNode,
                BoxedUiNodeList, EditableUiNodeList, EditableUiNodeListRef, FillUiNode, NilUiNode, PanelList, PanelListData as _,
                SortingList, UiNode, UiNodeList, UiNodeListChain as _, UiNodeListObserver, UiNodeOp, UiVec, ZIndex, SORTING_LIST,
            },
            property, ui_node, widget, widget_impl, widget_mixin, widget_set, AnyVarSubscribe as _, VarLayout as _, VarSubscribe as _,
            WidgetId, WidgetUpdateMode, WIDGET,
        },
        window::{MonitorId, WindowId, WINDOW},
        DInstant, Deadline, INSTANT,
    };

    pub use zng_var::{
        context_var, expr_var, impl_from_and_into_var, merge_var, response_done_var, response_var, state_var, var, var_from, when_var,
        AnyVar as _, AnyWeakVar as _, ArcVar, BoxedVar, ContextVar, IntoValue, IntoVar, LocalVar, ObservableVec, ReadOnlyArcVar,
        ResponderVar, ResponseVar, Var, VarCapability, VarHandle, VarHandles, VarUpdateId, VarValue, WeakVar as _,
    };

    pub use zng_layout::{
        context::{LayoutDirection, LayoutMetrics, DIRECTION_VAR, LAYOUT},
        unit::{
            Align, AngleDegree, AngleGradian, AngleRadian, AngleUnits as _, ByteUnits as _, Dip, DipBox, DipPoint, DipRect, DipSideOffsets,
            DipSize, DipToPx as _, DipVector, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits as _, Layout1d as _,
            Layout2d as _, LayoutAxis, Length, LengthUnits as _, Line, LineFromTuplesBuilder as _, Point, Px, PxBox, PxConstraints,
            PxConstraints2d, PxCornerRadius, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip as _, PxTransform, PxVector, Rect,
            RectFromTuplesBuilder as _, ResolutionUnits as _, SideOffsets, Size, TimeUnits as _, Transform, Vector,
        },
    };

    pub use zng_txt::{formatx, ToTxt, Txt};

    pub use zng_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use zng_task as task;

    pub use zng_app_context::{app_local, context_local, CaptureFilter, ContextLocal, ContextValueSet, LocalContext, RunOnDrop};

    pub use zng_state_map::{state_map, static_id, OwnedStateMap, StateId, StateMapMut, StateMapRef};

    pub use zng_unique_id::{IdEntry, IdMap, IdSet};

    pub use zng_color::{
        colors, gradient, hex, hsl, hsla, hsv, hsva, light_dark, rgb, rgba, web_colors, ColorScheme, Hsla, Hsva, LightDark,
        LightDarkVarExt as _, MixAdjust as _, MixBlendMode, Rgba,
    };

    pub use crate::node::{
        bind_state, border_node, command_property, event_property, event_state, event_state2, event_state3, event_state4, fill_node,
        list_presenter, presenter, presenter_opt, widget_state_get_state, widget_state_is_state, with_context_blend, with_context_local,
        with_context_local_init, with_context_var, with_context_var_init, with_widget_state, with_widget_state_modify,
    };

    pub use crate::{wgt_fn, CommandIconExt as _, WidgetFn};
}

pub mod node;

mod border_props;
mod clip_props;
mod color_props;
mod func;
mod hit_test_props;
mod interactivity_props;
mod layout_props;
mod node_events;
mod panel_props;
mod parallel_prop;
mod visibility_props;
mod wgt;

pub use border_props::*;
pub use clip_props::*;
pub use color_props::*;
pub use func::*;
pub use hit_test_props::*;
pub use interactivity_props::*;
pub use layout_props::*;
pub use node_events::*;
pub use panel_props::*;
pub use parallel_prop::*;
pub use visibility_props::*;
pub use wgt::*;
