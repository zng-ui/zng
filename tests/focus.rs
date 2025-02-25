use keyboard::KeyLocation;
use zng::{
    app::{AppExtended, AppExtension, HeadlessApp},
    data_view::{DataView, DataViewArgs},
    event::EventReceiver,
    focus::{
        DirectionalNav, FOCUS_CHANGED_EVENT, FocusChangedArgs, FocusChangedCause, RETURN_FOCUS_CHANGED_EVENT, ReturnFocusChangedArgs,
        TabIndex, TabNav, alt_focus_scope,
        cmd::{FOCUS_NEXT_CMD, FOCUS_PREV_CMD},
        directional_nav, focus_scope, focusable, tab_index, tab_nav,
    },
    keyboard::{Key, KeyCode, KeyState},
    prelude::*,
    widget::{Visibility, WidgetUpdateMode, info::InteractionPath, interactive, node::ArcNode, visibility},
};

#[test]
pub fn first_and_last_window_events() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
    ];

    let root_id = WidgetId::new_unique();
    let stack_id = WidgetId::new_unique();
    let button_0_id = buttons.item_id(0);

    let mut app = app.run_window(Window! {
        child = Stack!(id = stack_id; direction = StackDirection::top_to_bottom(); children = buttons);
        id = root_id;
    });
    let root_path = InteractionPath::new_enabled(app.window_id, vec![root_id].into());
    let button_0_path = InteractionPath::new_enabled(app.window_id, vec![root_id, stack_id, button_0_id].into());

    let events = app.take_focus_changed();
    assert_eq!(2, events.len());

    // "recover" focus to the focused window root.
    assert!(events[0].prev_focus.is_none());
    assert_eq!(Some(root_path.clone()), events[0].new_focus);
    assert_eq!(FocusChangedCause::Recovery, events[0].cause);
    assert!(!events[0].highlight);

    // root is a scope that auto-advanced focus to first focusable child.
    assert_eq!(Some(root_path), events[1].prev_focus);
    assert_eq!(Some(button_0_path.clone()), events[1].new_focus);
    assert_eq!(FocusChangedCause::ScopeGotFocus(false), events[1].cause);
    assert!(!events[1].highlight);

    let events = app.take_return_focus_changed();
    assert_eq!(1, events.len());

    // the window remembers its previous focused descendant.
    assert!(events[0].prev_return.is_none());
    assert_eq!(root_id, events[0].scope.as_ref().map(|p| p.widget_id()).unwrap());
    assert_eq!(Some(button_0_path.clone()), events[0].new_return);

    /*
        Last Events
    */

    app.close_main_window();

    let events = app.take_focus_changed();
    assert_eq!(1, events.len());

    // "recover" to focus nothing.
    assert_eq!(Some(button_0_path.clone()), events[0].prev_focus);
    assert!(events[0].new_focus.is_none());
    assert_eq!(FocusChangedCause::Recovery, events[0].cause);
    assert!(!events[0].highlight);

    let events = app.take_return_focus_changed();
    assert_eq!(1, events.len());

    // cleanup return focus.
    assert_eq!(Some(button_0_path), events[0].prev_return);
    assert!(events[0].new_return.is_none());
}

