//! Tests for `#[widget(..)]`  macro.
//!
//! Note: Compile error tests are in the integration tests folder: `tests/build/widget` and `tests/build/widget_new`

use self::util::Position;
use crate::{
    context::TestWidgetContext,
    var::Var,
    widget,
    widget_instance::{UiNode, WidgetId},
    widget_mixin,
};

// Used in multiple tests.
#[widget($crate::tests::widget::empty_wgt)]
pub mod empty_wgt {
    inherit!(crate::widget_base::base);
}

/*
 * Tests the implicitly inherited properties.
 */
#[test]
pub fn implicit_inherited() {
    let expected = WidgetId::new_unique();
    let wgt = empty_wgt! {
        id = expected;
    };
    let actual = wgt.with_context(|w| w.id).expect("expected widget");
    assert_eq!(expected, actual);
}

// Mixin used in inherit tests.
#[widget_mixin($crate::tests::widget::foo_mixin)]
pub mod foo_mixin {
    use super::util;

    properties! {
        pub util::trace as foo_trace = "foo_mixin";
    }
}

/*
 * Tests the inherited properties' default values and assigns.
 */
#[widget($crate::tests::widget::bar_wgt)]
pub mod bar_wgt {
    use super::{foo_mixin, util};

    inherit!(crate::widget_base::base);
    inherit!(foo_mixin);

    properties! {
        pub util::trace as bar_trace = "bar_wgt";
    }
}
#[test]
pub fn wgt_with_mixin_default_values() {
    let mut ctx = TestWidgetContext::new();

    let mut default = bar_wgt!();
    ctx.init(&mut default);

    // test default values used.
    assert!(util::traced(&default, "foo_mixin"));
    assert!(util::traced(&default, "bar_wgt"));
}
#[test]
pub fn wgt_with_mixin_assign_values() {
    let mut ctx = TestWidgetContext::new();

    let foo_trace = "foo!";
    let mut default = bar_wgt! {
        foo_trace; // shorthand assign test.
        bar_trace = "bar!";
    };
    ctx.init(&mut default);

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
    inherit!(crate::widget_base::base);
    inherit!(super::foo_mixin);

    properties! {
        foo_trace = "reset_wgt"
    }
}
#[test]
pub fn wgt_with_new_value_for_inherited() {
    let mut ctx = TestWidgetContext::new();

    let mut default = reset_wgt!();
    ctx.init(&mut default);

    assert!(util::traced(&default, "reset_wgt"));
    assert!(!util::traced(&default, "foo_mixin"));
}

/*
 * Tests new property from inherited property.
 */
#[widget($crate::tests::widget::alias_inherit_wgt)]
pub mod alias_inherit_wgt {
    inherit!(crate::widget_base::base);
    inherit!(super::foo_mixin);

    properties! {
        pub super::foo_mixin::foo_trace as alias_trace = "alias_inherit_wgt"
    }
}
#[test]
pub fn wgt_alias_inherit() {
    let mut ctx = TestWidgetContext::new();

    let mut default = alias_inherit_wgt!();
    ctx.init(&mut default);

    assert!(util::traced(&default, "foo_mixin"));
    assert!(util::traced(&default, "alias_inherit_wgt"));

    let mut assigned = alias_inherit_wgt!(
        foo_trace = "foo!";
        alias_trace = "alias!";
    );
    ctx.init(&mut assigned);

    assert!(util::traced(&assigned, "foo!"));
    assert!(util::traced(&assigned, "alias!"));
}

/*
 * Tests the property name when declared from path.
 */
#[widget($crate::tests::widget::property_from_path_wgt)]
pub mod property_from_path_wgt {
    inherit!(crate::widget_base::base);

    properties! {
        pub super::util::trace;
    }
}
#[test]
pub fn wgt_property_from_path() {
    let mut ctx = TestWidgetContext::new();

    let mut assigned = property_from_path_wgt!(
        trace = "trace!";
    );
    ctx.init(&mut assigned);

    assert!(util::traced(&assigned, "trace!"));
}

/*
 * Test unsetting default value.
 */
#[widget($crate::tests::widget::default_value_wgt)]
pub mod default_value_wgt {
    inherit!(crate::widget_base::base);

    properties! {
        pub super::util::trace = "default_value_wgt";
    }
}
#[test]
pub fn unset_default_value() {
    let mut ctx = TestWidgetContext::new();

    let mut default = default_value_wgt!();
    ctx.init(&mut default);

    assert!(util::traced(&default, "default_value_wgt"));

    let mut no_default = default_value_wgt! {
        trace = unset!;
    };
    ctx.init(&mut no_default);

    assert!(!util::traced(&no_default, "default_value_wgt"));
}

