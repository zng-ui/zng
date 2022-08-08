use zero_ui::{
    core::{
        app::{view_process::ViewProcessInitedEvent, HeadlessApp},
        image::{ImageDataFormat, Images},
    },
    prelude::*,
};

fn main() {
    zero_ui_view::run_same_process(|| {
        let mut app = App::default().run_headless(true);
        get_before_view_init(&mut app);
        app.exit();
    });
}

pub fn get_before_view_init(app: &mut HeadlessApp) {
    let img = Images::req(app).cache(image());

    assert!(img.get(app.ctx().vars).is_loading());

    let mut inited = false;
    while !inited {
        app.update_observe_event(
            |ctx, args| {
                if ViewProcessInitedEvent.update(args).is_some() {
                    inited = true;

                    assert!(img.get(ctx).is_loading());
                }
            },
            true,
        )
        .assert_wait();
    }

    app.run_task(async_clone_move_fn!(img, |ctx| {
        task::with_deadline(img.get_clone(&ctx).wait_done(), 5.secs()).await.unwrap();
    }));

    assert!(img.get(app.ctx().vars).is_loaded());
}

fn image() -> ImageSource {
    let color = [0, 0, 255, 255 / 2];

    let size = PxSize::new(Px(32), Px(32));
    let len = size.width.0 * size.height.0 * 4;
    let bgra: Vec<u8> = color.iter().copied().cycle().take(len as usize).collect();

    (bgra, ImageDataFormat::from(size)).into()
}