#[test]
pub fn window_tab_cycle_index_auto() {
    // default Window! cycles TAB navigation.
    t(|_| TabIndex::AUTO);
    t(TabIndex);
    t(|i| TabIndex(TabIndex::AUTO.0 - i - 1));

    fn t(make_index: impl FnMut(u32) -> TabIndex) {
        let app = TestApp::start();

        // all TAB navigation must respect the `tab_index` value
        // that by default is AUTO, but can be not in the same order
        // as the widgets are declared.
        let tab_ids: Vec<_> = (0..3).map(make_index).collect();

        let mut buttons = ui_vec![
            Button! {
                id = "btn-0";
                child = Text!("Button 0");
                tab_index = tab_ids[0]
            },
            Button! {
                id = "btn-1";
                child = Text!("Button 1");
                tab_index = tab_ids[1]
            },
            Button! {
                id = "btn-2";
                child = Text!("Button 2");
                tab_index = tab_ids[2]
            },
        ];
        // we collect the widget_id values in the TAB navigation order.
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.item_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let mut app = app.run(Stack!(top_to_bottom, buttons));

        // advance normally.
        assert_eq!(Some(ids[0]), app.focused());
        assert!(app.can_tab());
        app.press_tab();
        assert_eq!(Some(ids[1]), app.focused());
        app.press_tab();
        assert_eq!(Some(ids[2]), app.focused());
        // then cycles back.
        app.press_tab();
        assert_eq!(Some(ids[0]), app.focused());

        // same backwards.
        app.press_shift_tab();
        assert_eq!(Some(ids[2]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(ids[1]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(ids[0]), app.focused());
        // cycles back.
        app.press_shift_tab();
        assert_eq!(Some(ids[2]), app.focused());
    }
}

#[test]
pub fn window_tab_cycle_and_alt_scope() {
    // default Window! with an ALT scope, TAB navigation cycles
    // by default in the ALT scope too.

    t(|_| TabIndex::AUTO);
    t(TabIndex);
    t(|i| TabIndex(TabIndex::AUTO.0 - i - 1));

    fn t(make_index: impl FnMut(u32) -> TabIndex) {
        let app = TestApp::start();

        let tab_ids: Vec<_> = (0..5).map(make_index).collect();

        let mut buttons = ui_vec![
            Button! {
                id = "btn-0";
                child = Text!("Button 0");
                tab_index = tab_ids[0]
            },
            Button! {
                id = "btn-1";
                child = Text!("Button 1");
                tab_index = tab_ids[1]
            },
        ];
        let mut ids: Vec<_> = (0..2).map(|i| (buttons.item_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let mut alt_buttons = ui_vec![
            Button! {
                id = "alt-0";
                child = Text!("Alt 0");
                tab_index = tab_ids[2]
            },
            Button! {
                id = "alt-1";
                child = Text!("Alt 1");
                tab_index = tab_ids[3]
            },
            Button! {
                id = "alt-2";
                child = Text!("Alt 2");
                tab_index = tab_ids[4]
            },
        ];
        let mut alt_ids: Vec<_> = (0..3).map(|i| (alt_buttons.item_id(i), tab_ids[i + 2])).collect();
        alt_ids.sort_by_key(|(_, ti)| *ti);
        let alt_ids: Vec<_> = alt_ids.into_iter().map(|(id, _)| id).collect();

        let mut app = app.run(Stack!(
            top_to_bottom,
            ui_vec![
                Stack! {
                    direction = StackDirection::left_to_right();
                    alt_focus_scope = true;
                    children = alt_buttons;
                },
                Stack!(top_to_bottom, buttons)
            ]
        ));

        // cycle in the window scope does not enter the ALT scope.
        assert_eq!(Some(ids[0]), app.focused());
        app.press_tab();
        assert_eq!(Some(ids[1]), app.focused());
        app.press_tab();
        assert_eq!(Some(ids[0]), app.focused());
        // and back.
        app.press_shift_tab();
        assert_eq!(Some(ids[1]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(ids[0]), app.focused());

        // make the "Button 1" be the return focus from the ALT scope.
        app.press_tab();
        assert_eq!(Some(ids[1]), app.focused());

        // goes to the ALT scope.
        app.press_alt();
        assert_eq!(Some(alt_ids[0]), app.focused());

        // cycle in the ALT scope.
        app.press_tab();
        assert_eq!(Some(alt_ids[1]), app.focused());
        app.press_tab();
        assert_eq!(Some(alt_ids[2]), app.focused());
        app.press_tab();
        assert_eq!(Some(alt_ids[0]), app.focused());
        // and back.
        app.press_shift_tab();
        assert_eq!(Some(alt_ids[2]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(alt_ids[1]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(alt_ids[0]), app.focused());

        // return to the window scope that focus on the "Button 1".
        app.press_esc();
        assert_eq!(Some(ids[1]), app.focused());

        // we are back to cycling the window scope.
        app.press_tab();
        assert_eq!(Some(ids[0]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(ids[1]), app.focused());

        // also can return from ALT scope by pressing ALT again.
        app.press_alt();
        assert_eq!(Some(alt_ids[0]), app.focused());
        app.press_alt();
        assert_eq!(Some(ids[1]), app.focused());
    }
}

#[test]
pub fn window_tab_contained() {
    // TabNav::Contained stops at the last item and back
    // the root scope behaves just like any other Contained scope.
    window_tab_contained_and_continue(TabNav::Contained);
}
#[test]
pub fn window_tab_continue() {
    // TabNav::Continue in the root scope behaves like a Contained
    // scope because there is no outer-scope to continue to.
    window_tab_contained_and_continue(TabNav::Continue);
}
fn window_tab_contained_and_continue(tab_nav: TabNav) {
    t(tab_nav, |_| TabIndex::AUTO);
    t(tab_nav, TabIndex);
    t(tab_nav, |i| TabIndex(TabIndex::AUTO.0 - i - 1));

    fn t(tab_nav: TabNav, make_index: impl FnMut(u32) -> TabIndex) {
        let app = TestApp::start();

        let tab_ids: Vec<_> = (0..3).map(make_index).collect();

        let mut buttons = ui_vec![
            Button! {
                id = "btn-0";
                child = Text!("Button 0");
                tab_index = tab_ids[0]
            },
            Button! {
                id = "btn-1";
                child = Text!("Button 1");
                tab_index = tab_ids[1]
            },
            Button! {
                id = "btn-2";
                child = Text!("Button 2");
                tab_index = tab_ids[2]
            },
        ];
        // we collect the widget_id values in the TAB navigation order.
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.item_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let mut app = app.run_window(Window! {
            tab_nav;
            child = Stack!(top_to_bottom, buttons);
        });

        // navigates normally forward.
        assert_eq!(Some(ids[0]), app.focused());
        app.press_tab();
        assert_eq!(Some(ids[1]), app.focused());
        app.press_tab();
        assert_eq!(Some(ids[2]), app.focused());
        // but after reaching the end does not move.
        app.press_tab();
        assert_eq!(Some(ids[2]), app.focused());

        // same backwards.
        app.press_shift_tab();
        assert_eq!(Some(ids[1]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(ids[0]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(ids[0]), app.focused());
    }
}

#[test]
pub fn window_tab_once() {
    // we already start focused inside so Once==None in root widgets.
    window_tab_once_and_none(TabNav::Once);
}
#[test]
pub fn window_tab_none() {
    // we already start focused inside so Once==None in root widgets.
    window_tab_once_and_none(TabNav::None);
}
fn window_tab_once_and_none(tab_nav: TabNav) {
    t(tab_nav, |_| TabIndex::AUTO);
    t(tab_nav, TabIndex);
    t(tab_nav, |i| TabIndex(TabIndex::AUTO.0 - i - 1));

    fn t(tab_nav: TabNav, make_index: impl FnMut(u32) -> TabIndex) {
        let app = TestApp::start();

        let tab_ids: Vec<_> = (0..3).map(make_index).collect();

        let mut buttons = ui_vec![
            Button! {
                child = Text!("Button 0");
                tab_index = tab_ids[0]
            },
            Button! {
                child = Text!("Button 1");
                tab_index = tab_ids[1]
            },
            Button! {
                child = Text!("Button 2");
                tab_index = tab_ids[2]
            },
        ];
        // we collect the widget_id values in the TAB navigation order.
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.item_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let mut app = app.run_window(Window! {
            child = Stack!(top_to_bottom, buttons);
            tab_nav;
        });

        assert_eq!(Some(ids[0]), app.focused());
        app.press_tab();
        assert_eq!(Some(ids[0]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(ids[0]), app.focused());

        app.focus(ids[1]);

        app.press_tab();
        assert_eq!(Some(ids[1]), app.focused());
        app.press_shift_tab();
        assert_eq!(Some(ids[1]), app.focused());
    }
}

#[test]
pub fn two_continue_scopes_in_tab_cycle_window() {
    // TabNav::Continue in non-root widget scopes that are
    // FocusScopeOnFocus::FirstDescendant just behaves like normal containers.
    two_continue_scopes_or_containers_in_tab_cycle_window(true);
}
#[test]
pub fn two_containers_in_tab_cycle_window() {
    // the containers are not focus scopes, but they naturally
    // behave like one with TabNav::Continue, as long as the tab-indexes
    // are linear or AUTO.
    two_continue_scopes_or_containers_in_tab_cycle_window(false);
}
fn two_continue_scopes_or_containers_in_tab_cycle_window(focus_scope: bool) {
    let app = TestApp::start();

    let mut buttons_a = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids_a: Vec<_> = (0..3).map(|i| buttons_a.item_id(i)).collect();

    let mut buttons_b = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids_b: Vec<_> = (0..3).map(|i| buttons_b.item_id(i)).collect();

    let a = Stack! {
        direction = StackDirection::top_to_bottom();
        children = buttons_a;
        focus_scope;
        tab_nav = TabNav::Continue;
    };
    let b = Stack! {
        direction = StackDirection::top_to_bottom();
        children = buttons_b;
        focus_scope;
        tab_nav = TabNav::Continue;
    };
    let mut app = app.run(Stack!(left_to_right, ui_vec![a, b]));

    // forward nav goes through the first stack.
    assert_eq!(Some(ids_a[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids_a[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids_a[2]), app.focused());
    app.press_tab();
    // and then the second stack.
    assert_eq!(Some(ids_b[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids_b[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids_b[2]), app.focused());

    // and then cycles back to the first item in the first stack.
    app.press_tab();
    assert_eq!(Some(ids_a[0]), app.focused());

    // backward nav does the same in reverse.

    // cycles back to the last item of the last stack.
    app.press_shift_tab();
    assert_eq!(Some(ids_b[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[0]), app.focused());

    // then moves back to the last item of the first stack.
    app.press_shift_tab();
    assert_eq!(Some(ids_a[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[0]), app.focused());
    // then cycles again.
    app.press_shift_tab();
    assert_eq!(Some(ids_b[2]), app.focused());
}

#[test]
pub fn two_continue_scopes_with_mixed_indexes() {
    let app = TestApp::start();

    // the tab_index sequence goes back and forth, but
    // because the containers are scopes they do each container
    // at a time.
    //
    // we are testing that scopes contain their navigation here,
    // and because scopes exclude their branch when navigating out
    // it all works here. But you can break this by adding another
    // scope or other widgets with weird tab indexes in the root scope,
    // because the navigation goes back to the root momentarily, it can
    // jump back to a higher priority index without visiting all indexes.

    let mut buttons_a = ui_vec![
        Button! {
            child = Text!("Button 0");
            tab_index = 0;
        },
        Button! {
            child = Text!("Button 2");
            tab_index = 5;
        },
        Button! {
            child = Text!("Button 1");
            tab_index = 3;
        },
    ];
    let ids_a: Vec<_> = (0..3).map(|i| buttons_a.item_id(i)).collect();

    let mut buttons_b = ui_vec![
        Button! {
            child = Text!("Button 3");
            tab_index = 2;
        },
        Button! {
            child = Text!("Button 4");
            tab_index = 4;
        },
        Button! {
            child = Text!("Button 5");
            tab_index = 6;
        },
    ];
    let ids_b: Vec<_> = (0..3).map(|i| buttons_b.item_id(i)).collect();

    let a = Stack! {
        direction = StackDirection::top_to_bottom();
        children = buttons_a;
        focus_scope = true;
        tab_nav = TabNav::Continue;
    };
    let b = Stack! {
        direction = StackDirection::top_to_bottom();
        children = buttons_b;
        focus_scope = true;
        tab_nav = TabNav::Continue;
    };
    let mut app = app.run(Stack!(left_to_right, ui_vec![a, b]));

    // window starts at (0), that is also inside `a`.
    assert_eq!(Some(ids_a[0]), app.focused());

    // goes to next index in the same scope (3), does not goes to (2)
    app.press_tab();
    assert_eq!(Some(ids_a[2]), app.focused());

    // goes to next index in the same scope (5), again did not go to (4)
    app.press_tab();
    assert_eq!(Some(ids_a[1]), app.focused());

    // goes to (2) in the `b` scope now.
    app.press_tab();
    assert_eq!(Some(ids_b[0]), app.focused());
    // goes next to (4)
    app.press_tab();
    assert_eq!(Some(ids_b[1]), app.focused());
    // goes next to (6)
    app.press_tab();
    assert_eq!(Some(ids_b[2]), app.focused());

    // cycle back to (0)
    app.press_tab();
    assert_eq!(Some(ids_a[0]), app.focused());

    // the same backwards.
    app.press_shift_tab();
    assert_eq!(Some(ids_b[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[0]), app.focused());
}

#[test]
pub fn two_containers_with_mixed_indexes() {
    let app = TestApp::start();

    // the tab-indexes go back and forth in the containers
    // and because they are not each a scope the focus jumps
    // from on container to another.

    let mut buttons_a = ui_vec![
        Button! {
            child = Text!("Button 0");
            tab_index = 0;
        },
        Button! {
            child = Text!("Button 2");
            tab_index = 5;
        },
        Button! {
            child = Text!("Button 1");
            tab_index = 3;
        },
    ];
    let ids_a: Vec<_> = (0..3).map(|i| buttons_a.item_id(i)).collect();

    let mut buttons_b = ui_vec![
        Button! {
            child = Text!("Button 3");
            tab_index = 2;
        },
        Button! {
            child = Text!("Button 4");
            tab_index = 4;
        },
        Button! {
            child = Text!("Button 5");
            tab_index = 6;
        },
    ];
    let ids_b: Vec<_> = (0..3).map(|i| buttons_b.item_id(i)).collect();

    let a = Stack!(top_to_bottom, buttons_a);
    let b = Stack!(top_to_bottom, buttons_b);
    let mut app = app.run(Stack!(left_to_right, ui_vec![a, b]));

    // forward.

    // starts at `0`
    assert_eq!(Some(ids_a[0]), app.focused());
    // goes to `2` in `b`
    app.press_tab();
    assert_eq!(Some(ids_b[0]), app.focused());
    // goes to `3` back in `a`
    app.press_tab();
    assert_eq!(Some(ids_a[2]), app.focused());
    // goes to `4` back in `b`
    app.press_tab();
    assert_eq!(Some(ids_b[1]), app.focused());
    // goes to `5` back in `a`
    app.press_tab();
    assert_eq!(Some(ids_a[1]), app.focused());
    // goes to `6` back in `b`
    app.press_tab();
    assert_eq!(Some(ids_b[2]), app.focused());
    // cycle back to `0` in `a`
    app.press_tab();
    assert_eq!(Some(ids_a[0]), app.focused());

    // backward.
    app.press_shift_tab();
    assert_eq!(Some(ids_b[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[0]), app.focused());
}

#[test]
pub fn tab_index_skip() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            id = "Button 0";
            child = Text!("Button 0")
        },
        Button! {
            id = "Button 1";
            child = Text!("Button 1");
            tab_index = TabIndex::SKIP;
        },
        Button! {
            id = "Button 2";
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, buttons));

    assert_eq!(Some(ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[2]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[0]), app.focused());

    app.press_shift_tab();
    assert_eq!(Some(ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[2]), app.focused());
}

#[test]
pub fn tab_inner_container() {
    let app = TestApp::start();

    // sanity check for `tab_skip_inner_container`.

    let mut inner_buttons = ui_vec![
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let mut children = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack!(top_to_bottom, inner_buttons),
        Button! {
            child = Text!("Button 3")
        },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| children.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, children));

    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(item_ids[2]), app.focused());

    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());
}

#[test]
pub fn tab_skip_inner_container() {
    let app = TestApp::start();

    // we expect that TabIndex::SKIP skips the full widget branch
    // but that the items inside will still tab navigate if focused
    // directly.

    let mut inner_buttons = ui_vec![
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let mut children = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack! {
            direction = StackDirection::top_to_bottom();
            children = inner_buttons;
            tab_index = TabIndex::SKIP;
        },
        Button! {
            child = Text!("Button 3")
        },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| children.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, children));

    // assert skipped inner.
    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(item_ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());

    // assert that focused directly it still works.
    app.focus(inner_ids[0]);
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    // and continues normally.
    app.press_tab();
    assert_eq!(Some(item_ids[2]), app.focused());
    // but is skipped from the outside.
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());

    // and the same in reverse.
    app.focus(inner_ids[1]);
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(item_ids[2]), app.focused());
}

#[test]
pub fn tab_inner_scope_continue() {
    let app = TestApp::start();

    // sanity check for `tab_skip_inner_scope_continue`.

    let mut inner_buttons = ui_vec![
        Button! {
            id = "Button 1";
            child = Text!("Button 1")
        },
        Button! {
            id = "Button 2";
            child = Text!("Button 2")
        },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let mut children = ui_vec![
        Button! {
            id = "Button 0";
            child = Text!("Button 0")
        },
        Stack! {
            id = "Scope Continue";
            direction = StackDirection::top_to_bottom();
            children = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Continue;
        },
        Button! {
            id = "Button 3";
            child = Text!("Button 3")
        },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| children.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, children));

    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(item_ids[2]), app.focused());

    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());
}

#[test]
pub fn tab_skip_inner_scope_continue() {
    let app = TestApp::start();

    // we expect that TabIndex::SKIP skips the full widget branch
    // but that the items inside will still tab navigate if focused
    // directly.

    let mut inner_buttons = ui_vec![
        Button! {
            id = "Button 1";
            child = Text!("Button 1")
        },
        Button! {
            id = "Button 2";
            child = Text!("Button 2")
        },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let mut children = ui_vec![
        Button! {
            id = "Button 0";
            child = Text!("Button 0")
        },
        Stack! {
            id = "v_stack";
            direction = StackDirection::top_to_bottom();
            children = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Continue;
            tab_index = TabIndex::SKIP;
        },
        Button! {
            id = "Button 3";
            child = Text!("Button 3")
        },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| children.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, children));

    // assert skipped inner.
    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(item_ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());

    // assert that focused directly it still works.
    app.focus(inner_ids[0]);
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    // and continues normally.
    app.press_tab();
    assert_eq!(Some(item_ids[2]), app.focused());
    // but is skipped from the outside.
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());

    // and the same in reverse.
    app.focus(inner_ids[1]);
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(item_ids[2]), app.focused());
}

#[test]
pub fn tab_inner_scope_cycle() {
    let app = TestApp::start();

    // we expect tab navigation to enter the inner scope and get trapped in there.

    let mut inner_buttons = ui_vec![
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let mut children = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack! {
            direction = StackDirection::top_to_bottom();
            children = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Cycle;
        },
        Button! {
            child = Text!("Button 3")
        },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| children.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, children));

    // focus starts outside of inner cycle.
    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();

    // focus enters the inner cycle.
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    // we still are in the inner cycle.
    app.press_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());

    // same in reverse.
    app.focus(item_ids[2]);
    assert_eq!(Some(item_ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
}

#[test]
pub fn tab_inner_scope_contained() {
    let app = TestApp::start();

    // we expect tab navigation to enter the inner scope and get trapped in there.

    let mut inner_buttons = ui_vec![
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let mut children = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack! {
            direction = StackDirection::top_to_bottom();
            children = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Contained;
        },
        Button! {
            child = Text!("Button 3")
        },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| children.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, children));

    // focus starts outside of inner container.
    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();

    // focus enters the inner container.
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    // we still are in the inner container.
    app.press_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());

    // same in reverse.
    app.focus(item_ids[2]);
    assert_eq!(Some(item_ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[0]), app.focused());
}

#[test]
pub fn tab_inner_scope_once() {
    let app = TestApp::start();

    // we expect tab navigation to enter the inner scope but then leave it.

    let mut inner_buttons = ui_vec![
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let mut children = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack! {
            direction = StackDirection::top_to_bottom();
            children = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Once;
        },
        Button! {
            child = Text!("Button 3")
        },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| children.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, children));

    // focus starts outside of inner scope.
    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();

    // focus enters the inner scope.
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    // and we leave it already.
    assert_eq!(Some(item_ids[2]), app.focused());

    // same in reverse.
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(item_ids[0]), app.focused());
}

#[test]
pub fn tab_inner_scope_none() {
    let app = TestApp::start();

    // we expect tab navigation to enter the inner scope and then not move.

    let mut inner_buttons = ui_vec![
        Button! {
            id = "btn-1";
            child = Text!("Button 1")
        },
        Button! {
            id = "btn-2";
            child = Text!("Button 2")
        },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let mut children = ui_vec![
        Button! {
            id = "btn-0";
            child = Text!("Button 0")
        },
        Stack! {
            id = "v-stack";
            direction = StackDirection::top_to_bottom();
            children = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::None;
        },
        Button! {
            id = "btn-3";
            child = Text!("Button 3")
        },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| children.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, children));

    // focus starts outside of inner scope.
    assert_eq!(Some(item_ids[0]), app.focused());
    assert!(app.can_tab());
    app.press_tab();

    // focus enters the inner scope.
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    // and we did not move.
    assert_eq!(Some(inner_ids[0]), app.focused());
    assert!(!app.can_tab());

    // same in reverse.
    app.focus(item_ids[2]);
    assert_eq!(Some(item_ids[2]), app.focused());
    assert!(app.can_shift_tab());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert!(!app.can_shift_tab());
    assert_eq!(Some(inner_ids[1]), app.focused());
}

#[test]
pub fn tab_inner_scope_continue_to_non_focusable_siblings_focusable_child() {
    let app = TestApp::start();

    let btn1 = WidgetId::named("btn-1");
    let btn2 = WidgetId::named("btn-2");
    let mut app = app.run(Stack!(
        left_to_right,
        ui_vec![
            Stack! {
                id = "initial-scope";
                direction = StackDirection::top_to_bottom();
                focus_scope = true;
                tab_nav = TabNav::Continue;
                children = ui_vec![Button! {
                    id = btn1;
                    child = Text!("Btn 1");
                }];
            },
            Stack!(
                top_to_bottom,
                ui_vec![Button! {
                    id = btn2;
                    child = Text!("Btn 2");
                }]
            )
        ]
    ));

    assert_eq!(Some(btn1), app.focused());
    app.press_tab();
    assert_eq!(Some(btn2), app.focused());
}

#[test]
pub fn dont_focus_alt_when_alt_pressed_before_focusing_window() {
    let app = TestApp::start();

    let start_focus_id = WidgetId::new_unique();
    let buttons = ui_vec![
        Button! {
            child = Text!("Button 0");
            id = start_focus_id;
        },
        Button! {
            child = Text!("Button 1");
        },
    ];
    let alt_buttons = ui_vec![
        Button! {
            child = Text!("Alt 0");
        },
        Button! {
            child = Text!("Alt 1");
        },
    ];

    let mut app = app.run(Stack!(
        top_to_bottom,
        ui_vec![
            Stack! {
                direction = StackDirection::left_to_right();
                alt_focus_scope = true;
                children = alt_buttons;
            },
            Stack!(top_to_bottom, buttons)
        ]
    ));

    // clear
    app.take_focus_changed();
    app.take_return_focus_changed();
    assert_eq!(Some(start_focus_id), app.focused());

    // just an ALT release, no press:
    app.just_release_alt();
    assert!(app.take_focus_changed().is_empty());
}

#[test]
pub fn window_blur_focus() {
    let app = TestApp::start();

    let expected_id = WidgetId::new_unique();
    let buttons = ui_vec![
        Button! {
            child = Text!("Button 0");
        },
        Button! {
            child = Text!("Button 1");
            id = expected_id;
        },
    ];
    let alt_buttons = ui_vec![
        Button! {
            child = Text!("Alt 0");
        },
        Button! {
            child = Text!("Alt 1");
        },
    ];

    let mut app = app.run(Stack!(
        top_to_bottom,
        ui_vec![
            Stack! {
                alt_focus_scope = true;
                children = alt_buttons;
            },
            Stack!(top_to_bottom, buttons)
        ]
    ));

    app.press_tab();
    assert_eq!(Some(expected_id), app.focused());

    app.blur_window();
    assert_eq!(None, app.focused());
    app.focus_window();
    assert_eq!(Some(expected_id), app.focused());

    app.press_alt();
    assert_ne!(Some(expected_id), app.focused());

    app.blur_window();
    assert_eq!(None, app.focused());
    app.focus_window();
    assert_eq!(Some(expected_id), app.focused());
}

#[test]
pub fn focused_removed_by_interactivity() {
    let app = TestApp::start();
    let interactive = var(true);
    focused_removed_test(
        app,
        Button! {
            child = Text!("Button 1");
            interactive = interactive.clone()
        },
        || interactive.set(false),
    )
}
#[test]
pub fn focused_removed_by_collapsing() {
    let app = TestApp::start();
    let visibility = var(Visibility::Visible);
    focused_removed_test(
        app,
        Button! {
            child = Text!("Button 1");
            visibility = visibility.clone()
        },
        || visibility.set(Visibility::Collapsed),
    )
}
#[test]
pub fn focused_removed_by_making_not_focusable() {
    let app = TestApp::start();
    let focusable = var(true);
    focused_removed_test(
        app,
        Button! {
            child = Text!("Button 1");
            focusable = focusable.clone()
        },
        || focusable.set(false),
    )
}
fn focused_removed_test(app: TestAppBuilder<impl AppExtension>, button1: impl UiNode, set_var: impl FnOnce()) {
    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        button1,
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, buttons));

    app.focus(ids[1]);

    assert_eq!(Some(ids[1]), app.focused());

    app.set_vars(set_var);

    assert_ne!(Some(ids[1]), app.focused());
}
#[test]
pub fn focused_removed_by_deleting() {
    let app = TestApp::start();

    let exist = var(true);
    let button1_id = WidgetId::new_unique();

    let buttons = ui_vec! {
        Button! { child = Text!("Button 0") },
        DataView!(::<bool>, exist.clone(), hn!(|a: &DataViewArgs<bool>| {
            if a.data().get() {
                a.set_view(Button! { id = button1_id; child = Text!("Button 1") });
            } else {
                a.unset_view();
            }
        })),
        Button! { child = Text!("Button 2") }
    };

    let mut app = app.run(Stack!(top_to_bottom, buttons));

    app.focus(button1_id);
    assert_eq!(Some(button1_id), app.focused());

    app.set_vars(|| {
        exist.set(false);
    });

    assert_ne!(Some(button1_id), app.focused());
}

