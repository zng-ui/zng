use zng::{
    image::{IMAGES, ImageEntryKind},
    prelude::*,
};

pub(crate) fn render_test_icon() {
    let mut app = APP.defaults().run_headless(true);
    app.run_task(render_task());
}

async fn render_task() {
    let mut entries = Vec::with_capacity(10);
    for size in [256, 128, 96, 64, 48, 40, 32, 24, 20, 16] {
        let entry = IMAGES.render_node(window::RenderMode::Software, 1.fct(), None, move || {
            Text! {
                layout::size = size.px();
                txt = size.to_txt();
                txt_align = Align::CENTER;
                font_color = colors::WHITE;
                font_size = size / 2;
                widget::background_color = colors::BLACK;
                widget::border = (size as f32 * 0.01).max(1.0).px(), colors::GREEN;
                widget::corner_radius = (size as f32 * 0.1).max(1.0).px();
            }
        });
        entries.push(entry);
    }

    for img in &entries {
        img.wait_match(|i| i.is_loaded() || i.is_error()).await;
    }

    let img = entries[0].get();
    let entries: Vec<_> = entries[1..]
        .iter()
        .map(|i| (i.get(), ImageEntryKind::Reduced { synthetic: false }))
        .collect();

    img.save_with_entries(&entries, "ico", zng::env::res("test-icon.ico"))
        .await
        .unwrap();
}
