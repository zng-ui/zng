//! 9-patch image border.

use std::mem;

use zng_app::render::RepeatMode;
use zng_ext_image::{IMAGES, ImageCacheMode, ImageRenderArgs, ImageSource, Img};
use zng_wgt::prelude::*;

use crate::{IMAGE_CACHE_VAR, IMAGE_LIMITS_VAR, IMAGE_RENDERING_VAR};

/// 9-patch image border.
///
/// The `source` image is sliced by `slices` and projected onto `widths` in the widget layout.
///
/// See also [`border_img_repeat`] and [`border_img_fill`].
///
/// [`border_img_repeat`]: fn@border_img_repeat
/// [`border_img_fill`]: fn@border_img_fill
#[property(BORDER)]
pub fn border_img(
    child: impl IntoUiNode,
    widths: impl IntoVar<SideOffsets>,
    source: impl IntoVar<ImageSource>,
    slices: impl IntoVar<SideOffsets>,
) -> UiNode {
    let widths = widths.into_var();
    let source = source.into_var();
    let slices = slices.into_var();

    let mut img = var(Img::dummy(None)).read_only();
    let mut _img_sub = VarHandle::dummy();
    let mut slices_img_size = PxSize::zero();
    let mut slices_px = PxSideOffsets::zero();

    border_node(
        child,
        widths,
        match_node_leaf(move |op| match op {
            UiNodeOp::Init => {
                WIDGET
                    .sub_var(&source)
                    .sub_var(&IMAGE_CACHE_VAR)
                    .sub_var_render(&slices)
                    .sub_var_render(&BORDER_IMG_REPEAT_VAR)
                    .sub_var_render(&IMAGE_RENDERING_VAR)
                    .sub_var_render(&BORDER_IMG_FILL_VAR);

                let mode = if IMAGE_CACHE_VAR.get() {
                    ImageCacheMode::Cache
                } else {
                    ImageCacheMode::Ignore
                };
                let limits = IMAGE_LIMITS_VAR.get();

                let mut source = source.get();
                if let ImageSource::Render(_, args) = &mut source {
                    *args = Some(ImageRenderArgs::new(WINDOW.id()));
                }
                img = IMAGES.image(source, mode, limits, None, None);
                _img_sub = img.subscribe(UpdateOp::Update, WIDGET.id());

                let img = img.get();
                if img.is_loaded() {
                    slices_img_size = img.size();
                }
            }
            UiNodeOp::Deinit => {
                img = var(Img::dummy(None)).read_only();
                _img_sub = VarHandle::dummy();
            }
            UiNodeOp::Update { .. } => {
                if source.is_new() {
                    // source update:

                    let mut source = source.get();

                    if let ImageSource::Render(_, args) = &mut source {
                        *args = Some(ImageRenderArgs::new(WINDOW.id()));
                    }

                    let mode = if IMAGE_CACHE_VAR.get() {
                        ImageCacheMode::Cache
                    } else {
                        ImageCacheMode::Ignore
                    };
                    let limits = IMAGE_LIMITS_VAR.get();

                    img = IMAGES.image(source, mode, limits, None, None);
                } else if let Some(enabled) = IMAGE_CACHE_VAR.get_new() {
                    // cache-mode update:
                    let is_cached = img.with(|img| IMAGES.is_cached(img));
                    if enabled != is_cached {
                        img = if is_cached {
                            // must not cache, but is cached, detach from cache.

                            let img = mem::replace(&mut img, var(Img::dummy(None)).read_only());
                            IMAGES.detach(img)
                        } else {
                            // must cache, but image is not cached, get source again.

                            let source = source.get();
                            let limits = IMAGE_LIMITS_VAR.get();
                            IMAGES.image(source, ImageCacheMode::Cache, limits, None, None)
                        };
                    }
                }

                if img.is_new() {
                    let img = img.get();
                    if img.is_loaded() {
                        let s = img.size();
                        if s != slices_img_size {
                            slices_img_size = s;
                            WIDGET.layout();
                        }
                    }
                    WIDGET.render();
                }
            }
            UiNodeOp::Measure { desired_size, .. } => {
                *desired_size = LAYOUT.constraints().fill_size();
            }
            UiNodeOp::Layout { final_size, .. } => {
                *final_size = LAYOUT.constraints().fill_size();

                let metrics = LAYOUT
                    .metrics()
                    .with_constraints(PxConstraints2d::new_exact_size(slices_img_size))
                    .with_scale_factor(1.fct());
                let s = LAYOUT.with_context(metrics, || slices.layout());
                if s != slices_px {
                    slices_px = s;
                    WIDGET.render();
                }
            }
            UiNodeOp::Render { frame } => {
                let img = img.get();
                if img.is_loaded() {
                    let (rect, offsets) = BORDER.border_layout();
                    if !rect.size.is_empty() {
                        let repeats = BORDER_IMG_REPEAT_VAR.get();
                        frame.push_border_image(
                            rect,
                            offsets,
                            slices_px,
                            BORDER_IMG_FILL_VAR.get(),
                            repeats.top_bottom,
                            repeats.left_right,
                            &img,
                            IMAGE_RENDERING_VAR.get(),
                        );
                    }
                }
            }
            _ => {}
        }),
    )
}

