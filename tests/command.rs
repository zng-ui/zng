use zero_ui::core::{command::command, context::state_key, event::EventUpdateArgs, impl_ui_node};
use zero_ui::prelude::*;
use zero_ui_core::command::CommandHandle;
use zero_ui_core::keyboard::HeadlessAppKeyboardExt;

#[test]
fn scoped_notify() {
    let mut app = App::default().run_headless();
    let window_id = app.open_window(|_| notify_window());

    let cmd = FooCommand;
    let cmd_scoped = cmd.scoped(window_id);

    assert!(cmd_scoped.notify(&mut app, None));

    app.update(false);

    let trace = app.ctx().app_state.req::<TestTrace>();
    assert_eq!(1, trace.len());
    assert!(trace.contains(&format!("scoped-win / Window({:?})", window_id)))
}

#[test]
fn not_scoped_notify() {
    let mut app = App::default().run_headless();
    app.open_window(|_| notify_window());

    let cmd = FooCommand;
    assert!(cmd.notify(&mut app, None));

    app.update(false);

    let trace = app.ctx().app_state.req::<TestTrace>();
    assert_eq!(1, trace.len());
    assert!(trace.iter().any(|t| t == "no-scope / App"));
}

#[test]
fn not_scoped_shortcut() {
    let mut app = App::default().run_headless();
    let window_id = app.open_window(|_| notify_window());

    FooCommand.shortcut().set(&app, shortcut!(F)).unwrap();

    app.press_key(window_id, Key::F);

    let trace = app.ctx().app_state.req::<TestTrace>();
    assert_eq!(1, trace.len());
    assert!(trace.iter().any(|t| t == "no-scope / App"));
}

#[test]
fn scoped_shortcut() {
    let mut app = App::default().run_headless();
    let window_id = app.open_window(|_| notify_window());

    FooCommand.shortcut().set(&app, shortcut!(F)).unwrap();
    FooCommand.scoped(window_id).shortcut().set(&app, shortcut!(G)).unwrap();

    app.press_key(window_id, Key::G);

    {
        let trace = app.ctx().app_state.req_mut::<TestTrace>();
        assert_eq!(1, trace.len());
        assert!(trace.contains(&format!("scoped-win / Window({:?})", window_id)));
        trace.clear();
    }

    app.press_key(window_id, Key::F);
    let trace = app.ctx().app_state.req::<TestTrace>();

    assert_eq!(1, trace.len());
    assert!(trace.iter().any(|t| t == "no-scope / App"));
}

fn notify_window() -> Window {
    struct FooHandlerNode {
        handle: Option<CommandHandle>,
        handle_scoped: Option<CommandHandle>,
        handle_scoped_wgt: Option<CommandHandle>,
    }
    #[impl_ui_node(none)]
    impl UiNode for FooHandlerNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.handle = Some(FooCommand.new_handle(ctx, true));
            self.handle_scoped = Some(FooCommand.scoped(ctx.path.window_id()).new_handle(ctx, true));
            self.handle_scoped_wgt = Some(FooCommand.scoped(ctx.path.widget_id()).new_handle(ctx, true));
        }
        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            if let Some(args) = FooCommand.update(args) {
                ctx.app_state
                    .entry::<TestTrace>()
                    .or_default()
                    .push(format!("no-scope / {:?}", args.scope));
            }

            if let Some(args) = FooCommand.scoped(ctx.path.window_id()).update(args) {
                ctx.app_state
                    .entry::<TestTrace>()
                    .or_default()
                    .push(format!("scoped-win / {:?}", args.scope));
            }

            if let Some(args) = FooCommand.scoped(ctx.path.widget_id()).update(args) {
                ctx.app_state
                    .entry::<TestTrace>()
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
        content = container! {
            content = FooHandlerNode{ handle: None, handle_scoped: None, handle_scoped_wgt: None};
        }
    }
}

command! {
    pub FooCommand;
}

state_key! {
    struct TestTrace: Vec<String>;
}
