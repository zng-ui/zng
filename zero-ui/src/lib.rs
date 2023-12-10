//! Zero-Ui is the pure Rust GUI framework with batteries included.
//!
//! It provides all that you need to create a beautiful, fast and responsive multi-platform GUI apps, it includes many features
//! that allow you to get started quickly, without sacrificing customization or performance. With features like gesture events,
//! common widgets, layouts, data binding, async tasks, accessibility and localization
//! you can focus on what makes your app unique, not the boilerplate required to get modern apps up to standard.
//!
//! When you do need to customize, Zero-Ui is rightly flexible, you can create new widgets or customize existing ones, not just
//! new looks but new behavior, at a lower level you can introduce new event types or new event sources, making custom hardware seamless
//! integrate into the framework.
//!
//! # Usage
//!
//! First add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! zero-ui = "0.1"
//! ```
//!
//! Then create your first window:
//!
//! ```rust
//! use zero_ui::prelude::*;
//!
//! fn run() {
//!     APP.defaults().run_window(async {
//!         let size = var_from((800, 600));
//!         Window! {
//!             title = size.map(|s: &Size| formatx!("Button Example - {}", s));
//!             size;
//!             child = Button! {
//!                 on_click = hn!(|_| {
//!                     println!("Button clicked!");
//!                 });
//!                 margin = 10;
//!                 align = Align::CENTER;
//!                 text::font_size = 28;
//!                 child = Text!("Click Me!");
//!             }
//!         }
//!     })
//! }
//! ```

/// Types for general app development.
pub mod prelude {
    pub use crate::APP;

    pub use zero_ui_app::{
        event::{AnyEventArgs as _, Command, CommandHandle, CommandInfoExt as _, CommandNameExt as _, EventArgs as _},
        handler::{app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once, AppHandler, WidgetHandler},
        shortcut::{shortcut, CommandShortcutExt as _, Shortcut, ShortcutFilter, Shortcuts},
        timer::{DeadlineHandle, DeadlineVar, TimerHandle, TimerVar, TIMERS},
        update::{UpdateOp, UPDATES},
        widget::{
            border::{BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle},
            easing,
            info::{InteractionPath, Interactivity, Visibility, WidgetInfo, WidgetPath},
            instance::{
                ui_vec, ArcNode, ArcNodeList, BoxedUiNode, BoxedUiNodeList, EditableUiNodeList, EditableUiNodeListRef, SortingList, UiNode,
                UiNodeList, UiNodeListChain as _, UiNodeListObserver, UiNodeOp, UiNodeVec, ZIndex,
            },
            AnyVarSubscribe as _, StaticWidgetId, VarLayout as _, VarSubscribe as _, WidgetId, WIDGET,
        },
        window::{MonitorId, StaticWindowId, WindowId, WINDOW},
    };

    pub use zero_ui_var::{
        context_var, expr_var, impl_from_and_into_var, merge_var, response_done_var, response_var, state_var, var, var_from, when_var,
        AnyVar as _, ArcVar, BoxedVar, ContextVar, IntoValue, IntoVar, LocalVar, ReadOnlyArcVar, ResponderVar, ResponseVar, Var,
        VarCapabilities, VarHandle, VarHandles, VarValue,
    };

    pub use zero_ui_layout::units::{
        Align, AngleDegree, AngleGradian, AngleRadian, AngleUnits as _, ByteUnits as _, Deadline, Dip, DipBox, DipPoint, DipRect,
        DipSideOffsets, DipSize, DipToPx as _, DipVector, Factor, Factor2d, FactorPercent, FactorSideOffsets, FactorUnits as _,
        Layout1d as _, Layout2d as _, LayoutAxis, Length, LengthUnits as _, Line, LineFromTuplesBuilder as _, Point, Px, PxBox,
        PxConstraints, PxConstraints2d, PxCornerRadius, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip as _, PxTransform,
        PxVector, Rect, RectFromTuplesBuilder as _, ResolutionUnits as _, SideOffsets, Size, TimeUnits as _, Transform, Vector,
    };

    pub use zero_ui_txt::{formatx, ToText as _, Txt};

    pub use zero_ui_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

    pub use crate::task;

    pub use zero_ui_app_context::{app_local, context_local, RunOnDrop};

