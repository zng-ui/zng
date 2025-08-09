//! Widget info, builder and base, UI node and list.
//!
//! The [`Wgt!`](struct@Wgt) widget is a blank widget that entirely shaped by properties.
//!
//! ```
//! use zng::prelude::*;
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
//! See [`zng_app::widget`], [`zng_wgt`], [`zng_wgt_fill`], [`zng_wgt_image::border`], [`zng_wgt_image::fill`] for the full API.

pub use zng_app::widget::base::{HitTestMode, NonWidgetBase, PARALLEL_VAR, Parallel, WidgetBase, WidgetExt, WidgetImpl};

pub use zng_app::widget::{WIDGET, WidgetId, WidgetUpdateMode, widget_impl, widget_set};

pub use zng_app::widget::border::{
    BORDER, BorderSide, BorderSides, BorderStyle, CornerRadius, CornerRadiusFit, LineOrientation, LineStyle,
};
pub use zng_app::widget::info::Visibility;
pub use zng_app::widget::node::ZIndex;

pub use zng_app::render::RepeatMode;

pub use zng_wgt::{
    EDITORS, EditorRequestArgs, IS_MOBILE_VAR, OnNodeOpArgs, WeakWidgetFn, Wgt, WidgetFn, auto_hide, border, border_align, border_over,
    clip_to_bounds, corner_radius, corner_radius_fit, enabled, hit_test_mode, inline, interactive, is_collapsed, is_disabled, is_enabled,
    is_hidden, is_hit_testable, is_inited, is_mobile, is_visible, modal, modal_included, modal_includes, on_block, on_blocked_changed,
    on_collapse, on_deinit, on_disable, on_enable, on_enabled_changed, on_hide, on_info_init, on_init, on_interactivity_changed, on_move,
    on_node_op, on_pre_block, on_pre_blocked_changed, on_pre_collapse, on_pre_deinit, on_pre_disable, on_pre_enable,
    on_pre_enabled_changed, on_pre_hide, on_pre_init, on_pre_interactivity_changed, on_pre_move, on_pre_node_op, on_pre_show,
    on_pre_transform_changed, on_pre_unblock, on_pre_update, on_pre_vis_disable, on_pre_vis_enable, on_pre_vis_enabled_changed,
    on_pre_visibility_changed, on_show, on_transform_changed, on_unblock, on_update, on_vis_disable, on_vis_enable, on_vis_enabled_changed,
    on_visibility_changed, parallel, visibility, wgt_fn, z_index,
};

#[cfg(feature = "image")]
pub use zng_wgt_image::{
    border::{BorderRepeats, border_img, border_img_fill, border_img_repeat},
    fill::{
        background_img, background_img_align, background_img_crop, background_img_fit, background_img_offset, background_img_opacity,
        background_img_repeat, background_img_repeat_spacing, foreground_img, foreground_img_align, foreground_img_crop,
        foreground_img_fit, foreground_img_offset, foreground_img_opacity, foreground_img_repeat, foreground_img_repeat_spacing,
    },
};