/*
 * Tests value initialization order.
 */
#[test]
pub fn value_init_order() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();
    let mut wgt = empty_wgt! {
        util::count_border = Position::next("count_border");
        util::count_context = Position::next("count_context");
    };
    ctx.init(&mut wgt);

    // values evaluated in typed order.
    assert_eq!(util::sorted_value_init(&wgt), ["count_border", "count_context"]);

    // but properties init in the nest group order.
    assert_eq!(util::sorted_node_init(&wgt), ["count_context", "count_border"]);
}

#[test]
pub fn wgt_child_property_init_order() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();
    let mut wgt = empty_wgt! {
        util::count_border = Position::next("count_border");
        util::count_child_layout = Position::next("count_child_layout");
        util::count_context = Position::next("count_context");
    };
    ctx.init(&mut wgt);

    // values evaluated in typed order.
    assert_eq!(
        util::sorted_value_init(&wgt),
        ["count_border", "count_child_layout", "count_context"]
    );

    // but properties init in the nest group order (child first).
    assert_eq!(
        util::sorted_node_init(&wgt),
        ["count_context", "count_border", "count_child_layout"]
    );
}

/*
 * Tests the ordering of properties of the same nest group.
 */
#[widget($crate::tests::widget::same_nest_group_order_wgt)]
pub mod same_nest_group_order_wgt {
    inherit!(crate::widget_base::base);

    properties! {
        pub super::util::count_border as border_a;
        pub super::util::count_border as border_b;
    }
}
#[test]
pub fn wgt_same_nest_group_order() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();
    let mut wgt = same_nest_group_order_wgt! {
        border_a = Position::next("border_a");
        border_b = Position::next("border_b");
    };
    ctx.init(&mut wgt);

    // values evaluated in typed order.
    assert_eq!(util::sorted_value_init(&wgt), ["border_a", "border_b"]);

    // properties with the same nest group are set in reversed typed order.
    // inner_a is set after inner_b so it will contain inner_b:
    // let node = border_b(child, ..);
    // let node = border_a(node, ..);
    assert_eq!(util::sorted_node_init(&wgt), ["border_a", "border_b"]);

    Position::reset();
    // order of declaration(in the widget) doesn't impact the order of evaluation,
    // only the order of use does (in here).
    let mut wgt = same_nest_group_order_wgt! {
        border_b = Position::next("border_b");
        border_a = Position::next("border_a");
    };
    ctx.init(&mut wgt);

    assert_eq!(util::sorted_value_init(&wgt), ["border_b", "border_a"]);
    assert_eq!(util::sorted_node_init(&wgt), ["border_b", "border_a"]);
}

/*
 *  Tests widget when.
 */
#[widget($crate::tests::widget::when_wgt)]
pub mod when_wgt {
    inherit!(crate::widget_base::base);

    pub use super::util::is_state;
    pub use super::util::live_trace as msg;

    properties! {
        msg = "boo!";

        when *#is_state {
            msg = "ok.";
        }
    }
}
#[test]
pub fn wgt_when() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = when_wgt!();
    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "boo!"));

    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "ok."));

    util::set_state(&mut ctx, &mut wgt, false);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "boo!"));
}
#[test]
pub fn widget_user_when() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
        util::live_trace = "A";

        when *#util::is_state {
            util::live_trace = "B";
        }
    };
    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "A"));

    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "B"));

    util::set_state(&mut ctx, &mut wgt, false);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "A"));
}

/*
 * Tests multiple widget whens
 */
#[widget($crate::tests::widget::multi_when_wgt)]
pub mod multi_when_wgt {
    inherit!(crate::widget_base::base);

    use super::util::{is_state, live_trace as trace};
    properties! {
        trace = "default";
        when *#is_state {
            trace = "state_0";
        }
        when *#is_state {
            trace = "state_1";
        }
    }
}
#[test]
pub fn wgt_multi_when() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = multi_when_wgt!();
    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "default"));

    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "state_1"));

    util::set_state(&mut ctx, &mut wgt, false);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "default"));
}

/*
 * Tests widget property attributes.
 */
#[widget($crate::tests::widget::cfg_property_wgt)]
pub mod cfg_property_wgt {
    inherit!(crate::widget_base::base);

    use super::util::trace;