    pub use zero_ui_state_map::{state_map, OwnedStateMap, StateId, StateMapMut, StateMapRef, StaticStateId};

    pub use zero_ui_color::{
        color_scheme_highlight, color_scheme_map, color_scheme_pair, colors, filters as color_filters, gradient, hex, hsl, hsla, hsv, hsva,
        rgb, rgba, web_colors, ColorPair, ColorScheme, Hsla, Hsva, MixBlendMode, Rgba,
    };

    pub use zero_ui_ext_clipboard::CLIPBOARD;

    pub use zero_ui_ext_config::CONFIG;

    pub use zero_ui_ext_font::{
        font_features, FontSize, FontStretch, FontStyle, FontWeight, Hyphens, Justify, TextTransformFn, WhiteSpace, WordBreak, WordSpacing,
        FONTS,
    };

    pub use zero_ui_ext_fs_watcher::WATCHER;

    pub use zero_ui_ext_image::IMAGES;

    pub use zero_ui_ext_input::{
        focus::{iter::IterFocusableExt as _, DirectionalNav, TabIndex, TabNav, FOCUS},
        gesture::{ClickArgs, CommandShortcutMatchesExt as _},
        keyboard::{HeadlessAppKeyboardExt as _, Key, KeyCode, KeyInputArgs, KeyState},
        mouse::{ButtonState, ClickMode, ClickTrigger, WidgetInfoMouseExt as _},
        pointer_capture::CaptureMode,
    };

    pub use zero_ui_ext_l10n::{lang, Lang, L10N};

    pub use zero_ui_ext_undo::{CommandUndoExt as _, REDO_CMD, UNDO, UNDO_CMD};

    pub use zero_ui_ext_window::{
        AppRunWindowExt as _, AutoSize, HeadlessAppWindowExt as _, RenderMode, StartPosition, WINDOW_Ext as _, WidgetInfoImeArea as _,
        WindowChrome, WindowCloseRequestedArgs, WINDOWS,
    };

    pub use zero_ui_wgt_text::Text;

    pub use crate::text;

    pub use zero_ui_wgt_window::Window;

    pub use zero_ui_wgt_button::Button;
}

/// Prelude for declaring properties and widgets.
pub mod wgt_prelude {
    pub use zero_ui_wgt::prelude::*;

    pub use zero_ui_ext_window::WidgetInfoBuilderImeArea as _;
}

pub use zero_ui_state_map as state_map;

pub use zero_ui_clone_move::{async_clmv, async_clmv_fn, async_clmv_fn_once, clmv};

/// Parallel async tasks and async task runners.
///
/// This module fully re-exports [`zero_ui_task`], it provides common async utilities, all contextualized
/// in the running [`app::LocalContext`]. See the [`zero_ui_task`] crate level documentation for more details.
pub mod task {
    pub use zero_ui_task::*;

    pub use zero_ui_app::widget::UiTaskWidget;
}

/// Color and gradient types, functions and macros, [`Rgba`], [`color_filters`], [`hex!`] and more.
///
/// See [`zero_ui_color`] for the full API.
///
/// [`hex!`]: macro@crate::prelude::hex
/// [`color_filters`]: crate::prelude::color_filters
/// [`Rgba`]: crate::prelude::Rgba
pub mod color {
    pub use zero_ui_color::{
        color_scheme_highlight, color_scheme_map, color_scheme_pair, colors, filters, gradient, hex, hsl, hsla, hsla_sampler, hsv, hsva,
        lerp_space, linear_hsla_sampler, rgb, rgba, rgba_sampler, web_colors, with_lerp_space, ColorPair, ColorScheme, Hsla, Hsva,
        LerpSpace, MixBlendMode, PreMulRgba, RenderColor, RenderMixBlendMode, Rgba, COLOR_SCHEME_VAR,
    };
}

