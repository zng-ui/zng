use zero_ui_core::{command::*, context::*, handler::*, var::*};
command! { pub FooCommand; }

fn main() {
    TestWidgetContext::doc_test(
        (),
        async_hn!(|mut ctx, _| {
            let cmd = FooCommand;
            let cmd_scoped = cmd.scoped(ctx.window_id());

            let enabled = cmd.enabled();
            let enabled_scoped = cmd_scoped.enabled();

            let _handle = cmd_scoped.new_handle(&mut ctx, true);
            ctx.update().await;

            assert!(!enabled.copy(&ctx));
            assert!(enabled_scoped.copy(&ctx));
        }),
    )
}
