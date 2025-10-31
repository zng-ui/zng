use zng::{
    image::{IMAGES, Img},
    layout::LayoutPassId,
    prelude::*,
    prelude_wgt::*,
    window::RenderMode,
};

use crate::save_name;

pub async fn bw_rgb(render_mode: RenderMode, scale_factor: Factor) {
    let colors = [colors::BLACK, colors::WHITE, colors::RED, colors::GREEN, colors::BLUE];

    let img = IMAGES.render_node(
        render_mode,
        scale_factor,
        None,
        clmv!(colors, || {
            Stack! {
                direction = StackDirection::left_to_right();
                children = colors.iter().map(|c| {
                    Wgt! {
                        widget::background_color = *c;
                        layout::size = (5, 10);
                    }
                });
            }
        }),
    );

    while img.with(Img::is_loading) {
        if task::with_deadline(img.wait_update(), 20.secs()).await.is_err() {
            panic!(
                "img wait_update timeout after 20s, img.is_loading: {}, APP.is_running: {}, VIEW_PROCESS.is_connected: {}, VIEW_PROCESS ping: {}",
                img.with(Img::is_loading),
                APP.is_running(),
                zng_app::view_process::VIEW_PROCESS.is_connected(),
                zng_app::view_process::VIEW_PROCESS.image_decoders().is_ok(),
            );
        }
    }

    let img = img.get();

    if let Some(name) = save_name() {
        let file = format!("{name}.png");
        img.save(&file).await.unwrap();
        println!("saved to `{file}`");
    }

    let mut rect = LAYOUT.with_root_context(
        LayoutPassId::new(),
        LayoutMetrics::new(scale_factor, PxSize::splat(Px(1000)), Px(12)),
        || (5, 10).at(0, 0).layout(),
    );
    for color in colors {
        let (copied_rect, p) = img.copy_pixels(rect).unwrap_or_else(|| panic!("expected `{rect:?}`"));

        assert_eq!(copied_rect, rect);
        for cc in p.chunks_exact(4) {
            // BGRA
            let copied_color = rgba(cc[2], cc[1], cc[0], cc[3]);
            assert_eq!(color, copied_color, "expected all {color} in {rect:?}, found {copied_color}");
        }
        rect.origin.x += rect.size.width;
    }
}

// async fn save_rect(rect: PxRect, p: &[u8]) {
//     if let Some(name) = save_name() {
//         let img = IMAGES.from_data(
//             Arc::new(p.to_vec()),
//             ImageDataFormat::Bgra8 {
//                 size: rect.size,
//                 density: None,
//             },
//         );
//         while img.with(Img::is_loading) {
//             img.wait_update().await;
//         }
//         let img = img.get();
//         let file = format!(
//             "{name}.{}by{}at{}x{}.png",
//             rect.size.width.0, rect.size.height.0, rect.origin.x.0, rect.origin.y.0,
//         );
//         img.save(&file).await.unwrap();
//         println!("saved to `{file}`");
//     }
// }
