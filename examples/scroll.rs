#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
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
    App::default().run_window(async {
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
    Menu!(ui_vec![
        SubMenu!(
            "Scroll",
            ui_vec![
                cmd_btn(SCROLL_UP_CMD),
                cmd_btn(SCROLL_DOWN_CMD),
                cmd_btn(SCROLL_LEFT_CMD),
                cmd_btn(SCROLL_RIGHT_CMD),
                Hr!(),
                cmd_btn(PAGE_UP_CMD),
                cmd_btn(PAGE_DOWN_CMD),
                cmd_btn(PAGE_LEFT_CMD),
                cmd_btn(PAGE_RIGHT_CMD),
            ]
        ),
        SubMenu!(
            "Scroll to",
            ui_vec![
                cmd_btn(SCROLL_TO_TOP_CMD),
                cmd_btn(SCROLL_TO_BOTTOM_CMD),
                cmd_btn(SCROLL_TO_LEFTMOST_CMD),
                cmd_btn(SCROLL_TO_RIGHTMOST_CMD),
                Hr!(),
                scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::minimal(10)),
                scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::center()),
            ]
        )
    ])
}
fn cmd_btn(cmd: Command) -> impl UiNode {
    let cmd = cmd.scoped(WidgetId::named("scroll"));
    Button! {
        child = Text!(cmd.name_with_shortcut());
        enabled = cmd.is_enabled();
        // visibility = cmd.has_handlers().map_into();
        on_click = hn!(|_| {
            cmd.notify();
        });
    }
}
fn scroll_to_btn(target: WidgetId, mode: ScrollToMode) -> impl UiNode {
    use zero_ui::widgets::scroll::commands;

    let scroll = WidgetId::named("scroll");
    let cmd = commands::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} {}", target, if let ScrollToMode::Minimal{..} = &mode { "(minimal)" } else { "(center)" });
        enabled = cmd.is_enabled();
        on_click = hn!(|_| {
            commands::scroll_to(scroll, target, mode.clone());
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