    properties! {
        // property not included in widget.
        #[cfg(never)]
        trace as never_trace = "never-trace";

        // suppress warning.
        #[allow(non_snake_case)]
        trace as always_trace = {
            #[allow(clippy::needless_late_init)]
            let weird___name;
            weird___name = "always-trace";
            weird___name
        };
    }
}
#[test]
pub fn wgt_cfg_property() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = cfg_property_wgt!();
    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "always-trace"));
    assert!(!util::traced(&wgt, "never-trace"));
}
#[test]
pub fn user_cfg_property() {
    let mut ctx = TestWidgetContext::new();

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
            #[allow(clippy::needless_late_init)]
            let weird___name;
            weird___name = "always-trace";
            weird___name
        };
    };

    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "always-trace"));
    assert!(!util::traced(&wgt, "never-trace"));
}

/*
 * Tests widget when attributes.
 */
#[widget($crate::tests::widget::cfg_when_wgt)]
pub mod cfg_when_wgt {
    inherit!(crate::widget_base::base);

    use super::util::{is_state, live_trace};

    properties! {
        live_trace = "trace";

        // suppress warning in all assigns.
        #[allow(non_snake_case)]
        when *#is_state {
            live_trace = {
                #[allow(clippy::needless_late_init)]
                let weird___name;
                weird___name = "is_state";
                weird___name
            };
        }

        // when not applied.
        #[cfg(never)]
        when *#is_state {
            live_trace = "is_never_state";
        }
    }
}
#[test]
pub fn wgt_cfg_when() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = cfg_when_wgt!();

    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "trace"));

    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "is_state"));

    util::set_state(&mut ctx, &mut wgt, false);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "trace"));
}

#[test]
pub fn user_cfg_when() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
        util::live_trace = "trace";

        when *#util::is_state {
            util::live_trace = {
                #[allow(non_snake_case)]
                #[allow(clippy::needless_late_init)]
                let weird___name;
                weird___name = "is_state";
                weird___name
            };
        }

        #[cfg(never)]
        when *#util::is_state {
            util::live_trace = "is_never_state";
        }
    };

    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "trace"));

    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "is_state"));

    util::set_state(&mut ctx, &mut wgt, false);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "trace"));
}

/*
 *  Tests widget captures.
 */
#[widget($crate::tests::widget::capture_properties_wgt)]
pub mod capture_properties_wgt {
    inherit!(crate::widget_base::base);

    use crate::widget_builder::*;

    properties! {
        pub super::util::trace as new_trace = "new";
        pub super::util::trace as property_trace = "property";
    }

    fn build(mut wgt: WidgetBuilder) -> impl crate::widget_instance::UiNode {
        let msg: &'static str = wgt.capture_value(property_id!(self::new_trace)).unwrap();
        let msg = match msg {
            "new" => "custom new",
            "user-new" => "custom new (user)",
            o => panic!("unexpected {o:?}"),
        };
        wgt.push_property(
            Importance::WIDGET,
            property_args! {
                super::util::trace as build_trace = msg;
            },
        );

        crate::widget_base::nodes::build(wgt)
    }
}
#[test]
pub fn wgt_capture_properties() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = capture_properties_wgt!();
    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "property"));
    assert!(util::traced(&wgt, "custom new"));
    assert!(!util::traced(&wgt, "new"));
}
#[test]
pub fn wgt_capture_properties_reassign() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = capture_properties_wgt! {
        property_trace = "user-property";
        new_trace = "user-new";
    };
    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "user-property"));
    assert!(util::traced(&wgt, "custom new (user)"));

    assert!(!util::traced(&wgt, "new"));
    assert!(!util::traced(&wgt, "user-new"));
}

/*
 * Tests order properties are inited and applied.
 */

#[widget($crate::tests::widget::property_nest_group_sorting_wgt)]
pub mod property_nest_group_sorting_wgt {
    inherit!(crate::widget_base::base);

