//! Widget info, builder and base, UI node and list.
//!
//! The [`Wgt!`](struct@Wgt) widget is a blank widget that entirely shaped by properties.
//!
//! ```
//! use zero_ui::prelude::*;
//! # let _scope = APP.defaults();
//!
//! # let _ =
//! Wgt! {
//!     id = "sun";
//!
//!     widget::background_gradient = {
//!         axis: 0.deg(),
//!         stops: color::gradient::stops![hex!(#ff5226), hex!(#ffc926)],
//!     };
//!     layout::size = 100;
//!     widget::corner_radius = 100;
//!     layout::align = layout::Align::BOTTOM;
//!
//!     #[easing(2.secs())]
//!     layout::y = 100;
//!     when *#widget::is_inited {
//!         layout::y = -30;
//!     }
//! }
//! # ;
//! ```
//!
//! To learn more about the widget macros syntax see [`widget_set!`].
//!
//! To learn more about how widgets are declared see [`widget`].
//!
//! To learn more about how properties are declared see [`property`].
//!
//! # Full API
//!
//! See [`zero_ui_app::widget`] for the full API.

pub use zero_ui_app::widget::base::{HitTestMode, Parallel, WidgetBase, WidgetExt, WidgetImpl, PARALLEL_VAR};

pub use zero_ui_app::widget::{widget_impl, widget_set, StaticWidgetId, WidgetId, WidgetUpdateMode, WIDGET};

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
/// # Examples
///
/// The example declares a new widget type, `ShowProperties!`, that inherits from `Text!` and display what properties
/// are set on itself by accessing the [`WidgetBuilder`] at two points. First call is directly in the `widget_intrinsic` that
/// is called after inherited intrinsics, but before the instance properties are set. Second call is in a build action that is called when
/// the widget starts building, after the instance properties are set.
///
/// [`WidgetBuilder`]: builder::WidgetBuilder
///
/// ```
/// mod widgets {
///     use std::fmt::Write as _;
///     use zero_ui::prelude_wgt::*;
///
///     #[widget($crate::widgets::ShowProperties)]
///     pub struct ShowProperties(zero_ui::text::Text);
///
///     impl ShowProperties {
///         fn widget_intrinsic(&mut self) {
///             let txt = var(Txt::from(""));
///             widget_set! {
///                 self;
///                 txt = txt.clone();
///             }
///
///             let builder = self.widget_builder();
///
///             let mut t = Txt::from("Properties set by default:\n");
///             for p in builder.properties() {
///                 writeln!(&mut t, "• {}", p.args.property().name).unwrap();
///             }
///
///             builder.push_build_action(move |builder| {
///                 writeln!(&mut t, "\nAll properties set:").unwrap();
///                 for p in builder.properties() {
///                     writeln!(&mut t, "• {}", p.args.property().name).unwrap();
///                 }
///                 txt.set(t.clone());            
///             });
///         }
///     }
/// }
///
/// # fn main() {
/// # let _scope = zero_ui::APP.defaults();
/// # let _ =
/// widgets::ShowProperties! {
///     font_size = 20;
/// }
/// # ;
/// # }
/// ```
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
///
/// # Examples
///
/// The example declares a new info state for widgets and a property that sets the new state. The new state is then used
/// in a widget instance.
///
/// ```
/// mod custom {
///     use zero_ui::prelude_wgt::*;
///
///     static STATE_ID: StaticStateId<bool> = StaticStateId::new_unique();
///
///     #[property(CONTEXT)]
///     pub fn flag_state(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
///         let state = state.into_var();
///         match_node(child, move |_, op| match op {
///             UiNodeOp::Init => {
///                 WIDGET.sub_var_info(&state);
///             }
///             UiNodeOp::Info { info } => {
///                 info.set_meta(&STATE_ID, state.get());
///             }
///             _ => {}
///         })
///     }
///
///     pub trait StateExt {
///         fn state(&self) -> Option<bool>;
///     }
///     impl StateExt for WidgetInfo {
///         fn state(&self) -> Option<bool> {
///             self.meta().get_clone(&STATE_ID)
///         }
///     }
/// }
///
/// # fn main() {
/// # use zero_ui::prelude::*;
/// # let _scope = APP.defaults();
/// # let _ =
/// Wgt! {
///     custom::flag_state = true;
///     widget::on_info_init = hn!(|_| {
///         use custom::StateExt as _;
///         let info = WIDGET.info();
///         println!("state: {:?}", info.state());
///     });
/// }
/// # ;
/// # }
/// ```
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
            InspectPropertyPattern, InspectWidgetPattern, InspectorActualVars, InspectorInfo, InstanceItem, WidgetInfoInspectorExt,
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