pub use zng_wgt_fill::{
    background, background_color, background_conic, background_fn, background_gradient, background_radial, foreground, foreground_color,
    foreground_conic, foreground_fn, foreground_gradient, foreground_highlight, foreground_radial,
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
///     use zng::prelude_wgt::*;
///
///     #[widget($crate::widgets::ShowProperties)]
///     pub struct ShowProperties(zng::text::Text);
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
/// # let _scope = zng::APP.defaults();
/// # let _ =
/// widgets::ShowProperties! {
///     font_size = 20;
/// }
/// # ;
/// # }
/// ```
///
/// # Full API
///
/// See [`zng_app::widget::builder`] for the full API.
pub mod builder {
    pub use zng_app::widget::builder::{
        AnyWhenArcWidgetHandlerBuilder, ArcWidgetHandler, BuilderProperty, BuilderPropertyMut, BuilderPropertyRef, Importance, InputKind,
        NestGroup, NestPosition, PropertyArgs, PropertyBuildAction, PropertyBuildActionArgs, PropertyBuildActions,
        PropertyBuildActionsWhenData, PropertyId, PropertyInfo, PropertyInput, PropertyInputTypes, PropertyNewArgs, SourceLocation,
        WhenBuildAction, WhenInfo, WhenInput, WhenInputMember, WhenInputVar, WidgetBuilder, WidgetBuilderProperties, WidgetBuilding,
        WidgetType, property_args, property_id, property_info, property_input_types, source_location, widget_type,
    };
}

/// Widget info tree and info builder.
///
/// # Examples
///
/// The example declares a new info state for widgets and a property that sets the new state. The new state is then used
/// in a widget.
///
/// ```
/// mod custom {
///     use zng::prelude_wgt::*;
///
///     static_id! {
///         static ref STATE_ID: StateId<bool>;
///     }
///
///     #[property(CONTEXT)]
///     pub fn flag_state(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
///         let state = state.into_var();
///         match_node(child, move |_, op| match op {
///             UiNodeOp::Init => {
///                 WIDGET.sub_var_info(&state);
///             }
///             UiNodeOp::Info { info } => {
///                 info.set_meta(*STATE_ID, state.get());
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
///             self.meta().get_clone(*STATE_ID)
///         }
///     }
/// }
///
/// # fn main() {
/// # use zng::prelude::*;
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
    pub use zng_app::widget::info::{
        HitInfo, HitTestInfo, INTERACTIVITY_CHANGED_EVENT, InlineSegmentInfo, InteractionPath, Interactivity, InteractivityChangedArgs,
        InteractivityFilterArgs, ParallelBuilder, RelativeHitZ, TRANSFORM_CHANGED_EVENT, TransformChangedArgs, TreeFilter,
        VISIBILITY_CHANGED_EVENT, VisibilityChangedArgs, WIDGET_INFO_CHANGED_EVENT, WidgetBorderInfo, WidgetBoundsInfo,
        WidgetDescendantsRange, WidgetInfo, WidgetInfoBuilder, WidgetInfoChangedArgs, WidgetInfoMeta, WidgetInfoTree, WidgetInfoTreeStats,
        WidgetInlineInfo, WidgetInlineMeasure, WidgetPath, iter,
    };

    /// Accessibility metadata types.
    pub mod access {
        pub use zng_app::widget::info::access::{AccessBuildArgs, WidgetAccessInfo, WidgetAccessInfoBuilder};
    }

    /// Helper types for inspecting an UI tree.
    ///
    /// See also [`zng::window::inspector`] for higher level inspectors.
    pub mod inspector {
        pub use zng_app::widget::inspector::{
            InspectPropertyPattern, InspectWidgetPattern, InspectorActualVars, InspectorInfo, InstanceItem, WidgetInfoInspectorExt,
        };
    }
}

/// Widget node types, [`UiNode`], [`UiVec`] and others.
///
/// [`UiNode`]: crate::prelude::UiNode
/// [`UiVec`]: crate::prelude::UiVec
pub mod node {
    pub use zng_app::widget::node::{
        AdoptiveChildNode, AdoptiveNode, ArcNode, ChainList, DefaultPanelListData, EditableUiVec, EditableUiVecRef, FillUiNode, IntoUiNode,
        MatchNodeChild, MatchWidgetChild, OffsetUiListObserver, PanelList, PanelListData, PanelListRange, SORTING_LIST, SortingList,
        UiNode, UiNodeImpl, UiNodeListObserver, UiNodeOp, UiNodeOpMethod, UiVec, WeakNode, WhenUiNodeBuilder, WidgetUiNode,
        WidgetUiNodeImpl, Z_INDEX, extend_widget, match_node, match_node_leaf, match_widget, ui_vec,
    };

    pub use zng_wgt::node::{
        bind_state, bind_state_init, border_node, event_state, event_state2, event_state3, event_state4, fill_node, interactive_node,
        list_presenter, list_presenter_from_iter, presenter, presenter_opt, widget_state_get_state, widget_state_is_state,
        with_context_blend, with_context_local, with_context_local_init, with_context_var, with_context_var_init, with_index_len_node,
        with_index_node, with_rev_index_node, with_widget_state, with_widget_state_modify,
    };
}