#[test]
pub fn focus_widget_or_parent_goes_to_parent() {
    let app = TestApp::start();

    let first_focus_id = WidgetId::new_unique();
    let parent_id = WidgetId::new_unique();
    let child_id = WidgetId::new_unique();

    let mut app = app.run(Stack!(
        top_to_bottom,
        ui_vec![
            Button! {
                id = first_focus_id;
                child = Text!("initial focus")
            },
            Container! {
                id = parent_id;
                focusable = true;
                child = Text! {
                    id = child_id;
                    focusable = false;
                    txt = "not focusable"
                }
            }
        ]
    ));

    assert_eq!(Some(first_focus_id), app.focused());
    app.focus(child_id); // not focusable, does nothing.
    assert_eq!(Some(first_focus_id), app.focused());

    app.focus_or_parent(child_id);
    assert_eq!(Some(parent_id), app.focused());
}

#[test]
pub fn focus_widget_or_child_goes_to_child() {
    let app = TestApp::start();

    let first_focus_id = WidgetId::named("first_focus");
    let parent_id = WidgetId::named("parent");
    let child_id = WidgetId::named("child");

    let mut app = app.run(Stack!(
        top_to_bottom,
        ui_vec![
            Button! {
                id = first_focus_id;
                child = Text!("initial focus")
            },
            Container! {
                id = parent_id;
                focusable = false;
                child = Text! {
                    id = child_id;
                    focusable = true;
                    txt = "focusable focusable"
                }
            }
        ]
    ));

    assert_eq!(Some(first_focus_id), app.focused());
    app.focus(parent_id); // not focusable, does nothing.
    assert_eq!(Some(first_focus_id), app.focused());

    app.focus_or_child(parent_id);

    assert_eq!(Some(child_id), app.focused());
}

