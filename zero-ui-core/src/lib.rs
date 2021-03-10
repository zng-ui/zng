#![warn(unused_extern_crates)]
// examples of `widget! { .. }` and `#[property(..)]` need to be declared
// outside the main function, because they generate a `mod` with `use super::*;`
// that does not import `use` clauses declared inside the parent function.
#![allow(clippy::needless_doctest_main)]

//! Core infrastructure required for creating components and running an app.

#[macro_use]
extern crate bitflags;

// to make the proc-macro $crate substitute work in doc-tests.
#[doc(hidden)]
#[allow(unused_extern_crates)]
extern crate self as zero_ui_core;

#[macro_use]
mod crate_macros;

#[doc(hidden)]
pub use paste::paste;

pub mod animation;
pub mod app;
pub mod color;
pub mod context;
pub mod debug;
pub mod event;
pub mod focus;
pub mod gesture;
pub mod gradient;
pub mod keyboard;
pub mod mouse;
pub mod profiler;
pub mod render;
pub mod service;
pub mod sync;
pub mod text;
pub mod units;
pub mod var;
pub mod widget_base;
pub mod window;

mod ui_node;
pub use ui_node::*;

mod ui_list;
pub use ui_list::*;

// proc-macros used internally during widget creation.
#[doc(hidden)]
pub use zero_ui_proc_macros::{property_new, widget_declare, widget_inherit, widget_new, widget_new2, widget_stage2, widget_stage3};

/// Gets if the value indicates that any size is available during layout (positive infinity)
#[inline]
pub fn is_layout_any_size(f: f32) -> bool {
    f.is_infinite() && f.is_sign_positive()
}

/// Value that indicates that any size is available during layout.
pub const LAYOUT_ANY_SIZE: f32 = f32::INFINITY;

/// A map of TypeId -> Box<dyn Any>.
type AnyMap = fnv::FnvHashMap<std::any::TypeId, Box<dyn std::any::Any>>;

pub use zero_ui_proc_macros::{impl_ui_node, property, widget, widget2, widget_mixin, widget_mixin2};

/// Tests on the #[property(..)] code generator.
#[cfg(test)]
#[allow(dead_code)] // if it builds it passes.
mod property_tests {
    use crate::var::*;
    use crate::{property, UiNode};

    #[property(context)]
    fn basic_context(child: impl UiNode, arg: impl IntoVar<u8>) -> impl UiNode {
        let _arg = arg;
        child
    }
    #[test]
    fn basic_gen() {
        use basic_context::{code_gen, Args, ArgsImpl};
        let a = ArgsImpl::new(1);
        let b = code_gen! { named_new basic_context, __ArgsImpl { arg: 2 } };
        let a = a.unwrap().into_local();
        let b = b.unwrap().into_local();
        assert_eq!(1, *a.get_local());
        assert_eq!(2, *b.get_local());
    }

    #[property(context)]
    fn is_state(child: impl UiNode, state: StateVar) -> impl UiNode {
        let _ = state;
        child
    }
    #[test]
    fn default_value() {
        use is_state::{code_gen, Args, ArgsImpl};
        let _ = ArgsImpl::default().unwrap();
        let is_default;
        let is_not_default = false;
        code_gen! {
            if default=> {
                is_default = true;
            }
        };
        code_gen! {
            if !default=> {
                is_not_default = true;
            }
        };
        assert!(is_default);
        assert!(!is_not_default);
    }

    #[test]
    fn not_default_value() {
        use basic_context::code_gen;
        let is_default = false;
        let is_not_default;
        code_gen! {
            if default=> {
                is_default = true;
            }
        };
        code_gen! {
            if !default=> {
                is_not_default = true;
            }
        };
        assert!(!is_default);
        assert!(is_not_default);
    }
}

/// Tests on the #[widget(..)] and #[widget_mixin], widget_new! code generators.
#[cfg(test)]
mod widget_tests {
    use self::util::Position;
    use crate::{context::TestWidgetContext, widget2, widget_mixin2, Widget, WidgetId};
    use serial_test::serial;

    // Used in multiple tests.
    #[widget2($crate::widget_tests::empty_wgt)]
    pub mod empty_wgt {}

    /*
     * Tests the implicitly inherited properties.
     */
    #[test]
    pub fn implicit_inherited() {
        let expected = WidgetId::new_unique();
        let wgt = empty_wgt! {
            id = expected;
        };
        let actual = wgt.id();
        assert_eq!(expected, actual);
    }

