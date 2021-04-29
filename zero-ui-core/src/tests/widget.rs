//! Tests for `#[widget(..)]` and `#[widget_mixin(..)]` macros.
//!
//! Note: Compile error tests are in the integration tests folder: `tests/build/widget` and `tests/build/widget_new`

use self::util::Position;
use crate::{context::TestWidgetContext, widget, widget_mixin, Widget, WidgetId};
use serial_test::serial;

// Used in multiple tests.
#[widget($crate::tests::widget::empty_wgt)]
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
#[widget_mixin($crate::tests::widget::foo_mixin)]
pub mod foo_mixin {
    use super::util;

    properties! {
        util::trace as foo_trace = "foo_mixin";
    }
}

/*
 * Tests the inherited properties' default values and assigns.
 */
#[widget($crate::tests::widget::bar_wgt)]
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
    let foo_trace = "foo!";
    let mut default = bar_wgt! {
        foo_trace; // shorthand assign test.
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
#[widget($crate::tests::widget::reset_wgt)]
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
#[widget($crate::tests::widget::alias_inherit_wgt)]
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
#[widget($crate::tests::widget::property_from_path_wgt)]
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
 * Tests removing inherited property.
 */
#[widget($crate::tests::widget::remove_property_wgt)]
pub mod remove_property_wgt {
    inherit!(super::foo_mixin);

    properties! {
        remove { foo_trace }
    }
}
#[test]
pub fn wgt_remove_property() {
    let mut default = remove_property_wgt!();
    default.test_init(&mut TestWidgetContext::wait_new());

    assert!(!util::traced(&default, "foo_mixin"));
}

/*
 * Test unsetting default value.
 */
#[widget($crate::tests::widget::default_value_wgt)]
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
#[widget($crate::tests::widget::required_properties_wgt)]
pub mod required_properties_wgt {
    inherit!(super::foo_mixin);

    properties! {
        #[required]
        super::util::trace;
        #[required]
        foo_trace;
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
#[widget_mixin($crate::tests::widget::required_mixin)]
pub mod required_mixin {
    properties! {
        #[required]
        super::util::trace as required_trace;
    }
}

/*
 * Tests inheriting a required property.
 */
#[widget($crate::tests::widget::required_inherited_wgt)]
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
#[widget($crate::tests::widget::required_inherited_default_wgt)]
pub mod required_inherited_default_wgt {
    inherit!(super::required_mixin);

    properties! {
        required_trace = "required_inherited_default_wgt";
    }
}
#[widget($crate::tests::widget::required_inherited_default_depth2_wgt)]
pub mod required_inherited_default_depth2_wgt {
    inherit!(super::required_inherited_default_wgt);

    properties! {
        //remove { required_trace } // this line must cause a compile error.
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
#[widget($crate::tests::widget::required_inherited_default_required_wgt)]
pub mod required_inherited_default_required_wgt {
    inherit!(super::required_inherited_default_wgt);

    properties! {
        #[required]
        required_trace;
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
    assert_eq!(util::sorted_value_init(&wgt), ["count_inner", "count_context"]);

    // but properties init in the priority order.
    assert_eq!(util::sorted_node_init(&wgt), ["count_context", "count_inner"]);
}

/*
 * Tests value initialization order with child property.
 */
#[widget($crate::tests::widget::child_property_wgt)]
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
    assert_eq!(util::sorted_value_init(&wgt), ["count_inner", "count_child_inner", "count_context"]);

    // but properties init in the priority order (child first).
    assert_eq!(util::sorted_node_init(&wgt), ["count_context", "count_inner", "count_child_inner"]);
}

/*
 * Tests the ordering of properties of the same priority.
 */
#[widget($crate::tests::widget::same_priority_order_wgt)]
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
    assert_eq!(util::sorted_value_init(&wgt), ["inner_a", "inner_b"]);

    // properties with the same priority are set in reversed typed order.
    // inner_a is set after inner_b so it will contain inner_b:
    // let node = inner_b(node, ..);
    // let node = inner_a(child, ..);
    assert_eq!(util::sorted_node_init(&wgt), ["inner_a", "inner_b"]);

    Position::reset();
    // order of declaration(in the widget) doesn't impact the order of evaluation,
    // only the order of use does (in here).
    let mut wgt = same_priority_order_wgt! {
        inner_b = Position::next("inner_b");
        inner_a = Position::next("inner_a");
    };
    wgt.test_init(&mut TestWidgetContext::wait_new());

    assert_eq!(util::sorted_value_init(&wgt), ["inner_b", "inner_a"]);
    assert_eq!(util::sorted_node_init(&wgt), ["inner_b", "inner_a"]);
}

/*
 *  Tests widget when.
 */
#[widget($crate::tests::widget::when_wgt)]
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
#[widget($crate::tests::widget::multi_when_wgt)]
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
#[widget($crate::tests::widget::cfg_property_wgt)]
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
#[widget($crate::tests::widget::cfg_when_wgt)]
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
#[widget($crate::tests::widget::capture_properties_wgt)]
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
        let node = crate::widget_base::implicit_base::new_child();
        trace(node, msg)
    }

    fn new(node: impl UiNode, id: WidgetId, new_trace: &'static str) -> impl Widget {
        let msg = match new_trace {
            "new" => "custom new",
            "user-new" => "custom new (user)",
            o => panic!("unexpected {:?}", o),
        };
        let node = trace(node, msg);
        crate::widget_base::implicit_base::new(node, id)
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
#[widget($crate::tests::widget::new_capture_property_wgt)]
pub mod new_capture_property_wgt {
    use super::util::trace;
    use crate::UiNode;

    properties! {
        #[allowed_in_when = false]
        new_capture(&'static str, u32) = "new_capture-default", 42;
    }

    fn new_child(new_capture: (&'static str, u32)) -> impl UiNode {
        let msg = match new_capture {
            ("new_capture-default", 42) => "captured new_capture (default)",
            ("new_capture-user", 24) => "captured new_capture (user)",
            o => panic!("unexpected {:?}", o),
        };
        let node = crate::widget_base::implicit_base::new_child();
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
        new_capture = "new_capture-user", 24
    };
    wgt.test_init(&mut TestWidgetContext::wait_new());

    assert!(util::traced(&wgt, "captured new_capture (user)"));
}

#[widget($crate::tests::widget::new_capture_property_named_wgt)]
pub mod new_capture_property_named_wgt {
    use super::util::trace;
    use crate::UiNode;

    properties! {
        #[allowed_in_when = false]
        new_capture(name: &'static str, age: u32) = "name", 42;
    }

    fn new_child(new_capture: (&'static str, u32)) -> impl UiNode {
        let msg = match new_capture {
            ("name", 42) => "captured new_capture (default)",
            ("eman", 24) => "captured new_capture (user)",
            o => panic!("unexpected {:?}", o),
        };
        let node = crate::widget_base::implicit_base::new_child();
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
#[widget($crate::tests::widget::captured_property_wgt)]
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
        let node = crate::widget_base::implicit_base::new_child();
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

/*
 * Tests order properties are inited and applied.
 */

#[widget($crate::tests::widget::property_priority_sorting_wgt)]
pub mod property_priority_sorting_wgt {
    use super::util::{count_context, count_inner, count_outer, count_size, on_count};

    properties! {
        count_inner as count_inner2;
        count_size as count_size2;
        count_outer as count_outer2;
        on_count as count_event2;
        count_context as count_context2;

        count_inner as count_inner1;
        count_size as count_size1;
        count_outer as count_outer1;
        on_count as count_event1;
        count_context as count_context1;

        child {
            count_inner as child_count_inner2;
            count_size as child_count_size2;
            count_outer as child_count_outer2;
            on_count as child_count_event2;
            count_context as child_count_context2;

            count_inner as child_count_inner1;
            count_size as child_count_size1;
            count_outer as child_count_outer1;
            on_count as child_count_event1;
            count_context as child_count_context1;
        }
    }
}
fn property_priority_sorting_init1() -> impl Widget {
    property_priority_sorting_wgt! {
        count_inner1 = Position::next("count_inner1");
        count_inner2 = Position::next("count_inner2");
        count_size1 = Position::next("count_size1");
        count_size2 = Position::next("count_size2");
        count_outer1 = Position::next("count_outer1");
        count_outer2 = Position::next("count_outer2");
        count_event1 = Position::next("count_event1");
        count_event2 = Position::next("count_event2");
        count_context1 = Position::next("count_context1");
        count_context2 = Position::next("count_context2");

        child_count_inner1 = Position::next("child_count_inner1");
        child_count_inner2 = Position::next("child_count_inner2");
        child_count_size1 = Position::next("child_count_size1");
        child_count_size2 = Position::next("child_count_size2");
        child_count_outer1 = Position::next("child_count_outer1");
        child_count_outer2 = Position::next("child_count_outer2");
        child_count_event1 = Position::next("child_count_event1");
        child_count_event2 = Position::next("child_count_event2");
        child_count_context1 = Position::next("child_count_context1");
        child_count_context2 = Position::next("child_count_context2");
    }
}
#[test]
#[serial(priority)]
pub fn property_priority_sorting_value_init1() {
    Position::reset();

    let mut wgt = property_priority_sorting_init1();
    wgt.test_init(&mut TestWidgetContext::wait_new());

    // assert that value init is the same as typed.
    assert_eq!(
        util::sorted_value_init(&wgt),
        [
            "count_inner1",
            "count_inner2",
            "count_size1",
            "count_size2",
            "count_outer1",
            "count_outer2",
            "count_event1",
            "count_event2",
            "count_context1",
            "count_context2",
            "child_count_inner1",
            "child_count_inner2",
            "child_count_size1",
            "child_count_size2",
            "child_count_outer1",
            "child_count_outer2",
            "child_count_event1",
            "child_count_event2",
            "child_count_context1",
            "child_count_context2",
        ]
    );
}
fn property_priority_sorting_init2() -> impl Widget {
    property_priority_sorting_wgt! {
        child_count_context1 = Position::next("child_count_context1");
        child_count_context2 = Position::next("child_count_context2");
        child_count_event1 = Position::next("child_count_event1");
        child_count_event2 = Position::next("child_count_event2");
        child_count_outer1 = Position::next("child_count_outer1");
        child_count_outer2 = Position::next("child_count_outer2");
        child_count_size1 = Position::next("child_count_size1");
        child_count_size2 = Position::next("child_count_size2");
        child_count_inner1 = Position::next("child_count_inner1");
        child_count_inner2 = Position::next("child_count_inner2");

        count_context1 = Position::next("count_context1");
        count_context2 = Position::next("count_context2");
        count_event1 = Position::next("count_event1");
        count_event2 = Position::next("count_event2");
        count_outer1 = Position::next("count_outer1");
        count_outer2 = Position::next("count_outer2");
        count_size1 = Position::next("count_size1");
        count_size2 = Position::next("count_size2");
        count_inner1 = Position::next("count_inner1");
        count_inner2 = Position::next("count_inner2");
    }
}
#[test]
#[serial(priority)]
pub fn property_priority_sorting_value_init2() {
    Position::reset();

    let mut wgt = property_priority_sorting_init2();
    wgt.test_init(&mut TestWidgetContext::wait_new());

    // assert that value init is the same as typed.
    assert_eq!(
        util::sorted_value_init(&wgt),
        [
            "child_count_context1",
            "child_count_context2",
            "child_count_event1",
            "child_count_event2",
            "child_count_outer1",
            "child_count_outer2",
            "child_count_size1",
            "child_count_size2",
            "child_count_inner1",
            "child_count_inner2",
            "count_context1",
            "count_context2",
            "count_event1",
            "count_event2",
            "count_outer1",
            "count_outer2",
            "count_size1",
            "count_size2",
            "count_inner1",
            "count_inner2",
        ]
    );
}
fn assert_node_order(wgt: &impl Widget) {
    // assert that `UiNode::init` position is sorted by `child` and
    // property priorities, followed by the typed position.
    assert_eq!(
        util::sorted_node_init(wgt),
        [
            // each property wraps the next one and takes a position number before
            // delegating to the next property (child node).
            "count_context1",
            "count_context2",
            "count_event1",
            "count_event2",
            "count_outer1",
            "count_outer2",
            "count_size1",
            "count_size2",
            "count_inner1",
            "count_inner2",
            "child_count_context1",
            "child_count_context2",
            "child_count_event1",
            "child_count_event2",
            "child_count_outer1",
            "child_count_outer2",
            "child_count_size1",
            "child_count_size2",
            "child_count_inner1",
            "child_count_inner2",
        ]
    );
}
#[test]
#[serial(priority)]
pub fn property_priority_sorting_node_init1() {
    Position::reset();

    let mut wgt = property_priority_sorting_init1();
    wgt.test_init(&mut TestWidgetContext::wait_new());

    assert_node_order(&wgt);
}
#[test]
#[serial(priority)]
pub fn property_priority_sorting_node_init2() {
    Position::reset();

    let mut wgt = property_priority_sorting_init2();
    wgt.test_init(&mut TestWidgetContext::wait_new());

    assert_node_order(&wgt);
}
#[widget($crate::tests::widget::property_priority_sorting_inherited_wgt)]
pub mod property_priority_sorting_inherited_wgt {
    inherit!(super::property_priority_sorting_wgt);
}
#[test]
#[serial(priority)]
pub fn property_priority_sorting_node_inherited_init() {
    Position::reset();

    let mut wgt = property_priority_sorting_inherited_wgt! {
        child_count_context1 = Position::next("child_count_context1");
        child_count_context2 = Position::next("child_count_context2");
        child_count_event1 = Position::next("child_count_event1");
        child_count_event2 = Position::next("child_count_event2");
        child_count_outer1 = Position::next("child_count_outer1");
        child_count_outer2 = Position::next("child_count_outer2");
        child_count_size1 = Position::next("child_count_size1");
        child_count_size2 = Position::next("child_count_size2");
        child_count_inner1 = Position::next("child_count_inner1");
        child_count_inner2 = Position::next("child_count_inner2");

        count_context1 = Position::next("count_context1");
        count_context2 = Position::next("count_context2");
        count_event1 = Position::next("count_event1");
        count_event2 = Position::next("count_event2");
        count_outer1 = Position::next("count_outer1");
        count_outer2 = Position::next("count_outer2");
        count_size1 = Position::next("count_size1");
        count_size2 = Position::next("count_size2");
        count_inner1 = Position::next("count_inner1");
        count_inner2 = Position::next("count_inner2");
    };
    wgt.test_init(&mut TestWidgetContext::wait_new());

    assert_node_order(&wgt);
}

#[widget($crate::tests::widget::property_priority_sorting_defaults_wgt)]
pub mod property_priority_sorting_defaults_wgt {
    use super::util::Position;
    inherit!(super::property_priority_sorting_wgt);

    properties! {
        count_context1 = Position::next("count_context1");
        count_context2 = Position::next("count_context2");
        count_event1 = Position::next("count_event1");
        count_event2 = Position::next("count_event2");
        count_outer1 = Position::next("count_outer1");
        count_outer2 = Position::next("count_outer2");
        count_size1 = Position::next("count_size1");
        count_size2 = Position::next("count_size2");
        count_inner1 = Position::next("count_inner1");
        count_inner2 = Position::next("count_inner2");

        child {
            child_count_context1 = Position::next("child_count_context1");
            child_count_context2 = Position::next("child_count_context2");
            child_count_event1 = Position::next("child_count_event1");
            child_count_event2 = Position::next("child_count_event2");
            child_count_outer1 = Position::next("child_count_outer1");
            child_count_outer2 = Position::next("child_count_outer2");
            child_count_size1 = Position::next("child_count_size1");
            child_count_size2 = Position::next("child_count_size2");
            child_count_inner1 = Position::next("child_count_inner1");
            child_count_inner2 = Position::next("child_count_inner2");
        }
    }
}
#[test]
#[serial(priority)]
pub fn property_priority_sorting_defaults() {
    Position::reset();

    let mut wgt = property_priority_sorting_defaults_wgt!();
    wgt.test_init(&mut TestWidgetContext::wait_new());
    assert_node_order(&wgt);
}

/*
 * Tests property member access in when
 */

#[test]
pub fn when_property_member_default() {
    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(self.util::duo_members, "a");
           true
       } {
           util::live_trace = "true";
       }
    };

    let mut ctx = TestWidgetContext::wait_new();
    wgt.test_init(&mut ctx);
    assert!(util::traced(&wgt, "true"));
}

#[test]
pub fn when_property_member_index() {
    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(self.util::duo_members.0, "a");
           assert_eq!(self.util::duo_members.1, "b");
           true
       } {
           util::live_trace = "true";
       }
    };

    let mut ctx = TestWidgetContext::wait_new();
    wgt.test_init(&mut ctx);
    assert!(util::traced(&wgt, "true"));
}

#[test]
pub fn when_property_member_named() {
    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(self.util::duo_members.member_a, "a");
           assert_eq!(self.util::duo_members.member_b, "b");
           true
       } {
           util::live_trace = "true";
       }
    };

    let mut ctx = TestWidgetContext::wait_new();
    wgt.test_init(&mut ctx);
    assert!(util::traced(&wgt, "true"));
}

#[test]
pub fn when_property_member_default_method() {
    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(self.util::duo_members.len(), 1);
           true
       } {
           util::live_trace = "true";
       }
    };

    let mut ctx = TestWidgetContext::wait_new();
    wgt.test_init(&mut ctx);
    assert!(util::traced(&wgt, "true"));
}

#[test]
pub fn when_property_member_indexed_method() {
    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(self.util::duo_members.0.len(), 1);
           true
       } {
           util::live_trace = "true";
       }
    };

    let mut ctx = TestWidgetContext::wait_new();
    wgt.test_init(&mut ctx);
    assert!(util::traced(&wgt, "true"));
}