/// Expands a struct to a widget struct and macro.
///
/// Each widget is a struct and macro pair of the same name that builds a custom widget using [`WidgetBuilder`]. Widgets
/// *inherit* from one other widget and can also inherit multiple mix-ins. Widgets can have intrinsic nodes, default properties
/// and can build to a custom output type.
///
/// Properties can be strongly associated with the widget using the `#[property(.., widget_impl(Widget))]` directive, existing properties
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
/// use zng::prelude_wgt::*;
///
/// /// Minimal widget.
/// #[widget($crate::Foo)]
/// pub struct Foo(WidgetBase);
/// ```
///
/// # Inherit
///
/// The widget struct field must be the parent widget type. All widgets inherit from another or the
/// [`WidgetBase`], the parent widgets intrinsic properties and nodes are all included in the new widget. The intrinsic
/// properties are included by deref, the new widget will dereference to the parent widget, during widget build auto-deref will select
/// the property methods first, this mechanism even allows for property overrides.
///
/// # Intrinsic
///
/// The widget struct can define a method `widget_intrinsic` that includes custom build actions in the [`WidgetBuilder`], this special
/// method will be called once for the widget. The same method is also called for the inherited widgets.
///
/// ```
/// # fn main() { }
/// use zng::prelude_wgt::*;
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
/// The example above demonstrates the intrinsic method used to [`push_build_action`]. This is the primary mechanism for widgets to define their
/// own behavior that does not depend on properties. Note that the widget inherits from [`WidgetBase`], during instantiation
/// of `Foo!` the base `widget_intrinsic` is called first, then the `Foo!` `widget_intrinsic` is called.
///
/// The method does not need to be `pub`, and it is not required.
///
/// # Build
///
/// The widget struct can define a method that builds the final widget instance.
///
/// ```
/// # fn main() { }
/// use zng::prelude_wgt::*;
///
/// #[widget($crate::Foo)]
/// pub struct Foo(WidgetBase);
///
/// impl Foo {
///     /// Custom build.
///     pub fn widget_build(&mut self) -> UiNode {
///         println!("on build!");
///         WidgetBase::widget_build(self)
///     }
/// }
/// ```
///
/// The build method must have the same visibility as the widget, and can define its own
/// return type, this is the widget instance type. If the build method is not defined the inherited parent build method is used.
///
/// Unlike the [intrinsic](#intrinsic) method, the widget only has one `widget_build`, if defined it overrides the parent
/// `widget_build`. Most widgets don't define their own build, leaving it to be inherited from [`WidgetBase`]. The base instance type
/// is an opaque `UiNode`.
///
/// Normal widgets instance types must implement [`IntoUiNode`], otherwise they cannot be used as child of other widgets.
/// The widget outer-node also must implement the widget context, to ensure that the widget is correctly placed in the UI tree.
/// Note that you can still use the parent type build implementation, so even if you need
/// to run code on build or define a custom type you don't need to deref to the parent type to build.
///
/// # Defaults
///
/// The [`widget_set!`] macro can be used inside `widget_intrinsic` to set properties and when conditions that are applied on the widget by default,
/// if not overridden by derived widgets or the widget instance. During the call to `widget_intrinsic` the `self.importance()` value is
/// [`Importance::WIDGET`], after it is changed to [`Importance::INSTANCE`], so just by setting properties in `widget_intrinsic` they
/// will have less importance allowing for the override mechanism to replace them.
///
/// # Impl Properties
///
/// The [`widget_impl!`] macro can be used inside a `impl WgtIdent { }` block to strongly associate a property with the widget,
/// and the [`property`] attribute has a `widget_impl(WgtIdent)` directive that also strongly associates a property with the widget.
///
/// These two mechanisms can be used to define properties for the widget, the impl properties don't need to be imported and are
/// always selected over other properties of the same name. They also appear in the widget documentation and can have a distinct
/// visual in IDEs as they are represented by immutable methods while standalone properties are represented by mutable trait methods.
///
/// As a general rule only properties that are captured by the widget, or only work with the widget, or have an special meaning in the widget
/// are implemented like this, standalone properties that can be used in any widget are not implemented.
///
/// # Generated Macro
///
/// The generated widget macro has the same syntax as [`widget_set!`], except that is also starts the widget and builds it at the end.
///
/// This widget macro call:
///
/// ```
/// # use zng::prelude_wgt::*;
/// # #[widget($crate::Foo)]
/// # pub struct Foo(WidgetBase);
/// #
/// # fn main() {
/// let wgt = Foo! {
///     id = "foo";
/// };
/// # }
/// ```
///
/// Expands to this:
///
/// ```
/// # use zng::prelude_wgt::*;
/// # #[widget($crate::Foo)]
/// # pub struct Foo(WidgetBase);
/// #
/// # fn main() {
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
/// #### Custom Rules
///
/// You can declare custom rules for the widget macro, this can be used to declare custom shorthand syntax for the widget.
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
/// Example of a widget that declares a shorthand syntax to implicitly set the `id` property:
///
/// ```
/// use zng::prelude_wgt::*;
///
/// #[widget($crate::Foo {
///     ($id:expr) => {
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
/// # use zng::prelude_wgt::*;
/// # #[widget($crate::Foo)]
/// # pub struct Foo(WidgetBase);
/// #
/// # fn main() {
/// let wgt = Foo! {
///     id = "foo";
/// };
/// # }
/// ```
///
/// #### Limitations
///
/// The expanded tokens can only be a recursive input for the same widget macro, you can't expand to a different widget.
///
/// Some rules are intercepted by the default widget rules:
///
/// * `$(#[$attr:meta])* $($property:ident)::+ = $($rest:tt)*`, blocks all custom `$ident = $tt*` patterns.
/// * `$(#[$attr:meta])* when $($rest:tt)*`, blocks all custom `when $tt*` patterns.
///
/// Note that the default single property shorthand syntax is not blocked, for example `Text!(font_size)` will match
/// the custom shorthand rule and try to set the `txt` with the `font_size` variable, without the shorthand it would create a widget without
/// `txt` that sets `font_size`. So a custom rule `$p:expr` is only recommended for widgets that have a property of central importance.
///
/// # Widget Type
///
/// A public associated function `widget_type` is also generated for the widget, it returns a [`WidgetType`] instance that describes the
/// widget type. Note that this is not the widget instance type, only the struct and macro type. If compiled with the `"inspector"` feature
/// the type is also available in the widget info.
///
/// # See Also
///
/// See the [`WidgetBase`], [`WidgetBuilder`], [`WidgetBuilding`], [`NestGroup`] and [`Importance`] for more details.
///
/// [`WidgetBuilder`]: builder::WidgetBuilder
/// [`WidgetType`]: builder::WidgetType
/// [`WidgetBuilding`]: builder::WidgetBuilding
/// [`NestGroup`]: builder::NestGroup
/// [`Importance`]: builder::Importance
/// [`push_build_action`]: builder::WidgetBuilder::push_build_action
/// [`UiNode`]: node::UiNode
/// [`IntoUiNode`]: node::IntoUiNode
/// [`WidgetBase`]: struct@WidgetBase
/// [`Importance::WIDGET`]: builder::Importance::WIDGET
/// [`Importance::INSTANCE`]: builder::Importance::INSTANCE
///
/// <script>
/// // hide re-exported docs
/// let me = document.currentScript;
/// document.addEventListener("DOMContentLoaded", function() {
///     while(me.nextElementSibling !== null) {
///         me.nextElementSibling.remove();
///     }
/// });
/// </script>
pub use zng_app::widget::widget;

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
/// use zng::prelude_wgt::*;
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
///         pub zng::focus::focusable(enabled: impl IntoVar<bool>);
///     }
/// }
///
/// /// Foo is focusable.
/// #[widget($crate::Foo)]
/// pub struct Foo(FocusableMix<WidgetBase>);
/// ```
///
/// The example above declares a mix-in `FocusableMix<P>` and a widget `Foo`, the mix-in is used as a parent of the widget, only
/// the `Foo! { }` widget can be instantiated, and it will have the strongly associated property `focusable` from the mix-in.
///
/// All widget `impl` items can be declared in a mix-in, including the `fn widget_build(&mut self) -> T`. Multiple mix-ins can be inherited
/// by nesting the types in a full widget `Foo(AMix<BMix<Base>>)`. Mix-ins cannot inherit from other mix-ins.
///
/// <script>
/// // hide re-exported docs
/// let me = document.currentScript;
/// document.addEventListener("DOMContentLoaded", function() {
///     while(me.nextElementSibling !== null) {
///         me.nextElementSibling.remove();
///     }
/// });
/// </script>
pub use zng_app::widget::widget_mixin;