/// Expands a struct to a widget struct and macro.
///
/// Each widget is a struct and macro pair that constructs a [`WidgetBuilder`] and instantiates a custom widget type. Widgets
/// *inherit* from one other widget and multiple mix-ins, they can have intrinsic nodes and default properties and can build
/// to a custom output type.
///
/// Properties can be declared for the widget using the `#[property(.., widget_impl(Widget))]` directive, existing properties
/// can be implemented for the widget using the [`widget_impl!`] macro.
///
/// # Attribute
///
/// The widget attribute must be placed in a `struct Name(Parent);` declaration, only struct following the exact pattern are allowed,
/// different struct syntaxes will generate a compile error.
///
/// The attribute requires one argument, it must be a macro style `$crate` path to the widget struct, this is used in the generated macro
/// to find the struct during instantiation. The path must be to the *public* path to the struct, that is, the same path that will be used
/// to import the widget. After the required widget path [custom rules](#custom-rules) for the generated macro can be declared.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// /// Minimal widget.
/// #[widget($crate::Foo)]
/// pub struct Foo(WidgetBase);
/// ```
///
/// # Inherit
///
/// The widget struct field must be a path to the parent widget type, all widgets must inherit from another or the
/// [`WidgetBase`], the parent widgets intrinsic properties and nodes are all included in the new widget. The intrinsic
/// properties are included by deref, the new widget will dereference to the parent widget, during widget build auto-deref will select
/// the property methods first, this mechanism even allows for property overrides.
///
/// # Intrinsic
///
/// The widget struct can define a method `widget_intrinsic` that includes custom build actions in the [`WidgetBuilder`], this special
/// method will be called once for its own widget or derived widgets.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// #[widget($crate::Foo)]
/// pub struct Foo(WidgetBase);
///
/// impl Foo {
///     fn widget_intrinsic(&mut self) {
///         self.widget_builder().push_build_action(|b| {
///             // push_intrinsic, capture_var.
///         });
///     }
/// }
/// ```
///
/// The example above demonstrate the function used to [`push_build_action`]. This is the primary mechanism for widgets to define their
/// own behavior that does not depend on properties. Note that the widget inherits from [`WidgetBase`], during [instantiation](#instantiation)
/// of `Foo!` the base `widget_intrinsic` is called first, then the `Foo!` `widget_intrinsic` is called.
///
/// The method does not need to be `pub`, and is not required.
///
/// # Build
///
/// The widget struct can define a method that *builds* the final widget instance.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// #[widget($crate::Foo)]
/// pub struct Foo(WidgetBase);
///
/// impl Foo {
///     /// Custom build.
///     pub fn widget_build(&mut self) -> impl UiNode {
///         zero_ui_app::widget::base::node::build(self.widget_take())
///     }
/// }
/// ```
///
/// The build method must have the same visibility as the widget, and can define its own
/// return type, this is the **widget type**. If the build method is not defined the inherited parent build method is used.
///
/// Unlike the [widget_intrinsic](#intrinsic) method, the widget only has one `widget_build`, if defined it overrides the parent
/// `widget_build`. Most widgets don't define their own build, leaving it to be inherited from [`WidgetBase`]. The base type
/// is an opaque `impl UiNode`, normal widgets must implement [`UiNode`], otherwise they cannot be used as child of other widgets,
/// the widget outer-node also must implement the widget context, to ensure that the widget is correctly placed in the UI tree.
/// The base widget implementation is in [`zero_ui_app::widget::base::node::widget`], you can use it directly, so even if you need
/// to run code on build or define a custom type you don't need to start from scratch.
///
/// # Defaults
///
/// The [`widget_set!`] macro can be used inside `widget_intrinsic` to set properties and when conditions that are applied on the widget if not
/// overridden by derived widgets or the widget instance code. During the call to `widget_intrinsic` the `self.importance()` value is [`Importance::WIDGET`],
/// after it is changed to [`Importance::INSTANCE`], so just by setting properties in `widget_intrinsic` they define the *default* value.
///
/// # Impl Properties
///
/// The [`widget_impl!`] macro can be used inside a `impl WgtIdent { }` block to strongly associate a property with the widget,
/// and the [`property`] attribute has an `impl(WgtIdent)` directive that also strongly associates a property with the widget.
/// These two mechanisms can be used to define properties for the widget, the impl properties are visually different in the widget
/// macro as the have the *immutable method* style, while the other properties have the *mutable method* style.
///
/// # Generated Macro
///
/// The generated widget macro has the same syntax as [`widget_set!`], except that is also starts the widget and builds it at the end,
/// ```
/// use zero_ui::prelude_wgt::*;
///
/// #[widget($crate::Foo)]
/// pub struct Foo(WidgetBase);
///
/// # fn main() {
/// let wgt = Foo! {
///     id = "foo";
/// };
///
/// // is equivalent to:
///
/// let wgt = {
///     let mut wgt = Foo::widget_new();
///     widget_set! {
///         &mut wgt;
///         id = "foo";
///     }
///     wgt.widget_build()
/// };
/// # }
/// ```
///
/// ## Custom Rules
///
/// You can declare custom rules for the widget macro, this can be used to declare **custom shorthand** syntax for the widget.
///
/// The custom rules are declared inside braces after the widget path in the widget attribute. The syntax is similar to `macro_rules!`
/// rules, but the expanded tokens are the direct input of the normal widget expansion.
///
/// ```txt
/// (<rule>) => { <init> };
/// ```
///
/// The `<rule>` is any macro pattern rule, the `<init>` is the normal widget init code that the rule expands to.
///
/// Note that custom rules are not inherited, they apply only to the declaring widget macro, inherited widgets must replicate
/// the rules if desired.
///
/// ### Examples
///
/// Example of a text widget that declares a shorthand syntax to implicitly set the `id` property:
///
/// ```
/// use zero_ui::prelude_wgt::*;
///
/// #[widget($crate::Foo {
///     ($id:tt) => {
///         id = $id;
///     };
/// })]
/// pub struct Foo(WidgetBase);
///
/// # fn main() {
/// let wgt = Foo!("foo");
/// # }
/// ```
///
/// The macro instance above is equivalent to:
///
/// ```
/// # use zero_ui::prelude_wgt::*;
/// # #[widget($crate::Foo)]
/// # pub struct Foo(WidgetBase);
///
/// # fn main() {
/// let wgt = Foo! {
///     id = "foo";
/// };
/// # }
/// ```
///
/// ### Limitations
///
/// The expanded tokens can only be a recursive input for the same widget macro, you can't expand to a different widget.
///
/// Some rules are intercepted by the default widget rules:
///
/// * `$(#[$attr:meta])* $property:ident = $($rest:tt)*`, blocks all custom `$ident = $tt*` patterns.
/// * `$(#[$attr:meta])* when $($rest:tt)*`, blocks all custom `when $tt*` patterns.
///
/// Note that the default single property shorthand syntax is not blocked, for example `Text!(font_size)` will match
/// the custom shorthand rule and try to set the `txt` with the `font_size` variable, without the shorthand it would create a widget without
/// `txt` but with a set `font_size`. So a custom rule `$p:expr` is only recommended for widgets that have a property of central importance.
///
/// # Widget Type
///
/// A public associated function `widget_type` is also generated for the widget, it returns a [`WidgetType`] instance that describes the
/// widget type.
///
/// # Builder
///
/// Two public methods are available to call in a generated widget struct, `builder` and `take_builder` that first mutable borrows the
/// underlying [`WidgetBuilder`] and is usually used in `widget_intrinsic` to insert build actions, the second finalizes the insertion of
/// properties and returns the [`WidgetBuilder`] instance for use finalizing the build, this is usually called in custom `widget_build` implementations.
///
/// See the [`WidgetBuilder`], [`WidgetBuilding`], [`NestGroup`] and [`Importance`] for more details.
///
/// [`WidgetBuilder`]: widget_builder::WidgetBuilder
/// [`WidgetType`]: widget_builder::WidgetType
/// [`WidgetBuilding`]: widget_builder::WidgetBuilding
/// [`NestGroup`]: widget_builder::NestGroup
/// [`Importance`]: widget_builder::Importance
/// [`push_build_action`]: widget_builder::WidgetBuilder::push_build_action
/// [`UiNode`]: widget_node::UiNode
/// [`WidgetBase`]: struct@widget::base::WidgetBase
/// [`Importance::WIDGET`]: widget_builder::Importance::WIDGET
/// [`Importance::INSTANCE`]: widget_builder::Importance::INSTANCE
pub use zero_ui_app::widget::widget;