/*
* Inherit override
*/
#[widget_mixin($crate::tests::widget::inherit_override_a)]
pub mod inherit_override_a {
    use super::util::trace;

    properties! {
        trace = "base_a::property";
    }
}
#[widget_mixin($crate::tests::widget::inherit_override_b)]
pub mod inherit_override_b {
    use super::util::trace;

    properties! {
        trace = "base_b::property";
    }
}
#[widget($crate::tests::widget::inherit_override_wgt1)]
pub mod inherit_override_wgt1 {
    inherit!(super::inherit_override_a);
    inherit!(super::inherit_override_b);
}
#[widget($crate::tests::widget::inherit_override_wgt2)]
pub mod inherit_override_wgt2 {
    inherit!(super::inherit_override_b);
    inherit!(super::inherit_override_a);
}
#[test]
pub fn inherit_override() {
    let mut wgt = inherit_override_wgt1!();

    let mut ctx = TestWidgetContext::wait_new();
    wgt.test_init(&mut ctx);
    assert!(util::traced(&wgt, "base_b::property"));
    assert!(!util::traced(&wgt, "base_a::property"));

    let mut wgt = inherit_override_wgt2!();

    wgt.test_init(&mut ctx);
    assert!(!util::traced(&wgt, "base_b::property"));
    assert!(util::traced(&wgt, "base_a::property"));
}