    properties! {
        pub super::util::count_border as count_border2;
        pub super::util::count_border as count_border1;
        pub super::util::count_child_context as count_child_context2;
        pub super::util::count_child_context as count_child_context1;
        pub super::util::count_child_layout as count_child_layout2;
        pub super::util::count_child_layout as count_child_layout1;
        pub super::util::count_context as count_context2;
        pub super::util::count_context as count_context1;
        pub super::util::count_layout as count_layout2;
        pub super::util::count_layout as count_layout1;
        pub super::util::count_size as count_size2;
        pub super::util::count_size as count_size1;
        pub super::util::on_count as count_event2;
        pub super::util::on_count as count_event1;
    }
}
fn property_nest_group_sorting_init1() -> impl UiNode {
    property_nest_group_sorting_wgt! {
        count_border1 = Position::next("count_border1");
        count_border2 = Position::next("count_border2");
        count_size1 = Position::next("count_size1");
        count_size2 = Position::next("count_size2");
        count_layout1 = Position::next("count_layout1");
        count_layout2 = Position::next("count_layout2");
        count_event1 = Position::next("count_event1");
        count_event2 = Position::next("count_event2");
        count_context1 = Position::next("count_context1");
        count_context2 = Position::next("count_context2");

        count_child_layout1 = Position::next("count_child_layout1");
        count_child_layout2 = Position::next("count_child_layout2");
        count_child_context1 = Position::next("count_child_context1");
        count_child_context2 = Position::next("count_child_context2");
    }
}
#[test]
pub fn property_nest_group_sorting_value_init1() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();

    let mut wgt = property_nest_group_sorting_init1();
    ctx.init(&mut wgt);

    // assert that value init is the same as typed.
    pretty_assertions::assert_eq!(
        util::sorted_value_init(&wgt),
        [
            "count_border1",
            "count_border2",
            "count_size1",
            "count_size2",
            "count_layout1",
            "count_layout2",
            "count_event1",
            "count_event2",
            "count_context1",
            "count_context2",
            "count_child_layout1",
            "count_child_layout2",
            "count_child_context1",
            "count_child_context2",
        ]
    );
}
fn property_nest_group_sorting_init2() -> impl UiNode {
    property_nest_group_sorting_wgt! {
        count_child_context1 = Position::next("count_child_context1");
        count_child_context2 = Position::next("count_child_context2");
        count_child_layout1 = Position::next("count_child_layout1");
        count_child_layout2 = Position::next("count_child_layout2");

        count_context1 = Position::next("count_context1");
        count_context2 = Position::next("count_context2");
        count_event1 = Position::next("count_event1");
        count_event2 = Position::next("count_event2");
        count_layout1 = Position::next("count_layout1");
        count_layout2 = Position::next("count_layout2");
        count_size1 = Position::next("count_size1");
        count_size2 = Position::next("count_size2");
        count_border1 = Position::next("count_border1");
        count_border2 = Position::next("count_border2");
    }
}
#[test]
pub fn property_nest_group_sorting_value_init2() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();

    let mut wgt = property_nest_group_sorting_init2();
    ctx.init(&mut wgt);

    // assert that value init is the same as typed.
    pretty_assertions::assert_eq!(
        util::sorted_value_init(&wgt),
        [
            "count_child_context1",
            "count_child_context2",
            "count_child_layout1",
            "count_child_layout2",
            "count_context1",
            "count_context2",
            "count_event1",
            "count_event2",
            "count_layout1",
            "count_layout2",
            "count_size1",
            "count_size2",
            "count_border1",
            "count_border2",
        ]
    );
}
fn assert_node_order(wgt: &impl UiNode) {
    // assert that `UiNode::init` position is sorted by `child` and
    // property priorities, followed by the typed position.
    pretty_assertions::assert_eq!(
        util::sorted_node_init(wgt),
        [
            // each property wraps the next one and takes a position number before
            // delegating to the next property (child node).
            "count_context1",
            "count_context2",
            "count_event1",
            "count_event2",
            "count_layout1",
            "count_layout2",
            "count_size1",
            "count_size2",
            "count_border1",
            "count_border2",
            "count_child_context1",
            "count_child_context2",
            "count_child_layout1",
            "count_child_layout2",
        ]
    );
}
#[test]
pub fn property_nest_group_sorting_node_init1() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();

    let mut wgt = property_nest_group_sorting_init1();
    ctx.init(&mut wgt);

    assert_node_order(&wgt);
}
#[test]
pub fn property_nest_group_sorting_node_init2() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();

    let mut wgt = property_nest_group_sorting_init2();
    ctx.init(&mut wgt);

    assert_node_order(&wgt);
}
#[widget($crate::tests::widget::property_nest_group_sorting_inherited_wgt)]
pub mod property_nest_group_sorting_inherited_wgt {
    inherit!(super::property_nest_group_sorting_wgt);
}
#[test]
pub fn property_nest_group_sorting_node_inherited_init() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();

    let mut wgt = property_nest_group_sorting_inherited_wgt! {
        count_child_context1 = Position::next("count_child_context1");
        count_child_context2 = Position::next("count_child_context2");
        count_child_layout1 = Position::next("count_child_layout1");
        count_child_layout2 = Position::next("count_child_layout2");

        count_context1 = Position::next("count_context1");
        count_context2 = Position::next("count_context2");
        count_event1 = Position::next("count_event1");
        count_event2 = Position::next("count_event2");
        count_layout1 = Position::next("count_layout1");
        count_layout2 = Position::next("count_layout2");
        count_size1 = Position::next("count_size1");
        count_size2 = Position::next("count_size2");
        count_border1 = Position::next("count_border1");
        count_border2 = Position::next("count_border2");
    };
    ctx.init(&mut wgt);

    assert_node_order(&wgt);
}

