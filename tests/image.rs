use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use zng::{image::Img, prelude::*, widget::parallel};

#[test]
fn error_view_recursion() {
    let mut app = APP.defaults().run_headless(false);
    zng_app::test_log();

    let img = var(Img::dummy(Some("test error".to_txt()))).read_only();

    zng::image::IMAGES.load_in_headless().set(true);
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
            };
        }
    }));

    let _ = app.update(false);
    app.close_window(window_id);

    assert!(ok.load(Ordering::Relaxed));
}
