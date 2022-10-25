
use std::borrow::Cow;

use linear_map::LinearMap;

use crate::crate_util::IdMap;
use crate::text::Text;
use crate::var::VarUpdateId;
use crate::widget_info::WidgetInfoTree;

use super::*;

/// State for tracking updates in [`write_tree`].
pub struct WriteTreeState {
    #[allow(clippy::type_complexity)]
    widgets: IdMap<WidgetInstanceId, WriteWidgetState>,
}
struct WriteWidgetState {
    /// [(property_name, arg_name) => (value_version, value)]
    properties: LinearMap<(&'static str, &'static str), (VarUpdateId, Text)>,
}
impl WriteTreeState {
    /// No property update.
    pub fn none() -> Self {
        WriteTreeState {
            widgets: Default::default(),
        }
    }

    /// State represents no property update.
    pub fn is_none(&self) -> bool {
        self.widgets.is_empty()
    }

    /// State from `tree` that can be compared to future trees.
    pub fn new(tree: &WidgetInfoTree) -> Self {
        let mut widgets = IdMap::default();

        for w in tree.all_widgets() {
            if let Some(info) = w.instance() {
                let mut properties = LinearMap::new();
                for ctor in &info.constructors {
                    for p in ctor.captures.iter() {
                        for arg in p.args.iter() {
                            properties.insert((p.property_name, arg.name), (arg.value.last_update(), arg.value.get_debug()));
                        }
                    }
                }
                for p in info.properties.iter() {
                    for arg in p.args.iter() {
                        properties.insert((p.meta.property_name, arg.name), (arg.value.last_update(), arg.value.get_debug()));
                    }
                }

                widgets.insert(info.meta.instance_id, WriteWidgetState { properties });
            }
        }

        WriteTreeState { widgets }
    }

    /// Gets property argument and if it changed.
    pub fn arg_diff(&self, widget_id: WidgetInstanceId, property_name: &'static str, arg: &PropertyArg) -> WriteArgDiff {
        if !self.is_none() {
            if let Some(wgt_state) = self.widgets.get(&widget_id) {
                if let Some((value_version, value)) = wgt_state.properties.get(&(property_name, arg.name)) {
                    let mut r = WriteArgDiff {
                        value: Cow::Borrowed(value),
                        changed_version: false,
                        changed_value: false,
                    };

                    let arg_version = arg.value.last_update();
                    if *value_version != arg_version {
                        r.changed_version = true;
                        let arg_value = arg.value.get_debug();
                        if &arg_value != value {
                            r.value = Cow::Owned(arg_value.into());
                            r.changed_value = true;
                        }
                    }

                    return r;
                }
            }
        }

        WriteArgDiff {
            value: Cow::Owned(arg.value.get_debug().into()),
            changed_value: false,
            changed_version: false,
        }
    }
}

/// Represents the value of a property that may have changed.
pub struct WriteArgDiff<'a> {
    /// Value, is borrowed if it is the same as before.
    pub value: Cow<'a, str>,
    /// If the variable version changed since last read.
    pub changed_version: bool,
    /// If var version changed and the debug print is different.
    pub changed_value: bool,
}
impl<'a> WriteArgDiff<'a> {
    fn from_info(value: &dyn fmt::Debug) -> Self {
        WriteArgDiff {
            value: Cow::Owned(format!("{value:?}")),
            changed_version: false,
            changed_value: false,
        }
    }
}

/// Writes the widget `tree` to `out`.
///
/// When writing to a terminal the text is color coded and a legend is printed. The coloring
/// can be configured using environment variables, see [colored](https://github.com/mackwic/colored#features)
/// for details.
pub fn write_tree<W: std::io::Write>(vars: &Vars, tree: &WidgetInfoTree, updates_from: &WriteTreeState, out: &mut W) {
    let mut fmt = print_fmt::Fmt::new(out);
    write_impl(vars, updates_from, tree.root(), "", &mut fmt);
    fmt.write_legend();
}
fn write_impl(vars: &Vars, updates_from: &WriteTreeState, widget: WidgetInfo, parent_name: &str, fmt: &mut print_fmt::Fmt) {
    if let Some(info) = widget.instance() {
        fmt.open_widget(info.meta.widget_name, parent_name, info.parent_name.as_str());

        let mut write_property = |prop: PropertyOrCapture, group: &'static str| {
            let property_name = prop.property_name();
            let args = prop.args();

            if args.len() == 1 {
                let value = updates_from.arg_diff(info.meta.instance_id, property_name, &args[0]);
                fmt.write_property(
                    group,
                    property_name,
                    prop.user_assigned(),
                    args[0].value.capabilities().contains(VarCapabilities::NEW),
                    value,
                );
            } else {
                let user_assigned = prop.user_assigned();
                fmt.open_property(group, property_name, user_assigned);
                for arg in args.iter() {
                    fmt.write_property_arg(
                        arg.name,
                        user_assigned,
                        arg.value.capabilities().contains(VarCapabilities::NEW),
                        updates_from.arg_diff(info.meta.instance_id, property_name, arg),
                    );
                }
                fmt.close_property(user_assigned);
            }
        };

        if let Some(ctor) = info.constructor("new") {
            for cap in ctor.captures.iter() {
                write_property(PropertyOrCapture::Capture(cap), "new");
            }
        }
        for &priority in PropertyPriority::context_to_child_layout() {
            if let Some(ctor) = info.constructor(priority.name_constructor()) {
                for cap in ctor.captures.iter() {
                    write_property(PropertyOrCapture::Capture(cap), ctor.fn_name);
                }
            }

            let group = priority.name();
            for prop in info.properties.iter().filter(|p| p.meta.priority == priority) {
                write_property(PropertyOrCapture::Property(prop), group);
            }
        }
        if let Some(ctor) = info.constructor("new_child") {
            for cap in ctor.captures.iter() {
                write_property(PropertyOrCapture::Capture(cap), "new_child");
            }
        }

        write_info(widget, fmt);

        // fmt.writeln();

        for child in widget.children() {
            write_impl(vars, updates_from, child, info.meta.widget_name, fmt);
        }

        fmt.close_widget(info.meta.widget_name);
    } else {
        fmt.open_widget("<unknown>", "", "");

        fmt.write_property("<info>", "id", false, false, WriteArgDiff::from_info(&widget.widget_id()));
        write_info(widget, fmt);

        for child in widget.children() {
            write_impl(vars, updates_from, child, "<unknown>", fmt);
        }
        fmt.close_widget("<unknown>");
    }

    fn write_info(widget: WidgetInfo, fmt: &mut print_fmt::Fmt) {
        fmt.write_property(
            "<info>",
            "outer_bounds",
            false,
            false,
            WriteArgDiff::from_info(&widget.outer_bounds()),
        );
        fmt.write_property(
            "<info>",
            "inner_bounds",
            false,
            false,
            WriteArgDiff::from_info(&widget.inner_bounds()),
        );
        fmt.write_property("<info>", "visibility", false, false, WriteArgDiff::from_info(&widget.visibility()));
        fmt.write_property(
            "<info>",
            "interactivity",
            false,
            false,
            WriteArgDiff::from_info(&widget.interactivity()),
        );
    }
}
