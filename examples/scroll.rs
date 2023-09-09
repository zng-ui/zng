#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::widgets::icon::CommandIconExt;
use zero_ui::widgets::scroll::commands::ScrollToMode;

use zero_ui_view_prebuilt as zero_ui_view;

use rand::SeedableRng;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("scroll");
    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().extend(zero_ui_material_icons::MaterialFonts).run_window(async {
        Window! {
            title = "Scroll Example";
            child_insert_above = commands(), 0;
            child = Scroll! {
                id = "scroll";
                padding = 20;
                background_color = color_scheme_map(
                    hex!(#245E81),
                    colors::WHITE.with_alpha(80.pct()).mix_normal(hex!(#245E81))
                );
                // smooth_scrolling = false;
                child = Stack!{
                    direction = StackDirection::top_to_bottom();
                    children_align = Align::LEFT;
                    children = ui_vec![
                        Text! {
                            id = "Lorem 1";
                            txt = "Lorem 1";
                            font_size = 20;
                        },
                        Text!(ipsum()),
                        Text! {
                            id = "Lorem 2";
                            txt = "Lorem 2";
                            font_size = 20;
                        },
                        Text!(ipsum())
                    ];
                }
            };
        }
    })
}

fn commands() -> impl UiNode {
    use zero_ui::widgets::scroll::commands::*;

    SCROLL_TO_TOP_CMD.init_icon(wgt_fn!(|_| Icon!(zero_ui_material_icons::outlined::VERTICAL_ALIGN_TOP)));
    SCROLL_TO_BOTTOM_CMD.init_icon(wgt_fn!(|_| Icon!(zero_ui_material_icons::outlined::VERTICAL_ALIGN_BOTTOM)));

    let scope = WidgetId::named("scroll");
    use menu::CmdButton;

    Menu!(ui_vec![
        SubMenu!(
            "Scroll",
            ui_vec![
                CmdButton!(SCROLL_UP_CMD.scoped(scope)),
                CmdButton!(SCROLL_DOWN_CMD.scoped(scope)),
                CmdButton!(SCROLL_LEFT_CMD.scoped(scope)),
                CmdButton!(SCROLL_RIGHT_CMD.scoped(scope)),
            ]
        ),
        SubMenu!(
            "Page",
            ui_vec![
                CmdButton!(PAGE_UP_CMD.scoped(scope)),
                CmdButton!(PAGE_DOWN_CMD.scoped(scope)),
                CmdButton!(PAGE_LEFT_CMD.scoped(scope)),
                CmdButton!(PAGE_RIGHT_CMD.scoped(scope)),
            ]
        ),
        SubMenu!(
            "Scroll to",
            ui_vec![
                CmdButton!(SCROLL_TO_TOP_CMD.scoped(scope)),
                CmdButton!(SCROLL_TO_BOTTOM_CMD.scoped(scope)),
                CmdButton!(SCROLL_TO_LEFTMOST_CMD.scoped(scope)),
                CmdButton!(SCROLL_TO_RIGHTMOST_CMD.scoped(scope)),
                Hr!(),
                scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::minimal(10)),
                scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::center()),
            ]
        ),
        SubMenu!(
            "Zoom",
            ui_vec![
                CmdButton!(ZOOM_IN_CMD.scoped(scope)),
                CmdButton!(ZOOM_OUT_CMD.scoped(scope)),
                CmdButton!(ZOOM_RESET_CMD.scoped(scope)),
            ]
        ),
    ])
}
fn scroll_to_btn(target: WidgetId, mode: ScrollToMode) -> impl UiNode {
    use zero_ui::widgets::scroll::commands;

    let scroll = WidgetId::named("scroll");
    let cmd = commands::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} {}", target, if let ScrollToMode::Minimal{..} = &mode { "(minimal)" } else { "(center)" });
        enabled = cmd.is_enabled();
        on_click = hn!(|_| {
            cmd.notify_param(commands::ScrollToRequest { widget_id: target, mode: mode.clone(), zoom: None, });
        });
    }
}

fn ipsum() -> Txt {
    let mut s = String::new();
    let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
    for _ in 0..10 {
        for _ in 0..10 {
            s.push_str(&lipsum::lipsum_words_with_rng(&mut rng, 25));
            s.push('\n');
        }
        s.push('\n');
    }

    s.into()
}
