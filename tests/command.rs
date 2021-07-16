use zero_ui::prelude::*;
use zero_ui::core::{context::state_key, command::command, impl_ui_node, event::EventUpdateArgs};

#[test]
fn scoped_notify() {
    let mut app = App::default().run_headless();
    let window_id = app.open_window(|_| scoped_notify_window());

    let cmd = FooCommand;
    let cmd_scoped = cmd.scoped(window_id);

    cmd_scoped.notify(&mut app, None);

    app.update(false);

    let trace = app.ctx().app_state.req::<TestTrace>();
    assert_eq!(2, trace.len());

    // TODO
}

fn scoped_notify_window() -> Window {
    struct FooHandlerNode;
    #[impl_ui_node(none)]
    impl UiNode for FooHandlerNode {
        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let cmd = FooCommand;
            let cmd_scoped = cmd.scoped(ctx.path.window_id());

            if let Some(args) = cmd.update(args) {
                ctx.app_state.entry::<TestTrace>().or_default().push(format!("no-scope / {:?}", args.scope));
            }

            if let Some(args) = cmd_scoped.update(args) {
                ctx.app_state.entry::<TestTrace>().or_default().push(format!("scoped / {:?}", args.scope));
            }
        }
    }

    window! {
        content = container! {
            content = FooHandlerNode;
        }
    }
}

command! {
    pub FooCommand;
}

state_key! {
    struct TestTrace: Vec<String>;
}