#[widget($crate::tests::widget::property_nest_group_sorting_defaults_wgt)]
pub mod property_nest_group_sorting_defaults_wgt {
    use super::util::Position;

    inherit!(super::property_nest_group_sorting_wgt);

    properties! {
        count_context1 = Position::next("count_context1");
        count_context2 = Position::next("count_context2");
        count_event1 = Position::next("count_event1");
        count_event2 = Position::next("count_event2");
        count_layout1 = Position::next("count_layout1");
        count_layout2 = Position::next("count_layout2");
        count_size1 = Position::next("count_size1");
        count_size2 = Position::next("count_size2");
        count_border1 = Position::next("count_border1");
        count_border2 = Position::next("count_border2");

        count_child_context1 = Position::next("count_child_context1");
        count_child_context2 = Position::next("count_child_context2");
        count_child_layout1 = Position::next("count_child_layout1");
        count_child_layout2 = Position::next("count_child_layout2");
    }
}
#[test]
pub fn property_nest_group_sorting_defaults() {
    let mut ctx = TestWidgetContext::new();

    Position::reset();

    let mut wgt = property_nest_group_sorting_defaults_wgt!();
    ctx.init(&mut wgt);
    assert_node_order(&wgt);
}

/*
 * Tests property member access in when
 */

#[test]
pub fn when_property_member_default() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(*#util::duo_members, "a");
           true
       } {
           util::live_trace = "true";
       }
    };

    ctx.init(&mut wgt);
    assert!(util::traced(&wgt, "true"));
}

#[test]
pub fn when_property_member_index() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(*#util::duo_members.0, "a");
           assert_eq!(*#util::duo_members.1, "b");
           true
       } {
           util::live_trace = "true";
       }
    };

    ctx.init(&mut wgt);
    assert!(util::traced(&wgt, "true"));
}

#[test]
pub fn when_property_member_named() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(*#util::duo_members.member_a, "a");
           assert_eq!(*#util::duo_members.member_b, "b");
           true
       } {
           util::live_trace = "true";
       }
    };

    ctx.init(&mut wgt);
    assert!(util::traced(&wgt, "true"));
}

#[test]
pub fn when_property_member_default_method() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(#util::duo_members.len(), 1);
           true
       } {
           util::live_trace = "true";
       }
    };

    ctx.init(&mut wgt);
    assert!(util::traced(&wgt, "true"));
}

#[test]
pub fn when_property_member_indexed_method() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
       util::duo_members = "a", "b";
       util::live_trace = "";
       when {
           assert_eq!(#util::duo_members.0.len(), 1);
           true
       } {
           util::live_trace = "true";
       }
    };

    ctx.init(&mut wgt);
    assert!(util::traced(&wgt, "true"));
}

#[widget($crate::tests::widget::get_builder)]
pub mod get_builder {
    use crate::widget_builder::WidgetBuilder;

    fn build(mut wgt: WidgetBuilder) -> WidgetBuilder {
        wgt.set_custom_build(crate::widget_base::nodes::build);
        wgt
    }
}

#[test]
pub fn when_reuse() {
    let test = |pass: &str| {
        let mut ctx = TestWidgetContext::new();

        let builder = get_builder! {
            util::live_trace = "false";

            when *#util::is_state {
                util::live_trace = "true";
            }
        };
        let mut wgt = builder.build();

        ctx.init(&mut wgt);
        assert!(!util::traced(&wgt, "true"), "traced `true` in {pass} pass");
        assert!(util::traced(&wgt, "false"), "did not trace `false` in {pass} pass");

        util::set_state(&mut ctx, &mut wgt, true);
        ctx.update(&mut wgt, None);
        ctx.apply_updates();
        ctx.update(&mut wgt, None);
        assert!(util::traced(&wgt, "true"), "did not trace `true` after when in {pass} pass");

        util::set_state(&mut ctx, &mut wgt, false);
        ctx.update(&mut wgt, None);
        ctx.apply_updates();
        ctx.update(&mut wgt, None);
    };

    test("first");
    test("reuse");
}

/*
* Inherit override
*/
#[widget_mixin($crate::tests::widget::inherit_override_a_mixin)]
pub mod inherit_override_a_mixin {
    use super::util::trace;

