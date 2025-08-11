use zng_ext_input::mouse::*;
use zng_ext_window::{cmd::*, *};
use zng_wgt::{prelude::*, *};
use zng_wgt_button::Button;
use zng_wgt_container::*;
use zng_wgt_fill::*;
use zng_wgt_input::{mouse::*, *};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::Text;

/// Custom window chrome adorner used when the window manager does not provide one.
///
/// You also must set a padding of `5` for maximized window and `(28 + 5, 5, 5, 5)` for normal window.
pub fn fallback_chrome() -> UiNode {
    let vars = WINDOW.vars();
    let can_move = vars.state().map(|s| matches!(s, WindowState::Normal | WindowState::Maximized));
    let win_id = WINDOW.id();
    let title = Text! {
        txt = vars.title();
        align = Align::FILL_TOP;
        background_color = light_dark(colors::WHITE, colors::BLACK);
        zng_wgt_size_offset::height = 28;
        txt_align = Align::CENTER;

        child_right = Stack! {
            direction = StackDirection::left_to_right();
            zng_wgt_button::style_fn = zng_wgt_button::LightStyle! {
                corner_radius = 0;
                zng_wgt_button::cmd_child_fn = wgt_fn!(|cmd: Command| {
                    presenter((), cmd.icon().map(move |ico| wgt_fn!(ico, |_| {
                        let ico = ico(());
                        if ico.is_nil() {
                            // fallback to Unicode symbol
                            let cmd = cmd.scoped(zng_app::event::CommandScope::App);
                            let (symbol, size, padding_top) = if cmd == RESTORE_CMD {
                                ("🗗", 9, 0)
                            } else if cmd == MINIMIZE_CMD {
                                ("🗕", 9, 0)
                            } else if cmd == MAXIMIZE_CMD {
                                ("🗖", 9, 0)
                            } else if cmd == CLOSE_CMD {
                                ("🗙", 12, -5)
                            } else {
                                unreachable!("{cmd:?} what")
                            };
                            Text! {
                                font_family = "Noto Sans Symbols 2";
                                font_size = size.pt();
                                padding = (padding_top, 0, 0, 0);
                                txt = symbol;
                            }
                        } else {
                            ico
                        }
                    })))
                });
            };
            children = ui_vec![
                Button! {
                    cmd = MINIMIZE_CMD.scoped(win_id);
                },
                Button! {
                    cmd = MAXIMIZE_CMD.scoped(win_id);
                    when #is_disabled {
                        visibility = false;
                    }
                },
                Button! {
                    cmd = RESTORE_CMD.scoped(win_id);
                    when #is_disabled {
                        visibility = false;
                    }
                },
                Button! {
                    cmd = CLOSE_CMD.scoped(win_id);
                },
            ];
        }, 0;

        when *#{can_move.clone()} {
            cursor = CursorIcon::Move;
        }
        mouse::on_mouse_down = hn!(|args: &MouseInputArgs| {
            if args.is_primary() && can_move.get() && args.target.widget_id() == WIDGET.id() {
                DRAG_MOVE_RESIZE_CMD.scoped(WINDOW.id()).notify();
            }
        });

        gesture::on_context_click = hn!(|args: &gesture::ClickArgs| {
            if matches!(WINDOW.vars().state().get(), WindowState::Normal | WindowState::Maximized) && args.target.widget_id() == WIDGET.id()
                && let Some(p) = args.position() {
                    OPEN_TITLE_BAR_CONTEXT_MENU_CMD.scoped(WINDOW.id()).notify_param(p);
                }
        });
    };

    use zng_ext_window::cmd::ResizeDirection as RD;

    fn resize_direction(wgt_pos: PxPoint) -> Option<RD> {
        let p = wgt_pos;
        let s = WIDGET.bounds().inner_size();
        let b = WIDGET.border().offsets();
        let corner_b = b * FactorSideOffsets::from(3.fct());

        if p.x <= b.left {
            if p.y <= corner_b.top {
                Some(RD::NorthWest)
            } else if p.y >= s.height - corner_b.bottom {
                Some(RD::SouthWest)
            } else {
                Some(RD::West)
            }
        } else if p.x >= s.width - b.right {
            if p.y <= corner_b.top {
                Some(RD::NorthEast)
            } else if p.y >= s.height - corner_b.bottom {
                Some(RD::SouthEast)
            } else {
                Some(RD::East)
            }
        } else if p.y <= b.top {
            if p.x <= corner_b.left {
                Some(RD::NorthWest)
            } else if p.x >= s.width - corner_b.right {
                Some(RD::NorthEast)
            } else {
                Some(RD::North)
            }
        } else if p.y >= s.height - b.bottom {
            if p.x <= corner_b.left {
                Some(RD::SouthWest)
            } else if p.x >= s.width - corner_b.right {
                Some(RD::SouthEast)
            } else {
                Some(RD::South)
            }
        } else {
            None
        }
    }

    let cursor = var(CursorSource::Hidden);

    Container! {
        exclude_text_context = true;

        hit_test_mode = HitTestMode::Detailed;

        child = title;

        when matches!(#{vars.state()}, WindowState::Normal) {
            border = 5, light_dark(colors::WHITE, colors::BLACK).rgba().map_into();
            cursor = cursor.clone();
            on_mouse_move = hn!(|args: &MouseMoveArgs| {
                cursor.set(match args.position_wgt().and_then(resize_direction) {
                    Some(d) => CursorIcon::from(d).into(),
                    None => CursorSource::Hidden,
                });
            });
            on_mouse_down = hn!(|args: &MouseInputArgs| {
                if args.is_primary()
                    && let Some(d) = args.position_wgt().and_then(resize_direction) {
                        DRAG_MOVE_RESIZE_CMD.scoped(WINDOW.id()).notify_param(d);
                    }
            });
        }
    }
}

#[property(WIDGET)]
fn exclude_text_context(child: impl IntoUiNode, exclude: impl IntoValue<bool>) -> UiNode {
    assert!(exclude.into());

    // exclude all text context vars set on the window
    let excluded_set = {
        let mut c = ContextValueSet::new();
        Text::context_vars_set(&mut c);
        c
    };
    let child = match_node(child, move |c, op| {
        let mut filtered = LocalContext::capture_filtered(CaptureFilter::Exclude(excluded_set.clone()));
        filtered.with_context(|| {
            c.op(op);
        });
    });

    // override layout font size
    zng_wgt_text::font_size(child, 16)
}