/// Layout service, units and other types.
///
/// See also [`zero_ui_layout`] for the full API.
pub mod layout {
    pub use zero_ui_layout::units::{
        Align, AngleDegree, AngleGradian, AngleRadian, AngleTurn, AngleUnits, BoolVector2D, ByteLength, ByteUnits, CornerRadius2D,
        Deadline, Dip, DipBox, DipCornerRadius, DipPoint, DipRect, DipSideOffsets, DipSize, DipToPx, DipVector, DistanceKey, Factor,
        Factor2d, FactorPercent, FactorSideOffsets, FactorUnits, GridSpacing, Layout1d, Layout2d, LayoutAngle, LayoutAxis, LayoutMask,
        Length, LengthExpr, LengthUnits, Line, LineFromTuplesBuilder, Orientation2D, Point, Ppi, Ppm, Px, PxBox, PxConstraints,
        PxConstraints2d, PxCornerRadius, PxGridSpacing, PxLine, PxPoint, PxRect, PxSideOffsets, PxSize, PxToDip, PxTransform, PxVector,
        Rect, RectFromTuplesBuilder, ResolutionUnits, SideOffsets, SideOffsets2D, Size, TimeUnits, Transform, Vector,
    };

    pub use zero_ui_layout::context::{
        InlineConstraints, InlineConstraintsLayout, InlineConstraintsMeasure, InlineSegment, InlineSegmentPos, LayoutDirection,
        LayoutMetrics, LayoutMetricsSnapshot, LayoutPassId, TextSegmentKind, DIRECTION_VAR, LAYOUT,
    };

    pub use zero_ui_app::widget::info::{WidgetLayout, WidgetMeasure};
}

pub mod render {

}

pub mod var {
    
}

/// App extensions, context, events and commands API.
///
/// See [`zero_ui_app`] and [`zero_ui_app_context`] for the full API.
pub mod app {
    pub use zero_ui_app::{
        AppEventObserver, AppExtended, AppExtension, AppExtensionBoxed, AppExtensionInfo, ControlFlow, ExitRequestedArgs, HeadlessApp,
        EXIT_CMD, EXIT_REQUESTED_EVENT,
    };
    pub use zero_ui_app_context::{
        app_local, context_local, AppId, AppLocal, AppScope, CaptureFilter, ContextLocal, ContextValueSet, FullLocalContext, LocalContext,
        RunOnDrop, StaticAppId,
    };
}

/// Event and command API.
///
/// See [`zero_ui_app::event`] for the full event API.
pub mod event {
    pub use zero_ui_app::event::{
        command, event, event_args, AnyEvent, AnyEventArgs, Command, CommandArgs, CommandHandle, CommandInfoExt, CommandMeta,
        CommandMetaVar, CommandMetaVarId, CommandNameExt, CommandParam, CommandScope, Event, EventArgs, EventHandle, EventHandles,
        EventPropagationHandle, EVENTS,
    };
}

/// App update service and types.
///
/// See [`zero_ui_app::update`] for the full update API.
pub mod update {
    pub use zero_ui_app::update::{
        ContextUpdates, EventUpdate, InfoUpdates, LayoutUpdates, OnUpdateHandle, RenderUpdates, UpdateArgs, UpdateDeliveryList, UpdateOp,
        UpdateSubscribers, UpdatesTraceUiNodeExt, WeakOnUpdateHandle, WidgetUpdates, UPDATES,
    };
}

/// App timers service and types.
///
/// See [`zero_ui_app::timer`] for the full time API. Also see [`task::deadline`] for a timer decoupled from the app loop.
pub mod timer {
    pub use zero_ui_app::timer::{
        DeadlineArgs, DeadlineHandle, DeadlineVar, Timer, TimerArgs, TimerHandle, TimerVar, WeakDeadlineHandle, WeakTimerHandle, TIMERS,
    };
}

/// Widget info, builder and base, UI node and list.
///
/// See [`zero_ui_app::widget`] for the full API.
pub mod widget {
    pub use zero_ui_app::widget::base::{HitTestMode, Parallel, WidgetBase, WidgetExt, WidgetImpl, PARALLEL_VAR};

    pub use zero_ui_app::widget::border::{
        BorderSide, BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle, BORDER, BORDER_ALIGN_VAR,
        BORDER_OVER_VAR, CORNER_RADIUS_FIT_VAR, CORNER_RADIUS_VAR,
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
            iter, HitInfo, HitTestInfo, InlineSegmentInfo, InteractionPath, Interactivity, InteractivityChangedArgs,
            InteractivityFilterArgs, ParallelBuilder, RelativeHitZ, TransformChangedArgs, TreeFilter, Visibility, VisibilityChangedArgs,
            WidgetBorderInfo, WidgetBoundsInfo, WidgetDescendantsRange, WidgetInfo, WidgetInfoBuilder, WidgetInfoChangedArgs,
            WidgetInfoMeta, WidgetInfoTree, WidgetInfoTreeStats, WidgetInlineInfo, WidgetInlineMeasure, WidgetPath,
            INTERACTIVITY_CHANGED_EVENT, TRANSFORM_CHANGED_EVENT, VISIBILITY_CHANGED_EVENT, WIDGET_INFO_CHANGED_EVENT,
        };

