//! A simple command line based UI inspector.

use std::borrow::Cow;

use linear_map::LinearMap;

use crate::crate_util::IdMap;
use crate::widget_info::WidgetInfoTree;

use super::*;

/// State for tracking updates in [`write_tree`].
pub struct WriteTreeState {
    #[allow(clippy::type_complexity)]
    widgets: IdMap<WidgetInstanceId, WriteWidgetState>,
}
struct WriteWidgetState {
    /// [(property_name, arg_name) => (value_version, value)]
    properties: LinearMap<(&'static str, &'static str), (VarVersion, String)>,
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
    pub fn new(vars: &VarsRead, tree: &WidgetInfoTree) -> Self {
        let mut widgets = IdMap::default();

        for w in tree.all_widgets() {
            if let Some(info) = w.instance() {
                let mut properties = LinearMap::new();
                for ctor in &info.constructors {
                    for p in ctor.captures.iter() {
                        for arg in p.args.iter() {
                            let value = format!("{:?}", arg.value.get(vars));
                            properties.insert((p.property_name, arg.name), (arg.value.version(vars), value));
                        }
                    }
                }
                for p in info.properties.iter() {
                    for arg in p.args.iter() {
                        let value = format!("{:?}", arg.value.get(vars));
                        properties.insert((p.meta.property_name, arg.name), (arg.value.version(vars), value));
                    }
                }

                widgets.insert(info.meta.instance_id, WriteWidgetState { properties });
            }
        }

        WriteTreeState { widgets }
    }