/// Expands a struct to a widget mix-in.
///
/// Widget mix-ins can be inserted on a widgets inheritance chain, but they cannot be instantiated directly. Unlike
/// the full widgets it defines its parent as a generic type, that must be filled with a real widget when used.
///
/// By convention mix-ins have the suffix `Mix` and the generic parent is named `P`. The `P` must not have any generic bounds
/// in the declaration, the expansion will bound it to [`WidgetImpl`].
///
/// # Examples
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// /// Make a widget capable of receiving keyboard focus.
/// #[widget_mixin]
/// pub struct FocusableMix<P>(P);
/// impl<P: WidgetImpl> FocusableMix<P> {
///     fn widget_intrinsic(&mut self) {
///         widget_set! {
///             self;
///             focusable = true;
///         }
///     }
///     
///     widget_impl! {
///         /// If the widget can receive focus, enabled by default.
///         pub zero_ui::focus::focusable(enabled: impl IntoVar<bool>);
///     }
/// }
///
/// /// Foo is focusable.
/// #[widget($crate::Foo)]
/// pub struct Foo(FocusableMix<WidgetBase>);
/// ```
///
/// The example above declares a mix-in `FocusableMix<P>` and an widget `Foo`, the mix-in is used as a parent of the widget, only
/// the `Foo! { }` widget can be instantiated, and it will have the strongly associated property `focusable`.
///
/// All widget `impl` items can be declared in a mix-in, including the `fn widget_build(&mut self) -> T`, multiple mix-ins can be inherited
/// by nesting the types in a full widget `Foo(AMix<BMix<Base>>)`, mix-ins cannot inherit even from other mix-ins.
pub use zero_ui_app::widget::widget_mixin;

