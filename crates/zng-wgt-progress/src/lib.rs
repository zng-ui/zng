#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Progress indicator widget.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_app::widget::border::BorderSide;
use zng_layout::unit::euclid;
use zng_var::{
    VARS,
    animation::{AnimationHandle, Transition, easing},
};
use zng_wgt::{
    base_color,
    prelude::{colors::ACCENT_COLOR_VAR, *},
    visibility,
};
use zng_wgt_container::{self as container, Container};
use zng_wgt_fill::background_color;
use zng_wgt_size_offset::{height, width, x};
use zng_wgt_style::{Style, StyleMix, impl_named_style_fn, impl_style_fn};

pub use zng_task::Progress;

/// Progress indicator widget.
#[widget($crate::ProgressView { ($progress:expr) => { progress = $progress; }; })]
pub struct ProgressView(StyleMix<WidgetBase>);
impl ProgressView {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
    }
}
impl_style_fn!(ProgressView, DefaultStyle);

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
#[property(CONTEXT, default(false), widget_impl(ProgressView, DefaultStyle))]
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
#[property(EVENT, widget_impl(ProgressView, DefaultStyle))]
pub fn is_indeterminate(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state(child, PROGRESS_VAR.map(|p| p.is_indeterminate()), state)
}

/// Progress view default style (progress bar with message text).
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            base_color = light_dark(rgb(0.82, 0.82, 0.82), rgb(0.18, 0.18, 0.18));

            container::child = Container! {
                height = 5;
                background_color = colors::BASE_COLOR_VAR.rgba();

                clip_to_bounds = true;
                child = {
                    let ind_x = var(Length::from(0));
                    let ind_width = 10.pct();

                    zng_wgt::Wgt! {
                        background_color = colors::ACCENT_COLOR_VAR.rgba();

                        #[easing(200.ms())]
                        width = PROGRESS_VAR.map(|p| Length::from(p.fct()));

                        on_progress = {
                            let mut handle = VarHandle::dummy();
                            hn!(ind_x, |p| {
                                if p.is_indeterminate() {
                                    // only animates when actually indeterminate
                                    if handle.is_dummy() {
                                        handle =
                                            ind_x.sequence(move |x| x.set_ease(-ind_width, 100.pct(), 1.5.secs(), |t| easing::ease_out(easing::quad, t)));
                                    }
                                } else {
                                    handle = VarHandle::dummy();
                                }
                            })
                        };
                        when #{PROGRESS_VAR}.is_indeterminate() {
                            width = ind_width;
                            x = ind_x;
                        }
                    }
                };
            };

            container::child_spacing = 6;
            container::child_out_bottom = zng_wgt_text::Text! {
                txt = PROGRESS_VAR.map(|p| p.msg());
                zng_wgt::visibility = PROGRESS_VAR.map(|p| Visibility::from(!p.msg().is_empty()));
                zng_wgt::align = Align::CENTER;
            };
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
            container::child_out_bottom = unset!;
        }
    }
}

/// Circular progress indicator style.
#[widget($crate::CircularStyle)]
pub struct CircularStyle(Style);
impl_named_style_fn!(circular, CircularStyle);
impl CircularStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            replace = true;
            named_style_fn = CIRCULAR_STYLE_FN_VAR;
            container::child_start = {
                let start = var(0.rad());
                let end = var(0.rad());
                zng_wgt::Wgt! {
                    zng_wgt_size_offset::size = 1.4.em();
                    zng_wgt_fill::background = arc_shape(0.2.em(), ACCENT_COLOR_VAR.rgba(), start.clone(), end.clone());
                    on_progress = {
                        let mut ind_handle = AnimationHandle::dummy();
                        hn!(|args| {
                            if args.is_indeterminate() {
                                if ind_handle.is_stopped() {
                                    ind_handle = VARS.animate(clmv!(start, end, |a| {
                                        if a.count() == 0 {
                                            let t = a.elapsed_restart(1.secs());

                                            end.set(Transition::new(0.turn(), 1.turn()).sample(t.fct()));

                                            if let Some(t) = t.seg(80.pct()..) {
                                                start.set(Transition::new(0.turn(), 0.8.turn()).sample(t.fct()));
                                            }
                                        } else {
                                            let t = a.elapsed_restart(500.ms());
                                            let v = Transition::new(0.turn(), 1.turn()).sample(t.fct());
                                            start.set(v - 0.2.turn());
                                            end.set(v);
                                        }
                                    }));
                                }
                            } else {
                                if !ind_handle.is_stopped() {
                                    ind_handle = AnimationHandle::dummy();
                                    start.ease(0.rad(), 200.ms(), easing::linear).perm();
                                }
                                end.ease(args.fct().0.turn(), 200.ms(), |t| easing::ease_out(easing::quad, t))
                                    .perm();
                            }
                        })
                    };
                }
            };
            container::child_spacing = 6;
            container::child = zng_wgt_text::Text! {
                txt = PROGRESS_VAR.map(|p| p.msg());
                zng_wgt::visibility = PROGRESS_VAR.map(|p| Visibility::from(!p.msg().is_empty()));
            };
        }
    }
}

