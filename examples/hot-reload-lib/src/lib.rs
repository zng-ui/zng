use zng::{prelude::*, prelude_wgt::*};

// Declare hot reload dynamic entry.
zng::hot_reload::zng_hot_entry!();

/// Hot reloading node.
#[hot_node]
pub fn hot_node() -> UiNode {
    tracing::info!("`hot_node()` called");
    Text! {
        widget::on_init = hn!(|_| {
            tracing::info!("hot node on_init");
        });
        widget::on_deinit = hn!(|_| {
            tracing::info!("hot node on_deinit");
        });
        widget::background_color = rgb(0, 128, 255).darken(50.pct());
        txt = "Hello, this node is hot!";
    }
}

/// Hot reloading property.
///
/// Note that the `input` does not hot reload, only changes inside the property. As an alternative
/// you can declare a test `hot_node` that creates an widget that sets the property, in that context
/// the property input can be hot reloaded.
#[hot_node]
#[property(FILL)]
pub fn hot_prop(child: impl IntoUiNode, input: impl IntoVar<bool>) -> UiNode {
    let input = input.into_var();

    let mut clip = PxRect::zero();

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&input);
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);

            let s = Size::from(16).layout();
            let mut offset = Align::TOP_RIGHT.child_offset(s, *final_size, LAYOUT.direction());
            offset.x += s.width / 2;
            offset.y -= s.height / 2;
            let new_clip = PxRect::new(offset.to_point(), s);
            if clip != new_clip {
                clip = new_clip;
                WIDGET.render();
            }
        }
        UiNodeOp::Render { frame } => {
            c.render(frame);

            let color = match input.get() {
                true => colors::GREEN,
                false => colors::RED,
            };

            frame.push_clip_rounded_rect(clip, PxCornerRadius::new_all(clip.size), false, false, |frame| {
                frame.push_color(clip, color.into());
            });
        }
        _ => {}
    })
}