        /// Accessibility metadata types.
        pub mod access {
            pub use zero_ui_app::widget::info::access::{AccessBuildArgs, WidgetAccessInfo, WidgetAccessInfoBuilder};
        }
    }

    /// Widget instance types, [`UiNode`], [`UiNodeList`] and others.
    ///
    /// [`UiNode`]: crate::prelude::UiNode
    /// [`UiNodeList`]: crate::prelude::UiNodeList
    pub mod instance {
        pub use zero_ui_app::widget::instance::{
            extend_widget, match_node, match_node_leaf, match_node_list, match_node_typed, match_widget, ui_vec, AdoptiveChildNode,
            AdoptiveNode, ArcNode, ArcNodeList, BoxedUiNode, BoxedUiNodeList, DefaultPanelListData, EditableUiNodeList,
            EditableUiNodeListRef, FillUiNode, MatchNodeChild, MatchNodeChildren, MatchWidgetChild, NilUiNode, OffsetUiListObserver,
            PanelList, PanelListData, PanelListRange, PanelListRangeHandle, SortingList, UiNode, UiNodeList, UiNodeListChain,
            UiNodeListChainImpl, UiNodeListObserver, UiNodeOp, UiNodeOpMethod, UiNodeVec, WeakNode, WeakNodeList, WhenUiNodeBuilder,
            WhenUiNodeListBuilder, ZIndex, SORTING_LIST, Z_INDEX,
        };
    }

    pub use zero_ui_app::widget::{
        easing, property, ui_node, widget, widget_mixin, widget_set, StaticWidgetId, WidgetId, WidgetUpdateMode, WIDGET,
    };
}

/// Event handler API.
///
/// See [`zero_ui_app::handler`] for the full handler API.
pub mod handler {
    pub use zero_ui_app::handler::{
        app_hn, app_hn_once, async_app_hn, async_app_hn_once, async_hn, async_hn_once, hn, hn_once, AppHandler, AppHandlerArgs,
        AppWeakHandle, WidgetHandler,
    };
}

/// Clipboard service, commands and types.
///
/// See also [`zero_ui_ext_clipboard`] for the full clipboard API.
pub mod clipboard {
    pub use zero_ui_ext_clipboard::{ClipboardError, CLIPBOARD, COPY_CMD, CUT_CMD, PASTE_CMD};
}

/// Config service, sources and types.
///
/// See also [`zero_ui_ext_config`] for the full config API.
pub mod config {
    pub use zero_ui_ext_config::{
        AnyConfig, Config, ConfigKey, ConfigMap, ConfigStatus, ConfigValue, ConfigVars, FallbackConfig, FallbackConfigReset, JsonConfig,
        MemoryConfig, RawConfigValue, ReadOnlyConfig, RonConfig, SwapConfig, SwitchConfig, SyncConfig, TomlConfig, YamlConfig, CONFIG,
    };
}

/// Fonts service and text shaping.
///
/// See also [`zero_ui_ext_font`] for the full font and shaping API.
pub mod font {
    pub use zero_ui_ext_font::{
        font_features, unicode_bidi_levels, unicode_bidi_sort, BidiLevel, CaretIndex, ColorGlyph, ColorGlyphs, ColorPalette,
        ColorPaletteType, ColorPalettes, CustomFont, Font, FontChange, FontChangedArgs, FontColorPalette, FontFace, FontFaceList,
        FontFaceMetrics, FontList, FontMetrics, FontName, FontNames, FontSize, FontStretch, FontStyle, FontWeight, Hyphenation,
        HyphenationDataDir, HyphenationDataSource, Hyphens, Justify, LayoutDirections, LetterSpacing, LigatureCaret, LigatureCaretList,
        LineBreak, LineHeight, LineSpacing, OutlineHintingOptions, OutlineSink, ParagraphSpacing, SegmentedText, SegmentedTextIter,
        ShapedColoredGlyphs, ShapedLine, ShapedSegment, ShapedText, TabLength, TextLineThickness, TextOverflowInfo, TextSegment,
        TextSegmentKind, TextShapingArgs, TextTransformFn, UnderlineThickness, WhiteSpace, WordBreak, WordSpacing, FONTS,
        FONT_CHANGED_EVENT,
    };
}

