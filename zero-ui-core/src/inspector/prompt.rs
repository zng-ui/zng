//! A simple command line based UI inspector.

use rustc_hash::FxHashMap;

use crate::{
    text::formatx,
    units::PxRect,
    var::VarUpdateId,
    widget_builder::{property_id, Importance, InputKind, PropertyId},
    widget_info::{Interactivity, Visibility, WidgetInfo, WidgetInfoTree},
    widget_instance::WidgetId,
};

use super::WidgetInfoInspectorExt;

/// UI state for printing and tracking updates.
#[derive(Default, Clone)]
pub struct WriteTreeState {
    state: FxHashMap<(WidgetId, PropertyId, usize), PropertyState>,
    computed_state: FxHashMap<(WidgetId, &'static str), ComputedState>,
    widget_len: usize,
    fresh: u8,
}
impl WriteTreeState {
    /// New default empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// Update the state, flagging changes for highlight in the next print, then prints the tree.
    pub fn write_update(&mut self, tree: &WidgetInfoTree, out: &mut impl std::io::Write) {
        self.fresh = self.fresh.wrapping_add(1);

        let mut fmt = print_fmt::Fmt::new(out);
        self.write_widget(tree.root(), &mut fmt);
        fmt.write_legend();

        if tree.len() != self.widget_len && self.widget_len > 0 {
            self.state.retain(|_, s| s.fresh == self.fresh);
            self.computed_state.retain(|_, s| s.fresh == self.fresh);
        }
        self.widget_len = tree.len();
    }

    fn write_widget(&mut self, info: WidgetInfo, fmt: &mut print_fmt::Fmt) {
        let mut wgt_name = "<widget>";
        if let Some(inf) = info.inspector_info() {
            wgt_name = inf.builder.widget_mod().name();

            let widget_id = info.widget_id();
            let (parent_name, parent_prop) = match info.parent_property() {
                Some((p, _)) => (info.parent().unwrap().inspector_info().unwrap().builder.widget_mod().name(), p.name),
                None => ("", ""),
            };

            fmt.open_widget(wgt_name, parent_name, parent_prop);

            for item in inf.items.iter() {
                let args = match item {
                    super::InstanceItem::Property { args, .. } => args,
                    super::InstanceItem::Intrinsic { group, name } => {
                        fmt.write_instrinsic(group.name(), name);
                        continue;
                    }
                };

                let info = args.property();
                let inst = args.instance();
                let group = info.group.name();
                let name = inst.name;
                let user_assigned = if let Some(p) = inf.builder.property(args.id()) {
                    p.importance == Importance::INSTANCE
                } else {
                    false
                };

                if info.inputs.len() == 1 {
                    let version = match info.inputs[0].kind {
                        InputKind::Var | InputKind::StateVar => Some(args.var(0).last_update()),
                        _ => None,
                    };
                    let value = print_fmt::Diff {
                        value: args.debug(0),
                        changed: version.map(|ver| self.update((widget_id, args.id(), 0), ver)).unwrap_or(false),
                    };
                    fmt.write_property(group, name, user_assigned, version.is_some(), value);
                } else {
                    fmt.open_property(group, name, user_assigned);
                    for (i, input) in info.inputs.iter().enumerate() {
                        let version = match input.kind {
                            InputKind::Var | InputKind::StateVar => Some(args.var(i).last_update()),
                            _ => None,
                        };
                        let value = print_fmt::Diff {
                            value: args.debug(i),
                            changed: version.map(|ver| self.update((widget_id, args.id(), i), ver)).unwrap_or(false),
                        };
                        fmt.write_property_arg(input.name, user_assigned, version.is_some(), value);
                    }
                    fmt.close_property(user_assigned);
                }
            }
        } else {
            fmt.open_widget(wgt_name, "", "");
        }

        let widget_id = info.widget_id();

        if info.inspect_property(property_id!(crate::widget_base::id).impl_id).is_none() {
            fmt.write_property(
                "computed",
                "id",
                false,
                false,
                print_fmt::Diff {
                    value: formatx!("{:?}", widget_id),
                    changed: false,
                },
            );
        }
        let outer_bounds = info.outer_bounds();
        fmt.write_property(
            "computed",
            "outer_bounds",
            false,
            true,
            print_fmt::Diff {
                value: formatx!("{:?}", outer_bounds),
                changed: self.update_computed((widget_id, "outer_bounds"), outer_bounds),
            },
        );
        let inner_bounds = info.inner_bounds();
        fmt.write_property(
            "computed",
            "inner_bounds",
            false,
            true,
            print_fmt::Diff {
                value: formatx!("{:?}", inner_bounds),
                changed: self.update_computed((widget_id, "inner_bounds"), inner_bounds),
            },
        );
        let visibility = info.visibility();
        fmt.write_property(
            "computed",
            "visibility",
            false,
            true,
            print_fmt::Diff {
                value: formatx!("{:?}", visibility),
                changed: self.update_computed((widget_id, "visibility"), visibility),
            },
        );
        let interactivity = info.interactivity();
        fmt.write_property(
            "computed",
            "interactivity",
            false,
            true,
            print_fmt::Diff {
                value: formatx!("{:?}", interactivity),
                changed: self.update_computed((widget_id, "interactivity"), interactivity),
            },
        );

        for c in info.children() {
            self.write_widget(c, fmt);
        }

        fmt.close_widget(wgt_name)
    }