    /// Gets property argument and if it changed.
    pub fn arg_diff(&self, vars: &VarsRead, widget_id: WidgetInstanceId, property_name: &'static str, arg: &PropertyArg) -> WriteArgDiff {
        if !self.is_none() {
            if let Some(wgt_state) = self.widgets.get(&widget_id) {
                if let Some((value_version, value)) = wgt_state.properties.get(&(property_name, arg.name)) {
                    let mut r = WriteArgDiff {
                        value: Cow::Borrowed(value),
                        changed_version: false,
                        changed_value: false,
                    };

                    let arg_version = arg.value.version(vars);
                    if *value_version != arg_version {
                        r.changed_version = true;
                        let arg_value = format!("{:?}", arg.value.get(vars));
                        if &arg_value != value {
                            r.value = Cow::Owned(arg_value);
                            r.changed_value = true;
                        }
                    }

                    return r;
                }
            }
        }

        WriteArgDiff {
            value: Cow::Owned(format!("{:?}", arg.value.get(vars))),
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
pub fn write_tree<W: std::io::Write>(vars: &VarsRead, tree: &WidgetInfoTree, updates_from: &WriteTreeState, out: &mut W) {
    let mut fmt = print_fmt::Fmt::new(out);
    write_impl(vars, updates_from, tree.root(), "", &mut fmt);
    fmt.write_legend();
}
fn write_impl(vars: &VarsRead, updates_from: &WriteTreeState, widget: WidgetInfo, parent_name: &str, fmt: &mut print_fmt::Fmt) {
    if let Some(info) = widget.instance() {
        fmt.open_widget(info.meta.widget_name, parent_name, info.parent_name.as_str());

        let mut write_property = |prop: PropertyOrCapture, group: &'static str| {
            let property_name = prop.property_name();
            let args = prop.args();

            if args.len() == 1 {
                let value = updates_from.arg_diff(vars, info.meta.instance_id, property_name, &args[0]);
                fmt.write_property(group, property_name, prop.user_assigned(), args[0].value.can_update(), value);
            } else {
                let user_assigned = prop.user_assigned();
                fmt.open_property(group, property_name, user_assigned);
                for arg in args.iter() {
                    fmt.write_property_arg(
                        arg.name,
                        user_assigned,
                        arg.value.can_update(),
                        updates_from.arg_diff(vars, info.meta.instance_id, property_name, arg),
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

        fmt.writeln();

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
mod print_fmt {
    use super::WriteArgDiff;
    use colored::*;
    use std::fmt::Display;
    use std::io::Write;

    pub struct Fmt<'w> {
        depth: u32,
        output: &'w mut dyn Write,
        property_group: &'static str,
    }
    impl<'w> Fmt<'w> {
        pub fn new(output: &'w mut dyn Write) -> Self {
            Fmt {
                depth: 0,
                output,
                property_group: "",
            }
        }

        fn write_tabs(&mut self) {
            let _ = write!(&mut self.output, "{:d$}", "", d = self.depth as usize * 3);
        }

        fn write(&mut self, s: impl Display) {
            let _ = write!(&mut self.output, "{s}");
        }

        pub fn writeln(&mut self) {
            let _ = writeln!(&mut self.output);
        }

        pub fn write_comment(&mut self, comment: impl Display) {
            self.write_tabs();
            self.write_comment_after(comment);
        }

        fn write_comment_after(&mut self, comment: impl Display) {
            self.write("// ".truecolor(117, 113, 94));
            self.write(comment.to_string().truecolor(117, 113, 94));
            self.writeln();
        }

        pub fn open_widget(&mut self, name: &str, parent_name: &str, parent_scope: &str) {
            if !parent_scope.is_empty() {
                self.writeln();
                self.write_comment(format_args!("in {parent_name}::{parent_scope}"));
            }
            self.write_tabs();
            self.write(name.yellow());
            self.write("!".yellow());
            self.write(" {".bold());
            self.writeln();
            self.depth += 1;
        }

        fn write_property_header(&mut self, group: &'static str, name: &str, user_assigned: bool) {
            if self.property_group != group {
                self.write_comment(group);
                self.property_group = group;
            }

            self.write_tabs();
            if user_assigned {
                self.write(name.blue().bold());
            } else {
                self.write(name);
            }
            self.write(" = ");
        }

        fn write_property_end(&mut self, user_assigned: bool) {
            if user_assigned {
                self.write(";".blue().bold());
            } else {
                self.write(";");
            }
            self.writeln();
        }

        fn write_property_value(&mut self, value: WriteArgDiff, can_update: bool) {
            let mut l0 = true;
            for line in value.value.lines() {
                if l0 {
                    l0 = false;
                } else {
                    self.writeln();
                    self.write_tabs();
                }

                if value.changed_value {
                    self.write(line.truecolor(150, 255, 150).bold())
                } else if value.changed_version {
                    self.write(line.truecolor(100, 150, 100))
                } else if can_update {
                    self.write(line.truecolor(200, 150, 150));
                } else {
                    self.write(line.truecolor(150, 150, 200));
                }
            }
        }

        pub fn write_property(&mut self, group: &'static str, name: &str, user_assigned: bool, can_update: bool, value: WriteArgDiff) {
            self.write_property_header(group, name, user_assigned);
            self.write_property_value(value, can_update);
            self.write_property_end(user_assigned);
        }

        pub fn open_property(&mut self, group: &'static str, name: &str, user_assigned: bool) {
            self.write_property_header(group, name, user_assigned);
            if user_assigned {
                self.write("{".blue().bold());
            } else {
                self.write("{");
            }
            self.writeln();
            self.depth += 1;
        }

        pub fn write_property_arg(&mut self, name: &str, user_assigned: bool, can_update: bool, value: WriteArgDiff) {
            self.write_tabs();
            if user_assigned {
                self.write(name.blue().bold());
                self.write(": ".blue().bold());
            } else {
                self.write(name);
                self.write(": ");
            }
            self.write_property_value(value, can_update);
            if user_assigned {
                self.write(",".blue().bold());
            } else {
                self.write(",");
            }
            self.writeln();
        }

        pub fn close_property(&mut self, user_assigned: bool) {
            self.depth -= 1;
            self.write_tabs();
            if user_assigned {
                self.write("}".blue().bold());
            } else {
                self.write("}");
            }
            self.write_property_end(user_assigned);
        }

        pub fn close_widget(&mut self, name: &str) {
            self.depth -= 1;
            self.property_group = "";
            self.write_tabs();
            self.write("} ".bold());
            self.write_comment_after(format_args!("{name}!"));
        }

        pub fn write_legend(&mut self) {
            if !control::SHOULD_COLORIZE.should_colorize() {
                return;
            }

            self.writeln();
            self.write("▉".yellow());
            self.write("  - widget");
            self.writeln();

            self.write("▉".blue());
            self.write("  - property, set by user");
            self.writeln();

            self.write("▉  - property, set by widget");
            self.writeln();

            self.write("▉".truecolor(200, 150, 150));
            self.write("  - variable");
            self.writeln();

            self.write("▉".truecolor(150, 150, 200));
            self.write("  - static, init value");
            self.writeln();

            self.write("▉".truecolor(150, 255, 150));
            self.write("  - updated, new value");
            self.writeln();

            self.write("▉".truecolor(100, 150, 100));
            self.write("  - updated, same value");
            self.writeln();

            self.writeln();
        }
    }
}