#[test]
pub fn focus_continued_after_widget_id_move() {
    let app = TestApp::start();

    let id = WidgetId::new_unique();

    let do_move_id = var(false);

    let mut app = app.run(DataView!(
        ::<bool>,
        do_move_id.clone(),
        hn!(|a: &DataViewArgs<bool>| {
            if a.data().get() {
                a.set_view(Container! {
                    id = "some_other_place";
                    child = Button! {
                        id;
                        child = Text!("Button 1")
                    };
                });
            } else if a.view_is_nil() {
                a.set_view(Wgt! {
                    focusable = true;
                    id;
                });
            }
        }),
    ));

    assert_eq!(Some(id), app.focused());
    app.take_focus_changed();
    app.set_vars(|| do_move_id.set(true));

    assert_eq!(Some(id), app.focused());
    let evs = app.take_focus_changed();
    assert_eq!(1, evs.len());
    assert!(evs[0].is_widget_move());
    assert_eq!(FocusChangedCause::Recovery, evs[0].cause);
}

#[test]
pub fn focus_continued_after_widget_move_same_window() {
    let app = TestApp::start();

    let id = WidgetId::new_unique();
    let button = ArcNode::new(Button! {
        id;
        child = Text!("Click Me!");
    });
    let do_move = var(false);

    let mut app = app.run(Stack!(
        top_to_bottom,
        ui_vec![
            Container! {
                child = button.take_when(true)
            },
            Container! {
                child = button.take_when(do_move.clone())
            }
        ]
    ));
    assert_eq!(Some(id), app.focused());
    app.take_focus_changed();

    app.set_vars(|| do_move.set(true));

    assert_eq!(Some(id), app.focused());
    let evs = app.take_focus_changed();
    assert_eq!(1, evs.len());
    assert!(evs[0].is_widget_move());
    assert_eq!(FocusChangedCause::Recovery, evs[0].cause);
}

