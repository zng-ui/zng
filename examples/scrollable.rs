use zero_ui::prelude::*;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            content_align = unset!;
            content = z_stack(widgets![
                scrollable! {
                    id = "scrollable";
                    padding = 20;
                    content = text! {
                        background_color = hex!(#245E81);
                        padding = 10;
                        text = ipsum()
                    }
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
        align = Align::TOP_LEFT;
        padding = 5;
        background_color = rgba(0, 0, 0, 90.pct());
        corner_radius = (0, 0, 8, 0);
        button::theme::padding = 4;

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
                ]
            },
            button! {
                content = text(show.map(|s| if !s { "Commands" } else { "Close" }.to_text()));
                margin = show.map(|s| if !s { 0.into() } else { (3, 0, 0, 0).into() });
                corner_radius = (0, 0, 4, 0);
                on_click = hn!(|ctx, _| {
                    show.modify(ctx, |s| **s = !**s);
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
fn separator() -> impl Widget {
    blank! {
        size = (8, 8);
    }
}

fn ipsum() -> Text {
    let mut p = String::new();
    for _ in 0..10 {
        p.push('\n');
        p.push_str(&lipsum::lipsum_words(25));
    }

    let mut s = "Lorem Ipsum".to_owned();
    for _ in 0..10 {
        s.push('\n');
        s.push_str(&p);
    }

    s.into()
}