/// Circular progress indicator style without message text.
#[widget($crate::SimpleCircularStyle)]
pub struct SimpleCircularStyle(Style);
impl_named_style_fn!(simple_circular, SimpleCircularStyle);
impl SimpleCircularStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            container::child = unset!;
        }
    }
}

/// Render an arc line or circle.
///
/// The arc ellipses is defined by the fill area available for the node. If `start` and `end` are equal does not render, if
/// `end` overlaps one turn renders a full circle. 0º is at the top.
pub fn arc_shape(
    thickness: impl IntoVar<Length>,
    color: impl IntoVar<Rgba>,
    start: impl IntoVar<AngleRadian>,
    end: impl IntoVar<AngleRadian>,
) -> UiNode {
    // To leverage GPU rendering we render the arc using two halves of a circle drawn
    // with border+corner-radius and clips
    let thickness = thickness.into_var();
    let color = color.into_var();
    let start = start.into_var();
    let end = end.into_var();

    let mut render_thickness = Px(0);
    let mut render_size = PxSize::zero();
    let rotate_start_key = FrameValueKey::new_unique();
    let rotate_half0_key = FrameValueKey::new_unique();
    let rotate_half1_key = FrameValueKey::new_unique();

    // [start, half0, half1]
    fn rotates(area: PxSize, start: AngleRadian, end: AngleRadian) -> [PxTransform; 3] {
        let center = area.to_vector().cast::<f32>() * 0.5.fct();
        let rotate = |rad: f32| {
            PxTransform::translation(-center.x, -center.y)
                .then(&Transform::new_rotate(rad.rad()).layout())
                .then_translate(center)
        };

        // first half is round border top-right, clipped to left side of area
        // second is bottom-left, clipped to right side of area
        let trick = 45.0_f32.to_radians();

        let length = (end.0 - start.0).max(0.0).min(360.0_f32.to_radians());
        let half_rad = 180.0_f32.to_radians();
        let rotate_half = |length: f32, stitch: f32| {
            let t = rotate(trick - half_rad + length);

            // Webrender leaves a faint subpixel line at the edge of clips, translate to hide error
            if length < 0.001 || length > 180.0_f32.to_radians() - 0.001 {
                t.then_translate(euclid::vec2(stitch, 0.0))
            } else {
                t
            }
        };

        let stitch = if start.0.abs() > 0.001 { 1.5 } else { 1.0 };
        [
            rotate(start.0),
            rotate_half(length.min(half_rad), -stitch),
            rotate_half((length - half_rad).max(0.0), stitch),
        ]
    }

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_layout(&thickness)
                .sub_var_render(&color)
                .sub_var_render_update(&start)
                .sub_var_render_update(&end);
        }
        UiNodeOp::Layout { final_size, .. } => {
            *final_size = LAYOUT.constraints().fill_size();

            // Snap center point, without this can render a faint subpixel line, even with the correction implemented by `rotate_half`.
            let mut s = *final_size;
            s.width.0 = ((final_size.width.0 as f32 / 2.0).floor() * 2.0) as _;

            if render_size != s {
                render_size = s;
                WIDGET.render();
            }
            let t = thickness.layout_x();
            if render_thickness != t {
                render_thickness = t;
                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            let [start_t, half0_t, half1_t] = rotates(render_size, start.get(), end.get());
            let is_animating = start.is_animating() || end.is_animating();

            frame.push_reference_frame(
                rotate_start_key.into(),
                rotate_start_key.bind(start_t, is_animating),
                false,
                true,
                |frame| {
                    let half = PxPoint::new(render_size.width / Px(2), Px(0));
                    let color = BorderSide::from(color.get());

                    frame.push_clip_rect(PxRect::new(half, render_size), false, true, |frame| {
                        frame.push_reference_frame(
                            rotate_half0_key.into(),
                            rotate_half0_key.bind(half0_t, is_animating),
                            false,
                            true,
                            |frame| {
                                let mut b = BorderSides::hidden();
                                b.top = color;
                                b.right = b.top;
                                frame.push_border(
                                    PxRect::from(render_size),
                                    PxSideOffsets::new_all_same(render_thickness),
                                    b,
                                    PxCornerRadius::new_all(render_size),
                                );
                            },
                        );
                    });

                    frame.push_clip_rect(PxRect::new(-half, render_size), false, true, |frame| {
                        frame.push_reference_frame(
                            rotate_half1_key.into(),
                            rotate_half1_key.bind(half1_t, is_animating),
                            false,
                            true,
                            |frame| {
                                let mut b = BorderSides::hidden();
                                b.bottom = color;
                                b.left = b.bottom;
                                frame.push_border(
                                    PxRect::from(render_size),
                                    PxSideOffsets::new_all_same(render_thickness),
                                    b,
                                    PxCornerRadius::new_all(render_size),
                                );
                            },
                        );
                    });
                },
            );
        }
        UiNodeOp::RenderUpdate { update } => {
            let [start_t, half0_t, half1_t] = rotates(render_size, start.get(), end.get());
            let is_animating = start.is_animating() || end.is_animating();

            update.update_transform(rotate_start_key.update(start_t, is_animating), true);
            update.update_transform(rotate_half0_key.update(half0_t, is_animating), true);
            update.update_transform(rotate_half1_key.update(half1_t, is_animating), true);
        }
        _ => {}
    })
}