/// Expands a property assign to include an easing animation.
///
/// The attribute generates a [property build action] that applies [`Var::easing`] to the final variable inputs of the property.
///
/// # Arguments
///
/// The attribute takes one required argument and one optional that matches the [`Var::easing`]
/// parameters. The required first arg is the duration, the second arg is an easing function, if not present the [`easing::linear`] is used.
///
/// Some items are auto-imported in each argument scope, the [`TimeUnits`] are imported in the first argument, so you can use syntax
/// like `300.ms()` to declare the duration, all of the [`easing`] functions are imported in the second argument so you can use
/// the function names directly.
///
/// ## Unset
///
/// An alternative argument `unset` can be used instead to remove animations set by the inherited context or styles.
///
/// [`TimeUnits`]: zero_ui::unit::TimeUnits
/// [`easing`]: mod@zero_ui::var::animation::easing
/// [`easing::linear`]: zero_ui::var::animation::easing::linear
/// [property build action]: crate::widget::builder::WidgetBuilder::push_property_build_action
/// [`Var::easing`]: crate::var::Var::easing
///
/// ## When
///
/// The attribute can also be set in `when` assigns, in this case the easing will be applied when the condition is active, so
/// only the transition to the `true` value is animated using the conditional easing.
///
/// Note that you can't `unset` easing in when conditions, but you can set it to `0.ms()`, if all easing set for a property are `0`
/// no easing variable is generated, but in contexts that actually have animation the when value will be set *immediately*,
/// by a zero sized animation.
///
/// # Examples
///
/// The example demonstrates setting and removing easing animations.
///
/// ```
/// # use zero_ui::prelude_wgt::*;
/// # #[widget($crate::Foo)] pub struct Foo(WidgetBase);
/// # #[property(FILL, default(colors::BLACK))]
/// # pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
/// #    child
/// # }
/// # #[property(LAYOUT, default(0))]
/// # pub fn margin(child: impl UiNode, color: impl IntoVar<SideOffsets>) -> impl UiNode {
/// #    child
/// # }
/// # fn main() {
/// Foo! {
///     #[easing(300.ms(), expo)] // set/override the easing.
///     background_color = colors::RED;
///
///     #[easing(unset)] // remove easing set by style or widget defaults.
///     margin = 0;
/// }
/// # ; }
/// ```
///
/// # Limitations
///
/// The attribute only works in properties that only have variable inputs of types that are [`Transitionable`], if the attribute
/// is set in a property that does not match this a cryptic type error occurs, with a mention of `easing_property_input_Transitionable`.
///
/// [`Transitionable`]: crate::var::animation::Transitionable
pub use zero_ui_app::widget::easing;