    fn update(&mut self, key: (WidgetId, PropertyId, usize), version: VarUpdateId) -> bool {
        match self.state.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let changed = e.get().last_update != version;
                e.get_mut().last_update = version;
                e.get_mut().fresh = self.fresh;
                changed
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(PropertyState {
                    last_update: version,
                    fresh: self.fresh,
                });
                false
            }
        }
    }

    fn update_computed(&mut self, key: (WidgetId, &'static str), value: impl Into<ComputedValue>) -> bool {
        let value = value.into();
        match self.computed_state.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let changed = value != e.get().value;
                e.get_mut().value = value;
                e.get_mut().fresh = self.fresh;
                changed
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(ComputedState { value, fresh: self.fresh });
                false
            }
        }
    }
}

#[derive(Clone)]
struct PropertyState {
    last_update: VarUpdateId,
    fresh: u8,
}

#[derive(Clone)]
struct ComputedState {
    value: ComputedValue,
    fresh: u8,
}
#[derive(Clone, Copy, PartialEq)]
enum ComputedValue {
    PxRect(PxRect),
    Visibility(Visibility),
    Interactivity(Interactivity),
}
impl From<Interactivity> for ComputedValue {
    fn from(v: Interactivity) -> Self {
        Self::Interactivity(v)
    }
}
impl From<Visibility> for ComputedValue {
    fn from(v: Visibility) -> Self {
        Self::Visibility(v)
    }
}
impl From<PxRect> for ComputedValue {
    fn from(v: PxRect) -> Self {
        Self::PxRect(v)
    }
}

mod print_fmt {
    pub struct Diff {
        /// Debug value.
        pub value: Text,
        /// If the variable version changed since last read.
        pub changed: bool,
    }

    use colored::*;
    use std::fmt::Display;
    use std::io::Write;

    use crate::text::Text;

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

        pub fn open_widget(&mut self, name: &str, parent_name: &str, parent_property: &str) {
            if !parent_property.is_empty() {
                self.writeln();
                self.write_comment(format_args!("in {parent_name}.{parent_property}"));
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

        fn write_property_value(&mut self, value: Diff, can_update: bool) {
            let mut l0 = true;
            for line in value.value.lines() {
                if l0 {
                    l0 = false;
                } else {
                    self.writeln();
                    self.write_tabs();
                }

                if value.changed {
                    self.write(line.truecolor(150, 255, 150).bold())
                } else if can_update {
                    self.write(line.truecolor(200, 150, 150));
                } else {
                    self.write(line.truecolor(150, 150, 200));
                }
            }
        }

        pub fn write_instrinsic(&mut self, group: &'static str, name: &str) {
            if self.property_group != group {
                self.write_comment(group);
                self.property_group = group;
            }

            self.write_tabs();
            self.write(name.truecolor(180, 180, 180).dimmed().italic());
            self.writeln();
        }

        pub fn write_property(&mut self, group: &'static str, name: &str, user_assigned: bool, can_update: bool, value: Diff) {
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

        pub fn write_property_arg(&mut self, name: &str, user_assigned: bool, can_update: bool, value: Diff) {
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

            self.write("▉".truecolor(180, 180, 180).dimmed().italic());
            self.write("  - instrinsic");
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
            self.write("  - updated");
            self.writeln();

            self.writeln();
        }
    }
}
