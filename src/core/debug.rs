#![cfg(debug_assertions)]
//! Helper types for debugging an UI tree.

use super::{
    context::{state_key, WidgetContext},
    impl_ui_node,
    render::FrameBuilder,
    types::*,
    var::{BoxVar, ObjVar, Var, VarValue},
    UiNode,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

/// A location in source-code.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    /// [`file!`]
    pub file: &'static str,
    /// [`line!`]
    pub line: u32,
    /// [`column!`]
    pub column: u32,
}

pub use zero_ui_macros::source_location;

/// Debug information about a property of a widget instance.
#[derive(Debug, Clone)]
pub struct PropertyInstanceInfo {
    /// Property priority in a widget.
    ///
    /// See [the property doc](crate::core::property#priority) for more details.
    pub priority: PropertyPriority,
    /// Original name of the property.
    pub original_name: &'static str,
    /// Source-code location of the property declaration.
    pub decl_location: SourceLocation,

    /// Name of the property in the widget.
    pub property_name: &'static str,
    /// Source-code location of the widget instantiation or property assign.
    pub instance_location: SourceLocation,

    /// Property arguments, sorted by their index in the property.
    pub args: Box<[PropertyArgInfo]>,

    /// If the user assigned this property.
    pub user_assigned: bool,

    /// Time elapsed in the last call of each property `UiNode` methods.
    pub duration: UiNodeDurations,
    /// Count of calls of each property `UiNode` methods.
    pub count: UiNodeCounts,
}
impl PropertyInstanceInfo {
    /// If `init` and `deinit` count are the same.
    pub fn is_deinited(&self) -> bool {
        self.count.init == self.count.deinit
    }
}

/// A reference to a [`PropertyInstanceInfo`].
pub type PropertyInstance = Rc<RefCell<PropertyInstanceInfo>>;

/// Debug information about a property argument.
#[derive(Debug, Clone)]
pub struct PropertyArgInfo {
    /// Name of the argument.
    pub name: &'static str,
    /// Debug pretty-printed value.
    ///
    /// This equals [`NO_DEBUG_VAR`] when the
    /// value cannot be inspected.
    pub value: String,
    /// Value version from the source variable.
    pub value_version: u32,
}

/// Property priority in a widget.
///
/// See [the property doc](crate::core::property#priority) for more details.
#[derive(Debug, Clone, Copy)]
pub enum PropertyPriority {
    Context,
    Event,
    Outer,
    Size,
    Inner,
    CaptureOnly,
}

/// Time duration of a [`UiNode`] method in a property branch.
///
/// The durations is the sum of all descendent nodes.
#[derive(Debug, Clone, Default)]
pub struct UiNodeDurations {
    pub init: Duration,
    pub deinit: Duration,
    pub update: Duration,
    pub update_hp: Duration,
    pub measure: Duration,
    pub arrange: Duration,
    pub render: Duration,
}

/// Number of times a [`UiNode`] method was called in a property branch.
///
/// The durations is the sum of all descendent nodes.
#[derive(Debug, PartialEq, Eq, Clone, Default)]
pub struct UiNodeCounts {
    pub init: usize,
    pub deinit: usize,
    pub update: usize,
    pub update_hp: usize,
    pub measure: usize,
    pub arrange: usize,
    pub render: usize,
}

#[derive(Debug, Clone)]
pub struct WidgetInstanceInfo {
    /// Unique ID of the widget instantiation.
    pub instance_id: WidgetInstanceId,

    /// Widget type name.
    pub widget_name: &'static str,

    /// Source-code location of the widget declaration.
    pub decl_location: SourceLocation,

    /// Source-code location of the widget instantiation.
    pub instance_location: SourceLocation,
}

state_key! {
    struct PropertiesInfoKey: Vec<PropertyInstance>;
    struct WidgetInstanceInfoKey: WidgetInstanceInfo;
}

unique_id! {
    /// Unique ID of a widget instance.
    ///
    /// This is different from the `WidgetId` in that it cannot manipulated by the user
    /// and identifies the widget *instantiation* event during debug mode.
    pub WidgetInstanceId;
}

// Node inserted just before calling the widget new function in debug mode.
// It registers the `WidgetInstanceInfo` metadata.
#[doc(hidden)]
pub struct WidgetInstanceInfoNode {
    child: Box<dyn UiNode>,
    info: WidgetInstanceInfo,
}
impl WidgetInstanceInfoNode {
    pub fn new_v1(
        node: Box<dyn UiNode>,
        widget_name: &'static str,
        decl_location: SourceLocation,
        instance_location: SourceLocation,
    ) -> Self {
        WidgetInstanceInfoNode {
            child: node,
            info: WidgetInstanceInfo {
                instance_id: WidgetInstanceId::new_unique(),
                widget_name,
                decl_location,
                instance_location,
            },
        }
    }
}
#[impl_ui_node(child)]
impl UiNode for WidgetInstanceInfoNode {
    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().set(WidgetInstanceInfoKey, self.info.clone());
        self.child.render(frame);
    }
}