/// Expands a function to a widget property.
///
/// Property functions take one [`UiNode`] child input and one or more other inputs and produces a [`UiNode`] that implements
/// the property feature. The attribute expansion does not modify the function, it can still be used as a function directly, some
/// properties are implemented by calling other property functions to generate a derived effect. The attribute expansion generates
/// a hidden module of the same name and visibility, the module contains helper code that defines the property for widgets.
///
/// # Attribute
///
/// The property attribute has one required argument and three optional.
///
/// ## Nest Group
///
/// The first argument is the property [`NestGroup`], written as one the `const` group names. The group defines the overall nest position
/// of the property, for example, `LAYOUT` properties always wrap `FILL` properties. This is important as widgets are open and any combination
/// of properties may end-up instantiated in the same widget.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// #[property(LAYOUT)]
/// pub fn align(child: impl UiNode, align: impl IntoVar<Align>) -> impl UiNode {
/// #   child
/// }
/// ```
///
/// The nest group can be tweaked, by adding or subtracting integers, in the example bellow `size` is always inside
/// `max_size`, but both are in the `SIZE` range.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// #[property(SIZE+1)]
/// pub fn size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
/// #   child
/// }
///
/// #[property(SIZE)]
/// pub fn max_size(child: impl UiNode, size: impl IntoVar<Size>) -> impl UiNode {
/// #   child
/// }
/// ```
///
/// ## Default
///
/// The next argument is an optional `default(args..)`. It defines the value to use when the property must be instantiated and no value was provided.
/// The defaults should cause the property to behave as if it is not set, as the default value will be used in widgets that only set the
/// property in `when` blocks.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// #[property(FILL, default(rgba(0, 0, 0, 0)))]
/// pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
/// #   child
/// }
/// ```
///
/// In the example above the `background_color` defines a transparent color as the default, so if the background color is only set in a `when`
/// block if will only be visible when it is active.
///
/// For properties with multiple inputs the default args may be defined in a comma separated list of params, `default(dft0, dft1, ..)`.
///
/// ## Impl For
///
/// The last argument is an optional `impl(<widget-type>)`, it generates `impl <widget-type>` methods for the property strongly associating
/// the property with the widget, users can set this property on the widget or descendants without needing to import the property. Note that
/// this makes the property have priority over all others of the same name, only a derived widget can override with another strongly associated
/// property.
///
/// Note that you can use the [`widget_impl!`] in widget declarations to implement existing properties for a widget.
///
/// ## Capture
///
/// After the nest group and before default the `, capture, ` value indicates that the property is capture-only. This flag
/// changes how the property must be declared, the first argument is a property input and the function can have only one input,
/// no return type is allowed and the function body must be empty, unused input warnings are suppressed by the expanded code.
///
/// Capture-only properties must be captured by a widget and implemented as part of the widget's intrinsics, the reason for
/// a property function is purely to define the property signature and metadata, the capture-only property function can also
/// be used to set a property dynamically, such as in a style widget that is applied on the actual widget that captures the property.
///
/// A documentation sections explaining capture-only properties is generated for the property, it is also tagged differently in the functions list.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// /// Children property, must be captured by panel widgets.
/// #[property(CONTEXT, capture)]
/// pub fn children(children: impl UiNodeList) { }
/// ```
///
/// # Args
///
/// The property function requires at least two args, the first is the *child* node and the other(s) the input values. The
/// number and type of inputs is validated at compile time, the types are limited and are identified and validated by their
/// token name, so you cannot use renamed types.
///
/// ## Child
///
/// The first function argument must be of type `impl UiNode`, it represents the *child* node and the property node must
/// delegate to it so that the UI tree functions correctly. The type must be an `impl` generic, a full path to [`UiNode`]
/// is allowed, but no import renames as the proc-macro attribute can only use tokens to identify the type.
///
/// ## Inputs
///
/// The second arg and optional other args define the property inputs. When a property is assigned in a widget only these inputs
/// are defined by the user, the *child* arg is provided by the widget builder. Property inputs are limited, and must be identifiable
/// by their token name alone. The types are validated at compile time, the `impl` generic types must be declared using `impl` generics,
/// a full path to the generic traits is allowed, but no import renames.
///
/// ### Input Types
///
/// These are the allowed input types:
///
/// #### `impl IntoVar<T>`
///
/// The most common type, accepts any value that can be converted [`IntoVar<T>`], usually the property defines the `T`, but it can be generic.
/// The property node must respond to var updates. The input kind is [`InputKind::Var`]. No auto-default is generated for this type, property
/// implementation should provide a default value that causes the property to behave as if it was not set.
///
/// Only properties with inputs exclusive of this kind can be assigned in `when` blocks. The inputs can also be read in `when` expressions.
///
/// ##### Getter Properties
///
/// Most properties with var inputs are *setters*, that is the inputs configure an effect on the widget. But some properties
/// can be *getters*, detecting widget state and setting it on the *input* variable. These properties are usually named with
/// a prefix that indicates their input is actually for getting state, the prefixes `is_` and `has_` mark a property with
/// a single `bool` input that reads a widget state, the prefix `get_` and `actual_` marks a property that reads a non-boolean state from
/// the widget.
///
/// Getter properties are configured with a default read-write variable, so that they can be used in `when` expressions directly,
/// for example, `when *#is_pressed`, the `is_pressed` property has a `default(var(false))`, so it automatically initializes
/// with a read-write variable that is used in the when condition. The property attribute tries to generate defaults automatically
/// based on the prefix, attempting to use a read-write var with the `T::default()`, this can be overwritten just by setting
/// the default, but it enforces the requirement of a default, it is not possible to declare a getter property without default.
///
/// #### `impl IntoValue<T>`
///
/// The [`IntoValue<T>`] defines an initialization input that does not change for the property node instance, usually the property
/// defines the `T`, but it can be generic. The input kind is [`InputKind::Value`]. No auto-default is generated for this type.
///
/// The input can be read in `when` expressions, but cannot be assigned in `when` blocks.
///
/// #### `impl UiNode`
///
/// This input accepts another [`UiNode`], the implementation must handle it like it handles the *child* node, delegating all methods. The
/// input kind is [`InputKind::UiNode`]. The [`NilUiNode`] is used as the default value if no other is provided.
///
/// The input cannot be read in `when` expressions and cannot be assigned in `when` blocks.
///
/// #### `impl UiNodeList`
///
/// This input accepts another [`UiNodeList`], the implementation must handle it like it handles the *child* node, delegating all methods. The
/// input kind is [`InputKind::UiNodeList`]. An empty list is used as the default value if no other is provided.
///
/// The input cannot be read in `when` expressions and cannot be assigned in `when` blocks.
///
/// #### `impl WidgetHandler<A>`
///
/// This input accepts any [`WidgetHandler<A>`] for the argument type `A`, usually the property defines the `A`, but it can be generic.
/// The input kind is [`InputKind::WidgetHandler`]. A no-op handler is used for the default if no other is provided.
///
/// The input cannot be read in `when` expressions and cannot be assigned in `when` blocks.
///
/// Event handler properties usually have the `on_` name prefix and are generated by the [`event_property!`] macro.
///
/// # Generics
///
/// Apart from the `impl` generics of inputs and *child* a very limited named generic types is supported, only `T: VarValue`, that is
/// an simple ident name constrained by [`VarValue`]. Named generics can only be used as the argument for `impl IntoVar<T>`, `impl IntoValue<T>`
/// and `impl WidgetHandler<T>`.
///
/// # Output
///
/// The property output type must be any type that implements [`UiNode`], usually an opaque type `impl UiNode` is used. The property
/// node can be anything, as long as it delegates to the child node, see [`ui_node`] about implementing a node. Some common
/// property patterns have helpers functions, for example, to setup a context var you can use [`with_context_var`] function.
///
/// # More Details
///
/// See [`property_id!`] and [`property_args!`] for more details about what kind of meta-code is generated for properties.
///
/// [`NestGroup`]: crate::widget::builder::NestGroup
/// [`property_id!`]: crate::widget::builder::property_id
/// [`property_args!`]: crate::widget::builder::property_args
/// [`ui_node`]: macro@ui_node
/// [`with_context_var`]: zero_ui_app::var::with_context_var
/// [`VarValue`]: crate::var::VarValue
/// [`IntoValue<T>`]: crate::var::IntoValue
/// [`IntoVar<T>`]: crate::var::IntoVar
/// [`WidgetHandler<A>`]: crate::handler::WidgetHandler
/// [`UiNode`]: crate::widget::node::UiNode
/// [`UiNodeList`]: crate::widget::node::UiNodeList
/// [`NilUiNode`]: crate::widget::node::NilUiNode
/// [`InputKind::Var`]: crate::widget::builder::InputKind::Var
/// [`InputKind::Value`]: crate::widget::builder::InputKind::Value
/// [`InputKind::UiNode`]: crate::widget::builder::InputKind::UiNode
/// [`InputKind::UiNodeList`]: crate::widget::builder::InputKind::UiNodeList
/// [`InputKind::WidgetHandler`]: crate::widget::builder::InputKind::WidgetHandler
/// [`event_property!`]: crate::event::event_property
pub use zero_ui_app::widget::property;

