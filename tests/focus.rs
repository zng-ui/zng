use zero_ui::{
    core::{
        app::HeadlessApp,
        event::EventBuffer,
        focus::{FocusChangedArgs, FocusChangedCause, ReturnFocusChangedArgs, FOCUS_CHANGED_EVENT, RETURN_FOCUS_CHANGED_EVENT},
        gesture::HeadlessAppGestureExt,
        keyboard::HeadlessAppKeyboardExt,
        window::{HeadlessAppWindowExt, WindowId},
    },
    prelude::*,
};

#[test]
pub fn first_and_last_window_events() {
    let buttons = widgets![button! { content = text("Button 0") }, button! { content = text("Button 1") },];

    let root_id = WidgetId::new_unique();
    let stack_id = WidgetId::new_unique();
    let button_0_id = buttons.item_id(0);

    let mut app = TestApp::new_w(window! {
        content = v_stack!(id = stack_id; items = buttons);
        root_id;
    });
    let root_path = InteractionPath::new_enabled(app.window_id, [root_id]);
    let button_0_path = InteractionPath::new_enabled(app.window_id, [root_id, stack_id, button_0_id]);

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
    // default window! cycles TAB navigation.
    t(|_| TabIndex::AUTO);
    t(TabIndex);
    t(|i| TabIndex(TabIndex::AUTO.0 - i - 1));

    fn t(make_index: impl FnMut(u32) -> TabIndex) {
        // all TAB navigation must respect the `tab_index` value
        // that by default is AUTO, but can be not in the same order
        // as the widgets are declared.
        let tab_ids: Vec<_> = (0..3).map(make_index).collect();

        let buttons = widgets![
            button! { id = "btn-0"; content = text("Button 0"); tab_index = tab_ids[0] },
            button! { id = "btn-1"; content = text("Button 1"); tab_index = tab_ids[1] },
            button! { id = "btn-2"; content = text("Button 2"); tab_index = tab_ids[2] },
        ];
        // we collect the widget_id values in the TAB navigation order.
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.item_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let mut app = TestApp::new(v_stack(buttons));

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
    // default window! with an ALT scope, TAB navigation cycles
    // by default in the ALT scope too.

    t(|_| TabIndex::AUTO);
    t(TabIndex);
    t(|i| TabIndex(TabIndex::AUTO.0 - i - 1));

    fn t(make_index: impl FnMut(u32) -> TabIndex) {
        let tab_ids: Vec<_> = (0..5).map(make_index).collect();

        let buttons = widgets![
            button! { id = "btn-0"; content = text("Button 0"); tab_index = tab_ids[0] },
            button! { id = "btn-1"; content = text("Button 1"); tab_index = tab_ids[1] },
        ];
        let mut ids: Vec<_> = (0..2).map(|i| (buttons.item_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let alt_buttons = widgets![
            button! { id = "alt-0"; content = text("Alt 0"); tab_index = tab_ids[2] },
            button! { id = "alt-1"; content = text("Alt 1"); tab_index = tab_ids[3] },
            button! { id = "alt-2"; content = text("Alt 2"); tab_index = tab_ids[4] },
        ];
        let mut alt_ids: Vec<_> = (0..3).map(|i| (alt_buttons.item_id(i), tab_ids[i + 2])).collect();
        alt_ids.sort_by_key(|(_, ti)| *ti);
        let alt_ids: Vec<_> = alt_ids.into_iter().map(|(id, _)| id).collect();

        let mut app = TestApp::new(v_stack(widgets![
            h_stack! {
                alt_focus_scope = true;
                items = alt_buttons;
            },
            v_stack(buttons)
        ]));

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
        let tab_ids: Vec<_> = (0..3).map(make_index).collect();

        let buttons = widgets![
            button! { content = text("Button 0"); tab_index = tab_ids[0] },
            button! { content = text("Button 1"); tab_index = tab_ids[1] },
            button! { content = text("Button 2"); tab_index = tab_ids[2] },
        ];
        // we collect the widget_id values in the TAB navigation order.
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.item_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let mut app = TestApp::new_w(window! {
            tab_nav;
            content = v_stack(buttons);
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
        let tab_ids: Vec<_> = (0..3).map(make_index).collect();

        let buttons = widgets![
            button! { content = text("Button 0"); tab_index = tab_ids[0] },
            button! { content = text("Button 1"); tab_index = tab_ids[1] },
            button! { content = text("Button 2"); tab_index = tab_ids[2] },
        ];
        // we collect the widget_id values in the TAB navigation order.
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.item_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let mut app = TestApp::new_w(window! {
            content = v_stack(buttons);
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
    let buttons_a = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids_a: Vec<_> = (0..3).map(|i| buttons_a.item_id(i)).collect();

    let buttons_b = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids_b: Vec<_> = (0..3).map(|i| buttons_b.item_id(i)).collect();

    let a = v_stack! {
        items = buttons_a;
        focus_scope;
        tab_nav = TabNav::Continue;
    };
    let b = v_stack! {
        items = buttons_b;
        focus_scope;
        tab_nav = TabNav::Continue;
    };
    let mut app = TestApp::new(h_stack(widgets![a, b]));

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

    let buttons_a = widgets![
        button! { content = text("Button 0"); tab_index = 0; },
        button! { content = text("Button 2"); tab_index = 5; },
        button! { content = text("Button 1"); tab_index = 3; },
    ];
    let ids_a: Vec<_> = (0..3).map(|i| buttons_a.item_id(i)).collect();

    let buttons_b = widgets![
        button! { content = text("Button 3"); tab_index = 2; },
        button! { content = text("Button 4"); tab_index = 4; },
        button! { content = text("Button 5"); tab_index = 6; },
    ];
    let ids_b: Vec<_> = (0..3).map(|i| buttons_b.item_id(i)).collect();

    let a = v_stack! {
        items = buttons_a;
        focus_scope = true;
        tab_nav = TabNav::Continue;
    };
    let b = v_stack! {
        items = buttons_b;
        focus_scope = true;
        tab_nav = TabNav::Continue;
    };
    let mut app = TestApp::new(h_stack(widgets![a, b]));

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
    // the tab-indexes go back and forth in the containers
    // and because they are not each a scope the focus jumps
    // from on container to another.

    let buttons_a = widgets![
        button! { content = text("Button 0"); tab_index = 0; },
        button! { content = text("Button 2"); tab_index = 5; },
        button! { content = text("Button 1"); tab_index = 3; },
    ];
    let ids_a: Vec<_> = (0..3).map(|i| buttons_a.item_id(i)).collect();

    let buttons_b = widgets![
        button! { content = text("Button 3"); tab_index = 2; },
        button! { content = text("Button 4"); tab_index = 4; },
        button! { content = text("Button 5"); tab_index = 6; },
    ];
    let ids_b: Vec<_> = (0..3).map(|i| buttons_b.item_id(i)).collect();

    let a = v_stack(buttons_a);
    let b = v_stack(buttons_b);
    let mut app = TestApp::new(h_stack(widgets![a, b]));

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
    let buttons = widgets![
        button! { id = "Button 0"; content = text("Button 0") },
        button! { id = "Button 1"; content = text("Button 1"); tab_index = TabIndex::SKIP; },
        button! { id = "Button 2"; content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(buttons));

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
    // sanity check for  `tab_skip_inner_container`.

    let inner_buttons = widgets![button! { content = text("Button 1") }, button! { content = text("Button 2") },];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack(inner_buttons),
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

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
    // we expect that TabIndex::SKIP skips the full widget branch
    // but that the items inside will still tab navigate if focused
    // directly.

    let inner_buttons = widgets![button! { content = text("Button 1") }, button! { content = text("Button 2") },];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            tab_index = TabIndex::SKIP;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

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
    // sanity check for `tab_skip_inner_scope_continue`.

    let inner_buttons = widgets![
        button! { id = "Button 1"; content = text("Button 1") },
        button! { id = "Button 2"; content = text("Button 2") },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let items = widgets![
        button! { id = "Button 0";  content = text("Button 0") },
        v_stack! {
            id = "Scope Continue";
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Continue;
        },
        button! { id = "Button 3"; content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

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
    // we expect that TabIndex::SKIP skips the full widget branch
    // but that the items inside will still tab navigate if focused
    // directly.

    let inner_buttons = widgets![
        button! { id = "Button 1"; content = text("Button 1") },
        button! { id = "Button 2"; content = text("Button 2") },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let items = widgets![
        button! { id = "Button 0"; content = text("Button 0") },
        v_stack! {
            id = "v_stack";
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Continue;
            tab_index = TabIndex::SKIP;
        },
        button! { id = "Button 3"; content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

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
    // we expect tab navigation to enter the inner scope and get trapped in there.

    let inner_buttons = widgets![button! { content = text("Button 1") }, button! { content = text("Button 2") },];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Cycle;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

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
    // we expect tab navigation to enter the inner scope and get trapped in there.

    let inner_buttons = widgets![button! { content = text("Button 1") }, button! { content = text("Button 2") },];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Contained;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

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
    // we expect tab navigation to enter the inner scope but then leave it.

    let inner_buttons = widgets![button! { content = text("Button 1") }, button! { content = text("Button 2") },];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Once;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

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
    // we expect tab navigation to enter the inner scope and then not move.

    let inner_buttons = widgets![
        button! { id = "btn-1"; content = text("Button 1") },
        button! { id = "btn-2"; content = text("Button 2") },
    ];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.item_id(i)).collect();
    let items = widgets![
        button! { id = "btn-0"; content = text("Button 0") },
        v_stack! {
            id = "v-stack";
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::None;
        },
        button! { id = "btn-3"; content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

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
    let btn1 = WidgetId::named("btn-1");
    let btn2 = WidgetId::named("btn-2");
    let mut app = TestApp::new(h_stack(widgets![
        v_stack! {
            id = "initial-scope";
            focus_scope = true;
            tab_nav = TabNav::Continue;
            items = widgets![button! { id = btn1; content = text("Btn 1"); }];
        },
        v_stack(widgets![button! { id = btn2; content = text("Btn 2"); }])
    ]));

    assert_eq!(Some(btn1), app.focused());
    app.press_tab();
    assert_eq!(Some(btn2), app.focused());
}

#[test]
pub fn dont_focus_alt_when_alt_pressed_before_focusing_window() {
    let start_focus_id = WidgetId::new_unique();
    let buttons = widgets![
        button! { content = text("Button 0"); id = start_focus_id; },
        button! { content = text("Button 1"); },
    ];
    let alt_buttons = widgets![button! { content = text("Alt 0"); }, button! { content = text("Alt 1"); },];

    let mut app = TestApp::new(v_stack(widgets![
        h_stack! {
            alt_focus_scope = true;
            items = alt_buttons;
        },
        v_stack(buttons)
    ]));

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
    let expected_id = WidgetId::new_unique();
    let buttons = widgets![
        button! { content = text("Button 0"); },
        button! { content = text("Button 1"); id = expected_id; },
    ];
    let alt_buttons = widgets![button! { content = text("Alt 0"); }, button! { content = text("Alt 1"); },];

    let mut app = TestApp::new(v_stack(widgets![
        h_stack! {
            alt_focus_scope = true;
            items = alt_buttons;
        },
        v_stack(buttons)
    ]));

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
pub fn focused_removed_by_interacivity() {
    let interactive = var(true);
    focused_removed_test(button! { content = text("Button 1"); interactive = interactive.clone() }, |vars| {
        interactive.set(vars, false)
    })
}
#[test]
pub fn focused_removed_by_collapsing() {
    let visibility = var(Visibility::Visible);
    focused_removed_test(button! { content = text("Button 1"); visibility = visibility.clone() }, |vars| {
        visibility.set(vars, Visibility::Collapsed)
    })
}
#[test]
pub fn focused_removed_by_making_not_focusable() {
    let focusable = var(true);
    focused_removed_test(button! { content = text("Button 1"); focusable = focusable.clone() }, |vars| {
        focusable.set(vars, false)
    })
}
fn focused_removed_test(button1: impl Widget, set_var: impl FnOnce(&Vars)) {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button1,
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(buttons));

    app.focus(ids[1]);

    assert_eq!(Some(ids[1]), app.focused());

    app.set_vars(set_var);

    assert_ne!(Some(ids[1]), app.focused());
}
#[test]
pub fn focused_removed_by_deleting() {
    let exist = var(true);
    let button1_id = WidgetId::new_unique();

    let buttons = widgets! {
        button! { content = text("Button 0") },
        view(exist.clone(), NilUiNode.boxed(), move |ctx, exist| {
            if exist.copy(ctx) {
                View::Update(button! { id = button1_id; content = text("Button 1") }.boxed())
            } else {
                View::Update(NilUiNode.boxed())
            }
        }),
        button! { content = text("Button 2") }
    };

    let mut app = TestApp::new(v_stack(buttons));

    app.focus(button1_id);
    assert_eq!(Some(button1_id), app.focused());

    app.set_vars(|vars| {
        exist.set(vars, false);
    });

    assert_ne!(Some(button1_id), app.focused());
}

#[test]
pub fn focus_widget_or_parent_goes_to_parent() {
    let first_focus_id = WidgetId::new_unique();
    let parent_id = WidgetId::new_unique();
    let child_id = WidgetId::new_unique();

    let mut app = TestApp::new(v_stack(widgets![
        button! {
            id = first_focus_id;
            content = text("initial focus")
        },
        container! {
            id = parent_id;
            focusable = true;
            content = text! {
                id = child_id;
                focusable = false;
                text = "not focusable"
            }
        }
    ]));

    assert_eq!(Some(first_focus_id), app.focused());
    app.focus(child_id); // not focusable, does nothing.
    assert_eq!(Some(first_focus_id), app.focused());

    app.focus_or_parent(child_id);
    assert_eq!(Some(parent_id), app.focused());
}

#[test]
pub fn focus_widget_or_child_goes_to_child() {
    let first_focus_id = WidgetId::named("first_focus");
    let parent_id = WidgetId::named("parent");
    let child_id = WidgetId::named("child");

    let mut app = TestApp::new(v_stack(widgets![
        button! {
            id = first_focus_id;
            content = text("initial focus")
        },
        container! {
            id = parent_id;
            focusable = false;
            content = text! {
                id = child_id;
                focusable = true;
                text = "focusable focusable"
            }
        }
    ]));

    assert_eq!(Some(first_focus_id), app.focused());
    app.focus(parent_id); // not focusable, does nothing.
    assert_eq!(Some(first_focus_id), app.focused());

    app.focus_or_child(parent_id);

    assert_eq!(Some(child_id), app.focused());
}

#[test]
pub fn focus_continued_after_widget_id_move() {
    let id = WidgetId::new_unique();

    let do_move_id = var(false);

    let mut app = TestApp::new(view(
        do_move_id.clone(),
        blank! { focusable = true; id; }.boxed(),
        move |ctx, do_move_id| {
            if do_move_id.copy(ctx) {
                View::Update({
                    container! {
                        id = "some_other_place";
                        content = button! { id; content = text("Button 1") };
                    }
                    .boxed()
                })
            } else {
                View::Same
            }
        },
    ));

    assert_eq!(Some(id), app.focused());
    app.take_focus_changed();
    app.set_vars(|vars| do_move_id.set(vars, true));

    assert_eq!(Some(id), app.focused());
    let evs = app.take_focus_changed();
    assert_eq!(1, evs.len());
    assert!(evs[0].is_widget_move());
    assert_eq!(FocusChangedCause::Recovery, evs[0].cause);
}

#[test]
pub fn focus_continued_after_widget_move_same_window() {
    let id = WidgetId::new_unique();
    let button = RcNode::new(button! {
        id;
        content = text("Click Me!");
    });
    let do_move = var(false);

    let mut app = TestApp::new(v_stack(widgets![
        container! {
            content = button.slot(slot::take_on_init())
        },
        container! {
            content = button.slot(do_move.clone())
        }
    ]));
    assert_eq!(Some(id), app.focused());
    app.take_focus_changed();

    app.set_vars(|vars| do_move.set(vars, true));

    assert_eq!(Some(id), app.focused());
    let evs = app.take_focus_changed();
    assert_eq!(1, evs.len());
    assert!(evs[0].is_widget_move());
    assert_eq!(FocusChangedCause::Recovery, evs[0].cause);
}

#[test]
pub fn focus_moves_to_new_window() {
    let main_id = WidgetId::new_unique();
    let win2_id = WidgetId::new_unique();
    let win3_id = WidgetId::new_unique();

    let mut app = TestApp::new(button! {
        id = main_id;
        content = text("Button in main window");
    });
    assert_eq!(Some(main_id), app.focused());

    app.open_window(button! {
        id = win2_id;
        content = text("Button in second window");
    });
    assert_eq!(Some(win2_id), app.focused());

    app.open_window(button! {
        id = win3_id;
        content = text("Button in third window");
    });
    assert_eq!(Some(win3_id), app.focused());
}

#[test]
pub fn focus_goes_to_parent_after_remove() {
    let parent_id = WidgetId::named("parent");
    let child_id = WidgetId::named("child");

    let interactive = var(true);

    let mut app = TestApp::new(v_stack(widgets![container! {
        id = parent_id;
        focusable = true;
        content = button! {
            id = child_id;
            interactive = interactive.clone();
            content = text( "item 'removed'")
        }
    }]));

    app.focus(child_id);
    assert_eq!(Some(child_id), app.focused());
    app.take_focus_changed();

    app.set_vars(|vars| {
        interactive.set(vars, false);
    });
    assert_eq!(Some(parent_id), app.focused());
    let evs = app.take_focus_changed();
    assert_eq!(1, evs.len());
    assert_eq!(FocusChangedCause::Recovery, evs[0].cause);
}

#[test]
pub fn directional_focus_up() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(buttons));

    app.focus(ids[2]);
    assert_eq!(Some(ids[2]), app.focused());

    app.press_up();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_up();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_focus_down() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(buttons));

    assert_eq!(Some(ids[0]), app.focused());

    app.press_down();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_down();
    assert_eq!(Some(ids[2]), app.focused());
}

#[test]
pub fn directional_focus_left() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(h_stack(buttons));

    app.focus(ids[2]);
    assert_eq!(Some(ids[2]), app.focused());

    app.press_left();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_left();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_focus_right() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(h_stack(buttons));

    assert_eq!(Some(ids[0]), app.focused());

    app.press_right();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_right();
    assert_eq!(Some(ids[2]), app.focused());
}

#[test]
pub fn directional_cycle_vertical() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new_w(window! {
        directional_nav = DirectionalNav::Cycle;
        content = v_stack(buttons);
    });
    assert_eq!(Some(ids[0]), app.focused());

    app.press_up();
    assert_eq!(Some(ids[2]), app.focused());

    app.press_down();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_cycle_horizontal() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new_w(window! {
        directional_nav = DirectionalNav::Cycle;
        content = h_stack(buttons);
    });
    assert_eq!(Some(ids[0]), app.focused());

    app.press_left();
    assert_eq!(Some(ids[2]), app.focused());

    app.press_right();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_contained_vertical() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new_w(window! {
        directional_nav = DirectionalNav::Contained;
        content = v_stack(buttons);
    });
    assert_eq!(Some(ids[0]), app.focused());

    app.press_up();
    assert_eq!(Some(ids[0]), app.focused());

    app.press_down();
    assert_eq!(Some(ids[1]), app.focused());
}

#[test]
pub fn directional_contained_horizontal() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new_w(window! {
        directional_nav = DirectionalNav::Contained;
        content = h_stack(buttons);
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
        let buttons = widgets![
            button! { content = text("Button 0") },
            button! { content = text("Button 1") },
            button! { content = text("Button 2") },
        ];
        let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

        let mut app = TestApp::new_w(window! {
            directional_nav = DirectionalNav::None;
            content = h_stack(buttons);
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
    let start_id = WidgetId::new_unique();
    let buttons = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            focus_scope = true;
            directional_nav = DirectionalNav::Continue;
            items = widgets![
                button! { content = text("Button 1"); id = start_id; },
            ];
        },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(buttons));

    app.focus(start_id);
    assert_eq!(Some(start_id), app.focused());

    app.press_up();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_continue_down() {
    let start_id = WidgetId::new_unique();
    let buttons = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            focus_scope = true;
            directional_nav = DirectionalNav::Continue;
            items = widgets![
                button! { content = text("Button 1"); id = start_id; },
            ];
        },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(v_stack(buttons));

    app.focus(start_id);
    assert_eq!(Some(start_id), app.focused());

    app.press_down();
    assert_eq!(Some(ids[2]), app.focused());
}

#[test]
pub fn directional_continue_left() {
    let start_id = WidgetId::new_unique();
    let buttons = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            focus_scope = true;
            directional_nav = DirectionalNav::Continue;
            items = widgets![
                button! { content = text("Button 1"); id = start_id; },
            ];
        },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(h_stack(buttons));

    app.focus(start_id);
    assert_eq!(Some(start_id), app.focused());

    app.press_left();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
pub fn directional_continue_right() {
    let start_id = WidgetId::new_unique();
    let buttons = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            focus_scope = true;
            directional_nav = DirectionalNav::Continue;
            items = widgets![
                button! { content = text("Button 1"); id = start_id; },
            ];
        },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.item_id(i)).collect();

    let mut app = TestApp::new(h_stack(buttons));

    app.focus(start_id);
    assert_eq!(Some(start_id), app.focused());

    app.press_right();
    assert_eq!(Some(ids[2]), app.focused());
}

struct TestApp {
    app: HeadlessApp,
    pub window_id: WindowId,

    focus_changed: EventBuffer<FocusChangedArgs>,
    return_focus_changed: EventBuffer<ReturnFocusChangedArgs>,
}
impl TestApp {
    pub fn new(content: impl UiNode) -> Self {
        Self::new_w(window!(content; root_id = "window root"))
    }
    pub fn new_w(window: Window) -> Self {
        let mut app = App::default().run_headless(false);

        let (focus_changed, return_focus_changed) = {
            let ctx = app.ctx();
            let a = ctx.events.buffer(FOCUS_CHANGED_EVENT);
            let b = ctx.events.buffer(RETURN_FOCUS_CHANGED_EVENT);
            (a, b)
        };

        let window_id = app.open_window(move |_| window);
        TestApp {
            app,
            window_id,
            focus_changed,
            return_focus_changed,
        }
    }

    pub fn set_vars(&mut self, set: impl FnOnce(&Vars)) {
        set(self.app.ctx().vars);
        let _ = self.app.update(false);
    }

    pub fn close_main_window(&mut self) {
        let closed = self.app.close_window(self.window_id);
        assert!(closed);
    }

    pub fn open_window(&mut self, content: impl UiNode) -> WindowId {
        let id = self.app.open_window(|_| {
            window! {
                content
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
        self.focus_changed.pop_all()
    }

    pub fn take_return_focus_changed(&mut self) -> Vec<ReturnFocusChangedArgs> {
        self.return_focus_changed.pop_all()
    }

    pub fn focused(&mut self) -> Option<WidgetId> {
        let ctx = self.app.ctx();
        Focus::req(ctx.services).focused().get(ctx.vars).as_ref().map(|w| w.widget_id())
    }

    pub fn can_tab(&self) -> bool {
        zero_ui::core::focus::commands::FOCUS_NEXT_CMD.is_enabled().copy(self.app.vars())
    }
    pub fn can_shift_tab(&self) -> bool {
        zero_ui::core::focus::commands::FOCUS_PREV_CMD.is_enabled().copy(self.app.vars())
    }

    pub fn press_tab(&mut self) {
        self.app.press_key(self.window_id, Key::Tab)
    }
    pub fn press_shift_tab(&mut self) {
        self.app.press_shortcut(self.window_id, shortcut!(SHIFT + Tab));
    }

    pub fn press_alt(&mut self) {
        self.app.press_key(self.window_id, Key::LAlt);
    }
    pub fn press_esc(&mut self) {
        self.app.press_key(self.window_id, Key::Escape);
    }

    pub fn press_up(&mut self) {
        self.app.press_key(self.window_id, Key::Up);
    }
    pub fn press_down(&mut self) {
        self.app.press_key(self.window_id, Key::Down);
    }
    pub fn press_left(&mut self) {
        self.app.press_key(self.window_id, Key::Left);
    }
    pub fn press_right(&mut self) {
        self.app.press_key(self.window_id, Key::Right);
    }

    pub fn just_release_alt(&mut self) {
        self.app.on_keyboard_input(self.window_id, Key::LAlt, KeyState::Released);
        let _ = self.app.update(false);
    }

    pub fn focus(&mut self, widget_id: WidgetId) {
        Focus::req(&mut self.app).focus_widget(widget_id, true);
        let _ = self.app.update(false);
    }

    pub fn focus_or_parent(&mut self, widget_id: WidgetId) {
        Focus::req(&mut self.app).focus_widget_or_exit(widget_id, true);
        let _ = self.app.update(false);
    }

    pub fn focus_or_child(&mut self, widget_id: WidgetId) {
        Focus::req(&mut self.app).focus_widget_or_enter(widget_id, true);
        let _ = self.app.update(false);
    }

    pub fn focus_window(&mut self) {
        self.app.focus_window(self.window_id)
    }

    pub fn blur_window(&mut self) {
        self.app.blur_window(self.window_id)
    }

    #[allow(unused)]
    pub fn write_tree(&mut self) {
        use zero_ui::core::inspector::prompt::*;

        let ctx = self.app.ctx();
        let tree = Windows::req(ctx.services).widget_tree(self.window_id).unwrap();
        write_tree(ctx.vars, tree, &WriteTreeState::none(), &mut std::io::stdout());
    }
}