/// File system watcher service and types.
///
/// See also [`zero_ui_ext_fs_watcher`] for the full watcher API.
pub mod fs_watcher {
    pub use zero_ui_ext_fs_watcher::{
        FsChange, FsChangeNote, FsChangeNoteHandle, FsChangesArgs, WatchFile, WatcherHandle, WatcherReadStatus, WatcherSyncStatus,
        WatcherSyncWriteNote, WriteFile, FS_CHANGES_EVENT, WATCHER,
    };
}

/// Images service and types.
///
/// See also [`zero_ui_ext_image`] for the full image API.
pub mod image {
    pub use zero_ui_ext_image::{
        ImageCacheMode, ImageDownscale, ImageHash, ImageHasher, ImageLimits, ImagePpi, ImageRenderArgs, ImageSource, ImageSourceFilter,
        ImageVar, Img, PathFilter, IMAGES, IMAGE_RENDER,
    };
}

/// Accessibility service, events and properties.
///
/// See also [`zero_ui_app::access`] and [`zero_ui_wgt_access`] for the full API.
pub mod access {
    pub use zero_ui_app::access::{
        AccessClickArgs, AccessExpanderArgs, AccessIncrementArgs, AccessInitedArgs, AccessNumberArgs, AccessScrollArgs,
        AccessSelectionArgs, AccessTextArgs, AccessToolTipArgs, ScrollCmd, ACCESS, ACCESS_CLICK_EVENT, ACCESS_EXPANDER_EVENT,
        ACCESS_INCREMENT_EVENT, ACCESS_INITED_EVENT, ACCESS_NUMBER_EVENT, ACCESS_SCROLL_EVENT, ACCESS_SELECTION_EVENT, ACCESS_TEXT_EVENT,
        ACCESS_TOOLTIP_EVENT,
    };
    pub use zero_ui_wgt_access::{
        access_commands, access_role, accessible, active_descendant, auto_complete, checked, col_count, col_index, col_span, controls,
        current, described_by, details, error_message, expanded, flows_to, invalid, item_count, item_index, label, labelled_by,
        labelled_by_child, level, live, modal, multi_selectable, on_access_click, on_access_expander, on_access_increment,
        on_access_number, on_access_scroll, on_access_selection, on_access_text, on_access_tooltip, on_pre_access_click,
        on_pre_access_expander, on_pre_access_increment, on_pre_access_number, on_pre_access_scroll, on_pre_access_selection,
        on_pre_access_text, on_pre_access_tooltip, orientation, owns, placeholder, popup, read_only, required, row_count, row_index,
        row_span, scroll_horizontal, scroll_vertical, selected, sort, value, value_max, value_min, AccessCmdName, AccessRole, AutoComplete,
        CurrentKind, Invalid, LiveIndicator, Orientation, Popup, SortDirection,
    };
}

/// Keyboard service, events and types.
///
/// See also [`zero_ui_ext_input::keyboard`] for the full keyboard API.
pub mod keyboard {
    pub use zero_ui_app::shortcut::ModifiersState;
    pub use zero_ui_ext_input::keyboard::{
        HeadlessAppKeyboardExt, Key, KeyCode, KeyInputArgs, KeyRepeatConfig, KeyState, ModifiersChangedArgs, NativeKeyCode, KEYBOARD,
        KEY_INPUT_EVENT, MODIFIERS_CHANGED_EVENT,
    };
}

/// Mouse service, events and types.
///
/// See also [`zero_ui_ext_input::mouse`] for the full mouse API.
pub mod mouse {
    pub use zero_ui_ext_input::mouse::{
        ButtonRepeatConfig, ButtonState, ClickMode, ClickTrigger, MouseButton, MouseClickArgs, MouseHoverArgs, MouseInputArgs,
        MouseMoveArgs, MousePosition, MouseScrollDelta, MouseWheelArgs, MultiClickConfig, WidgetInfoBuilderMouseExt, WidgetInfoMouseExt,
        MOUSE, MOUSE_CLICK_EVENT, MOUSE_HOVERED_EVENT, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT, MOUSE_WHEEL_EVENT,
    };
}