// Node inserted around each widget property in debug mode.
//
// It collects information about the UiNode methods, tracks property variable values
// and registers the property in the widget metadata in a frame.
#[doc(hidden)]
pub struct PropertyInfoNode {
    child: Box<dyn UiNode>,
    arg_debug_vars: Box<[BoxVar<String>]>,
    info: PropertyInstance,
}
impl PropertyInfoNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new_v1(
        node: Box<dyn UiNode>,

        priority: PropertyPriority,
        original_name: &'static str,
        decl_location: SourceLocation,

        property_name: &'static str,
        instance_location: SourceLocation,

        arg_names: &[&'static str],
        arg_debug_vars: Box<[BoxVar<String>]>,

        user_assigned: bool,
    ) -> Self {
        assert_eq!(arg_names.len(), arg_debug_vars.len());
        PropertyInfoNode {
            child: node,
            arg_debug_vars,
            info: Rc::new(RefCell::new(PropertyInstanceInfo {
                priority,
                original_name,
                decl_location,
                property_name,
                instance_location,
                args: arg_names
                    .iter()
                    .map(|n| PropertyArgInfo {
                        name: n,
                        value: String::new(),
                        value_version: 0,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),

                user_assigned,
                duration: UiNodeDurations::default(),
                count: UiNodeCounts::default(),
            })),
        }
    }
}
macro_rules! ctx_mtd {
    ($self:ident.$mtd:ident, $ctx:ident, mut $info:ident) => {
        let t = Instant::now();
        $self.child.$mtd($ctx);
        let d = t.elapsed();
        let mut $info = $self.info.borrow_mut();
        $info.duration.$mtd = d;
        $info.count.$mtd += 1;
    };
}
impl UiNode for PropertyInfoNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        ctx_mtd!(self.init, ctx, mut info);

        for (var, arg) in self.arg_debug_vars.iter_mut().zip(info.args.iter_mut()) {
            arg.value = var.get(ctx.vars).clone();
            arg.value_version = var.version(ctx.vars);
        }
    }
    fn deinit(&mut self, ctx: &mut WidgetContext) {
        ctx_mtd!(self.deinit, ctx, mut info);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        ctx_mtd!(self.update, ctx, mut info);

        for (var, arg) in self.arg_debug_vars.iter_mut().zip(info.args.iter_mut()) {
            if let Some(new) = var.update(ctx.vars) {
                arg.value = new.clone();
                arg.value_version = var.version(ctx.vars);
            }
        }
    }
    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        ctx_mtd!(self.update, ctx, mut info);
    }

    fn measure(&mut self, available_size: LayoutSize, pixels: PixelGrid) -> LayoutSize {
        let t = Instant::now();
        let r = self.child.measure(available_size, pixels);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.measure = d;
        info.count.measure += 1;
        r
    }
    fn arrange(&mut self, final_size: LayoutSize, pixels: PixelGrid) {
        let t = Instant::now();
        self.child.arrange(final_size, pixels);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.arrange = d;
        info.count.arrange += 1;
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().entry(PropertiesInfoKey).or_default().push(Rc::clone(&self.info));
        let t = Instant::now();
        self.child.render(frame);
        let d = t.elapsed();
        let mut info = self.info.borrow_mut();
        info.duration.render = d;
        info.count.render += 1;
    }
}

#[doc(hidden)]
pub fn debug_var<T: VarValue>(var: impl Var<T>) -> BoxVar<String> {
    var.into_map(|t| format!("{:#?}", t)).boxed()
}

#[doc(hidden)]
pub fn no_debug_var() -> BoxVar<String> {
    crate::core::var::OwnedVar(NO_DEBUG_VAR.to_owned()).boxed()
}

/// Value when the [property value](PropertyArgInfo::value) cannot be inspected.
pub static NO_DEBUG_VAR: &str = "<!allowed_in_when>";

#[doc(hidden)]
pub type DebugArgs = Box<[BoxVar<String>]>;

// Generate this type for each property struct name P_property_name ?
// Advantage: shows property name in stack-trace.
// Disadvantage: more things to compile in debug mode.
#[allow(unused)]
macro_rules! property_name_in_stack_tace {
    () => {
        mod button {
            #[doc(hidden)]
            pub struct P_margin {
                child: PropertyInfoNode,
            }
            #[impl_ui_node(child)]
            impl UiNode for P_margin {}
        }
    };
}
