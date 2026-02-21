use keyboard::KeyLocation;
use zng::{
    focus::focusable,
    keyboard::{Key, KeyCode},
    layout::size,
    prelude::*,
    prelude_wgt::*,
};

#[test]
fn notify_no_scope() {
    zng::env::init!();
    let mut app = APP.defaults().run_headless(false);
    app.open_window(WindowId::new_unique(), listener_window(false));

    let cmd = FOO_CMD;
    cmd.notify();

    let _ = app.update(false);

    assert_eq!(&*TEST_TRACE.read(), &vec!["no-scope / App".to_owned()]);
}

#[test]
fn notify_scoped() {
    zng::env::init!();
    let mut app = APP.defaults().run_headless(false);
    let window_id = WindowId::new_unique();
    app.open_window(window_id, listener_window(false));

    let cmd = FOO_CMD;
    let cmd_scoped = cmd.scoped(window_id);

    cmd_scoped.notify();

    let _ = app.update(false);

    assert_eq!(&*TEST_TRACE.read(), &vec![format!("scoped-win / Window({window_id:?})")]);
}

#[test]
fn shortcut_basic() {
    zng::env::init!();
    let mut app = APP.defaults().run_headless(false);
    let window_id = WindowId::new_unique();
    app.open_window(window_id, listener_window(false));

    FOO_CMD.shortcut().set(shortcut!('F'));
    let _ = app.update(false);

    app.press_key(window_id, KeyCode::KeyF, KeyLocation::Standard, Key::Char('F'));

    // because of parallelism any of these targets can receive first
    let trace = TEST_TRACE.read();
    assert!(!trace.is_empty());
    assert!(!trace.contains(&format!("scoped-win / Window({window_id:?})")));
}

#[test]
fn shortcut_scoped() {
    zng::env::init!();
    let mut app = APP.defaults().run_headless(false);
    let window_id = WindowId::new_unique();
    app.open_window(window_id, listener_window(false));

    let widget_id = WidgetId::named("test-widget");
    FOO_CMD.scoped(widget_id).shortcut().set(shortcut!('F'));
    FOO_CMD.scoped(window_id).shortcut().set(shortcut!('G'));
    let _ = app.update(false);

    app.press_key(window_id, KeyCode::KeyG, KeyLocation::Standard, Key::Char('G'));

    {
        let mut trace = TEST_TRACE.write();
        assert_eq!(&*trace, &vec![format!("scoped-win / Window({window_id:?})")]);
        trace.clear();
    }

    app.press_key(window_id, KeyCode::KeyF, KeyLocation::Standard, Key::Char('F'));
    assert_eq!(&*TEST_TRACE.read(), &vec![format!("scoped-wgt / Widget({widget_id:?})")]);
}

async fn listener_window(focused_wgt: bool) -> window::WindowRoot {
    fn foo_handler() -> UiNode {
        let mut _handle = None;
        let mut _handle_scoped = None;
        match_node_leaf(move |op| match op {
            UiNodeOp::Init => {
                _handle = Some(FOO_CMD.subscribe(true));
                _handle_scoped = Some(FOO_CMD.scoped(WIDGET.id()).subscribe(true));
            }
            UiNodeOp::Deinit => {
                _handle = None;
                _handle_scoped = None;
            }
            UiNodeOp::Update { .. } => {
                FOO_CMD.scoped(WIDGET.id()).each_update(true, false, |args| {
                    args.propagation.stop();
                    TEST_TRACE.write().push(format!("scoped-wgt / {:?}", args.scope));
                });
                FOO_CMD.each_update(true, false, |args| {
                    args.propagation.stop();
                    TEST_TRACE.write().push(format!("no-scope / {:?}", args.scope));
                });
            }
            _ => {}
        })
    }

    fn foo_window_handler() -> UiNode {
        let mut _handle_scoped = None;

        match_node_leaf(move |op| match op {
            UiNodeOp::Init => {
                _handle_scoped = Some(FOO_CMD.scoped(WINDOW.id()).subscribe(true));
            }
            UiNodeOp::Deinit => {
                _handle_scoped = None;
            }
            UiNodeOp::Update { .. } => {
                FOO_CMD.scoped(WINDOW.id()).each_update(true, false, |args| {
                    args.propagation.stop();
                    TEST_TRACE.write().push(format!("scoped-win / {:?}", args.scope));
                });
            }
            _ => {}
        })
    }

    Window! {
        child_top = foo_window_handler();
        child = Stack! {
            direction = StackDirection::top_to_bottom();
            children = ui_vec![
                Container! {
                    id = "test-widget";
                    size = (100, 100);
                    child = foo_handler();
                },
                Container! {
                    id = "other-widget";
                    size = (100, 100);
                    focusable = focused_wgt;
                    child = foo_handler();
                }
            ];
        };
    }
}

command! {
    pub static FOO_CMD;
}

app_local! {
    static TEST_TRACE: Vec<String> = const { vec![] };
}