/// Touch service, events and types.
///
/// See also [`zero_ui_ext_input::touch`] for the full touch API.
pub mod touch {
    pub use zero_ui_ext_input::touch::{
        TouchConfig, TouchForce, TouchId, TouchInputArgs, TouchLongPressArgs, TouchMove, TouchMoveArgs, TouchPhase, TouchPosition,
        TouchTapArgs, TouchTransformArgs, TouchTransformInfo, TouchTransformMode, TouchUpdate, TouchedArgs, TOUCH, TOUCHED_EVENT,
        TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT, TOUCH_TRANSFORM_EVENT,
    };
}

/// Touch service, events and types.
///
/// See also [`zero_ui_ext_input::focus`] for the full focus API.
pub mod focus {
    pub use zero_ui_ext_input::focus::{
        commands, iter, DirectionalNav, FocusChangedArgs, FocusChangedCause, FocusInfo, FocusInfoBuilder, FocusInfoTree, FocusNavAction,
        FocusRequest, FocusScopeOnFocus, FocusTarget, ReturnFocusChangedArgs, TabIndex, TabNav, WidgetFocusInfo, WidgetInfoFocusExt, FOCUS,
        FOCUS_CHANGED_EVENT, RETURN_FOCUS_CHANGED_EVENT,
    };
}

/// Pointer capture service, events and types.
///
/// See also [`zero_ui_ext_input::pointer_capture`] for the full pointer capture API.
pub mod pointer_capture {
    pub use zero_ui_ext_input::pointer_capture::{CaptureInfo, CaptureMode, PointerCaptureArgs, POINTER_CAPTURE, POINTER_CAPTURE_EVENT};
}

/// Gesture service, events, shortcuts and other types.
///
/// See also [`zero_ui_ext_input::gesture`] for the full gesture API and [`zero_ui_app::shortcut`] for the shortcut API.
///
/// [`zero_ui_app::shortcut`]: mod@zero_ui_app::shortcut
pub mod gesture {
    pub use zero_ui_ext_input::gesture::{
        ClickArgs, ClickArgsSource, CommandShortcutMatchesExt, HeadlessAppGestureExt, ShortcutActions, ShortcutArgs, ShortcutClick,
        ShortcutsHandle, WeakShortcutsHandle, CLICK_EVENT, GESTURES, SHORTCUT_EVENT,
    };

    pub use zero_ui_app::shortcut::{
        shortcut, CommandShortcutExt, GestureKey, KeyChord, KeyGesture, ModifierGesture, Shortcut, ShortcutFilter, Shortcuts,
    };
}

/// Localization service, sources and types.
///
/// See also [`zero_ui_ext_l10n`] for the full localization API.
pub mod l10n {
    pub use zero_ui_ext_l10n::{
        IntoL10nVar, L10nArgument, L10nDir, L10nMessageBuilder, L10nSource, Lang, LangMap, LangResource, LangResourceStatus, LangResources,
        Langs, NilL10nSource, SwapL10nSource, L10N, LANG_VAR,
    };
}

/// Undo service, commands and types.
///
/// See also [`zero_ui_ext_undo`] for the full undo API.
pub mod undo {
    pub use zero_ui_ext_undo::{
        CommandUndoExt, RedoAction, UndoAction, UndoActionMergeArgs, UndoFullOp, UndoInfo, UndoOp, UndoSelect, UndoSelectInterval,
        UndoSelectLtEq, UndoSelector, UndoStackInfo, UndoTransaction, UndoVarModifyTag, WidgetInfoUndoExt, WidgetUndoScope,
        CLEAR_HISTORY_CMD, REDO_CMD, UNDO, UNDO_CMD, UNDO_INTERVAL_VAR, UNDO_LIMIT_VAR,
    };
}

/// Window service, widget, events, commands and types.
///
/// See also [`zero_ui_ext_window`], [`zero_ui_app::window`] and [`zero_ui_wgt_window`] for the full window API.
pub mod window {
    pub use zero_ui_app::window::{MonitorId, StaticMonitorId, StaticWindowId, WindowId, WindowMode, WINDOW};