/*
* Property Default Value
*/

#[test]
pub fn allowed_in_when_without_wgt_assign() {
    let mut wgt = empty_wgt! {
        // util::live_trace_default = "default-trace";
        when self.util::is_state {
            util::live_trace_default = "when-trace";
        }
    };

    let mut ctx = TestWidgetContext::wait_new();
    wgt.test_init(&mut ctx);
    assert!(util::traced(&wgt, "default-trace"));
    assert!(!util::traced(&wgt, "when-trace"));

    util::set_state(&mut wgt, true);
    wgt.test_update(&mut ctx);
    ctx.apply_updates();
    wgt.test_update(&mut ctx);
    assert!(util::traced(&wgt, "when-trace"));
}

/*
* Generated Names Don't Shadow Each Other
*/
#[crate::property(context)]
pub fn util_live_trace(child: impl crate::UiNode, not_str: impl crate::var::IntoVar<bool>) -> impl crate::UiNode {
    let _ = not_str;
    child
}

#[test]
pub fn generated_name_collision() {
    let _ = empty_wgt! {
        util::live_trace = "!";
        util_live_trace = false;
    };
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
    #[property(context, allowed_in_when = false)]
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
    #[property(context, allowed_in_when = false)]
    pub fn count(child: impl UiNode, count: Position) -> impl UiNode {
        CountNode { child, value_pos: count }
    }

    pub use count as count_context;

    /// Same as [`count`] but with `inner` priority.
    #[property(inner, allowed_in_when = false)]
    pub fn count_inner(child: impl UiNode, count: Position) -> impl UiNode {
        CountNode { child, value_pos: count }
    }

    /// Same as [`count`] but with `outer` priority.
    #[property(outer, allowed_in_when = false)]
    pub fn count_outer(child: impl UiNode, count: Position) -> impl UiNode {
        CountNode { child, value_pos: count }
    }

    /// Same as [`count`] but with `size` priority.
    #[property(size, allowed_in_when = false)]
    pub fn count_size(child: impl UiNode, count: Position) -> impl UiNode {
        CountNode { child, value_pos: count }
    }

    /// Same as [`count`] but with `event` priority.
    #[property(event, allowed_in_when = false)]
    pub fn on_count(child: impl UiNode, count: Position) -> impl UiNode {
        CountNode { child, value_pos: count }
    }

    /// Count adds one every [`Self::next`] call.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Position {
        pub pos: u32,
        pub tag: &'static str,
    }
    static COUNT: AtomicU32 = AtomicU32::new(0);
    static COUNT_INIT: AtomicU32 = AtomicU32::new(0);
    impl Position {
        pub fn next(tag: &'static str) -> Self {
            Position {
                pos: COUNT.fetch_add(1, atomic::Ordering::AcqRel),
                tag,
            }
        }

        fn next_init() -> u32 {
            COUNT_INIT.fetch_add(1, atomic::Ordering::AcqRel)
        }

        pub fn reset() {
            COUNT.store(0, atomic::Ordering::SeqCst);
            COUNT_INIT.store(0, atomic::Ordering::SeqCst);
        }
    }

    /// Gets the [`Position`] tags sorted by call to [`Position::next`].
    pub fn sorted_value_init(wgt: &impl Widget) -> Vec<&'static str> {
        wgt.state()
            .get(ValuePositionKey)
            .map(|m| {
                let mut vec: Vec<_> = m.iter().collect();
                vec.sort_by_key(|(_, i)| *i);
                vec.into_iter().map(|(&t, _)| t).collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    /// Gets the [`Position`] tags sorted by the [`UiNode::init` call.
    pub fn sorted_node_init(wgt: &impl Widget) -> Vec<&'static str> {
        wgt.state()
            .get(NodePositionKey)
            .map(|m| {
                let mut vec: Vec<_> = m.iter().collect();
                vec.sort_by_key(|(_, i)| *i);
                vec.into_iter().map(|(&t, _)| t).collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    state_key! {
        struct ValuePositionKey: HashMap<&'static str, u32>;
        struct NodePositionKey: HashMap<&'static str, u32>;
    }

    struct CountNode<C: UiNode> {
        child: C,
        value_pos: Position,
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for CountNode<C> {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.widget_state
                .entry(ValuePositionKey)
                .or_default()
                .insert(self.value_pos.tag, self.value_pos.pos);

            ctx.widget_state
                .entry(NodePositionKey)
                .or_default()
                .insert(self.value_pos.tag, Position::next_init());

            self.child.init(ctx);
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
    /// A [trace] that can update and has a default value of `"default-trace"`.
    #[property(context, default("default-trace"))]
    pub fn live_trace_default(child: impl UiNode, trace: impl IntoVar<&'static str>) -> impl UiNode {
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
    #[property(capture_only, allowed_in_when = false)]
    pub fn capture_only_trace(trace: &'static str) -> ! {}

    #[property(context)]
    pub fn duo_members(child: impl UiNode, member_a: impl IntoVar<&'static str>, member_b: impl IntoVar<&'static str>) -> impl UiNode {
        let _ = member_a;
        let _ = member_b;
        child
    }
}
