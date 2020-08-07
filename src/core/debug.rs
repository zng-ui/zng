#![cfg(debug_assertions)]
//! Helper types for debugging an UI tree.

use super::{
    context::{state_key, WidgetContext},
    impl_ui_node,
    render::FrameBuilder,
    types::*,
    var::BoxVar,
    UiNode,
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

/// Debug information about a property of a widget.
#[derive(Debug, Clone)]
pub struct PropertyInfo {
    pub priority: PropertyPriority,
    pub property_name: &'static str,
    pub line: u32,
    pub column: u32,
    pub args: Box<[PropertyArgInfo]>,
    pub duration: UiNodeDurations,
    pub count: UiNodeCounts,
}
impl PropertyInfo {
    pub fn is_deinited(&self) -> bool {
        self.count.init == self.count.deinit
    }
}

/// A reference to a [`PropertyInfo`].
pub type PropertyInfoRef = Rc<RefCell<PropertyInfo>>;

#[derive(Debug, Clone)]
pub struct PropertyArgInfo {
    pub arg_name: &'static str,
    pub debug_value: String,
    pub value_version: u32,
}

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

state_key! {
    struct PropertiesInfoKey: Vec<PropertyInfoRef>;
    struct WidgetInstanceInfoKey: WidgetInstanceInfo;
}

unique_id! {
    WidgetInstanceId;
}
#[derive(Clone)]
struct WidgetInstanceInfo {
    instance_id: WidgetInstanceId,
    widget_name: &'static str,
}

// Node inserted just before calling the widget new function in debug mode.
// It registers the `WidgetInstanceInfo` metadata.
#[doc(hidden)]
pub struct WidgetInstanceInfoNode {
    child: Box<dyn UiNode>,
    info: WidgetInstanceInfo,
}
impl WidgetInstanceInfoNode {
    pub fn new(node: Box<dyn UiNode>, widget_name: &'static str) -> Self {
        WidgetInstanceInfoNode {
            child: node,
            info: WidgetInstanceInfo {
                instance_id: WidgetInstanceId::new_unique(),
                widget_name,
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
    info: PropertyInfoRef,
}
impl PropertyInfoNode {
    #[allow(clippy::too_many_arguments)]
    pub fn new_v1(
        node: Box<dyn UiNode>,
        priority: PropertyPriority,
        property_name: &'static str,
        line: u32,
        column: u32,
        arg_names: &[&'static str],
        arg_debug_vars: Box<[BoxVar<String>]>,
    ) -> Self {
        assert_eq!(arg_names.len(), arg_debug_vars.len());
        PropertyInfoNode {
            child: node,
            arg_debug_vars,
            info: Rc::new(RefCell::new(PropertyInfo {
                priority,
                property_name,
                line,
                column,
                args: arg_names
                    .iter()
                    .map(|n| PropertyArgInfo {
                        arg_name: n,
                        debug_value: String::new(),
                        value_version: 0,
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
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
            arg.debug_value = var.get(ctx.vars).clone();
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
                arg.debug_value = new.clone();
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
