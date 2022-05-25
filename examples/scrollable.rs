#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::widgets::scrollable::commands::ScrollToMode;

use zero_ui_view_prebuilt as zero_ui_view;


fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-scrollable.json.gz", &[("example", &"scrollable")], |_| true);   

    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            content = z_stack(widgets![
                scrollable! {
                    id = "scrollable";
                    padding = 20;
                    background_color = hex!(#245E81);
                    // smooth_scrolling = false;
                    content = v_stack(widgets![
                        text! {
                            id = "Lorem 1";
                            text = "Lorem 1";
                            font_size = 20;
                            align = Align::LEFT;
                        },
                        text(ipsum()),
                        text! {
                            id = "Lorem 2";
                            text = "Lorem 2";
                            font_size = 20;
                            align = Align::LEFT;
                        },
                        text(ipsum())
                    ])
                },
                commands()
            ]);
        }
    })
}

fn commands() -> impl Widget {
    use zero_ui::widgets::scrollable::commands::*;

    let show = var(false);

    v_stack! {
        align = Align::TOP;
        padding = 5;
        background_color = rgba(0, 0, 0, 90.pct());
        corner_radius = (0, 0, 8, 8);
        button::theme::padding = 4;
        alt_focus_scope = true;

        items = widgets![
            v_stack! {
                visibility = show.map_into();
                spacing = 3;
                button::theme::corner_radius = 0;

                items = widgets![
                    cmd_btn(ScrollUpCommand),
                    cmd_btn(ScrollDownCommand),
                    cmd_btn(ScrollLeftCommand),
                    cmd_btn(ScrollRightCommand),
                    separator(),
                    cmd_btn(PageUpCommand),
                    cmd_btn(PageDownCommand),
                    cmd_btn(PageLeftCommand),
                    cmd_btn(PageRightCommand),
                    separator(),
                    cmd_btn(ScrollToTopCommand),
                    cmd_btn(ScrollToBottomCommand),
                    cmd_btn(ScrollToLeftmostCommand),
                    cmd_btn(ScrollToRightmostCommand),
                    separator(),
                    scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::minimal(10)),
                    scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::center()),
                    separator(),
                ]
            },
            button! {
                content = text(show.map(|s| if !s { "Commands" } else { "Close" }.to_text()));
                margin = show.map(|s| if !s { 0.into() } else { (3, 0, 0, 0).into() });
                corner_radius = (0, 0, 4, 4);
                on_click = hn!(|ctx, _| {
                    show.modify(ctx, |mut s| *s = !*s);
                });
            }
        ];
    }
}
fn cmd_btn(cmd: impl Command) -> impl Widget {
    let cmd = cmd.scoped(WidgetId::named("scrollable"));
    button! {
        content = text(cmd.name_with_shortcut());
        enabled = cmd.enabled();
        // visibility = cmd.has_handlers().map_into();
        on_click = hn!(|ctx, _| {
            cmd.notify_cmd(ctx, None);
        })
    }
}
fn scroll_to_btn(target: WidgetId, mode: ScrollToMode) -> impl Widget {
    use zero_ui::widgets::scrollable::commands;

    let scrollable = WidgetId::named("scrollable");
    let cmd = commands::ScrollToCommand.scoped(scrollable);
    button! {
        content = text(formatx!("Scroll To {} {}", target, if let ScrollToMode::Minimal{..} = &mode { "(minimal)" } else { "(center)" }));
        enabled = cmd.enabled();
        on_click = hn!(|ctx, _| {
            commands::scroll_to(ctx, scrollable, target, mode.clone());
        });
    }
}
fn separator() -> impl Widget {
    blank! {
        size = (8, 8);
    }
}

fn ipsum() -> Text {
    let mut s = String::new();
    for _ in 0..10 {
        for _ in 0..10 {
            s.push_str(&lipsum::lipsum_words(25));
            s.push('\n');
        }
        s.push('\n');
    }

    s.into()
}
