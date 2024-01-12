use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use zero_ui::{image::Img, prelude::*, widget::parallel};

#[test]
fn error_view_recursion() {
    zero_ui_app::test_log();

    let img = var(Img::dummy(Some("test error".to_txt()))).read_only();

    let mut app = APP.defaults().run_headless(false);
    zero_ui::image::IMAGES.load_in_headless().set(true);
    let ok = Arc::new(AtomicBool::new(false));
    let window_id = app.open_window(async_clmv!(ok, {
        Window! {
            parallel = false;
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
