use crate::prelude::*;

/// Clips the widget child to the area of the widget when set to `true`.
///
/// Any content rendered outside the widget inner bounds is clipped, hit test shapes are also clipped. The clip is
/// rectangular and can have rounded corners if [`corner_radius`] is set. If the widget is inlined during layout the first
/// row advance and last row trail are also clipped.
///
/// # Examples
///
/// ```
/// # macro_rules! _demo { () => {
/// Container! {
///     background_color = rgb(255, 0, 0);
///     size = (200, 300);
///     corner_radius = 5;
///     clip_to_bounds = true;
///     child = Container! {
///         background_color = rgb(0, 255, 0);
///         // fixed size ignores the layout available size.
///         size = (1000, 1000);
///         child = Text!("1000x1000 green clipped to 200x300");
///     };
/// }
/// # }}
/// ```
///
/// [`corner_radius`]: fn@crate::corner_radius
#[property(FILL, default(false))]
pub fn clip_to_bounds(child: impl UiNode, clip: impl IntoVar<bool>) -> impl UiNode {
    let clip = clip.into_var();
    let mut corners = PxCornerRadius::zero();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.layout().render();
        }
        UiNodeOp::Update { .. } => {
            if clip.is_new() {
                WIDGET.layout().render();
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            let bounds = child.layout(wl);

            if clip.get() {
                let c = BORDER.border_radius();
                if c != corners {
                    corners = c;
                    WIDGET.render();
                }
            }

            *final_size = bounds;
        }
        UiNodeOp::Render { frame } => {
            if clip.get() {
                frame.push_clips(
                    |c| {
                        let wgt_bounds = WIDGET.bounds();
                        let bounds = PxRect::from_size(wgt_bounds.inner_size());

                        if corners != PxCornerRadius::zero() {
                            c.push_clip_rounded_rect(bounds, corners, false, true);
                        } else {
                            c.push_clip_rect(bounds, false, true);
                        }

                        if let Some(inline) = wgt_bounds.inline() {
                            for r in inline.negative_space().iter() {
                                c.push_clip_rect(*r, true, true);
                            }
                        };
                    },
                    |f| child.render(f),
                );
            } else {
                child.render(frame);
            }
        }
        _ => {}
    })
}
