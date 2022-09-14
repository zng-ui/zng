#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // zero_ui_view::run_same_process(app_main);
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        let mut demos = widget_vec![];
        for icon in CURSORS {
            demos.push(cursor_demo(Some(*icon)));
        }

        window! {
            title = "Cursor Example";
            resizable = false;
            auto_size = true;
            padding = 20;
            content = v_stack(widgets![
                uniform_grid! {
                    columns = 5;
                    items = demos;
                },
                center(cursor_demo(None)),
            ])
        }
    })
}

fn cursor_demo(icon: Option<(CursorIcon, &'static [u8])>) -> impl Widget {
    container! {
        cursor = icon.map(|i| i.0);

        size = (150, 80);

        margin = 1;
        background_color = theme::pair(colors::BLACK, colors::WHITE);
        background = match icon {
            Some((_, img)) => image!{
                source = img;
                fit = ImageFit::None;
                invert_color = theme::pair(true, false);
            }.boxed(),
            None => NilUiNode.boxed(),
        };

        text_color = theme::pair(rgb(140, 140, 140), rgb(115, 115, 115));

        when self.is_hovered {
            text_color = theme::pair(colors::WHITE, colors::BLACK);
        }

        content_align = Align::TOP_LEFT;
        padding = (2, 5);

        content = text! {
            text = match icon {
                Some((ico, _)) => formatx!("{ico:?}"),
                None => Text::from_static("<none>"),
            };

            font_style = match icon {
                Some(_) => FontStyle::Normal,
                None => FontStyle::Italic,
            };

            font_family = "monospace";
            font_size = 16;
            font_weight = FontWeight::BOLD;
        };
    }
}

pub const CURSORS: &[(CursorIcon, &[u8])] = &[
    (CursorIcon::Default, include_bytes!("res/cursor/default.png")),
    (CursorIcon::Crosshair, include_bytes!("res/cursor/crosshair.png")),
    (CursorIcon::Hand, include_bytes!("res/cursor/pointer.png")),
    (CursorIcon::Arrow, include_bytes!("res/cursor/default.png")),
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
