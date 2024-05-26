use zng::{
    image::{Img, IMAGES},
    layout::LayoutPassId,
    prelude::*,
    prelude_wgt::*,
    window::RenderMode,
};

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
                }).collect::<UiNodeVec>()
            }
        }),
    );

    while img.with(Img::is_loading) {
        img.wait_update().await;
    }

    let img = img.get();

    let mut rect = LAYOUT.with_root_context(
        LayoutPassId::new(),
        LayoutMetrics::new(scale_factor, PxSize::splat(Px(1000)), Px(12)),
        || (5, 10).at(0, 0).layout(),
    );
    for color in colors {
        let (copied_rect, p) = img.copy_pixels(rect).unwrap_or_else(|| panic!("expected `{rect:?}`"));
        assert_eq!(copied_rect, rect);
        for cc in p.chunks_exact(4) {
            let copied_color = rgba(cc[0], cc[1], cc[2], cc[3]);
            assert_eq!(copied_color, color);
        }
        rect.origin.x += rect.size.width;
    }
}
