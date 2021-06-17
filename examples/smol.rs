use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        let size = var(0);
        window! {
            title = "`smol` example";
            content = text(size.map(|s| formatx!("examples/res/icon-bytes.png is {} bytes", s)));
            on_open = move |ctx, _| {
                let size = ctx.vars.sender(&size);
                Tasks::run(async move {
                    let bytes = smol::fs::read("examples/res/icon-bytes.png").await.unwrap();
                    let _ = size.send(bytes.len());
                })
            };
        }
    })
}
