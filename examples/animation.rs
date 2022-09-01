#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("animation");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        window! {
            title = "Animation Example";
            padding = 10;
            content_align = Align::CENTER;
            content = example(ctx.vars);
        }
    })
}

const FROM_COLOR: Rgba = colors::RED;
const TO_COLOR: Rgba = colors::GREEN;
const FPS: u32 = 60;

fn example(vars: &Vars) -> impl Widget {
    // vars.animation_time_scale().set(vars, 0.5.fct());
    vars.frame_duration().set(vars, (1.0 / FPS as f32).secs());

    let x = var(0.dip());
    let color = var(FROM_COLOR);

    // x.trace_value(vars, move |v| {
    //     tracing::debug_span!("x", value = ?v, thread = "<x>").entered()
    // })
    // .perm();

    use animation::EasingModifierFn::*;
    let easing_mod = var(EaseOut);

    v_stack! {
        spacing = 10;
        items_align = Align::TOP;
        items = widgets![
            container! {
                id = "demo";
                width = 301;
                background = ruler();
                margin = (0, 0, 40, 0);
                content_align = Align::LEFT;
                content = blank! {
                    id = "ball";
                    size = (40, 40);
                    corner_radius = 20;
                    background_color = color.clone();

                    x = x.map(|x| x.clone() - 20.dip());

                    when self.is_hovered {
                        background_color = colors::LIME;
                    }
                };
            },
            h_stack! {
                id = "mod-menu";
                spacing = 2;
                toggle::selection = toggle::SingleSel::new(easing_mod.clone());
                items = {
                    let mode = |m: animation::EasingModifierFn| toggle! {
                        content = text(m.to_text());
                        value = m;
                    };
                    widgets![
                        mode(EaseIn),
                        mode(EaseOut),
                        mode(EaseInOut),
                        mode(EaseOutIn),
                    ]
                }
            },
            uniform_grid! {
                id = "easing-menu";
                spacing = 2;
                columns = 7;
                button::vis::dark = theme_generator!(|_, _| {
                    button::vis::dark_theme! {
                        padding = 3;
                    }
                });
                button::vis::light = theme_generator!(|_, _| {
                    button::vis::light_theme! {
                        padding = 3;
                    }
                });
                items = widgets![
                    ease_btn(&x, &color, "linear", easing::linear, &easing_mod),
                    ease_btn(&x, &color, "quad", easing::quad, &easing_mod),
                    ease_btn(&x, &color, "cubic", easing::cubic, &easing_mod),
                    ease_btn(&x, &color, "quart", easing::quart, &easing_mod),
                    ease_btn(&x, &color, "quint", easing::quint, &easing_mod),
                    ease_btn(&x, &color, "sine", easing::sine, &easing_mod),
                    ease_btn(&x, &color, "expo", easing::expo, &easing_mod),
                    ease_btn(&x, &color, "circ", easing::circ, &easing_mod),
                    ease_btn(&x, &color, "back", easing::back, &easing_mod),
                    ease_btn(&x, &color, "elastic", easing::elastic, &easing_mod),
                    ease_btn(&x, &color, "bounce", easing::bounce, &easing_mod),
                    ease_btn(&x, &color, "step_ceil(6)", |t| easing::step_ceil(6, t), &easing_mod),
                    ease_btn(&x, &color, "step_floor(6)", |t| easing::step_floor(6, t), &easing_mod),
                    ease_btn(&x, &color, "none", easing::none, &easing_mod),
                ]
            },
            button! {
                content = text("reset");
                foreground_highlight = {
                    offsets: -2,
                    widths: 1,
                    sides: colors::DARK_RED,
                };
                click_shortcut = shortcut![Escape];
                on_click = hn!(x, color, |ctx, _| {
                    x.set(ctx, 0);
                    color.set(ctx, FROM_COLOR);
                });
            },
        ]
    }
}

fn ease_btn(
    l: &RcVar<Length>,
    color: &RcVar<Rgba>,
    name: impl Into<Text>,
    easing: impl Fn(EasingTime) -> EasingStep + Copy + 'static,
    easing_mod: &RcVar<animation::EasingModifierFn>,
) -> impl Widget {
    let in_plot = plot(easing);
    let out_plot = plot(move |t| easing::ease_out(easing, t));
    let in_out_plot = plot(move |t| easing::ease_in_out(easing, t));
    let out_in_plot = plot(move |t| easing::ease_out_in(easing, t));

    use animation::EasingModifierFn::*;

    button! {
        content = v_stack! {
            spacing = 2;
            items_align = Align::TOP;
            items = widgets![
                text(name.into()),
                image! {
                    scale_ppi = true;
                    loading_view = view_generator!(|_, _| blank! {
                        size = (64, 64);
                        margin = 10;
                    });
                    source = easing_mod.map(move |m| match m {
                        EaseIn => in_plot.clone(),
                        EaseOut => out_plot.clone(),
                        EaseInOut => in_out_plot.clone(),
                        EaseOutIn => out_in_plot.clone(),
                    });
                },
            ]
        };
        on_click = hn!(l, color, easing_mod, |ctx, _| {
            l.set_ease(ctx, 0, 300, 1.secs(), easing_mod.get(ctx).modify_fn(easing)).perm();
            color.set_ease(ctx, FROM_COLOR, TO_COLOR, 1.secs(), easing_mod.get(ctx).modify_fn(easing)).perm();
        });
    }
}
fn plot(easing: impl Fn(EasingTime) -> EasingStep + 'static) -> ImageSource {
    let size = (64, 64);
    ImageSource::render_node(
        RenderMode::Software,
        clone_move!(size, |ctx, _| {
            let mut items = widget_vec![];
            let color_t = animation::Transition::new(FROM_COLOR, TO_COLOR);
            let fps_f = FPS as f32;
            for i in 0..=FPS {
                let x_fct = (i as f32 / fps_f).fct();
                let x = size.0 * x_fct;

                let y_fct = easing(EasingTime::new(x_fct));
                let y = size.1 * (1.fct() - y_fct);

                items.push(blank! {
                    offset = (x, y);
                    size = (3, 3);
                    corner_radius = 2;
                    translate = -1.5, -1.5;
                    background_color = color_t.sample(y_fct);
                })
            }

            let meta_color = WindowVars::req(&ctx.window_state).actual_theme().map(|t| match t {
                WindowTheme::Light => rgba(0, 0, 0, 0.4),
                WindowTheme::Dark => rgba(255, 255, 255, 0.4),
            });

            #[allow(clippy::precedence)]
            items.push(text! {
                text = "v";
                font_size = 12;
                font_style = FontStyle::Italic;
                color = meta_color.clone();
                offset = (-3.dip() - 100.pct(), -3.dip());
            });
            items.push(text! {
                text = "t";
                font_size = 12;
                font_style = FontStyle::Italic;
                color = meta_color.clone();
                offset = (size.0.dip() - 100.pct() - 3.dip(), size.1 - 3);
            });
            z_stack! {
                items_align = Align::TOP_LEFT;
                items;
                size;
                border = (0, 0, 1, 1), meta_color.map_into();
                margin = 10;
            }
        }),
    )
}

fn ruler() -> impl Widget {
    z_stack! {
        items_align = Align::LEFT;
        items = (0..=300).step_by(10)
            .map(|x| rule_line! {
                orientation = LineOrientation::Vertical;
                color = TEXT_COLOR_VAR.map(|c| c.with_alpha(40.pct()));
                x = x.dip();
                height = if x % 100 == 0 { 52 } else if x % 50 == 0 { 22 } else { 12 };
            }
            .boxed_wgt())
            .collect::<WidgetVec>(),
    }
}
