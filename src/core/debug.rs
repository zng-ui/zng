#![cfg(debug_assertions)]
//! Helper types for debugging an UI tree.

use super::{
    context::{state_key, WidgetContext},
    impl_ui_node,
    render::{FrameBuilder, FrameInfo, WidgetInfo},
    types::*,
    var::{context_var, BoxVar, ObjVar, Var, VarValue},
    UiNode,
};
use std::{
    cell::RefCell,
    collections::HashSet,
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

    /// If [`args`](Self::args) values can be inspected.
    ///
    /// Only properties that are `allowed_in_when` are guaranteed to have
    /// variable arguments with values that can print debug. For other properties
    /// the [`value`](PropertyArgInfo::value) is always an empty string and
    /// [`value_version`](PropertyArgInfo::value_version) is always zero.
    pub can_debug_args: bool,

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

/// A reference to a [`WidgetInstanceInfo`].
pub type WidgetInstance = Rc<RefCell<WidgetInstanceInfo>>;

/// Debug information about a property argument.
#[derive(Debug, Clone)]
pub struct PropertyArgInfo {
    /// Name of the argument.
    pub name: &'static str,
    /// Debug pretty-printed value.
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

    /// Properties this widget captured.
    pub captured: Box<[CapturedPropertyInfo]>,

    /// When blocks setup by this widget instance.
    pub whens: Box<[WhenInfo]>,

    /// Name of the parent widget property that introduces this widget.
    ///
    /// Empty string (`""`) when the widget has no parent with debug enabled.
    pub parent_property: &'static str,
}

/// Debug information about a *property* captured by a widget instance.
#[derive(Debug, Clone)]
pub struct CapturedPropertyInfo {
    /// Name of the property in the widget.
    pub property_name: &'static str,
    /// Source-code location of the widget instantiation or property assign.
    pub instance_location: SourceLocation,

    /// Property arguments, sorted by their index in the property.
    pub args: Box<[PropertyArgInfo]>,

    /// If [`args`](Self::args) values can be inspected.
    ///
    /// Only properties that are `allowed_in_when` are guaranteed to have
    /// variable arguments with values that can print debug. For other properties
    /// the [`value`](PropertyArgInfo::value) is always an empty string and
    /// [`value_version`](PropertyArgInfo::value_version) is always zero.
    pub can_debug_args: bool,

    /// If the user assigned this property.
    pub user_assigned: bool,
}

/// When block setup by a widget instance.
#[derive(Debug, Clone)]
pub struct WhenInfo {
    /// When condition expression.
    pub condition_expr: &'static str,
    /// Current when condition result.
    pub condition: bool,
    /// Condition value version.
    pub condition_version: u32,
    /// Properties affected by this when block.
    pub properties: HashSet<&'static str>,

    /// Source-code location of the when block declaration.
    pub decl_location: SourceLocation,

    /// Source-code location of the widget instantiation or property assign.
    pub instance_location: SourceLocation,

    /// If the user declared the when block in the widget instance.
    pub user_declared: bool,
}

state_key! {
    struct PropertiesInfoKey: Vec<PropertyInstance>;
    struct WidgetInstanceInfoKey: WidgetInstance;
}

unique_id! {
    /// Unique ID of a widget instance.
    ///
    /// This is different from the `WidgetId` in that it cannot manipulated by the user
    /// and identifies the widget *instantiation* event during debug mode.
    pub WidgetInstanceId;
}

context_var! {
    struct ParentPropertyName: &'static str = const "";
}

// Node inserted just before calling the widget new function in debug mode.
// It registers the `WidgetInstanceInfo` metadata.
#[doc(hidden)]
pub struct WidgetInstanceInfoNode {
    child: Box<dyn UiNode>,
    info: WidgetInstance,
    // debug vars per property.
    debug_vars: Box<[Box<[BoxVar<String>]>]>,
    // when condition result variables.
    when_vars: Box<[BoxVar<bool>]>,
}
#[doc(hidden)]
pub struct CapturedPropertyV1 {
    pub property_name: &'static str,
    pub instance_location: SourceLocation,
    pub arg_names: &'static [&'static str],
    pub arg_debug_vars: DebugArgs,
    pub user_assigned: bool,
}
#[doc(hidden)]
pub struct WhenInfoV1 {
    pub condition_expr: &'static str,
    pub condition_var: Option<BoxVar<bool>>,
    pub properties: Vec<&'static str>,
    pub decl_location: SourceLocation,
    pub instance_location: SourceLocation,
    pub user_declared: bool,
}
impl WidgetInstanceInfoNode {
    pub fn new_v1(
        node: Box<dyn UiNode>,
        widget_name: &'static str,
        decl_location: SourceLocation,
        instance_location: SourceLocation,
        mut captured: Vec<CapturedPropertyV1>,
        mut whens: Vec<WhenInfoV1>,
    ) -> Self {
        let debug_vars = captured
            .iter_mut()
            .map(|c| std::mem::take(&mut c.arg_debug_vars))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let captured = captured
            .into_iter()
            .map(|c| CapturedPropertyInfo {
                property_name: c.property_name,
                instance_location: c.instance_location,
                args: c
                    .arg_names
                    .iter()
                    .map(|n| PropertyArgInfo {
                        name: n,
                        value: String::new(),
                        value_version: 0,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                can_debug_args: c.arg_names.len() == c.arg_debug_vars.len(),
                user_assigned: c.user_assigned,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let when_vars = whens
            .iter_mut()
            .map(|w| w.condition_var.take().unwrap())
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let whens = whens
            .into_iter()
            .map(|w| WhenInfo {
                condition_expr: w.condition_expr,
                condition: false,
                condition_version: 0,
                properties: w.properties.into_iter().collect(),
                decl_location: w.decl_location,
                instance_location: w.instance_location,
                user_declared: w.user_declared,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        WidgetInstanceInfoNode {
            child: node,
            info: Rc::new(RefCell::new(WidgetInstanceInfo {
                instance_id: WidgetInstanceId::new_unique(),
                widget_name,
                decl_location,
                instance_location,
                captured,
                whens,
                parent_property: "",
            })),
            debug_vars,
            when_vars,
        }
    }
}
#[impl_ui_node(child)]
impl UiNode for WidgetInstanceInfoNode {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);

        let mut info = self.info.borrow_mut();

        for (property, vars) in info.captured.iter_mut().zip(self.debug_vars.iter()) {
            for (arg, var) in property.args.iter_mut().zip(vars.iter()) {
                arg.value = var.get(ctx.vars).clone();
                arg.value_version = var.version(ctx.vars);
            }
        }
        for (when, var) in info.whens.iter_mut().zip(self.when_vars.iter()) {
            when.condition = *var.get(ctx.vars);
            when.condition_version = var.version(ctx.vars);
        }
        info.parent_property = ParentPropertyName::var().get(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let mut info = self.info.borrow_mut();

        for (property, vars) in info.captured.iter_mut().zip(self.debug_vars.iter()) {
            for (arg, var) in property.args.iter_mut().zip(vars.iter()) {
                if let Some(update) = var.update(ctx.vars) {
                    arg.value = update.clone();
                    arg.value_version = var.version(ctx.vars);
                }
            }
        }
        for (when, var) in info.whens.iter_mut().zip(self.when_vars.iter()) {
            if let Some(update) = var.update(ctx.vars) {
                when.condition = *update;
                when.condition_version = var.version(ctx.vars);
            }
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.meta().set(WidgetInstanceInfoKey, Rc::clone(&self.info));
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
        assert!(!arg_names.is_empty() && (arg_debug_vars.is_empty() || arg_names.len() == arg_debug_vars.len()));
        let can_debug_args = !arg_debug_vars.is_empty();
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
                can_debug_args,
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
        let mut info = self.info.borrow_mut();
        let child = &mut self.child;
        let property_name = info.property_name;

        ctx.vars.with_context(ParentPropertyName, &property_name, false, 0, || {
            let t = Instant::now();
            child.init(ctx);
            let d = t.elapsed();
            info.duration.init = d;
            info.count.init += 1;
        });

        for (var, arg) in self.arg_debug_vars.iter().zip(info.args.iter_mut()) {
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
    var.into_map(|t| format!("{:?}", t)).boxed()
}

#[doc(hidden)]
pub type DebugArgs = Box<[BoxVar<String>]>;

/// Adds debug information to [`WidgetInfo`].
pub trait WidgetDebugInfo<'a> {
    /// If the widget was instantiated with `@debug_enabled`.
    fn debug_enabled(self) -> bool;

    /// If any of the widget descendants are [`debug_enabled`](WidgetDebugInfo::debug_enabled).
    fn contains_debug(self) -> bool;

    /// Gets the widget instance info if the widget is [`debug_enabled`](WidgetDebugInfo::debug_enabled).
    fn instance(self) -> Option<&'a WidgetInstance>;

    /// Gets the widget properties info.
    ///
    /// Returns empty if not [`debug_enabled`](Self::debug_enabled).
    fn properties(self) -> &'a [PropertyInstance];
}
impl<'a> WidgetDebugInfo<'a> for WidgetInfo<'a> {
    #[inline]
    fn debug_enabled(self) -> bool {
        self.meta().contains(WidgetInstanceInfoKey)
    }

    #[inline]
    fn contains_debug(self) -> bool {
        self.descendants().any(|w| w.debug_enabled())
    }

    #[inline]
    fn instance(self) -> Option<&'a WidgetInstance> {
        self.meta().get(WidgetInstanceInfoKey)
    }

    #[inline]
    fn properties(self) -> &'a [PropertyInstance] {
        self.meta().get(PropertiesInfoKey).map(|v| &v[..]).unwrap_or(&[])
    }
}

#[inline]
pub fn print_frame(frame: &FrameInfo) {
    print_widget(frame.root())
}

#[inline]
pub fn print_widget(widget: WidgetInfo) {
    print_widget_(widget, 0)
}

fn print_widget_(widget: WidgetInfo, depth: usize) {
    use colored::*;
    const DEPTH_MUL: usize = 3;
    let w_depth = depth * DEPTH_MUL;

    if let Some(info) = widget.instance() {
        let instance = info.borrow();

        // widget! {
        println!("{:d$}{}{} {}", "", instance.widget_name.yellow(), "!".yellow(), "{".bold(), d = w_depth);

        // property: value;
        // OR
        // property: {
        //    foo: value,
        //    bar: value,
        // };
        // OR
        // property: ?;
        let p_depth = (depth + 1) * DEPTH_MUL;

        macro_rules! print_prop {
            ($prop:ident) => {
                if $prop.user_assigned {
                    print!("{:d$}{}{} ", "", $prop.property_name.blue().bold(), ":".blue().bold(), d = p_depth);
                } else {
                    print!("{:d$}{}: ", "", $prop.property_name, d = p_depth);
                }

                if $prop.can_debug_args {
                    if $prop.args.len() == 1 {
                        print!("{}", $prop.args[0].value);                 
                    } else {
                        println!("{{");
                        let a_depth = (depth + 2) * DEPTH_MUL;
                        for arg in $prop.args.iter() {
                                 if arg.value.len() > 50 {
                                println!("{:d$}{}: {}..,", "", arg.name, &arg.value[..50], d = a_depth);
                            } else {
                                println!("{:d$}{}: {},", "", arg.name, &arg.value, d = a_depth);
                            };

                        }
                        print!("{:d$}}}", "", d = p_depth);
                    }
                } else {
                    print!("?");
                }
                if $prop.user_assigned {
                    println!("{}", ";".blue().bold());
                } else {
                    println!(";");
                }
            };
        }

        for prop in instance.captured.iter() {
            print_prop!(prop);
        }
        for property in widget.properties() {
            let prop = property.borrow();            
            print_prop!(prop);
        }

        if (!instance.captured.is_empty() || !widget.properties().is_empty()) && widget.has_children() {
            println!();
        }

        let child_depth = depth + 1;
        for child in widget.children() {
            print_widget_(child, child_depth);
            println!();
        }

        println!("{:d$}{} {} {}", "", "}".bold(), "//".green(), instance.widget_name.green().dimmed(), d = w_depth);
    } else {
        println!("{:d$}{} {}", "", "<widget>!".yellow().dimmed(), "{".bold().dimmed(), d = w_depth);

        if widget.contains_debug() {
            let child_depth = depth + 1;
            for child in widget.children() {
                print_widget_(child, child_depth);
            }
        } else {
            let msg = format!("<{} omitted>", widget.descendants().count());
            println!("{:d$}{}", "", msg.dimmed(), d = (depth + 1) * DEPTH_MUL)
        }

        println!("{:d$}{}", "", "}".bold().dimmed(), d = w_depth);
    }
}