    // Mixin used in inherit tests.
    #[widget_mixin2($crate::widget_tests::foo_mixin)]
    pub mod foo_mixin {
        use super::util;

        properties! {
            util::trace as foo_trace = "foo_mixin";
        }
    }

    /*
     * Tests the inherited properties' default values and assigns.
     */
    #[widget2($crate::widget_tests::bar_wgt)]
    pub mod bar_wgt {
        use super::{foo_mixin, util};

        inherit!(foo_mixin);

        properties! {
            util::trace as bar_trace = "bar_wgt";
        }
    }
    #[test]
    pub fn wgt_with_mixin_default_values() {
        let mut default = bar_wgt!();
        default.test_init(&mut TestWidgetContext::wait_new());

        // test default values used.
        assert!(util::traced(&default, "foo_mixin"));
        assert!(util::traced(&default, "bar_wgt"));
    }
    #[test]
    pub fn wgt_with_mixin_assign_values() {
        let mut default = bar_wgt! {
            foo_trace = "foo!";
            bar_trace = "bar!";
        };
        default.test_init(&mut TestWidgetContext::wait_new());

        // test new values used.
        assert!(util::traced(&default, "foo!"));
        assert!(util::traced(&default, "bar!"));

        // test default values not used.
        assert!(!util::traced(&default, "foo_mixin"));
        assert!(!util::traced(&default, "bar_wgt"));
    }

    /*
     * Tests changing the default value of the inherited property.
     */
    #[widget2($crate::widget_tests::reset_wgt)]
    pub mod reset_wgt {
        inherit!(super::foo_mixin);

