//! Demonstrates each `CursorIcon`, tooltip anchored to cursor.

use zng::{color::filter::invert_color, image::ImageFit, mouse::CursorIcon, prelude::*, prelude_wgt::NilUiNode};

mod widgets;

fn main() {
    zng::env::init!();

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
            child = Stack!(
                top_to_bottom,
                ui_vec![
                    Grid! {
                        columns = ui_vec![grid::Column!(1.lft()); 5];
                        auto_grow_fn = wgt_fn!(|_| grid::Row!(1.lft()));
                        cells = demos;
                    },
                    cursor_demo(None),
                ]
            )
        }
    })
}

fn cursor_demo(icon: Option<(CursorIcon, &'static [u8])>) -> UiNode {
    widgets::DemoEntry! {
        mouse::cursor = match icon {
            Some(i) => i.0.into(),
            None => mouse::CursorSource::Hidden,
        };

        widget::background = match icon {
            Some((_, img)) => Image! {
                source = img;
                img_fit = ImageFit::None;
                invert_color = color::COLOR_SCHEME_VAR.map(|c| (*c == color::ColorScheme::Dark).into());
            }
            .boxed(),
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

fn cursor_img_demo(label: &'static str, img: &'static [u8], hotspot: (i32, i32), fallback: CursorIcon) -> UiNode {
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
    (CursorIcon::Default, include_bytes!("../res/default.png")),
    (CursorIcon::Crosshair, include_bytes!("../res/crosshair.png")),
    (CursorIcon::Pointer, include_bytes!("../res/pointer.png")),
    (CursorIcon::Move, include_bytes!("../res/move.png")),
    (CursorIcon::Text, include_bytes!("../res/text.png")),
    (CursorIcon::Wait, include_bytes!("../res/wait.png")),
    (CursorIcon::Help, include_bytes!("../res/help.png")),
    (CursorIcon::Progress, include_bytes!("../res/progress.png")),
    (CursorIcon::NotAllowed, include_bytes!("../res/not-allowed.png")),
    (CursorIcon::ContextMenu, include_bytes!("../res/context-menu.png")),
    (CursorIcon::Cell, include_bytes!("../res/cell.png")),
    (CursorIcon::VerticalText, include_bytes!("../res/vertical-text.png")),
    (CursorIcon::Alias, include_bytes!("../res/alias.png")),
    (CursorIcon::Copy, include_bytes!("../res/copy.png")),
    (CursorIcon::NoDrop, include_bytes!("../res/no-drop.png")),
    (CursorIcon::Grab, include_bytes!("../res/grab.png")),
    (CursorIcon::Grabbing, include_bytes!("../res/grabbing.png")),
    (CursorIcon::AllScroll, include_bytes!("../res/all-scroll.png")),
    (CursorIcon::ZoomIn, include_bytes!("../res/zoom-in.png")),
    (CursorIcon::ZoomOut, include_bytes!("../res/zoom-out.png")),
    (CursorIcon::EResize, include_bytes!("../res/e-resize.png")),
    (CursorIcon::NResize, include_bytes!("../res/n-resize.png")),
    (CursorIcon::NeResize, include_bytes!("../res/ne-resize.png")),
    (CursorIcon::NwResize, include_bytes!("../res/nw-resize.png")),
    (CursorIcon::SResize, include_bytes!("../res/s-resize.png")),
    (CursorIcon::SeResize, include_bytes!("../res/se-resize.png")),
    (CursorIcon::SwResize, include_bytes!("../res/sw-resize.png")),
    (CursorIcon::WResize, include_bytes!("../res/w-resize.png")),
    (CursorIcon::EwResize, include_bytes!("../res/3-resize.png")),
    (CursorIcon::NsResize, include_bytes!("../res/6-resize.png")),
    (CursorIcon::NeswResize, include_bytes!("../res/1-resize.png")),
    (CursorIcon::NwseResize, include_bytes!("../res/4-resize.png")),
    (CursorIcon::ColResize, include_bytes!("../res/col-resize.png")),
    (CursorIcon::RowResize, include_bytes!("../res/row-resize.png")),
];

// (label, cursor_img, fallback)
#[expect(clippy::type_complexity)]
pub const CURSOR_IMGS: &[(&str, (&[u8], i32, i32), CursorIcon)] =
    &[("custom", (include_bytes!("../../image/res/RGBA8.png"), 4, 6), CursorIcon::Default)];
