use zero_ui::{
    core::{
        context::StaticStateId,
        event::{command, CommandHandle, EventUpdate},
        impl_ui_node,
        keyboard::HeadlessAppKeyboardExt,
    },
    prelude::*,
};

#[test]
fn notify() {
    let mut app = App::default().run_headless(false);
    app.open_window(|_| listener_window(false));

    let cmd = FOO_CMD;
    cmd.notify(&mut app);

    let _ = app.update(false);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE);
    assert_eq!(trace, &vec!["no-scope / App".to_owned()]);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE_IGNORE_PROPAGATION);
    assert_eq!(trace, &vec!["no-scope / App".to_owned(), "no-scope / App".to_owned()]);
    // two handlers
}

#[test]
fn notify_scoped() {
    let mut app = App::default().run_headless(false);
    let window_id = app.open_window(|_| listener_window(false));

    let cmd = FOO_CMD;
    let cmd_scoped = cmd.scoped(window_id);

    cmd_scoped.notify(&mut app);

    let _ = app.update(false);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE);
    assert_eq!(trace, &vec![format!("scoped-win / Window({window_id:?})")]);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE_IGNORE_PROPAGATION);
    assert_eq!(
        trace,
        &vec![
            format!("scoped-win / Window({window_id:?})"),
            format!("scoped-win / Window({window_id:?})")
        ]
    );
}

#[test]
fn shortcut() {
    let mut app = App::default().run_headless(false);
    let window_id = app.open_window(|_| listener_window(false));

    FOO_CMD.shortcut().set(&app, shortcut!(F)).unwrap();

    app.press_key(window_id, Key::F);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE);
    let widget_id = WidgetId::named("test-widget");
    // because we target the scoped first.
    assert_eq!(trace, &vec![format!("scoped-wgt / Widget({widget_id:?})")]);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE_IGNORE_PROPAGATION);
    assert_eq!(
        trace,
        &vec![
            format!("scoped-wgt / Widget({widget_id:?})"), // first scoped widget only
            format!("scoped-win / Window({window_id:?})"), // then focused window
            format!("scoped-win / Window({window_id:?})"), // two handlers
            "no-scope / App".to_owned(),                   // then app
            "no-scope / App".to_owned(),                   // two handlers
        ]
    );
}

#[test]
fn shortcut_with_focused_scope() {
    let mut app = App::default().run_headless(false);
    let window_id = app.open_window(|_| listener_window(true));

    FOO_CMD.shortcut().set(&app, shortcut!(F)).unwrap();

    app.press_key(window_id, Key::F);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE);
    let widget_id = WidgetId::named("other-widget");
    assert_eq!(1, trace.len()); // because we target the focused first.
    assert_eq!(&trace[0], &format!("scoped-wgt / Widget({widget_id:?})"));

    let trace = app.ctx().app_state.into_req(&TEST_TRACE_IGNORE_PROPAGATION);
    assert_eq!(
        trace,
        &vec![
            format!("scoped-wgt / Widget({widget_id:?})"), // first focused widget only
            format!("scoped-win / Window({window_id:?})"), // then focused window
            format!("scoped-win / Window({window_id:?})"), // two handlers
            "no-scope / App".to_owned(),                   // then app
            "no-scope / App".to_owned(),                   // two handlers
        ]
    );
}