/// Expands a property assign to include an easing animation.
///
/// The attribute generates a [property build action] that applies [`Var::easing`] to the final variable inputs of the property.
///
/// # Arguments
///
/// The attribute takes one required argument and one optional that matches the [`Var::easing`]
/// parameters. The required first arg is the duration, the second arg is an easing function, if not present the [`easing::linear`] is used.
///
/// Some items are auto-imported in each argument scope, [`TimeUnits`] for the first arg and the [`easing`] functions
/// for the second. This enables syntax like `#[easing(300.ms(), expo)]`.
///
/// ## Unset
///
/// An alternative argument `unset` can be used instead to remove animations set by the inherited context or styles.
///
/// [`TimeUnits`]: zng::layout::TimeUnits
/// [`easing`]: mod@zng::var::animation::easing
/// [`easing::linear`]: zng::var::animation::easing::linear
/// [property build action]: crate::widget::builder::WidgetBuilder::push_property_build_action
/// [`Var::easing`]: crate::var::Var::easing
///
/// ## When
///
/// The attribute can also be set in `when` assigns, in this case the easing will be applied when the condition is active, so
/// only the transition to the `true` value is animated using the conditional easing.
///
/// Note that you can't `unset` easing in when conditions, but you can set it to `0.ms()`, if all easing set for a property are `0`
/// no easing variable is generated, in contexts that actually have animation the `when` value will be set immediately,
/// by a zero sized animation.
///
/// # Examples
///
/// The example demonstrates setting and removing easing animations.
///
/// ```
/// # use zng::prelude_wgt::*;
/// # #[widget($crate::Foo)] pub struct Foo(WidgetBase);
/// # #[property(FILL, default(colors::BLACK))]
/// # pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode {
/// #    child
/// # }
/// # #[property(LAYOUT, default(0))]
/// # pub fn margin(child: impl IntoUiNode, color: impl IntoVar<SideOffsets>) -> UiNode {
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
/// is set in a property that does not match this a compile time type error occurs, with a mention of `easing_property_input_Transitionable`.
///
/// <script>
/// // hide re-exported docs
/// let me = document.currentScript;
/// document.addEventListener("DOMContentLoaded", function() {
///     while(me.nextElementSibling !== null) {
///         me.nextElementSibling.remove();
///     }
/// });
/// </script>
///
/// [`Transitionable`]: crate::var::animation::Transitionable
pub use zng_app::widget::easing;

