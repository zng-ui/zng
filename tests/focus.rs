use zero_ui::core::app::HeadlessApp;
use zero_ui::core::gesture::HeadlessAppGestureExt;
use zero_ui::core::keyboard::HeadlessAppKeyboardExt;
use zero_ui::core::window::{HeadlessAppOpenWindowExt, WindowId};
use zero_ui::prelude::*;

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
            button! { content = text("Button 0"); tab_index = tab_ids[0] },
            button! { content = text("Button 1"); tab_index = tab_ids[1] },
            button! { content = text("Button 2"); tab_index = tab_ids[2] },
        ];
        // we collect the widget_id values in the TAB navigation order.
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.widget_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let mut app = TestApp::new(v_stack(buttons));

        // advance normally.
        assert_eq!(Some(ids[0]), app.focused());
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
            button! { content = text("Button 0"); tab_index = tab_ids[0] },
            button! { content = text("Button 1"); tab_index = tab_ids[1] },
        ];
        let mut ids: Vec<_> = (0..2).map(|i| (buttons.widget_id(i), tab_ids[i])).collect();
        ids.sort_by_key(|(_, ti)| *ti);
        let ids: Vec<_> = ids.into_iter().map(|(id, _)| id).collect();

        let alt_buttons = widgets![
            button! { content = text("Alt 0"); tab_index = tab_ids[2] },
            button! { content = text("Alt 1"); tab_index = tab_ids[3] },
            button! { content = text("Alt 2"); tab_index = tab_ids[4] },
        ];
        let mut alt_ids: Vec<_> = (0..3).map(|i| (alt_buttons.widget_id(i), tab_ids[i + 2])).collect();
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
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.widget_id(i), tab_ids[i])).collect();
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
        let mut ids: Vec<_> = (0..3).map(|i| (buttons.widget_id(i), tab_ids[i])).collect();
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
    // behave like one with TabNav::Continue.
    two_continue_scopes_or_containers_in_tab_cycle_window(false);
}
fn two_continue_scopes_or_containers_in_tab_cycle_window(focus_scope: bool) {
    let buttons_a = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids_a: Vec<_> = (0..3).map(|i| buttons_a.widget_id(i)).collect();

    let buttons_b = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids_b: Vec<_> = (0..3).map(|i| buttons_b.widget_id(i)).collect();

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
pub fn tab_index_skip() {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1"); tab_index = TabIndex::SKIP; },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.widget_id(i)).collect();

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
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.widget_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack(inner_buttons),
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.widget_id(i)).collect();

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
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.widget_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            tab_index = TabIndex::SKIP;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.widget_id(i)).collect();

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

    let inner_buttons = widgets![button! { content = text("Button 1") }, button! { content = text("Button 2") },];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.widget_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Continue;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.widget_id(i)).collect();

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

    let inner_buttons = widgets![button! { content = text("Button 1") }, button! { content = text("Button 2") },];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.widget_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Continue;
            tab_index = TabIndex::SKIP;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.widget_id(i)).collect();

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
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.widget_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Cycle;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.widget_id(i)).collect();

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
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.widget_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Contained;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.widget_id(i)).collect();

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
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.widget_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::Once;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.widget_id(i)).collect();

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

    let inner_buttons = widgets![button! { content = text("Button 1") }, button! { content = text("Button 2") },];
    let inner_ids: Vec<_> = (0..2).map(|i| inner_buttons.widget_id(i)).collect();
    let items = widgets![
        button! { content = text("Button 0") },
        v_stack! {
            items = inner_buttons;
            focus_scope = true;
            tab_nav = TabNav::None;
        },
        button! { content = text("Button 3") },
    ];
    let item_ids: Vec<_> = (0..3).map(|i| items.widget_id(i)).collect();

    let mut app = TestApp::new(v_stack(items));

    // focus starts outside of inner scope.
    assert_eq!(Some(item_ids[0]), app.focused());
    app.press_tab();

    // focus enters the inner scope.
    assert_eq!(Some(inner_ids[0]), app.focused());
    app.press_tab();
    // and we did not move.
    assert_eq!(Some(inner_ids[0]), app.focused());

    // same in reverse.
    app.focus(item_ids[2]);
    assert_eq!(Some(item_ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(inner_ids[1]), app.focused());
}

struct TestApp {
    app: HeadlessApp,
    window_id: WindowId,
}
impl TestApp {
    pub fn new(content: impl UiNode) -> Self {
        Self::new_w(window!(content))
    }
    pub fn new_w(window: Window) -> Self {
        let mut app = App::default().run_headless();
        let window_id = app.open_window(move |_| window);
        TestApp { app, window_id }
    }

    pub fn focused(&mut self) -> Option<WidgetId> {
        self.app
            .with_context(|ctx| ctx.services.req::<Focus>().focused().map(|w| w.widget_id()))
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

    pub fn focus(&mut self, widget_id: WidgetId) {
        self.app
            .with_context(|ctx| ctx.services.req::<Focus>().focus_widget(widget_id, true));
        self.app.update();
    }
}
