//! Demonstrates each `CursorIcon`, tooltip anchored to cursor.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::{
    color::{color_scheme_map, filter::invert_color},
    image::ImageFit,
    mouse::CursorIcon,
    prelude::*,
    prelude_wgt::NilUiNode,
};

fn main() {
    examples_util::print_info();
    zng::env::init!();
    zng::app::crash_handler::init_debug();
    app_main();
}

fn app_main() {
    APP.defaults().run_window(async {
        let mut demos = ui_vec![];
        for icon in CURSORS {
            demos.push(cursor_demo(Some(*icon)));
        }
        for (label, (img, x, y), fallback) in CURSOR_IMGS {
            demos.push(cursor_img_demo(label, img, (*x, *y), *fallback));
        }

        Window! {
            title = "Cursor Example";
            resizable = false;
            enabled_buttons = !window::WindowButton::MAXIMIZE;
            auto_size = true;
            padding = 20;
            child = Stack!(top_to_bottom, ui_vec![
                Grid! {
                    columns = ui_vec![grid::Column!(1.lft()); 5];
                    auto_grow_fn = wgt_fn!(|_| grid::Row!(1.lft()));
                    cells = demos;
                },
                cursor_demo(None),
            ])
        }
    })
}

fn cursor_demo(icon: Option<(CursorIcon, &'static [u8])>) -> impl UiNode {
    widgets::DemoEntry! {
        mouse::cursor = match icon {
            Some(i) => i.0.into(),
            None => mouse::CursorSource::Hidden
        };

        widget::background = match icon {
            Some((_, img)) => Image!{
                source = img;
                img_fit = ImageFit::None;
                invert_color = color_scheme_map(true, false);
            }.boxed(),
            None => NilUiNode.boxed(),
        };

        child = Text! {
            txt = match icon {
                Some((ico, _)) => formatx!("{ico:?}"),
                None => Txt::from_static("<none>"),
            };

            font_style = match icon {
                Some(_) => FontStyle::Normal,
                None => FontStyle::Italic,
            };
        };
    }
}

fn cursor_img_demo(label: &'static str, img: &'static [u8], hotspot: (i32, i32), fallback: CursorIcon) -> impl UiNode {
    widgets::DemoEntry! {
        mouse::cursor = mouse::CursorImg {
            source: img.into(),
            hotspot: hotspot.into(),
            fallback,
        };

        widget::background = Image! {
            source = img;
            img_fit = ImageFit::None;
        };

        child = Text! {
            txt = label;
            font_style = FontStyle::Italic;
        }
    }
}

// (cursor, demo image)
pub const CURSORS: &[(CursorIcon, &[u8])] = &[
    (CursorIcon::Default, include_bytes!("res/cursor/default.png")),
    (CursorIcon::Crosshair, include_bytes!("res/cursor/crosshair.png")),
    (CursorIcon::Pointer, include_bytes!("res/cursor/pointer.png")),
    (CursorIcon::Move, include_bytes!("res/cursor/move.png")),
    (CursorIcon::Text, include_bytes!("res/cursor/text.png")),
    (CursorIcon::Wait, include_bytes!("res/cursor/wait.png")),
    (CursorIcon::Help, include_bytes!("res/cursor/help.png")),
    (CursorIcon::Progress, include_bytes!("res/cursor/progress.png")),
    (CursorIcon::NotAllowed, include_bytes!("res/cursor/not-allowed.png")),
    (CursorIcon::ContextMenu, include_bytes!("res/cursor/context-menu.png")),
    (CursorIcon::Cell, include_bytes!("res/cursor/cell.png")),
    (CursorIcon::VerticalText, include_bytes!("res/cursor/vertical-text.png")),
    (CursorIcon::Alias, include_bytes!("res/cursor/alias.png")),
    (CursorIcon::Copy, include_bytes!("res/cursor/copy.png")),
    (CursorIcon::NoDrop, include_bytes!("res/cursor/no-drop.png")),
    (CursorIcon::Grab, include_bytes!("res/cursor/grab.png")),
    (CursorIcon::Grabbing, include_bytes!("res/cursor/grabbing.png")),
    (CursorIcon::AllScroll, include_bytes!("res/cursor/all-scroll.png")),
    (CursorIcon::ZoomIn, include_bytes!("res/cursor/zoom-in.png")),
    (CursorIcon::ZoomOut, include_bytes!("res/cursor/zoom-out.png")),
    (CursorIcon::EResize, include_bytes!("res/cursor/e-resize.png")),
    (CursorIcon::NResize, include_bytes!("res/cursor/n-resize.png")),
    (CursorIcon::NeResize, include_bytes!("res/cursor/ne-resize.png")),
    (CursorIcon::NwResize, include_bytes!("res/cursor/nw-resize.png")),
    (CursorIcon::SResize, include_bytes!("res/cursor/s-resize.png")),
    (CursorIcon::SeResize, include_bytes!("res/cursor/se-resize.png")),
    (CursorIcon::SwResize, include_bytes!("res/cursor/sw-resize.png")),
    (CursorIcon::WResize, include_bytes!("res/cursor/w-resize.png")),
    (CursorIcon::EwResize, include_bytes!("res/cursor/3-resize.png")),
    (CursorIcon::NsResize, include_bytes!("res/cursor/6-resize.png")),
    (CursorIcon::NeswResize, include_bytes!("res/cursor/1-resize.png")),
    (CursorIcon::NwseResize, include_bytes!("res/cursor/4-resize.png")),
    (CursorIcon::ColResize, include_bytes!("res/cursor/col-resize.png")),
    (CursorIcon::RowResize, include_bytes!("res/cursor/row-resize.png")),
];

// (label, cursor_img, fallback)
#[allow(clippy::type_complexity)]
pub const CURSOR_IMGS: &[(&str, (&[u8], i32, i32), CursorIcon)] =
    &[("custom", (include_bytes!("res/image/RGBA8.png"), 4, 6), CursorIcon::Default)];

mod widgets {
    use zng::{prelude::*, prelude_wgt::*};

    #[widget($crate::widgets::DemoEntry)]
    pub struct DemoEntry(Container);

    impl DemoEntry {
        fn widget_intrinsic(&mut self) {
            widget_set! {
                self;
                grid::cell::at = grid::cell::AT_AUTO;

                layout::size = (150, 80);
                layout::align = Align::CENTER;

                tooltip = Tip!(Text!("tooltip position"));
                tip::tooltip_anchor = {
                    let mut mode = AnchorMode::tooltip();
                    mode.transform = layer::AnchorTransform::Cursor {
                        offset: layer::AnchorOffset::out_bottom_in_left(),
                        include_touch: true,
                        bounds: None,
                    };
                    mode
                };
                tip::tooltip_delay = 0.ms();

                layout::margin = 1;
                widget::background_color = color_scheme_map(colors::BLACK, colors::WHITE);

                #[easing(150.ms())]
                text::font_color = color_scheme_map(rgb(140, 140, 140), rgb(115, 115, 115));

                when *#gesture::is_hovered {
                    #[easing(0.ms())]
                    text::font_color = color_scheme_map(colors::WHITE, colors::BLACK);
                }

                text::font_family = "monospace";
                text::font_size = 16;
                text::font_weight = FontWeight::BOLD;

                child_align = Align::TOP_LEFT;
                padding = (2, 5);
            }
        }
    }
}