#[test]
pub fn focus_moves_to_new_window() {
    let app = TestApp::start();

    let main_id = WidgetId::new_unique();
    let win2_id = WidgetId::new_unique();
    let win3_id = WidgetId::new_unique();

    let mut app = app.run(Button! {
        id = main_id;
        child = Text!("Button in main window");
    });
    assert_eq!(Some(main_id), app.focused());

    app.open_window(Button! {
        id = win2_id;
        child = Text!("Button in second window");
    });
    assert_eq!(Some(win2_id), app.focused());

    app.open_window(Button! {
        id = win3_id;
        child = Text!("Button in third window");
    });
    assert_eq!(Some(win3_id), app.focused());
}

#[test]
pub fn focus_goes_to_parent_after_remove() {
    let app = TestApp::start();

    let parent_id = WidgetId::named("parent");
    let child_id = WidgetId::named("child");

    let interactive = var(true);

    let mut app = app.run(Stack!(
        top_to_bottom,
        ui_vec![Container! {
            id = parent_id;
            focusable = true;
            child = Button! {
                id = child_id;
                interactive = interactive.clone();
                child = Text!("item 'removed'")
            }
        }]
    ));

    app.focus(child_id);
    assert_eq!(Some(child_id), app.focused());
    app.take_focus_changed();

    app.set_vars(|| {
        interactive.set(false);
    });
    assert_eq!(Some(parent_id), app.focused());
    let evs = app.take_focus_changed();
    assert_eq!(1, evs.len());
    assert_eq!(FocusChangedCause::Recovery, evs[0].cause);
}

