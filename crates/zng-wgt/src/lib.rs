#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
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
        DInstant, Deadline, INSTANT,
        event::{
            Command, CommandHandle, CommandInfoExt as _, CommandNameExt as _, Event, EventArgs as _, EventPropagationHandle, command,
            event, event_args,
        },
        handler::{Handler, HandlerExt as _, async_hn, async_hn_once, hn, hn_once},
        render::{FrameBuilder, FrameUpdate, FrameValue, FrameValueKey, FrameValueUpdate, SpatialFrameId, TransformStyle},
        shortcut::{CommandShortcutExt as _, Shortcut, ShortcutFilter, Shortcuts, shortcut},
        timer::{DeadlineHandle, DeadlineVar, TIMERS, TimerHandle, TimerVar},
        update::{UPDATES, UpdateDeliveryList, UpdateOp, WidgetUpdates},
        widget::{
            AnyVarSubscribe as _, VarLayout as _, VarSubscribe as _, WIDGET, WidgetId, WidgetUpdateMode,
            base::{WidgetBase, WidgetImpl},
            border::{BORDER, BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle},
            builder::{NestGroup, WidgetBuilder, WidgetBuilding, property_id},
            easing,
            info::{
                InteractionPath, Interactivity, Visibility, WidgetBorderInfo, WidgetBoundsInfo, WidgetInfo, WidgetInfoBuilder,
                WidgetLayout, WidgetMeasure, WidgetPath,
            },
            node::{
                ArcNode, ChainList, EditableUiVec, EditableUiVecRef, FillUiNode, IntoUiNode, PanelList, PanelListData as _, SORTING_LIST,
                SortingList, UiNode, UiNodeImpl, UiNodeListObserver, UiNodeOp, UiVec, ZIndex, match_node, match_node_leaf, match_widget,
                ui_vec,
            },
            property, widget, widget_impl, widget_mixin, widget_set,
        },
        window::{MonitorId, WINDOW, WindowId},
    };

    pub use zng_var::{
        ContextVar, IntoValue, IntoVar, ObservableVec, ResponderVar, ResponseVar, Var, VarCapability, VarHandle, VarHandles, VarUpdateId,
        VarValue, WeakVar, const_var, context_var, expr_var, flat_expr_var, impl_from_and_into_var, merge_var, response_done_var,
        response_var, var, var_default, var_from, var_state, when_var,
    };

    pub use zng_layout::{
        context::{DIRECTION_VAR, LAYOUT, LayoutDirection, LayoutMetrics},
        unit::{
            Align, AngleDegree, AngleGradian, AngleRadian, AngleUnits as _, ByteUnits as _, Dip, DipBox, DipPoint, DipRect, DipSideOffsets,
            DipSize, DipToPx as _, DipVector, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits as _, Layout1d as _,
            Layout2d as _, LayoutAxis, Length, LengthUnits as _, Line, LineFromTuplesBuilder as _, Point, Px, PxBox, PxConstraints,
            PxConstraints2d, PxCornerRadius, PxDensity, PxDensity2d, PxDensityUnits as _, PxDensityUnits, PxLine, PxPoint, PxRect,
            PxSideOffsets, PxSize, PxToDip as _, PxTransform, PxVector, Rect, RectFromTuplesBuilder as _, SideOffsets, Size,
            TimeUnits as _, Transform, Vector,
        },
    };

    pub use zng_txt::{ToTxt, Txt, formatx};

    pub use zng_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use zng_task as task;

    pub use zng_app_context::{CaptureFilter, ContextLocal, ContextValueSet, LocalContext, RunOnDrop, app_local, context_local};

    pub use zng_state_map::{OwnedStateMap, StateId, StateMapMut, StateMapRef, state_map, static_id};

    pub use zng_unique_id::{IdEntry, IdMap, IdSet};

    pub use zng_color::{
        ColorScheme, Hsla, Hsva, LightDark, LightDarkVarExt as _, MixAdjust as _, MixBlendMode, Rgba, colors, gradient, hex, hsl, hsla,
        hsv, hsva, light_dark, rgb, rgba, web_colors,
    };

    pub use crate::node::{
        VarPresent as _, VarPresentData as _, VarPresentList as _, VarPresentListFromIter as _, VarPresentOpt as _, bind_state,
        border_node, fill_node, list_presenter, list_presenter_from_iter, presenter, presenter_opt, widget_state_get_state,
        widget_state_is_state, with_context_blend, with_context_local, with_context_local_init, with_context_var, with_context_var_init,
        with_widget_state, with_widget_state_modify,
    };

    pub use crate::{CommandIconExt as _, WidgetFn, wgt_fn};
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