        properties! {
            foo_trace = "reset_wgt"
        }
    }
    #[test]
    pub fn wgt_with_new_value_for_inherited() {
        let mut default = reset_wgt!();
        default.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&default, "reset_wgt"));
        assert!(!util::traced(&default, "foo_mixin"));
    }

    /*
     * Tests new property from inherited property.
     */
    #[widget2($crate::widget_tests::alias_inherit_wgt)]
    pub mod alias_inherit_wgt {
        inherit!(super::foo_mixin);

        properties! {
            foo_trace as alias_trace = "alias_inherit_wgt"
        }
    }
    #[test]
    pub fn wgt_alias_inherit() {
        let mut default = alias_inherit_wgt!();
        default.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&default, "foo_mixin"));
        assert!(util::traced(&default, "alias_inherit_wgt"));

        let mut assigned = alias_inherit_wgt!(
            foo_trace = "foo!";
            alias_trace = "alias!";
        );
        assigned.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&assigned, "foo!"));
        assert!(util::traced(&assigned, "alias!"));
    }

    /*
     * Tests the property name when declared from path.
     */
    #[widget2($crate::widget_tests::property_from_path_wgt)]
    pub mod property_from_path_wgt {
        properties! {
            super::util::trace;
        }
    }
    #[test]
    pub fn wgt_property_from_path() {
        let mut assigned = property_from_path_wgt!(
            trace = "trace!";
        );
        assigned.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&assigned, "trace!"));
    }

    /*
     * Tests unsetting inherited property.
     */
    #[widget2($crate::widget_tests::unset_property_wgt)]
    pub mod unset_property_wgt {
        inherit!(super::foo_mixin);

        properties! {
            foo_trace = unset!;
        }
    }
    #[test]
    pub fn wgt_unset_property() {
        let mut default = unset_property_wgt!();
        default.test_init(&mut TestWidgetContext::wait_new());

        assert!(!util::traced(&default, "foo_mixin"));
    }

    /*
     * Test unsetting default value.
     */
    #[widget2($crate::widget_tests::default_value_wgt)]
    pub mod default_value_wgt {
        properties! {
            super::util::trace = "default_value_wgt";
        }
    }
    #[test]
    pub fn unset_default_value() {
        let mut default = default_value_wgt!();
        default.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&default, "default_value_wgt"));

        let mut no_default = default_value_wgt! {
            trace = unset!;
        };
        no_default.test_init(&mut TestWidgetContext::wait_new());

        assert!(!util::traced(&no_default, "default_value_wgt"));
    }

    /*
     * Tests declaring required properties, new and inherited.
     */
    #[widget2($crate::widget_tests::required_properties_wgt)]
    pub mod required_properties_wgt {
        inherit!(super::foo_mixin);

        properties! {
            super::util::trace = required!;
            foo_trace = required!;
        }
    }
    #[test]
    pub fn wgt_required_property() {
        let mut required = required_properties_wgt!(
            trace = "required!";
            foo_trace = "required2!"
        );
        required.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&required, "required!"));
        assert!(util::traced(&required, "required2!"));
    }

    // Mixin used in inherit required tests.
    #[widget_mixin2($crate::widget_tests::required_mixin)]
    pub mod required_mixin {
        properties! {
            super::util::trace as required_trace = required!;
        }
    }

    /*
     * Tests inheriting a required property.
     */
    #[widget2($crate::widget_tests::required_inherited_wgt)]
    pub mod required_inherited_wgt {
        inherit!(super::required_mixin);
    }
    #[test]
    pub fn wgt_required_inherited() {
        let mut required = required_inherited_wgt! {
            required_trace = "required!";// removing this line must cause a compile error.
        };
        required.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&required, "required!"))
    }

    /*
     * Tests inheriting a required property and setting it with a default value.
     */
    #[widget2($crate::widget_tests::required_inherited_default_wgt)]
    pub mod required_inherited_default_wgt {
        inherit!(super::required_mixin);

        properties! {
            //required_trace = unset!; // this line must cause a compile error.
            required_trace = "required_inherited_default_wgt";
        }
    }
    #[widget2($crate::widget_tests::required_inherited_default_depth2_wgt)]
    pub mod required_inherited_default_depth2_wgt {
        inherit!(super::required_inherited_default_wgt);

        properties! {
            //required_trace = unset!; // this line must cause a compile error.
        }
    }
    #[test]
    pub fn wgt_required_inherited_default() {
        let mut required = required_inherited_default_wgt! {
            //required_trace = unset!; // uncommenting this line must cause a compile error.
        };
        required.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&required, "required_inherited_default_wgt"));

        let mut required2 = required_inherited_default_depth2_wgt! {
            //required_trace = unset!; // uncommenting this line must cause a compile error.
        };
        required2.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&required2, "required_inherited_default_wgt"));
    }

    /*
     * Tests inheriting a required property with default value changing to required without value.
     */
    #[widget2($crate::widget_tests::required_inherited_default_required_wgt)]
    pub mod required_inherited_default_required_wgt {
        inherit!(super::required_inherited_default_wgt);

        properties! {
            required_trace = required!;
        }
    }
    #[test]
    pub fn wgt_required_inherited_default_required() {
        let mut required = required_inherited_default_required_wgt! {
            required_trace = "required!"; // commenting this line must cause a compile error.
        };
        required.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&required, "required!"))
    }

    /*
     * Tests value initialization order.
     */
    #[test]
    #[serial(priority)]
    pub fn value_init_order() {
        Position::reset();
        let mut wgt = empty_wgt! {
            util::count_inner = Position::next("count_inner");
            util::count_context = Position::next("count_context");
        };
        wgt.test_init(&mut TestWidgetContext::wait_new());

        // values evaluated in typed order.
        assert_eq!(util::sorted_pos(&wgt), ["count_inner", "count_context"]);

        // but properties init in the priority order.
        assert_eq!(util::sorted_init_count(&wgt), ["count_context", "count_inner"]);
    }

    /*
     * Tests value initialization order with child property.
     */
    #[widget2($crate::widget_tests::child_property_wgt)]
    pub mod child_property_wgt {
        properties! {
            child {
                super::util::count_inner as count_child_inner;
            }
        }
    }
    #[test]
    #[serial(priority)]
    pub fn wgt_child_property_init_order() {
        Position::reset();
        let mut wgt = child_property_wgt! {
            util::count_inner = Position::next("count_inner");
            count_child_inner = Position::next("count_child_inner");
            util::count_context = Position::next("count_context");
        };
        wgt.test_init(&mut TestWidgetContext::wait_new());

        // values evaluated in typed order.
        assert_eq!(util::sorted_pos(&wgt), ["count_inner", "count_child_inner", "count_context"]);

        // but properties init in the priority order (child first).
        assert_eq!(util::sorted_init_count(&wgt), ["count_context", "count_inner", "count_child_inner"]);
    }

    /*
     * Tests the ordering of properties of the same priority.
     */
    #[widget2($crate::widget_tests::same_priority_order_wgt)]
    pub mod same_priority_order_wgt {
        properties! {
            super::util::count_inner as inner_a;
            super::util::count_inner as inner_b;
        }
    }
    #[test]
    #[serial(priority)]
    pub fn wgt_same_priority_order() {
        Position::reset();
        let mut wgt = same_priority_order_wgt! {
            inner_a = Position::next("inner_a");
            inner_b = Position::next("inner_b");
        };
        wgt.test_init(&mut TestWidgetContext::wait_new());

        // values evaluated in typed order.
        assert_eq!(util::sorted_pos(&wgt), ["inner_a", "inner_b"]);

        // properties with the same priority are set in the typed order.
        // inner_a is set before inner_b who therefore contains inner_a:
        // let node = inner_a(child, ..);
        // let node = inner_b(node, ..);
        assert_eq!(util::sorted_init_count(&wgt), ["inner_b", "inner_a"]);

        Position::reset();
        // order of declaration doesn't impact the order of evaluation,
        // only the order of use does.
        let mut wgt = same_priority_order_wgt! {
            inner_b = Position::next("inner_b");
            inner_a = Position::next("inner_a");
        };
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert_eq!(util::sorted_pos(&wgt), ["inner_b", "inner_a"]);
        assert_eq!(util::sorted_init_count(&wgt), ["inner_a", "inner_b"]);
    }

    /*
     *  Tests widget when.
     */
    #[widget2($crate::widget_tests::when_wgt)]
    pub mod when_wgt {
        use super::util::is_state;
        use super::util::live_trace as msg;

        properties! {
            msg = "boo!";

            when self.is_state {
                msg = "ok.";
            }
        }
    }
    #[test]
    pub fn wgt_when() {
        let mut wgt = when_wgt!();
        let mut ctx = TestWidgetContext::wait_new();
        wgt.test_init(&mut ctx);

        assert!(util::traced(&wgt, "boo!"));

        util::set_state(&mut wgt, true);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "ok."));

        util::set_state(&mut wgt, false);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "boo!"));
    }
    #[test]
    pub fn widget_user_when() {
        let mut wgt = empty_wgt! {
            util::live_trace = "A";

            when self.util::is_state {
                util::live_trace = "B";
            }
        };
        let mut ctx = TestWidgetContext::wait_new();
        wgt.test_init(&mut ctx);

        assert!(util::traced(&wgt, "A"));

        util::set_state(&mut wgt, true);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "B"));

        util::set_state(&mut wgt, false);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "A"));
    }

    /*
     * Tests multiple widget whens
     */
    #[widget2($crate::widget_tests::multi_when_wgt)]
    pub mod multi_when_wgt {
        use super::util::{is_state, live_trace as trace};
        properties! {
            trace = "default";
            when self.is_state {
                trace = "state_0";
            }
            when self.is_state {
                trace = "state_1";
            }
        }
    }
    #[test]
    pub fn wgt_multi_when() {
        let mut wgt = multi_when_wgt!();
        let mut ctx = TestWidgetContext::wait_new();
        wgt.test_init(&mut ctx);

        assert!(util::traced(&wgt, "default"));

        util::set_state(&mut wgt, true);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "state_1"));

        util::set_state(&mut wgt, false);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "default"));
    }

    /*
     * Tests widget property attributes.
     */
    #[widget2($crate::widget_tests::cfg_property_wgt)]
    pub mod cfg_property_wgt {
        use super::util::trace;

        properties! {
            // property not included in widget.
            #[cfg(never)]
            trace as never_trace = "never-trace";

            // suppress warning.
            #[allow(non_snake_case)]
            trace as always_trace = {
                let weird___name;
                weird___name = "always-trace";
                weird___name
            };
        }
    }
    #[test]
    pub fn wgt_cfg_property() {
        let mut wgt = cfg_property_wgt!();
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "always-trace"));
        assert!(!util::traced(&wgt, "never-trace"));
    }
    #[test]
    pub fn user_cfg_property() {
        #[allow(unused_imports)]
        use util::trace as never_trace;
        use util::trace as always_trace;
        let mut wgt = empty_wgt! {
            // property not set.
            #[cfg(never)]
            never_trace = "never-trace";

            // suppress warning.
            #[allow(non_snake_case)]
            always_trace = {
                let weird___name;
                weird___name = "always-trace";
                weird___name
            };
        };

        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "always-trace"));
        assert!(!util::traced(&wgt, "never-trace"));
    }

    /*
     * Tests widget when attributes.
     */
    #[widget2($crate::widget_tests::cfg_when_wgt)]
    pub mod cfg_when_wgt {
        use super::util::{is_state, live_trace};

        properties! {
            live_trace = "trace";

            // suppress warning in all assigns.
            #[allow(non_snake_case)]
            when self.is_state {
                live_trace = {
                    let weird___name;
                    weird___name = "is_state";
                    weird___name
                };
            }

            // when not applied.
            #[cfg(never)]
            when self.is_state {
                live_trace = "is_never_state";
            }
        }
    }
    #[test]
    pub fn wgt_cfg_when() {
        let mut wgt = cfg_when_wgt!();

        let mut ctx = TestWidgetContext::wait_new();
        wgt.test_init(&mut ctx);

        assert!(util::traced(&wgt, "trace"));

        util::set_state(&mut wgt, true);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "is_state"));

        util::set_state(&mut wgt, false);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "trace"));
    }

    #[test]
    pub fn user_cfg_when() {
        let mut wgt = empty_wgt! {
            util::live_trace = "trace";

            #[allow(non_snake_case)]
            when self.util::is_state {
                util::live_trace = {
                    let weird___name;
                    weird___name = "is_state";
                    weird___name
                };
            }

            #[cfg(never)]
            when self.util::is_state {
                util::live_trace = "is_never_state";
            }
        };

        let mut ctx = TestWidgetContext::wait_new();
        wgt.test_init(&mut ctx);

        assert!(util::traced(&wgt, "trace"));

        util::set_state(&mut wgt, true);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "is_state"));

        util::set_state(&mut wgt, false);
        wgt.test_update(&mut ctx);
        ctx.apply_updates();
        wgt.test_update(&mut ctx);

        assert!(util::traced(&wgt, "trace"));
    }

    /*
     *  Tests widget captures.
     */
    #[widget2($crate::widget_tests::capture_properties_wgt)]
    pub mod capture_properties_wgt {
        use super::util::trace;
        use crate::{UiNode, Widget, WidgetId};

        properties! {
            trace as new_child_trace = "new-child";
            trace as new_trace = "new";
            trace as property_trace = "property";
        }

        fn new_child(new_child_trace: &'static str) -> impl UiNode {
            let msg = match new_child_trace {
                "new-child" => "custom new_child",
                "user-new-child" => "custom new_child (user)",
                o => panic!("unexpected {:?}", o),
            };
            let node = crate::widget_base::default_widget_new_child();
            trace(node, msg)
        }

        fn new(node: impl UiNode, id: WidgetId, new_trace: &'static str) -> impl Widget {
            let msg = match new_trace {
                "new" => "custom new",
                "user-new" => "custom new (user)",
                o => panic!("unexpected {:?}", o),
            };
            let node = trace(node, msg);
            crate::widget_base::default_widget_new2(node, id)
        }
    }
    #[test]
    pub fn wgt_capture_properties() {
        let mut wgt = capture_properties_wgt!();
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "property"));
        assert!(util::traced(&wgt, "custom new_child"));
        assert!(util::traced(&wgt, "custom new"));

        assert!(!util::traced(&wgt, "new-child"));
        assert!(!util::traced(&wgt, "new"));
    }
    #[test]
    pub fn wgt_capture_properties_reassign() {
        let mut wgt = capture_properties_wgt! {
            //new_child_trace = unset!;// compile error here
            new_child_trace = "user-new-child";
            property_trace = "user-property";
            new_trace = "user-new";
        };
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "user-property"));
        assert!(util::traced(&wgt, "custom new_child (user)"));
        assert!(util::traced(&wgt, "custom new (user)"));

        assert!(!util::traced(&wgt, "new-child"));
        assert!(!util::traced(&wgt, "new"));
        assert!(!util::traced(&wgt, "user-new-child"));
        assert!(!util::traced(&wgt, "user-new"));
    }

    /*
     * Tests capture-only property declaration in widget.
     */
    #[widget2($crate::widget_tests::new_capture_property_wgt)]
    pub mod new_capture_property_wgt {
        use super::util::trace;
        use crate::UiNode;

        properties! {
            new_capture: &'static str = "new_capture-default";
        }

        fn new_child(new_capture: &'static str) -> impl UiNode {
            let msg = match new_capture {
                "new_capture-default" => "captured new_capture (default)",
                "new_capture-user" => "captured new_capture (user)",
                o => panic!("unexpected {:?}", o),
            };
            let node = crate::widget_base::default_widget_new_child();
            trace(node, msg)
        }
    }
    #[test]
    pub fn wgt_new_capture_property() {
        let mut wgt = new_capture_property_wgt!();
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "captured new_capture (default)"));
    }
    #[test]
    pub fn wgt_new_capture_property_reassign() {
        let mut wgt = new_capture_property_wgt! {
            new_capture = "new_capture-user"
        };
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "captured new_capture (user)"));
    }

    #[widget2($crate::widget_tests::new_capture_property_named_wgt)]
    pub mod new_capture_property_named_wgt {
        use super::util::trace;
        use crate::UiNode;

        properties! {
            new_capture: {
                name: &'static str,
                age: u32,
            } = "name", 42;
        }

        fn new_child(new_capture: (&'static str, u32)) -> impl UiNode {
            let msg = match new_capture {
                ("name", 42) => "captured new_capture (default)",
                ("eman", 24) => "captured new_capture (user)",
                o => panic!("unexpected {:?}", o),
            };
            let node = crate::widget_base::default_widget_new_child();
            trace(node, msg)
        }
    }
    #[test]
    pub fn wgt_new_capture_property_named() {
        let mut wgt = new_capture_property_named_wgt!();
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "captured new_capture (default)"));
    }
    #[test]
    pub fn wgt_new_capture_property_named_reassign() {
        let mut wgt = new_capture_property_named_wgt! {
            new_capture = {
                name: "eman",
                age: 24
            }
        };
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "captured new_capture (user)"));
    }

    /*
     * Tests external capture_only properties
     */
    #[widget2($crate::widget_tests::captured_property_wgt)]
    pub mod captured_property_wgt {
        use super::util::{capture_only_trace, trace};
        use crate::UiNode;

        properties! {
            capture_only_trace = "capture-default";
        }

        fn new_child(capture_only_trace: &'static str) -> impl UiNode {
            let msg = match capture_only_trace {
                "capture-default" => "captured capture (default)",
                "capture-user" => "captured capture (user)",
                o => panic!("unexpected {:?}", o),
            };
            let node = crate::widget_base::default_widget_new_child();
            trace(node, msg)
        }
    }
    #[test]
    pub fn wgt_captured_property() {
        let mut wgt = captured_property_wgt!();
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "captured capture (default)"));
    }
    #[test]
    pub fn wgt_captured_property_reassign() {
        let mut wgt = captured_property_wgt! {
            capture_only_trace = "capture-user"
        };
        wgt.test_init(&mut TestWidgetContext::wait_new());

        assert!(util::traced(&wgt, "captured capture (user)"));
    }

    mod util {
        use std::{
            collections::{HashMap, HashSet},
            sync::atomic::{self, AtomicU32},
        };

        use crate::{
            context::WidgetContext,
            impl_ui_node, property, state_key,
            var::{IntoVar, StateVar, Var},
            UiNode, Widget,
        };

        /// Insert `trace` in the widget state. Can be probed using [`traced`].
        #[property(context)]
        pub fn trace(child: impl UiNode, trace: &'static str) -> impl UiNode {
            TraceNode { child, trace }
        }

        /// Probe for a [`trace`] in the widget state.
        pub fn traced(wgt: &impl Widget, trace: &'static str) -> bool {
            wgt.state().get(TraceKey).map(|t| t.contains(trace)).unwrap_or_default()
        }

        state_key! {
            struct TraceKey: HashSet<&'static str>;
        }
        struct TraceNode<C: UiNode> {
            child: C,
            trace: &'static str,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for TraceNode<C> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.child.init(ctx);
                ctx.widget_state.entry(TraceKey).or_default().insert(self.trace);
            }
        }

        /// Insert `count` in the widget state. Can get using [`Count::get`].
        #[property(context)]
        pub fn count(child: impl UiNode, count: Position) -> impl UiNode {
            CountNode { child, count }
        }

        pub use count as count_context;

        /// Same as [`count`] but with `inner` priority.
        #[property(inner)]
        pub fn count_inner(child: impl UiNode, count: Position) -> impl UiNode {
            CountNode { child, count }
        }

        /// Count adds one every [`Self::next`] call.
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub struct Position(pub u32, pub &'static str);
        static COUNT: AtomicU32 = AtomicU32::new(0);
        static COUNT_INIT: AtomicU32 = AtomicU32::new(0);
        impl Position {
            pub fn next(tag: &'static str) -> Self {
                Position(COUNT.fetch_add(1, atomic::Ordering::AcqRel), tag)
            }

            fn next_init() -> u32 {
                COUNT_INIT.fetch_add(1, atomic::Ordering::AcqRel)
            }

            pub fn reset() {
                COUNT.store(0, atomic::Ordering::SeqCst);
                COUNT_INIT.store(0, atomic::Ordering::SeqCst);
            }
        }

        /// Gets the [`Position`] tags sorted by their number.
        pub fn sorted_pos(wgt: &impl Widget) -> Vec<&'static str> {
            wgt.state()
                .get(PositionKey)
                .map(|m| {
                    let mut vec: Vec<_> = m.iter().collect();
                    vec.sort_by_key(|(_, i)| *i);
                    vec.into_iter().map(|(&t, _)| t).collect::<Vec<_>>()
                })
                .unwrap_or_default()
        }

        /// Gets the [`Position`] tags sorted by their init position.
        pub fn sorted_init_count(wgt: &impl Widget) -> Vec<&'static str> {
            wgt.state()
                .get(InitPositionKey)
                .map(|m| {
                    let mut vec: Vec<_> = m.iter().collect();
                    vec.sort_by_key(|(_, i)| *i);
                    vec.into_iter().map(|(&t, _)| t).collect::<Vec<_>>()
                })
                .unwrap_or_default()
        }

        state_key! {
            struct PositionKey: HashMap<&'static str, u32>;
            struct InitPositionKey: HashMap<&'static str, u32>;
        }

        struct CountNode<C: UiNode> {
            child: C,
            count: Position,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for CountNode<C> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                ctx.widget_state
                    .entry(InitPositionKey)
                    .or_default()
                    .insert(self.count.1, Position::next_init());
                self.child.init(ctx);
                ctx.widget_state.entry(PositionKey).or_default().insert(self.count.1, self.count.0);
            }
        }

        /// Test state property, state can be set using [`set_state`] followed by updating.
        #[property(context)]
        pub fn is_state(child: impl UiNode, state: StateVar) -> impl UiNode {
            IsStateNode { child, state }
        }
        /// Sets the [`is_state`] of an widget.
        ///
        /// Note only applies after update.
        pub fn set_state(wgt: &mut impl Widget, state: bool) {
            *wgt.state_mut().entry(IsStateKey).or_default() = state;
        }
        struct IsStateNode<C: UiNode> {
            child: C,
            state: StateVar,
        }
        impl<C: UiNode> IsStateNode<C> {
            fn update_state(&mut self, ctx: &mut WidgetContext) {
                let wgt_state = ctx.widget_state.get(IsStateKey).copied().unwrap_or_default();
                if wgt_state != *self.state.get(ctx.vars) {
                    self.state.set(ctx.vars, wgt_state);
                }
            }
        }
        #[impl_ui_node(child)]
        impl<C: UiNode> UiNode for IsStateNode<C> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.child.init(ctx);
                self.update_state(ctx);
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                self.child.update(ctx);
                self.update_state(ctx);
            }
        }

        state_key! {
            struct IsStateKey: bool;
        }

        /// A [trace] that can update.
        #[property(context)]
        pub fn live_trace(child: impl UiNode, trace: impl IntoVar<&'static str>) -> impl UiNode {
            LiveTraceNode {
                child,
                trace: trace.into_var(),
            }
        }
        struct LiveTraceNode<C: UiNode, T: Var<&'static str>> {
            child: C,
            trace: T,
        }
        #[impl_ui_node(child)]
        impl<C: UiNode, T: Var<&'static str>> UiNode for LiveTraceNode<C, T> {
            fn init(&mut self, ctx: &mut WidgetContext) {
                self.child.init(ctx);
                ctx.widget_state.entry(TraceKey).or_default().insert(self.trace.get(ctx.vars));
            }

            fn update(&mut self, ctx: &mut WidgetContext) {
                self.child.update(ctx);
                if let Some(trace) = self.trace.get_new(ctx.vars) {
                    ctx.widget_state.entry(TraceKey).or_default().insert(trace);
                }
            }
        }

        /// A capture_only property.
        #[property(capture_only)]
        pub fn capture_only_trace(trace: &'static str) -> ! {}
    }
}