#[test]
pub fn directional_focus_up() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, buttons));

    app.focus(ids[2]);
    assert_eq!(Some(ids[2]), app.focused());

    app.press_up();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_up();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_focus_down() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, buttons));

    assert_eq!(Some(ids[0]), app.focused());

    app.press_down();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_down();
    assert_eq!(Some(ids[2]), app.focused());
}

#[test]
pub fn directional_focus_left() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(left_to_right, buttons));

    app.focus(ids[2]);
    assert_eq!(Some(ids[2]), app.focused());

    app.press_left();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_left();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_focus_right() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(left_to_right, buttons));

    assert_eq!(Some(ids[0]), app.focused());

    app.press_right();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_right();
    assert_eq!(Some(ids[2]), app.focused());
}

#[test]
pub fn directional_cycle_vertical() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run_window(Window! {
        directional_nav = DirectionalNav::Cycle;
        child = Stack!(top_to_bottom, buttons);
    });
    assert_eq!(Some(ids[0]), app.focused());

    app.press_up();
    assert_eq!(Some(ids[2]), app.focused());

    app.press_down();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_cycle_horizontal() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run_window(Window! {
        directional_nav = DirectionalNav::Cycle;
        child = Stack!(left_to_right, buttons);
    });
    assert_eq!(Some(ids[0]), app.focused());

    app.press_left();
    assert_eq!(Some(ids[2]), app.focused());

    app.press_right();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_contained_vertical() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run_window(Window! {
        directional_nav = DirectionalNav::Contained;
        child = Stack!(top_to_bottom, buttons);
    });
    assert_eq!(Some(ids[0]), app.focused());

    app.press_up();
    assert_eq!(Some(ids[0]), app.focused());

    app.press_down();
    assert_eq!(Some(ids[1]), app.focused());
}