    pub use zero_ui_ext_window::{
        AppRunWindowExt, AutoSize, CloseWindowResult, CursorImage, FrameCaptureMode, FrameImageReadyArgs, HeadlessAppWindowExt,
        HeadlessMonitor, ImeArgs, MonitorInfo, MonitorQuery, MonitorsChangedArgs, ParallelWin, RenderMode, RendererDebug, StartPosition,
        WINDOW_Ext, WidgetInfoBuilderImeArea, WidgetInfoImeArea, WindowChangedArgs, WindowChrome, WindowCloseArgs,
        WindowCloseRequestedArgs, WindowIcon, WindowLoadingHandle, WindowOpenArgs, WindowRoot, WindowRootExtenderArgs, WindowState,
        WindowStateAllowed, WindowVars, FRAME_IMAGE_READY_EVENT, IME_EVENT, MONITORS, MONITORS_CHANGED_EVENT, WINDOWS,
        WINDOW_CHANGED_EVENT, WINDOW_CLOSE_EVENT, WINDOW_CLOSE_REQUESTED_EVENT, WINDOW_LOAD_EVENT, WINDOW_OPEN_EVENT,
    };

    /// Window commands.
    pub mod commands {
        pub use zero_ui_ext_window::commands::*;
        pub use zero_ui_wgt_window::commands::*;
    }

    pub use zero_ui_wgt_window::{SaveState, Window};
}

/// Text widget, properties and types.
///
/// See [`zero_ui_wgt_text`] for the full widget API.
pub mod text {
    pub use zero_ui_txt::*;

    pub use zero_ui_wgt_text::{
        accepts_enter, accepts_tab, auto_selection, caret_color, caret_touch_shape, change_stop_delay, commands, direction, font_aa,
        font_annotation, font_caps, font_char_variant, font_cn_variant, font_color, font_common_lig, font_contextual_alt,
        font_discretionary_lig, font_ea_width, font_family, font_features, font_historical_forms, font_historical_lig, font_jp_variant,
        font_kerning, font_num_fraction, font_num_spacing, font_numeric, font_ornaments, font_palette, font_palette_colors, font_position,
        font_size, font_stretch, font_style, font_style_set, font_stylistic, font_swash, font_synthesis, font_variations, font_weight,
        get_caret_index, get_caret_status, get_chars_count, get_lines_len, get_lines_wrap_count, get_overflow, hyphen_char, hyphens,
        ime_underline, is_line_overflown, is_overflown, is_parse_pending, justify, lang, letter_spacing, line_break, line_height,
        line_spacing, max_chars_count, obscure_txt, obscuring_char, on_change_stop, overline, overline_color, paragraph_spacing,
        selection_color, selection_toolbar, selection_toolbar_anchor, selection_toolbar_fn, strikethrough, strikethrough_color, tab_length,
        txt_align, txt_editable, txt_highlight, txt_overflow, txt_overflow_align, underline, white_space, word_break, word_spacing,
        AutoSelection, CaretShape, CaretStatus, ChangeStopArgs, ChangeStopCause, Em, FontFeaturesMix, FontMix, LangMix, LinesWrapCount,
        ParagraphMix, SelectionToolbarArgs, Strong, Text, TextAlignMix, TextDecorationMix, TextEditMix, TextFillMix, TextOverflow,
        TextSpacingMix, TextTransformMix, TextWrapMix, TxtParseValue, UnderlinePosition, UnderlineSkip,
    };
}

/// Button widget, style and properties.
///
/// See [`zero_ui_wgt_button`] for the full widget API.
pub mod button {
    pub use zero_ui_wgt_button::{base_colors, extend_style, replace_style, Button, DefaultStyle};
}

/// Start and manage an app process.
///
/// # View Process
///
/// A view-process must be initialized before starting an app. Panics on `run` if there is
/// no view-process, also panics if the current process is already executing as a view-process.
pub struct APP;
impl std::ops::Deref for APP {
    type Target = zero_ui_app::APP;

    fn deref(&self) -> &Self::Target {
        &zero_ui_app::APP
    }
}