    properties! {
        trace = "base_a::property";
    }
}
#[widget_mixin($crate::tests::widget::inherit_override_b_mixin)]
pub mod inherit_override_b_mixin {
    use super::util::trace;

    properties! {
        trace = "base_b::property";
    }
}
#[widget($crate::tests::widget::inherit_override_wgt1)]
pub mod inherit_override_wgt1 {
    inherit!(crate::widget_base::base);
    inherit!(super::inherit_override_a_mixin);
    inherit!(super::inherit_override_b_mixin);
}
#[widget($crate::tests::widget::inherit_override_wgt2)]
pub mod inherit_override_wgt2 {
    inherit!(crate::widget_base::base);
    inherit!(super::inherit_override_b_mixin);
    inherit!(super::inherit_override_a_mixin);
}
#[test]
pub fn inherit_override() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = inherit_override_wgt1!();

    ctx.init(&mut wgt);
    assert!(util::traced(&wgt, "base_b::property"));
    assert!(!util::traced(&wgt, "base_a::property"));

    let mut wgt = inherit_override_wgt2!();

    ctx.init(&mut wgt);
    assert!(!util::traced(&wgt, "base_b::property"));
    assert!(util::traced(&wgt, "base_a::property"));
}

/*
* Property Default Value
*/

#[test]
pub fn allowed_in_when_without_wgt_assign1() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
        // util::live_trace_default = "default-trace";
        when *#util::is_state {
            util::live_trace_default = "when-trace";
        }
    };

    ctx.init(&mut wgt);
    assert!(util::traced(&wgt, "default-trace"));
    assert!(!util::traced(&wgt, "when-trace"));

    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();

    ctx.update(&mut wgt, None);
    assert!(util::traced(&wgt, "when-trace"));
}

#[widget($crate::tests::widget::declare_prop_with_default_wgt)]
pub mod declare_prop_with_default_wgt {
    inherit!(crate::widget_base::base);

    properties! {
        pub super::util::live_trace_default as trace;
    }
}

#[test]
pub fn allowed_in_when_without_wgt_assign2() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = declare_prop_with_default_wgt! {
        // live_trace_default = "default-trace";
        when *#util::is_state {
            trace = "when-trace";
        }
    };

    ctx.init(&mut wgt);
    assert!(util::traced(&wgt, "default-trace"));
    assert!(!util::traced(&wgt, "when-trace"));

    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);
    assert!(util::traced(&wgt, "when-trace"));
}

/*
* Generated Names Don't Shadow Each Other
*/
#[crate::property(CONTEXT)]
pub fn util_live_trace(
    child: impl crate::widget_instance::UiNode,
    not_str: impl crate::var::IntoVar<bool>,
) -> impl crate::widget_instance::UiNode {
    let var = not_str.into_var().map(|&b| if b { "true" } else { "false" });
    util::live_trace(child, var)
}

#[test]
pub fn generated_name_collision() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
        util::live_trace = "!";
        util_live_trace = false;
    };

    ctx.init(&mut wgt);

    assert!(util::traced(&wgt, "!"));
    assert!(util::traced(&wgt, "false"));
}

#[test]
pub fn generated_name_collision_in_when() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
        util::live_trace = "1";
        when *#util::is_state {
            util::live_trace = "2";
        }
        when *#util::is_state {
            util::live_trace = "3";
        }
    };

    ctx.init(&mut wgt);
    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "3"));
    assert!(!util::traced(&wgt, "2"));
}

#[test]
pub fn generated_name_collision_in_when_assign() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = empty_wgt! {
        util::live_trace = "0";
        util_live_trace = false;

        when *#util::is_state {
            util::live_trace = "1";
            util_live_trace = true;
        }
    };

    ctx.init(&mut wgt);
    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "1"));
    assert!(util::traced(&wgt, "true"));
}

#[widget($crate::tests::widget::name_collision_wgt_when)]
pub mod name_collision_wgt_when {
    inherit!(crate::widget_base::base);

    use super::util::{is_state, live_trace};

    properties! {
        live_trace = "1";

        when *#is_state {
            live_trace = "2";
        }
        when *#is_state {
            live_trace = "3";
        }
    }
}
#[test]
pub fn name_collision_wgt_when() {
    let mut ctx = TestWidgetContext::new();

    let mut wgt = name_collision_wgt_when!();

    ctx.init(&mut wgt);
    util::set_state(&mut ctx, &mut wgt, true);
    ctx.update(&mut wgt, None);
    ctx.apply_updates();
    ctx.update(&mut wgt, None);

    assert!(util::traced(&wgt, "3"));
    assert!(!util::traced(&wgt, "2"));
}

