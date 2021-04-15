use zero_ui::core::app::HeadlessApp;
use zero_ui::core::gesture::HeadlessAppGestureExt;
use zero_ui::core::keyboard::HeadlessAppKeyboardExt;
use zero_ui::core::window::{HeadlessAppOpenWindowExt, WindowId};
use zero_ui::prelude::*;

#[test]
fn window_tab_cycle() {
    // default window! cycles TAB navigation

    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.widget_id(i)).collect();

    let mut app = TestApp::new(v_stack(buttons));

    assert_eq!(Some(ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[2]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[0]), app.focused());
}
#[test]
fn window_prev_tab_cycle() {
    // default window! cycles TAB navigation

    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.widget_id(i)).collect();

    let mut app = TestApp::new(v_stack(buttons));

    assert_eq!(Some(ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
fn window_tab_cycle_and_alt_scope() {
    // default window! with an ALT scope, TAB navigation cycles
    // by default in the ALT scope too.

    let buttons = widgets![button! { content = text("Button 0") }, button! { content = text("Button 1") },];
    let ids: Vec<_> = (0..2).map(|i| buttons.widget_id(i)).collect();

    let alt_buttons = widgets![
        button! { content = text("Alt 0") },
        button! { content = text("Alt 1") },
        button! { content = text("Alt 2") },
    ];
    let alt_ids: Vec<_> = (0..3).map(|i| alt_buttons.widget_id(i)).collect();

    let mut app = TestApp::new(v_stack(widgets![
        h_stack! {
            alt_focus_scope = true;
            items = alt_buttons;
        },
        v_stack(buttons)
    ]));

    assert_eq!(Some(ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_alt();
    assert_eq!(Some(alt_ids[0]), app.focused());

    app.press_tab();
    assert_eq!(Some(alt_ids[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(alt_ids[2]), app.focused());
    app.press_tab();
    assert_eq!(Some(alt_ids[0]), app.focused());

    app.press_esc();
    assert_eq!(Some(ids[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[0]), app.focused());
}
#[test]
fn window_prev_cycle_and_alt_scope() {
    // default window! with an ALT scope, TAB navigation cycles
    // by default in the ALT scope too.

    let buttons = widgets![button! { content = text("Button 0") }, button! { content = text("Button 1") },];
    let ids: Vec<_> = (0..2).map(|i| buttons.widget_id(i)).collect();

    let alt_buttons = widgets![
        button! { content = text("Alt 0") },
        button! { content = text("Alt 1") },
        button! { content = text("Alt 2") },
    ];
    let alt_ids: Vec<_> = (0..3).map(|i| alt_buttons.widget_id(i)).collect();

    let mut app = TestApp::new(v_stack(widgets![
        h_stack! {
            alt_focus_scope = true;
            items = alt_buttons;
        },
        v_stack(buttons)
    ]));

    assert_eq!(Some(ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[1]), app.focused());

    app.press_alt();
    assert_eq!(Some(alt_ids[0]), app.focused());

    app.press_shift_tab();
    assert_eq!(Some(alt_ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(alt_ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(alt_ids[0]), app.focused());

    app.press_alt(); // alt toggles when there is no inner alt scope.
    assert_eq!(Some(ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
fn window_tab_contained() {
    // window with TabNav::Contained.
    window_tab_contained_(TabNav::Contained);
}
#[test]
fn window_tab_continue() {
    // same as Contained for root widgets.
    window_tab_contained_(TabNav::Continue);
}
fn window_tab_contained_(tab_nav: TabNav) {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.widget_id(i)).collect();

    let mut app = TestApp::new_w(window! {
        tab_nav;
        content = v_stack(buttons);
    });

    assert_eq!(Some(ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[2]), app.focused());
    app.press_tab();
    // did not move.
    assert_eq!(Some(ids[2]), app.focused());
}
#[test]
fn window_prev_tab_contained() {
    // window with TabNav::Contained.
    window_prev_tab_contained_(TabNav::Contained);
}
#[test]
fn window_prev_tab_continue() {
    // same as Contained for root widgets.
    window_prev_tab_contained_(TabNav::Continue);
}
fn window_prev_tab_contained_(tab_nav: TabNav) {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.widget_id(i)).collect();

    let mut app = TestApp::new_w(window! {
        tab_nav;
        content = v_stack(buttons);
    });

    assert_eq!(Some(ids[0]), app.focused());
    app.press_shift_tab();
    // did not move
    assert_eq!(Some(ids[0]), app.focused());

    app.focus(ids[2]);
    assert_eq!(Some(ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[0]), app.focused());
}

#[test]
fn window_tab_once() {
    // we already start focused inside so Once==None in root widgets.
    window_tab_once_(TabNav::Once);
}
#[test]
fn window_tab_none() {
    // we already start focused inside so Once==None in root widgets.
    window_tab_once_(TabNav::None);
}
fn window_tab_once_(tab_nav: TabNav) {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.widget_id(i)).collect();

    let mut app = TestApp::new_w(window! {
        content = v_stack(buttons);
        tab_nav;
    });

    assert_eq!(Some(ids[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids[0]), app.focused());
}
#[test]
fn window_prev_tab_once() {
    // we already start focused inside so Once==None in root widgets.
    window_prev_tab_once_(TabNav::Once);
}
#[test]
fn window_prev_tab_none() {
    // we already start focused inside so Once==None in root widgets.
    window_prev_tab_once_(TabNav::None);
}
fn window_prev_tab_once_(tab_nav: TabNav) {
    let buttons = widgets![
        button! { content = text("Button 0") },
        button! { content = text("Button 1") },
        button! { content = text("Button 2") },
    ];
    let ids: Vec<_> = (0..3).map(|i| buttons.widget_id(i)).collect();

    let mut app = TestApp::new_w(window! {
        content = v_stack(buttons);
        tab_nav;
    });

    assert_eq!(Some(ids[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[0]), app.focused());

    app.focus(ids[2]);
    assert_eq!(Some(ids[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids[2]), app.focused());
}

#[test]
fn two_continue_scopes_in_tab_cycle_window() {
    // TabNav::Continue in non-root widget scopes that are
    // FocusScopeOnFocus::FirstDescendant just behaves like normal containers.
    two_continue_scopes_in_tab_cycle_window_(true);
}
#[test]
fn two_containers_in_tab_cycle_window() {
    two_continue_scopes_in_tab_cycle_window_(false);
}
fn two_continue_scopes_in_tab_cycle_window_(focus_scope: bool) {
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

    assert_eq!(Some(ids_a[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids_a[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids_a[2]), app.focused());
    app.press_tab();

    assert_eq!(Some(ids_b[0]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids_b[1]), app.focused());
    app.press_tab();
    assert_eq!(Some(ids_b[2]), app.focused());
    app.press_tab();

    assert_eq!(Some(ids_a[0]), app.focused());
    app.press_tab();
}

#[test]
fn two_continue_scopes_in_tab_cycle_window_prev_tab() {
    // TabNav::Continue in non-root widget scopes that are
    // FocusScopeOnFocus::FirstDescendant just behaves like normal containers.
    two_continue_scopes_in_tab_cycle_window_prev_tab_(true);
}
#[test]
fn two_containers_in_tab_cycle_window_prev_tab() {
    two_continue_scopes_in_tab_cycle_window_prev_tab_(false);
}
fn two_continue_scopes_in_tab_cycle_window_prev_tab_(focus_scope: bool) {
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

    assert_eq!(Some(ids_a[0]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_b[0]), app.focused());

    app.press_shift_tab();
    assert_eq!(Some(ids_a[2]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[1]), app.focused());
    app.press_shift_tab();
    assert_eq!(Some(ids_a[0]), app.focused());

    app.press_shift_tab();
    assert_eq!(Some(ids_b[2]), app.focused());
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
