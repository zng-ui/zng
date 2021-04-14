use zero_ui::core::app::HeadlessApp;
use zero_ui::core::keyboard::HeadlessAppKeyboardExt;
use zero_ui::core::window::{HeadlessAppOpenWindowExt, WindowId};
use zero_ui::prelude::*;

#[test]
fn basic_cycle() {
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
fn basic_cycle_and_alt_scope() {
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

struct TestApp {
    app: HeadlessApp,
    window_id: WindowId,
}
impl TestApp {
    pub fn new(content: impl UiNode) -> Self {
        let mut app = App::default().run_headless();
        let window_id = app.open_window(|_| window!(content));
        TestApp { app, window_id }
    }

    pub fn focused(&mut self) -> Option<WidgetId> {
        self.app
            .with_context(|ctx| ctx.services.req::<Focus>().focused().map(|w| w.widget_id()))
    }

    pub fn press_tab(&mut self) {
        self.app.press_key(self.window_id, Key::Tab)
    }

    pub fn press_alt(&mut self) {
        self.app.press_key(self.window_id, Key::LAlt);
    }

    pub fn press_esc(&mut self) {
        self.app.press_key(self.window_id, Key::Escape);
    }
}