#[test]
pub fn directional_contained_horizontal() {
    let app = TestApp::start();

    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Button! {
            child = Text!("Button 1")
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run_window(Window! {
        directional_nav = DirectionalNav::Contained;
        child = Stack!(left_to_right, buttons);
    });
    assert_eq!(Some(ids[0]), app.focused());

    app.press_left();
    assert_eq!(Some(ids[0]), app.focused());

    app.press_right();
    assert_eq!(Some(ids[1]), app.focused());
}

#[test]
pub fn directional_none() {
    fn test(press: impl Fn(&mut TestApp)) {
        let app = TestApp::start();

        let mut buttons = ui_vec![
            Button! {
                child = Text!("Button 0")
            },
            Button! {
                child = Text!("Button 1")
            },
            Button! {
                child = Text!("Button 2")
            },
        ];
        let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

        let mut app = app.run_window(Window! {
            directional_nav = DirectionalNav::None;
            child = Stack!(left_to_right, buttons);
        });

        app.focus(ids[1]);
        assert_eq!(Some(ids[1]), app.focused());

        press(&mut app);
        assert_eq!(Some(ids[1]), app.focused());
    }

    test(|a| a.press_up());
    test(|a| a.press_down());
    test(|a| a.press_left());
    test(|a| a.press_right());
}

#[test]
pub fn directional_continue_up() {
    let app = TestApp::start();

    let start_id = WidgetId::new_unique();
    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack! {
            direction = StackDirection::top_to_bottom();
            focus_scope = true;
            directional_nav = DirectionalNav::Continue;
            children = ui_vec![Button! {
                child = Text!("Button 1");
                id = start_id;
            }];
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, buttons));

    app.focus(start_id);
    assert_eq!(Some(start_id), app.focused());

    app.press_up();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_continue_down() {
    let app = TestApp::start();

    let start_id = WidgetId::new_unique();
    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack! {
            direction = StackDirection::top_to_bottom();
            focus_scope = true;
            directional_nav = DirectionalNav::Continue;
            children = ui_vec![Button! {
                child = Text!("Button 1");
                id = start_id;
            }];
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(top_to_bottom, buttons));

    app.focus(start_id);
    assert_eq!(Some(start_id), app.focused());

    app.press_down();
    assert_eq!(Some(ids[2]), app.focused());
}

#[test]
pub fn directional_continue_left() {
    let app = TestApp::start();

    let start_id = WidgetId::new_unique();
    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack! {
            direction = StackDirection::top_to_bottom();
            focus_scope = true;
            directional_nav = DirectionalNav::Continue;
            children = ui_vec![Button! {
                child = Text!("Button 1");
                id = start_id;
            }];
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(left_to_right, buttons));

    app.focus(start_id);
    assert_eq!(Some(start_id), app.focused());

    app.press_left();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_continue_right() {
    let app = TestApp::start();

    let start_id = WidgetId::new_unique();
    let mut buttons = ui_vec![
        Button! {
            child = Text!("Button 0")
        },
        Stack! {
            direction = StackDirection::top_to_bottom();
            focus_scope = true;
            directional_nav = DirectionalNav::Continue;
            children = ui_vec![Button! {
                child = Text!("Button 1");
                id = start_id;
            }];
        },
        Button! {
            child = Text!("Button 2")
        },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = app.run(Stack!(left_to_right, buttons));

    app.focus(start_id);
    assert_eq!(Some(start_id), app.focused());

    app.press_right();
    assert_eq!(Some(ids[2]), app.focused());
}

struct TestAppBuilder<E: AppExtension> {
    app: AppExtended<E>,
}
impl<E: AppExtension> TestAppBuilder<E> {
    pub fn run(self, child: impl UiNode) -> TestApp {
        self.run_window(Window!(child; id = "window root"))
    }
    pub fn run_window(self, window: window::WindowRoot) -> TestApp {
        let mut app = self.app.run_headless(false);

        let (focus_changed, return_focus_changed) = {
            let a = FOCUS_CHANGED_EVENT.receiver();
            let b = RETURN_FOCUS_CHANGED_EVENT.receiver();
            (a, b)
        };

        let window_id = app.open_window(async move { window });
        TestApp {
            app,
            window_id,
            focus_changed,
            return_focus_changed,
        }
    }
}

struct TestApp {
    app: HeadlessApp,
    pub window_id: WindowId,

    focus_changed: EventReceiver<FocusChangedArgs>,
    return_focus_changed: EventReceiver<ReturnFocusChangedArgs>,
}
impl TestApp {
    /// Start app scope.
    pub fn start() -> TestAppBuilder<impl AppExtension> {
        TestAppBuilder { app: APP.defaults() }
    }

    pub fn set_vars(&mut self, set: impl FnOnce()) {
        set();
        let _ = self.app.update(false);
    }

    pub fn close_main_window(&mut self) {
        let closed = self.app.close_window(self.window_id);
        assert!(closed);
    }

    pub fn open_window(&mut self, child: impl UiNode) -> WindowId {
        let id = self.app.open_window(async {
            Window! {
                child
            }
        });
        let _ = self.app.update(false);
        id
    }

    /*
    pub fn close_window(&mut self, window_id: WindowId) {
        let closed = self.app.close_window(window_id);
        assert!(closed);
    }
    */

    pub fn take_focus_changed(&mut self) -> Vec<FocusChangedArgs> {
        self.focus_changed.try_iter().collect()
    }

    pub fn take_return_focus_changed(&mut self) -> Vec<ReturnFocusChangedArgs> {
        self.return_focus_changed.try_iter().collect()
    }

    pub fn focused(&mut self) -> Option<WidgetId> {
        FOCUS.focused().get().as_ref().map(|w| w.widget_id())
    }

    pub fn can_tab(&self) -> bool {
        FOCUS_NEXT_CMD.is_enabled().get()
    }
    pub fn can_shift_tab(&self) -> bool {
        FOCUS_PREV_CMD.is_enabled().get()
    }

    pub fn press_tab(&mut self) {
        self.app.press_key(self.window_id, KeyCode::Tab, KeyLocation::Standard, Key::Tab)
    }
    pub fn press_shift_tab(&mut self) {
        self.app.press_shortcut(self.window_id, shortcut!(SHIFT + Tab));
    }

    pub fn press_alt(&mut self) {
        self.app
            .press_key(self.window_id, KeyCode::AltLeft, KeyLocation::Standard, Key::Alt);
    }
    pub fn press_esc(&mut self) {
        self.app
            .press_key(self.window_id, KeyCode::Escape, KeyLocation::Standard, Key::Escape);
    }

    pub fn press_up(&mut self) {
        self.app
            .press_key(self.window_id, KeyCode::ArrowUp, KeyLocation::Standard, Key::ArrowUp);
    }
    pub fn press_down(&mut self) {
        self.app
            .press_key(self.window_id, KeyCode::ArrowDown, KeyLocation::Standard, Key::ArrowDown);
    }
    pub fn press_left(&mut self) {
        self.app
            .press_key(self.window_id, KeyCode::ArrowLeft, KeyLocation::Standard, Key::ArrowLeft);
    }
    pub fn press_right(&mut self) {
        self.app
            .press_key(self.window_id, KeyCode::ArrowRight, KeyLocation::Standard, Key::ArrowRight);
    }

    pub fn just_release_alt(&mut self) {
        self.app.on_keyboard_input(
            self.window_id,
            KeyCode::AltLeft,
            KeyLocation::Standard,
            Key::Alt,
            KeyState::Released,
        );
        let _ = self.app.update(false);
    }

    pub fn focus(&mut self, widget_id: WidgetId) {
        FOCUS.focus_widget(widget_id, true);
        let _ = self.app.update(false);
    }

    pub fn focus_or_parent(&mut self, widget_id: WidgetId) {
        FOCUS.focus_widget_or_exit(widget_id, false, true);
        let _ = self.app.update(false);
    }

    pub fn focus_or_child(&mut self, widget_id: WidgetId) {
        FOCUS.focus_widget_or_enter(widget_id, false, true);
        let _ = self.app.update(false);
    }

    pub fn focus_window(&mut self) {
        self.app.focus_window(self.window_id)
    }

    pub fn blur_window(&mut self) {
        self.app.blur_window(self.window_id)
    }
}

trait TestList {
    fn item_id(&mut self, i: usize) -> WidgetId;
}
impl<L: UiNodeList> TestList for L {
    fn item_id(&mut self, i: usize) -> WidgetId {
        self.with_node(i, |n| n.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()).unwrap())
    }
}