/// Expands a function to a widget property.
///
/// Property functions take one [`IntoUiNode`] child input and one or more other inputs and produces an [`UiNode`] that implements
/// the property feature.
///
/// The attribute expansion does not modify the function, it can still be used as a function directly. Some
/// properties are implemented by calling other property functions to generate a derived effect.
///
/// The attribute expansion generates a hidden trait of the same name and visibility, the trait is implemented for widget builders,
/// the widget macros use this to set the property. Because it has the same name it is imported together with the property
/// function, in practice this only matters in doc links where you must use the `fn@` disambiguator.
///
/// # Attribute
///
/// The property attribute has one required argument and three optional.
///
/// #### Nest Group
///
/// The first argument is the property [`NestGroup`]. The group defines the overall nest position
/// of the property, for example, `LAYOUT` properties always wrap `FILL` properties. This is important as widgets are open and any combination
/// of properties may end-up instantiated in the same widget.
///
/// ```
/// # fn main() { }
/// use zng::prelude_wgt::*;
///
/// #[property(LAYOUT)]
/// pub fn align(child: impl IntoUiNode, align: impl IntoVar<Align>) -> UiNode {
///     // ..
/// #   child
/// }
/// ```
///
/// The nest group can be tweaked, by adding or subtracting integers, in the example bellow both properties are in the `SIZE` group,
/// but `size` is always inside `max_size`.
///
/// ```
/// # fn main() { }
/// use zng::prelude_wgt::*;
///
/// #[property(SIZE+1)]
/// pub fn size(child: impl IntoUiNode, size: impl IntoVar<Size>) -> UiNode {
///     // ..
/// #   child
/// }
///
/// #[property(SIZE)]
/// pub fn max_size(child: impl IntoUiNode, size: impl IntoVar<Size>) -> UiNode {
///     // ..
/// #   child
/// }
/// ```
///
/// #### Default
///
/// The next argument is an optional `default(args..)`. It defines the value to use when the property must be instantiated and no value was provided.
/// The defaults should cause the property to behave as if it is not set, as the default value will be used in widgets that only set the
/// property in `when` blocks.
///
/// ```
/// # fn main() { }
/// use zng::prelude_wgt::*;
///
/// #[property(FILL, default(rgba(0, 0, 0, 0)))]
/// pub fn background_color(child: impl IntoUiNode, color: impl IntoVar<Rgba>) -> UiNode {
///     // ..
/// #   child
/// }
/// ```
///
/// In the example above the `background_color` defines a transparent color as the default, so if the background color is only set in a `when`
/// block if will only be visible when it is active.
///
/// For properties with multiple inputs the default args may be defined in a comma separated list of params, `default(dft0, ..)`.
///
/// #### Impl For
///
/// The last argument is an optional `impl(<widget-type>)`, it strongly associates
/// the property with a widget, users can set this property on the widget without needing to import the property.
///
/// Note that this makes the property have priority over all others of the same name, only a derived widget can override
/// with another strongly associated property.
///
/// Note that you can also use the [`widget_impl!`] in widget declarations to implement existing properties for a widget.
///
/// #### Capture
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
/// use zng::prelude_wgt::*;
///
/// /// Children property, must be captured by panel widgets.
/// #[property(CONTEXT, capture)]
/// pub fn children(children: impl IntoUiNode) { }
/// ```
///
/// # Args
///
/// The property function requires at least two args, the first is the child node and the other(s) the input values. The
/// number and type of inputs is validated at compile time, the types are limited and are identified and validated by their
/// token name, so you cannot use renamed types.
///
/// #### Child
///
/// The first function arg must be of type `impl IntoUiNode`, it represents the child node and the property node must
/// delegate to it so that the UI tree functions correctly. The type must be an `impl` generic, a full path to [`IntoUiNode`]
/// is allowed, but no import renames as the proc-macro attribute can only use tokens to identify the type.
///
/// #### Inputs
///
/// The second arg and optional other args define the property inputs. When a property is assigned in a widget only these inputs
/// are defined by the user, the child arg is provided by the widget builder. Property inputs are limited, and must be identifiable
/// by their token name alone. The types are validated at compile time, they must be declared using `impl` generics,
/// a full path to the generic traits is allowed, but no import renames.
///
/// #### Input Types
///
/// These are the allowed input types:
///
/// ##### `impl IntoVar<T>`
///
/// The most common type, accepts any value that can be converted [`IntoVar<T>`], usually the property defines the `T`, but it can be generic.
/// The property node must respond to var updates. The input kind is [`InputKind::Var`]. No auto-default is generated for this type, property
/// implementation should provide a default value that causes the property to behave as if it was not set.
///
/// The input can be read in `when` expressions and can be assigned in `when` blocks.
///
/// ##### `impl IntoValue<T>`
///
/// Accepts any value that can be converted [`IntoValue<T>`] that does not change, usually the property
/// defines the `T`, but it can be generic. The input kind is [`InputKind::Value`]. No auto-default is generated for this type.
///
/// The input can be read in `when` expressions, but cannot be assigned in `when` blocks.
///
/// ##### `impl IntoUiNode`
///
/// This input accepts another [`UiNode`], the implementation must handle it like it handles the child node, delegating all methods. The
/// input kind is [`InputKind::UiNode`]. The [`UiNode::nil`] is used as the default value if no other is provided.
///
/// The input cannot be read in `when` expressions, but can be assigned in `when` blocks.
/// 
/// Note that UI lists like [`ui_vec!`] are also nodes, so panel children properties also receive `impl IntoUiNode`.
///
/// ##### `impl WidgetHandler<A>`
///
/// This input accepts any [`WidgetHandler<A>`] for the argument type `A`, usually the property defines the `A`, but it can be generic.
/// The input kind is [`InputKind::WidgetHandler`]. A no-op handler is used for the default if no other is provided.
///
/// Event handler properties usually have the `on_` name prefix. You can use the [`event_property!`] macro to generate standard event properties.
///
/// The input cannot be read in `when` expressions, but can be assigned in `when` blocks.
///
/// # Getter Properties
///
/// Most properties with var inputs are *setters*, that is the inputs affect the widget. Some properties
/// can be *getters*, detecting widget state and setting it on the *input* variable. These properties are usually named with
/// a prefix that indicates their input is actually for getting state, the prefixes `is_` and `has_` mark a property with
/// a single `bool` input that reads a widget state, the prefix `get_` and `actual_` marks a property that reads a non-boolean state from
/// the widget.
///
/// Getter properties are configured with a default read-write variable, so that they can be used in `when` expressions directly,
/// for example, `when *#is_pressed`, the `is_pressed` property has a `default(var(false))`, so it automatically initializes
/// with a read-write variable that is used in the when condition. The property attribute generates defaults automatically
/// based on the prefix, the default is `var(T::default())`, this can be overwritten just by setting the default,
/// it is not possible to declare a getter property without default.
///
/// Note that if a property is used in `when` condition without being set and without default value the when block is discarded on
/// widget build. If you are implementing a getter property that is not named using the prefixes listed above you must set `default(var(T::default())`.
///
/// # Generics
///
/// Apart from the `impl` generics of inputs and child, there is some support for named generic types, only one named generic is allowed
/// for inputs `impl IntoVar<T>`, `impl IntoValue<T>` and `impl WidgetHandler<A>`.
///
/// # Output
///
/// The property output type must be [`UiNode`]. The property node implementation can be anything, as long as it delegates
/// to the child node, see [`match_node`] or [`ui_node`] about implementing a node.
///
/// Some common property patterns have helper functions, for example, to setup a context var you can use the [`with_context_var`] function.
///
/// # More Details
///
/// See [`property_id!`] and [`property_args!`] for more details about what kind of meta-code is generated for properties.
///
/// [`NestGroup`]: crate::widget::builder::NestGroup
/// [`property_id!`]: crate::widget::builder::property_id
/// [`property_args!`]: crate::widget::builder::property_args
/// [`ui_node`]: macro@ui_node
/// [`match_node`]: crate::widget::node::match_node
/// [`with_context_var`]: crate::widget::node::with_context_var
/// [`VarValue`]: crate::var::VarValue
/// [`IntoValue<T>`]: crate::var::IntoValue
/// [`IntoVar<T>`]: crate::var::IntoVar
/// [`WidgetHandler<A>`]: crate::handler::WidgetHandler
/// [`UiNode`]: crate::widget::node::UiNode
/// [`IntoUiNode`]: crate::widget::node::IntoUiNode
/// [`UiNode::nil`]: crate::widget::node::UiNode::nil
/// [`ui_vec!`]: crate::widget::node::ui_vec
/// [`InputKind::Var`]: crate::widget::builder::InputKind::Var
/// [`InputKind::Value`]: crate::widget::builder::InputKind::Value
/// [`InputKind::UiNode`]: crate::widget::builder::InputKind::UiNode
/// [`InputKind::UiNodeList`]: crate::widget::builder::InputKind::UiNodeList
/// [`InputKind::WidgetHandler`]: crate::widget::builder::InputKind::WidgetHandler
/// [`event_property!`]: crate::event::event_property
///
/// <script>
/// // hide re-exported docs
/// let me = document.currentScript;
/// document.addEventListener("DOMContentLoaded", function() {
///     while(me.nextElementSibling !== null) {
///         me.nextElementSibling.remove();
///     }
/// });
/// </script>
pub use zng_app::widget::property;