/*
* macro_rules! generated widget
*/

mod macro_rules_generated {
    use crate::widget;

    macro_rules! test {
        ($name:ident) => {
           test! {
               [$] $name
           }
        };
        ([$dollar:tt] $name:ident) => {
            #[widget($dollar crate::tests::widget::macro_rules_generated::$name)]
            pub mod $name {
                use crate::var::IntoVar;
                use crate::widget_instance::UiNode;

                inherit!($crate::widget_base::base);

                #[$crate::property(CONTEXT)]
                pub fn margin(child: impl UiNode, margin: impl IntoVar<$crate::units::SideOffsets>) -> impl UiNode {
                    let _ = margin;
                    child
                }

                properties! {
                    margin;
                }
            }
        }
    }

    test! {
        bar
    }
}

#[test]
fn macro_rules_generated() {
    let _ = macro_rules_generated::bar! {
        margin = 10;
    };
}

mod util {
    use std::{
        cell::Cell,
        collections::{HashMap, HashSet},
    };

    use crate::{
        context::{StaticStateId, TestWidgetContext, WidgetContext, WidgetUpdates},
        property, ui_node,
        var::{IntoValue, IntoVar, Var},
        widget_instance::UiNode,
    };

    /// Insert `trace` in the widget state. Can be probed using [`traced`].
    #[property(CONTEXT)]
    pub fn trace(child: impl UiNode, trace: impl IntoValue<&'static str>) -> impl UiNode {
        TraceNode {
            child,
            trace: trace.into(),
        }
    }

    /// Probe for a [`trace`] in the widget state.
    pub fn traced(wgt: &impl UiNode, trace: &'static str) -> bool {
        wgt.with_context(|ctx| ctx.widget_state.get(&TRACE_ID).map(|t| t.contains(trace)).unwrap_or_default())
            .expect("expected widget")
    }

    static TRACE_ID: StaticStateId<HashSet<&'static str>> = StaticStateId::new_unique();

