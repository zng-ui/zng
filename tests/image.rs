use zero_ui::{
    app::HeadlessApp,
    image::{ImageDataFormat, ImageSource},
    prelude::*,
};
use zero_ui_app::view_process::VIEW_PROCESS_INITED_EVENT;

fn main() {
    zero_ui_view::run_same_process(|| {
        let mut app = APP.defaults().run_headless(true);
        get_before_view_init(&mut app);
        app.exit();
    });
    std::thread::sleep(1.secs()); // time for backtrace
}

pub fn get_before_view_init(app: &mut HeadlessApp) {
    let img = IMAGES.cache(image());

    assert!(img.get().is_loading());

    let mut inited = false;
    while !inited {
        app.update_observe_event(
            |update| {
                if VIEW_PROCESS_INITED_EVENT.has(update) {
                    inited = true;

                    assert!(img.get().is_loading());
                }
            },
            true,
        )
        .assert_wait();
    }

    app.run_task(async_clmv!(img, {
        task::with_deadline(img.get().wait_done(), 5.secs()).await.unwrap();
    }));

    assert!(img.get().is_loaded());
}

fn image() -> ImageSource {
    let color = [0, 0, 255, 255 / 2];

    let size = PxSize::new(Px(32), Px(32));
    let len = size.width.0 * size.height.0 * 4;
    let bgra: Vec<u8> = color.iter().copied().cycle().take(len as usize).collect();

    (bgra, ImageDataFormat::from(size)).into()
}

#[test]
fn error_view_recursion() {
    crate::core::test_log();

    let img = var(crate::core::image::Img::dummy(Some("test error".to_string()))).read_only();

    let mut app = APP.defaults().run_headless(false);
    IMAGES.load_in_headless().set(true);
    let ok = Arc::new(AtomicBool::new(false));
    let window_id = app.open_window(async_clmv!(ok, {
        Window! {
            crate::core::widget_base::parallel = false;
            child = Image! {
                source = img.clone();
                img_error_fn = wgt_fn!(ok, |_| {
                    ok.store(true, Ordering::Relaxed);
                    Image! {
                        source = img.clone();
                    }
                });
            }
        }
    }));

    let _ = app.update(false);
    app.close_window(window_id);

    assert!(ok.load(Ordering::Relaxed));
}