mod defaults {
    use zero_ui_app::{AppExtended, AppExtension};
    use zero_ui_ext_clipboard::ClipboardManager;
    use zero_ui_ext_config::ConfigManager;
    use zero_ui_ext_font::FontManager;
    use zero_ui_ext_fs_watcher::FsWatcherManager;
    use zero_ui_ext_image::ImageManager;
    use zero_ui_ext_input::{
        focus::FocusManager, gesture::GestureManager, keyboard::KeyboardManager, mouse::MouseManager,
        pointer_capture::PointerCaptureManager, touch::TouchManager,
    };
    use zero_ui_ext_l10n::L10nManager;
    use zero_ui_ext_undo::UndoManager;
    use zero_ui_ext_window::WindowManager;

    impl super::APP {
        /// App with default extensions.
        ///     
        /// # Extensions
        ///
        /// Extensions included.
        ///
        /// * [`FsWatcherManager`]
        /// * [`ConfigManager`]
        /// * [`L10nManager`]
        /// * [`PointerCaptureManager`]
        /// * [`MouseManager`]
        /// * [`TouchManager`]
        /// * [`KeyboardManager`]
        /// * [`GestureManager`]
        /// * [`WindowManager`]
        /// * [`FontManager`]
        /// * [`FocusManager`]
        /// * [`ImageManager`]
        /// * [`ClipboardManager`]
        /// * [`UndoManager`]
        /// * [`MaterialFonts`] if `cfg(feature = "material_icons")`.
        ///
        /// [`MaterialFonts`]: zero_ui_wgt_material_icons::MaterialFonts
        pub fn defaults(&self) -> AppExtended<impl AppExtension> {
            let r = self
                .minimal()
                .extend(FsWatcherManager::default())
                .extend(ConfigManager::default())
                .extend(L10nManager::default())
                .extend(PointerCaptureManager::default())
                .extend(MouseManager::default())
                .extend(TouchManager::default())
                .extend(KeyboardManager::default())
                .extend(GestureManager::default())
                .extend(WindowManager::default())
                .extend(FontManager::default())
                .extend(FocusManager::default())
                .extend(ImageManager::default())
                .extend(ClipboardManager::default())
                .extend(UndoManager::default());

            #[cfg(feature = "material_icons")]
            let r = r.extend(zero_ui_wgt_material_icons::MaterialFonts);

            r.extend(DefaultsInit {})
        }
    }

    struct DefaultsInit {}
    impl AppExtension for DefaultsInit {
        fn init(&mut self) {
            use zero_ui_app::widget::instance::ui_vec;
            use zero_ui_ext_clipboard::COPY_CMD;
            use zero_ui_ext_window::WINDOWS;
            use zero_ui_wgt_text::icon::CommandIconExt as _;
            use zero_ui_wgt_text::{commands::SELECT_ALL_CMD, icon::Icon, SelectionToolbarArgs};
            use zero_ui_wgt_view::wgt_fn;

            WINDOWS.register_root_extender(|a| {
                let child = a.root;

                // `zero_ui_wgt_menu` depends on `zero_ui_wgt_text` so we can't set this in the text crate.
                zero_ui_wgt_text::selection_toolbar_fn(
                    child,
                    wgt_fn!(|args: SelectionToolbarArgs| {
                        use zero_ui_wgt_menu as menu;
                        menu::context::ContextMenu! {
                            style_fn = menu::context::TouchStyle!();
                            children = ui_vec![
                                menu::TouchCmdButton!(COPY_CMD.scoped(args.anchor_id)),
                                menu::TouchCmdButton!(SELECT_ALL_CMD.scoped(args.anchor_id)),
                            ];
                        }
                    }),
                )
            });

            #[cfg(feature = "material_icons")]
            {
                use zero_ui_ext_clipboard::*;
                use zero_ui_ext_undo::*;
                use zero_ui_wgt_material_icons::outlined as icons;

                CUT_CMD.init_icon(wgt_fn!(|_| Icon!(icons::CUT)));
                COPY_CMD.init_icon(wgt_fn!(|_| Icon!(icons::COPY)));
                PASTE_CMD.init_icon(wgt_fn!(|_| Icon!(icons::PASTE)));

                UNDO_CMD.init_icon(wgt_fn!(|_| Icon!(icons::UNDO)));
                REDO_CMD.init_icon(wgt_fn!(|_| Icon!(icons::REDO)));

                // !!: TODO review "static \w+_CMD" and add more icons when the icon example is running again.
            }
        }
    }
}