#[test]
fn shortcut_scoped() {
    let mut app = App::default().run_headless(false);
    let window_id = app.open_window(|_| listener_window(false));

    FOO_CMD.shortcut().set(&app, shortcut!(F)).unwrap();
    FOO_CMD.scoped(window_id).shortcut().set(&app, shortcut!(G)).unwrap();

    app.press_key(window_id, Key::G);

    {
        let trace = app.ctx().app_state.into_req_mut(&TEST_TRACE);
        assert_eq!(trace, &vec![format!("scoped-win / Window({window_id:?})")]);
        trace.clear();

        let trace = app.ctx().app_state.into_req_mut(&TEST_TRACE_IGNORE_PROPAGATION);
        assert_eq!(
            trace,
            &vec![
                format!("scoped-win / Window({window_id:?})"),
                format!("scoped-win / Window({window_id:?})")
            ]
        );
        trace.clear();
    }

    app.press_key(window_id, Key::F);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE);
    let widget_id = WidgetId::named("test-widget");
    assert_eq!(trace, &vec![format!("scoped-wgt / Widget({widget_id:?})")]);

    let trace = app.ctx().app_state.into_req(&TEST_TRACE_IGNORE_PROPAGATION);
    assert_eq!(
        trace,
        &vec![
            format!("scoped-wgt / Widget({widget_id:?})"),
            "no-scope / App".to_owned(),
            "no-scope / App".to_owned()
        ]
    );
}

fn listener_window(focused_wgt: bool) -> Window {
    struct FooHandlerNode {
        handle: Option<CommandHandle>,
        handle_scoped: Option<CommandHandle>,
        handle_scoped_wgt: Option<CommandHandle>,
    }
    #[impl_ui_node(none)]
    impl UiNode for FooHandlerNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.handle = Some(FOO_CMD.new_handle(ctx, true));
            self.handle_scoped = Some(FOO_CMD.scoped(ctx.path.window_id()).new_handle(ctx, true));
            self.handle_scoped_wgt = Some(FOO_CMD.scoped(ctx.path.widget_id()).new_handle(ctx, true));
        }
        fn event(&mut self, ctx: &mut WidgetContext, update: &mut EventUpdate) {
            if let Some(args) = FOO_CMD.on(update) {
                args.handle(|args| {
                    ctx.app_state
                        .entry(&TEST_TRACE)
                        .or_default()
                        .push(format!("no-scope / {:?}", args.scope));
                });

                ctx.app_state
                    .entry(&TEST_TRACE_IGNORE_PROPAGATION)
                    .or_default()
                    .push(format!("no-scope / {:?}", args.scope));
            }

            if let Some(args) = FOO_CMD.scoped(ctx.path.window_id()).on(update) {
                args.handle(|args| {
                    ctx.app_state
                        .entry(&TEST_TRACE)
                        .or_default()
                        .push(format!("scoped-win / {:?}", args.scope));
                });

                ctx.app_state
                    .entry(&TEST_TRACE_IGNORE_PROPAGATION)
                    .or_default()
                    .push(format!("scoped-win / {:?}", args.scope));
            }

            if let Some(args) = FOO_CMD.scoped(ctx.path.widget_id()).on(update) {
                args.handle(|args| {
                    ctx.app_state
                        .entry(&TEST_TRACE)
                        .or_default()
                        .push(format!("scoped-wgt / {:?}", args.scope));
                });

                ctx.app_state
                    .entry(&TEST_TRACE_IGNORE_PROPAGATION)
                    .or_default()
                    .push(format!("scoped-wgt / {:?}", args.scope));
            }
        }
        fn deinit(&mut self, _: &mut WidgetContext) {
            self.handle = None;
            self.handle_scoped = None;
            self.handle_scoped_wgt = None;
        }
    }

    window! {
        content = v_stack(widgets![
            container! {
                id = "test-widget";
                size = (100, 100);
                content = FooHandlerNode { handle: None, handle_scoped: None, handle_scoped_wgt: None };
            },
            container! {
                id = "other-widget";
                size = (100, 100);
                focusable = focused_wgt;
                content = FooHandlerNode { handle: None, handle_scoped: None, handle_scoped_wgt: None };
            }
        ])
    }
}

command! {
    pub static FOO_CMD;
}

static TEST_TRACE: StaticStateId<Vec<String>> = StaticStateId::new_unique();
static TEST_TRACE_IGNORE_PROPAGATION: StaticStateId<Vec<String>> = StaticStateId::new_unique();