/// Expands an `impl` block into an [`UiNode`] trait implementation or new node declaration.
///
/// Missing [`UiNode`] methods are generated by this macro. The generation is configured in the macro arguments.
/// The arguments can be a single keyword, a delegate or an entire struct declaration.
///
/// The general idea is you implement only the methods required by your node and configure this macro to generate the methods
/// that are just boilerplate UI tree propagation, and in [new node](#new-node) mode var and event handlers can be inited automatically
/// as well.
///
/// # Delegate to single `impl UiNode`
///
/// If your node contains a single child node, like most property nodes, you can configure the code
/// generator to delegate the method calls for the child node.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// struct MyNode<C> {
///     child: C
/// }
/// #[ui_node(delegate = &mut self.child)]
/// impl<C: UiNode> UiNode for MyNode<C> { }
/// ```
///
/// If the child node is in a field named `child` you can use this shorthand to the same effect:
///
/// ```
/// # fn main() { }
/// # use zero_ui::prelude_wgt::*;
/// # struct MyNode<C> { child: C }
/// #[ui_node(child)]
/// impl<C: UiNode> UiNode for MyNode<C> { }
/// ```
///
/// The generated code simply calls the same [`UiNode`] method in the child.
///
/// # Delegate to a `impl UiNodeList`
///
/// If your node contains multiple children nodes in a type that implements [`UiNodeList`],
/// you can configure the code generator to delegate to the equivalent list methods.
///
/// ```
/// # fn main() { }
/// use zero_ui::prelude_wgt::*;
///
/// struct MyNode<L> {
///     children: L
/// }
/// #[ui_node(delegate_list = &mut self.children)]
/// impl<L: UiNodeList> UiNode for MyNode<L> { }
/// ```
///
/// If the children list is a member named `children` you can use this shorthand to the same effect:
///
/// ```
/// # fn main() { }
/// # use zero_ui::prelude_wgt::*;
/// # struct MyNode<L> { children: L }
/// #[ui_node(children)]
/// impl<L: UiNodeList> UiNode for MyNode<L> { }
/// ```
///
/// The generated code simply calls the equivalent [`UiNodeList`] method in the list.
/// That is the same method name with the `_all` prefix. So `UiNode::init` maps to `UiNodeList::init_all` and so on.
///
/// ## Don't Delegate
///
/// If your node does not have any child nodes you can configure the code generator to generate empty missing methods.
///
/// ```
/// # fn main() { }
/// # use zero_ui::prelude_wgt::*;
/// # struct MyNode { }
/// #[ui_node(none)]
/// impl UiNode for MyNode { }
/// ```
///
/// The generated [`measure`] and [`layout`] code returns the fill size.
///
/// The other generated methods are empty.
///
/// # Validation
///
/// If delegation is configured but no delegation occurs in the manually implemented methods
/// you get the error ``"auto impl delegates call to `{}` but this manual impl does not"``.
///
/// To disable this error use `#[allow_(zero_ui::missing_delegate)]` in the method or in the `impl` block. The
/// error is also not shown if the method body contains a call to the [`todo!()`] macro.
///
/// The [`measure`] method is an exception to this and will not show the error, its ideal implementation
/// is one where the entire sub-tree is skipped from the the computation.
///
/// # Mixing Methods
///
/// You can use the same `impl` block to define [`UiNode`] methods and
/// associated methods by using this attribute in a `impl` block without trait. The [`UiNode`]
/// methods must be tagged with the `#[UiNode]` pseudo-attribute.
///
/// ```
/// # fn main() { }
/// # use zero_ui::prelude_wgt::*;
/// # struct MyNode { child: BoxedUiNode }
/// #[ui_node(child)]
/// impl MyNode {
///     fn do_the_thing(&mut self) {
///         // ..
///     }
///
///     #[UiNode]
///     fn init(&mut self) {
///         self.child.init();
///         self.do_the_thing();
///     }
///
///     #[UiNode]
///     fn update(&mut self, updates: &WidgetUpdates) {
///         self.child.update(updates);
///         self.do_the_thing();
///     }
/// }
/// ```
///
/// The above code expands to two `impl` blocks, one with the associated method and the other with
/// the [`UiNode`] implementation.
///
/// This is particularly useful for nodes that have a large amount of generic constraints, you just type then once.
///
/// # New Node
///
/// In all the usage seen so far you must declare the `struct` type yourself, and the generic bounds to
/// make it work in the `impl` block, and any var or event in it needs to be subscribed manually. You can
/// avoid this extra boilerplate by declaring the node `struct` as an arg for the macro.
///
/// ```
/// # fn main() { }
/// # use zero_ui::prelude_wgt::*;
/// fn my_widget_node(child: impl UiNode, number: impl IntoVar<u32>) -> impl UiNode {
///     #[ui_node(struct MyNode {
///         child: impl UiNode,
///         #[var] number: impl Var<u32>,
///     })]
///     impl UiNode for MyNode {
///         fn update(&mut self, updates: &WidgetUpdates) {
///             self.child.update(updates);
///             if let Some(n) = self.number.get_new() {
///                 println!("new number: {n}");
///             }
///         }
///     }
///     MyNode {
///         child,
///         number: number.into_var(),
///     }
/// }
/// ```
///
/// In the example above the `MyNode` struct is declared with two generic params: `T_child` and `T_var`, the unimplemented
/// node methods are delegated to `child` because of the name, and the `number` var is subscribed automatically because of
/// the `#[var]` pseudo attribute.
///
/// Note that you can also use [`node::match_node`] to declare *anonymous* nodes, most of the properties are implemented using
/// this node instead of the `#[ui_node]` macro.
///
/// ## Generics
///
/// You can declare named generics in the `struct`, those are copied to the implement block, you can also have members with type
/// `impl Trait`, a named generic is generated for these, the generated name is `T_member`. You can use named generics in the `impl`
/// generics the same way as you would in a function.
///
/// ## Impl Block
///
/// The impl block cannot have any generics, they are added automatically, the `UiNode for` part is optional, like in the delegating
/// mode, if you omit the trait you must annotate each node method with the `#[UiNode]` pseudo attribute.
///
/// ## Delegation
///
/// Delegation is limited to members named `child` or `children`, there is no way to declare a custom delegation in *new node*
/// mode. If no specially named member is present the `none` delegation is used.
///
/// ## Subscription
///
/// You can mark members with the `#[var]` or `#[event]` pseudo attributes to generate initialization code that subscribes the var or
/// event to the [`WIDGET`]  context. The init code is placed in a method with signature `fn auto_subs(&mut self)`,
/// if you manually implement the `init` node method you must call `self.auto_subs();` in it, a compile time error is emitted if the call is missing.
///
/// ## Limitations
///
/// The new node type must be private, you cannot set visibility modifiers. The struct cannot have any attribute set on it, but you can
/// have attributes in members, the `#[cfg]` attribute is copied to generated generics. The `impl Trait` auto-generics only works for
/// the entire type of a generic, you cannot declare a type `Vec<impl Debug>` for example.
///
/// The new node syntax is designed to alleviate the boilerplate of declaring nodes that are just implementation detail of properties and widgets.
///
/// [`UiNode`]: crate::widget::node::UiNode
/// [`UiNodeList`]: crate::widget::node::UiNodeList
/// [`measure`]: crate::widget::node::UiNode::measure
/// [`layout`]: crate::widget::node::UiNode::layout
/// [`render`]: crate::widget::node::UiNode::render
/// [`WIDGET`]: crate::update::WIDGET
pub use zero_ui_app::widget::ui_node;
