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
}
