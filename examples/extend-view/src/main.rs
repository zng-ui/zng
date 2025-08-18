//! Demonstrates the `zng-view` extension API and the provided renderer debug and ANGLE extensions.

use zng::{color::filter::hue_rotate, layout::size, prelude::*};
// use zng_wgt_webrender_debug as wr;

// Examples of how to extend the view-process with custom renderers.
//
// This is an advanced API, use it only if you really can't render the effect you want
// using custom nodes/properties.

fn main() {
    zng::env::init!();
    app_main();
}

mod get_info;
mod prefer_angle;
mod using_blob;
mod using_display_items;
mod using_gl_overlay;
mod using_gl_texture;

fn app_main() {
    APP.defaults().run_window(async {
        Window! {
            // property uses the built-in "zng-view.webrender_debug" extension to draw renderer debug info.
            //
            // wr::renderer_debug = {
            //     wr::DebugFlags::TEXTURE_CACHE_DBG | wr::DebugFlags::TEXTURE_CACHE_DBG_CLEAR_EVICTED
            // };

            // example property that uses built-in extension.
            prefer_angle::use_angle_egl = true;

            title = "Extend-View Example";
            width = 900;

            on_frame_image_ready = hn_once!(|_| {
                let h = get_info::app_side::window_handle(WINDOW.id()).unwrap();
                tracing::info!("RAW-WINDOW-HANDLE: {h}");

                let i = get_info::app_side::gl_version(WINDOW.id()).unwrap();
                tracing::info!("OPEN-GL: {i}")
            });

            child = Stack! {
                children_align = Align::CENTER;
                direction = StackDirection::left_to_right();
                spacing = 20;

                children = ui_vec![
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::CENTER;
                        spacing = 5;
                        children = ui_vec![
                            Text!("Using Display Items"),
                            Container! {
                                size = 30.vmin_pct();
                                child = using_display_items::app_side::custom_render_node();
                            },
                            Container! {
                                size = 30.vmin_pct();
                                hue_rotate = 180.deg();
                                child = using_display_items::app_side::custom_render_node();
                            },
                        ];
                    },
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::CENTER;
                        spacing = 5;
                        children = ui_vec![
                            Text!("Using Blob Images"),
                            Container! {
                                size = 30.vmin_pct();
                                child = using_blob::app_side::custom_render_node();
                            },
                            Container! {
                                size = 30.vmin_pct();
                                hue_rotate = 180.deg();
                                child = using_blob::app_side::custom_render_node();
                            },
                        ];
                    },
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::CENTER;
                        spacing = 5;
                        children = ui_vec![
                            Text!("Using GL Overlay"),
                            Container! {
                                size = 30.vmin_pct();
                                child = using_gl_overlay::app_side::custom_render_node();
                            },
                            Container! {
                                size = 30.vmin_pct();
                                hue_rotate = 180.deg(); // no effect
                                child = using_gl_overlay::app_side::custom_render_node();
                            },
                        ];
                    },
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::CENTER;
                        spacing = 5;
                        children = ui_vec![
                            Text!("Using GL Texture"),
                            Container! {
                                size = 30.vmin_pct();
                                child = using_gl_texture::app_side::custom_render_node();
                            },
                            Container! {
                                size = 30.vmin_pct();
                                hue_rotate = 180.deg();
                                child = using_gl_texture::app_side::custom_render_node();
                            },
                        ];
                    },
                ];
            };
        }
    })
}
