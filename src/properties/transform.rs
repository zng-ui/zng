//! Transform properties, [`scale`](module@scale), [`rotate`](module@rotate), [`transform`](module@transform) and more.

use crate::core::{
    context::{LayoutContext, WidgetContext},
    render::{FrameBinding, FrameBindingKey, FrameBuilder, FrameUpdate},
    units::{self, *},
    var::{merge_var, IntoVar, LocalVar, ObjVar, Var},
};
use crate::core::{impl_ui_node, property, UiNode};

struct TransformNode<C: UiNode, T: LocalVar<Transform>> {
    child: C,
    transform: T,
    layout_transform: LayoutTransform,
    frame_key: Option<FrameBindingKey<LayoutTransform>>,
}

#[impl_ui_node(child)]
impl<C: UiNode, T: LocalVar<Transform>> UiNode for TransformNode<C, T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);
        self.transform.init_local(ctx.vars);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);
        if self.transform.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout();
            // ctx.updates.push_render_update(); TODO?
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.layout_transform = self.transform.get_local().to_layout(final_size, ctx);
        self.child.arrange(final_size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let transform = if let Some(frame_key) = self.frame_key {
            frame_key.bind(self.layout_transform)
        } else {
            FrameBinding::Value(self.layout_transform)
        };
        frame.push_transform(transform, |frame| self.child.render(frame));
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        if let Some(frame_key) = self.frame_key {
            update.update_transform(frame_key.update(self.layout_transform));
        }
        self.child.render_update(update);
    }
}

/// Custom transform.
///
/// See [`Transform`] for how to initialize a custom transform.
///
/// This property does not affect layout, the widget is transformed only during rendering.
#[property(outer)]
pub fn transform(child: impl UiNode, transform: impl IntoVar<Transform>) -> impl UiNode {
    let transform = transform.into_local();
    let frame_key = if transform.can_update() {
        Some(FrameBindingKey::new_unique())
    } else {
        None
    };
    TransformNode {
        child,
        transform,
        frame_key,
        layout_transform: LayoutTransform::identity(),
    }
}

/// Rotate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`rotate(angle)`](units::rotate) using variable mapping.
///
/// This property does not affect layout, the widget is rotated only during rendering.
#[property(outer)]
pub fn rotate(child: impl UiNode, angle: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform::set(child, angle.into_var().map(|&a| units::rotate(a)))
}

/// Scale transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale(s)`](units::scale) using variable mapping.
///
/// This property does not affect layout, the widget is scaled only during rendering.
#[property(outer)]
pub fn scale(child: impl UiNode, s: impl IntoVar<FactorNormal>) -> impl UiNode {
    transform::set(child, s.into_var().map(|&x| units::scale(x)))
}

/// Scale X and Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_xy(x, y)`](units::scale) using variable merging.
///
/// This property does not affect layout, the widget is scaled only during rendering.
#[property(outer)]
pub fn scale_xy(child: impl UiNode, x: impl IntoVar<FactorNormal>, y: impl IntoVar<FactorNormal>) -> impl UiNode {
    transform::set(child, merge_var!(x.into_var(), y.into_var(), |&x, &y| units::scale_xy(x, y)))
}

/// Scale X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_x(x)`](units::scale_x) using variable mapping.
///
/// This property does not affect layout, the widget is scaled only during rendering.
#[property(outer)]
pub fn scale_x(child: impl UiNode, x: impl IntoVar<FactorNormal>) -> impl UiNode {
    transform::set(child, x.into_var().map(|&x| units::scale_x(x)))
}

/// Scale Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`scale_y(y)`](units::scale_y) using variable mapping.
///
/// This property does not affect layout, the widget is scaled only during rendering.
#[property(outer)]
pub fn scale_y(child: impl UiNode, y: impl IntoVar<FactorNormal>) -> impl UiNode {
    transform::set(child, y.into_var().map(|&y| units::scale_y(y)))
}

/// Skew transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew(x, y)`](units::skew) using variable merging.
///
/// This property does not affect layout, the widget is skewed only during rendering.
#[property(outer)]
pub fn skew(child: impl UiNode, x: impl IntoVar<AngleRadian>, y: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform::set(child, merge_var!(x.into_var(), y.into_var(), |&x, &y| units::skew(x, y)))
}

/// Skew X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew_x(x)`](units::skew_x) using variable mapping.
///
/// This property does not affect layout, the widget is skewed only during rendering.
#[property(outer)]
pub fn skew_x(child: impl UiNode, x: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform::set(child, x.into_var().map(|&x| units::skew_x(x)))
}

/// Skew Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`skew_y(y)`](units::skew_y) using variable mapping.
///
/// This property does not affect layout, the widget is skewed only during rendering.
#[property(outer)]
pub fn skew_y(child: impl UiNode, y: impl IntoVar<AngleRadian>) -> impl UiNode {
    transform::set(child, y.into_var().map(|&y| units::skew_y(y)))
}

/// Translate transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate(x, y)`](units::translate) using variable merging.
///
/// This property does not affect layout, the widget is moved only during rendering.
#[property(outer)]
pub fn translate(child: impl UiNode, x: impl IntoVar<Length>, y: impl IntoVar<Length>) -> impl UiNode {
    transform::set(child, merge_var!(x.into_var(), y.into_var(), |&x, &y| units::translate(x, y)))
}

/// Translate X transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate_x(x)`](units::translate_x) using variable mapping.
///
/// This property does not affect layout, the widget is moved only during rendering.
#[property(outer)]
pub fn translate_x(child: impl UiNode, x: impl IntoVar<Length>) -> impl UiNode {
    transform::set(child, x.into_var().map(|&x| units::translate_x(x)))
}

/// Translate Y transform.
///
/// This property is a shorthand way of setting [`transform`] to [`translate_y(y)`](units::translate_y) using variable mapping.
///
/// This property does not affect layout, the widget is moved only during rendering.
#[property(outer)]
pub fn translate_y(child: impl UiNode, y: impl IntoVar<Length>) -> impl UiNode {
    transform::set(child, y.into_var().map(|&y| units::translate_y(y)))
}