    #[ui_node(struct TraceNode {
        child: impl UiNode,
        trace: &'static str,
    })]
    impl UiNode for TraceNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            ctx.widget_state.entry(&TRACE_ID).or_default().insert(self.trace);
        }
    }

    /// Insert `count` in the widget state. Can get using [`Count::get`].
    #[property(CONTEXT)]
    pub fn count(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        CountNode {
            child,
            value_pos: count.into(),
        }
    }

    pub use count as count_context;

    /// Same as [`count`] but in `CHILD_CONTEXT` group.
    #[property(CHILD_CONTEXT)]
    pub fn count_child_context(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        CountNode {
            child,
            value_pos: count.into(),
        }
    }

    /// Same as [`count`] but in `CHILD_LAYOUT` group.
    #[property(CHILD_LAYOUT)]
    pub fn count_child_layout(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        CountNode {
            child,
            value_pos: count.into(),
        }
    }

    /// Same as [`count`] but in `BORDER` group.
    #[property(BORDER)]
    pub fn count_border(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        CountNode {
            child,
            value_pos: count.into(),
        }
    }

    /// Same as [`count`] but in `LAYOUT` group.
    #[property(LAYOUT)]
    pub fn count_layout(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        CountNode {
            child,
            value_pos: count.into(),
        }
    }

    /// Same as [`count`] but in `SIZE` group.
    #[property(SIZE)]
    pub fn count_size(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        CountNode {
            child,
            value_pos: count.into(),
        }
    }

    /// Same as [`count`] but in `EVENT` group.
    #[property(EVENT)]
    pub fn on_count(child: impl UiNode, count: impl IntoValue<Position>) -> impl UiNode {
        CountNode {
            child,
            value_pos: count.into(),
        }
    }

    /// Count adds one every [`Self::next`] call.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Position {
        pub pos: u32,
        pub tag: &'static str,
    }
    thread_local! {
        static COUNT: Cell<u32> = Cell::new(0);
        static COUNT_INIT: Cell<u32> = Cell::new(0);
    }
    impl Position {
        pub fn next(tag: &'static str) -> Self {
            Position {
                pos: COUNT.with(|c| {
                    let r = c.get();
                    c.set(r + 1);
                    r
                }),
                tag,
            }
        }

        fn next_init() -> u32 {
            COUNT_INIT.with(|c| {
                let r = c.get();
                c.set(r + 1);
                r
            })
        }

        pub fn reset() {
            COUNT.with(|c| c.set(0));
            COUNT_INIT.with(|c| c.set(0));
        }
    }

    /// Gets the [`Position`] tags sorted by call to [`Position::next`].
    pub fn sorted_value_init(wgt: &impl UiNode) -> Vec<&'static str> {
        let mut vec = vec![];
        wgt.with_context(|n| {
            if let Some(m) = n.widget_state.get(&VALUE_POSITION_ID) {
                for (key, value) in m {
                    vec.push((*key, *value));
                }
            }
        });
        vec.sort_by_key(|(_, i)| *i);
        vec.into_iter().map(|(t, _)| t).collect()
    }

    /// Gets the [`Position`] tags sorted by the [`UiNode::init` call.
    pub fn sorted_node_init(wgt: &impl UiNode) -> Vec<&'static str> {
        let mut vec = vec![];
        wgt.with_context(|n| {
            if let Some(m) = n.widget_state.get(&NODE_POSITION_ID) {
                for (key, value) in m {
                    vec.push((*key, *value));
                }
            }
        });
        vec.sort_by_key(|(_, i)| *i);
        vec.into_iter().map(|(t, _)| t).collect()
    }

    static VALUE_POSITION_ID: StaticStateId<HashMap<&'static str, u32>> = StaticStateId::new_unique();
    static NODE_POSITION_ID: StaticStateId<HashMap<&'static str, u32>> = StaticStateId::new_unique();

    #[ui_node(struct CountNode {
        child: impl UiNode,
        value_pos: Position,
    })]
    impl UiNode for CountNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            ctx.widget_state
                .entry(&VALUE_POSITION_ID)
                .or_default()
                .insert(self.value_pos.tag, self.value_pos.pos);

            ctx.widget_state
                .entry(&NODE_POSITION_ID)
                .or_default()
                .insert(self.value_pos.tag, Position::next_init());

            self.child.init(ctx);
        }
    }

    /// Test state property, state can be set using [`set_state`] followed by updating.
    #[property(CONTEXT)]
    pub fn is_state(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
        IsStateNode {
            child,
            state: state.into_var(),
        }
    }
    /// Sets the [`is_state`] of a widget.
    ///
    /// Note only applies after update.
    pub fn set_state(ctx: &mut TestWidgetContext, wgt: &mut impl UiNode, state: bool) {
        wgt.with_context_mut(|w_ctx| {
            ctx.updates.update(w_ctx.id);
            *w_ctx.widget_state.entry(&IS_STATE_ID).or_default() = state;
        })
        .expect("expected widget");
    }

    #[ui_node(struct IsStateNode {
        child: impl UiNode,
        state: impl Var<bool>,
    })]
    impl IsStateNode {
        fn update_state(&mut self, ctx: &mut WidgetContext) {
            let wgt_state = ctx.widget_state.get(&IS_STATE_ID).copied().unwrap_or_default();
            if wgt_state != self.state.get() {
                let _ = self.state.set(ctx.vars, wgt_state);
            }
        }

        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            self.update_state(ctx);
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            self.update_state(ctx);
        }
    }

    static IS_STATE_ID: StaticStateId<bool> = StaticStateId::new_unique();

    /// A [trace] that can update.
    #[property(CONTEXT)]
    pub fn live_trace(child: impl UiNode, trace: impl IntoVar<&'static str>) -> impl UiNode {
        LiveTraceNode {
            child,
            trace: trace.into_var(),
        }
    }
    /// A [trace] that can update and has a default value of `"default-trace"`.
    #[property(CONTEXT, default("default-trace"))]
    pub fn live_trace_default(child: impl UiNode, trace: impl IntoVar<&'static str>) -> impl UiNode {
        LiveTraceNode {
            child,
            trace: trace.into_var(),
        }
    }

    #[ui_node(struct LiveTraceNode {
        child: impl UiNode,
        #[var] trace: impl Var<&'static str>,
    })]
    impl UiNode for LiveTraceNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);
            ctx.widget_state.entry(&TRACE_ID).or_default().insert(self.trace.get());
            self.auto_subs(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            if let Some(trace) = self.trace.get_new(ctx) {
                ctx.widget_state.entry(&TRACE_ID).or_default().insert(trace);
            }
        }
    }

    /// A capture_only property.
    #[property(CONTEXT)]
    #[allow(unreachable_code)]
    pub fn capture_only_trace(_child: impl UiNode, trace: impl IntoValue<&'static str>) -> impl UiNode {
        let _ = trace;
        panic!("capture-only property");
        _child
    }

    #[property(CONTEXT)]
    pub fn duo_members(child: impl UiNode, member_a: impl IntoVar<&'static str>, member_b: impl IntoVar<&'static str>) -> impl UiNode {
        let _ = member_a;
        let _ = member_b;
        child
    }
}