/// Defines how the 9-patch edge slices are used to fill the widths.
///
/// This property sets the [`BORDER_IMG_REPEAT_VAR`].
#[property(CONTEXT, default(BORDER_IMG_REPEAT_VAR))]
pub fn border_img_repeat(child: impl IntoUiNode, repeats: impl IntoVar<BorderRepeats>) -> UiNode {
    with_context_var(child, BORDER_IMG_REPEAT_VAR, repeats)
}

/// Defines if the middle slice of the 9-patch image is also rendered.
///
/// This property sets the [`BORDER_IMG_FILL_VAR`].
#[property(CONTEXT, default(BORDER_IMG_FILL_VAR))]
pub fn border_img_fill(child: impl IntoUiNode, fill: impl IntoVar<bool>) -> UiNode {
    with_context_var(child, BORDER_IMG_FILL_VAR, fill)
}

/// Defines how the 9-patch edge slices are used to fill the widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub struct BorderRepeats {
    /// Top and bottom edges.
    ///
    /// Also middle if *fill* is set.
    pub top_bottom: RepeatMode,
    /// Left and right edges.
    pub left_right: RepeatMode,
}
impl BorderRepeats {
    /// Top-bottom and left-right equal. From any [`RepeatMode`] type.
    pub fn new<TB: Into<RepeatMode>, LR: Into<RepeatMode>>(top_bottom: TB, left_right: LR) -> Self {
        BorderRepeats {
            top_bottom: top_bottom.into(),
            left_right: left_right.into(),
        }
    }

    /// All sides equal. From any [`RepeatMode`] type.
    pub fn new_all<T: Into<RepeatMode>>(all_sides: T) -> Self {
        let all_sides = all_sides.into();
        Self::new(all_sides, all_sides)
    }
}
context_var! {
    /// Defines how the 9-patch edge slices are used to fill the widths.
    pub static BORDER_IMG_REPEAT_VAR: BorderRepeats = BorderRepeats::default();

    /// If the middle slice is rendered in a 9-patch border.
    pub static BORDER_IMG_FILL_VAR: bool = false;
}

impl_from_and_into_var! {
    fn from(repeat: RepeatMode) -> BorderRepeats {
        BorderRepeats::new_all(repeat)
    }

    /// `true` is `Repeat`, `false` is `Stretch`.
    fn from(repeat_or_stretch: bool) -> BorderRepeats {
        BorderRepeats::new_all(repeat_or_stretch)
    }

    fn from<TB: Into<RepeatMode>, LR: Into<RepeatMode>>((top_bottom, left_right): (TB, LR)) -> BorderRepeats {
        BorderRepeats::new(top_bottom, left_right)
    }

    fn from(_: ShorthandUnit![Stretch]) -> BorderRepeats {
        RepeatMode::Stretch.into()
    }
    fn from(_: ShorthandUnit![Repeat]) -> BorderRepeats {
        RepeatMode::Repeat.into()
    }
    fn from(_: ShorthandUnit![Round]) -> BorderRepeats {
        RepeatMode::Round.into()
    }
    fn from(_: ShorthandUnit![Space]) -> BorderRepeats {
        RepeatMode::Space.into()
    }
}
