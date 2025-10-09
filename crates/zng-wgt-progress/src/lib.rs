#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Progress indicator widget.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_wgt::{base_color, prelude::*, visibility};
use zng_wgt_container::{Container, child_out_bottom};
use zng_wgt_fill::background_color;
use zng_wgt_size_offset::{height, width, x};
use zng_wgt_style::{Style, StyleMix, impl_named_style_fn, impl_style_fn, style_fn};

pub use zng_task::Progress;

/// Progress indicator widget.
#[widget($crate::ProgressView { ($progress:expr) => { progress = $progress; }; })]
pub struct ProgressView(StyleMix<WidgetBase>);
impl ProgressView {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
        }
    }
}
impl_style_fn!(ProgressView);

context_var! {
    /// The progress status value in a [`ProgressView`](struct@ProgressView)
    pub static PROGRESS_VAR: Progress = Progress::indeterminate();
}

/// The progress status to be displayed.
///
/// This property sets the [`PROGRESS_VAR`].
#[property(CONTEXT, default(PROGRESS_VAR), widget_impl(ProgressView))]
pub fn progress(child: impl IntoUiNode, progress: impl IntoVar<Progress>) -> UiNode {
    with_context_var(child, PROGRESS_VAR, progress)
}

/// Collapse visibility when [`Progress::is_complete`].
#[property(CONTEXT, default(false), widget_impl(ProgressView))]
pub fn collapse_complete(child: impl IntoUiNode, collapse: impl IntoVar<bool>) -> UiNode {
    let collapse = collapse.into_var();
    visibility(
        child,
        expr_var! {
            if #{PROGRESS_VAR}.is_complete() && *#{collapse} {
                Visibility::Collapsed
            } else {
                Visibility::Visible
            }
        },
    )
}

/// Event raised for each progress update, and once after info init.
///
/// This event works in any context that sets [`PROGRESS_VAR`].
#[property(EVENT, widget_impl(ProgressView))]
pub fn on_progress(child: impl IntoUiNode, handler: Handler<Progress>) -> UiNode {
    // copied from `on_info_init`
    enum State {
        WaitInfo,
        InfoInited,
        Done,
    }
    let mut state = State::WaitInfo;
    let mut handler = handler.into_wgt_runner();

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&PROGRESS_VAR);
            state = State::WaitInfo;
        }
        UiNodeOp::Deinit => {
            handler.deinit();
        }
        UiNodeOp::Info { .. } => {
            if let State::WaitInfo = &state {
                state = State::InfoInited;
                WIDGET.update();
            }
        }
        UiNodeOp::Update { updates } => {
            c.update(updates);

            match state {
                State::Done => {
                    if PROGRESS_VAR.is_new() {
                        PROGRESS_VAR.with(|u| handler.event(u));
                    } else {
                        handler.update();
                    }
                }
                State::InfoInited => {
                    PROGRESS_VAR.with(|u| handler.event(u));
                    state = State::Done;
                }
                State::WaitInfo => {}
            }
        }
        _ => {}
    })
}

/// Event raised when progress updates to a complete state or inits completed.
///
/// This event works in any context that sets [`PROGRESS_VAR`].
#[property(EVENT, widget_impl(ProgressView))]
pub fn on_complete(child: impl IntoUiNode, handler: Handler<Progress>) -> UiNode {
    let mut is_complete = false;
    on_progress(
        child,
        handler.filtered(move |u| {
            let complete = u.is_complete();
            if complete != is_complete {
                is_complete = complete;
                return is_complete;
            }
            false
        }),
    )
}

/// Getter property that is `true` when progress is indeterminate.
///
/// This event works in any context that sets [`PROGRESS_VAR`].
#[property(EVENT, widget_impl(ProgressView))]
pub fn is_indeterminate(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state(child, PROGRESS_VAR.map(|p| p.is_indeterminate()), state)
}

/// Progress view default style (progress bar with message text).
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        let indeterminate_x = var(Length::from(0));
        let mut indeterminate_animation = None;
        let indeterminate_width = 10.pct();
        widget_set! {
            self;
            base_color = light_dark(rgb(0.82, 0.82, 0.82), rgb(0.18, 0.18, 0.18));

            zng_wgt_container::child = Container! {
                height = 5;
                background_color = colors::BASE_COLOR_VAR.rgba();

                clip_to_bounds = true;
                child_align = Align::FILL_START;
                child = zng_wgt::Wgt! {
                    background_color = colors::ACCENT_COLOR_VAR.rgba();

                    #[easing(200.ms())]
                    width = PROGRESS_VAR.map(|p| Length::from(p.fct()));

                    on_progress = hn!(indeterminate_x, |p| {
                        if p.is_indeterminate() {
                            // only animates when actually indeterminate
                            if indeterminate_animation.is_none() {
                                let h = indeterminate_x.sequence(move |i| {
                                    use zng_var::animation::easing;
                                    i.set_ease(-indeterminate_width, 100.pct(), 1.5.secs(), |t| easing::ease_out(easing::quad, t))
                                });
                                indeterminate_animation = Some(h);
                            }
                        } else {
                            indeterminate_animation = None;
                        }
                    });
                    when *#{PROGRESS_VAR.map(|p| p.is_indeterminate())} {
                        width = indeterminate_width;
                        x = indeterminate_x;
                    }
                };
            };

            child_out_bottom =
                zng_wgt_text::Text! {
                    txt = PROGRESS_VAR.map(|p| p.msg());
                    zng_wgt::visibility = PROGRESS_VAR.map(|p| Visibility::from(!p.msg().is_empty()));
                    zng_wgt::align = Align::CENTER;
                },
                6,
            ;
        }
    }
}

/// Progress view style that is only the progress bar, no message text.
#[widget($crate::SimpleBarStyle)]
pub struct SimpleBarStyle(DefaultStyle);
impl_named_style_fn!(simple_bar, SimpleBarStyle);
impl SimpleBarStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            named_style_fn = SIMPLE_BAR_STYLE_FN_VAR;
            child_out_bottom = unset!;
        }
    }
}
