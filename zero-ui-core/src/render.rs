//! Frame render and metadata API.

use crate::{
    app::view_process::ViewRenderer,
    border::BorderSides,
    color::{self, filters::RenderFilter, RenderColor},
    context::{WIDGET, WINDOW},
    gradient::{RenderExtendMode, RenderGradientStop},
    text::FontAntiAliasing,
    units::*,
    var::{self, impl_from_and_into_var},
    widget_info::{HitTestClips, WidgetBoundsInfo, WidgetInfoTree, WidgetRenderInfo},
    widget_instance::{WidgetId, ZIndex},
};

use std::{marker::PhantomData, mem, ops};

use webrender_api::{FontRenderMode, PipelineId};
pub use zero_ui_view_api::{
    webrender_api, DisplayListBuilder, FilterOp, FrameId, FrameValue, FrameValueUpdate, RenderMode, RendererDebug, ReuseRange,
};
use zero_ui_view_api::{
    webrender_api::{DynamicProperties, GlyphInstance, GlyphOptions, MixBlendMode, SpatialTreeItemKey},
    DisplayList, ReuseStart,
};

/// A text font.
///
/// This trait is an interface for the renderer into the font API used in the application.
///
/// # Font API
///
/// The default font API is provided by [`FontManager`] that is included
/// in the app default extensions. The default font type is [`Font`] that implements this trait.
///
/// [`FontManager`]: crate::text::FontManager
/// [`Font`]: crate::text::Font
pub trait Font {
    /// Gets if the font is the fallback that does not have any glyph.
    fn is_empty_fallback(&self) -> bool;

    /// Gets the instance key in the `renderer` namespace.
    ///
    /// The font configuration must be provided by `self`, except the `synthesis` that is used in the font instance.
    fn instance_key(
        &self,
        renderer: &ViewRenderer,
        synthesis: FontSynthesis,
    ) -> (webrender_api::FontInstanceKey, webrender_api::FontInstanceFlags);
}

/// A loaded or loading image.
///
/// This trait is an interface for the renderer into the image API used in the application.
///
/// The ideal image format is BGRA with pre-multiplied alpha.
///
/// # Image API
///
/// The default image API is provided by [`ImageManager`] that is included
/// in the app default extensions. The default image type is [`Img`] that implements this trait.
///
/// [`ImageManager`]: crate::image::ImageManager
/// [`Img`]: crate::image::Img
pub trait Img {
    /// Gets the image key in the `renderer` namespace.
    ///
    /// The image must be loaded asynchronously by `self` and does not need to
    /// be loaded yet when the key is returned.
    fn image_key(&self, renderer: &ViewRenderer) -> webrender_api::ImageKey;

    /// Returns a value that indicates if the image is already pre-multiplied.
    ///
    /// The faster option is pre-multiplied, that is also the default return value.
    fn alpha_type(&self) -> webrender_api::AlphaType {
        webrender_api::AlphaType::PremultipliedAlpha
    }
}

/// Image scaling algorithm in the renderer.
///
/// If an image is not rendered at the same size as their source it must be up-scaled or
/// down-scaled. The algorithms used for this scaling can be selected using this `enum`.
///
/// Note that the algorithms used in the renderer value performance over quality and do a good
/// enough job for small or temporary changes in scale only, such as a small size correction or a scaling animation.
/// If and image is constantly rendered at a different scale you should considered scaling it on the CPU using a
/// slower but more complex algorithm or pre-scaling it before including in the app.
///
/// You can use the [`Img`] type to re-scale an image, image widgets probably can be configured to do this too.
///
/// [`Img`]: crate::image::Img
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ImageRendering {
    /// Let the renderer select the algorithm, currently this is the same as [`CrispEdges`].
    ///
    /// [`CrispEdges`]: ImageRendering::CrispEdges
    Auto = 0,
    /// The image is scaled with an algorithm that preserves contrast and edges in the image,
    /// and which does not smooth colors or introduce blur to the image in the process.
    ///
    /// Currently the [Bilinear] interpolation algorithm is used.
    ///
    /// [Bilinear]: https://en.wikipedia.org/wiki/Bilinear_interpolation
    CrispEdges = 1,
    /// When scaling the image up, the image appears to be composed of large pixels.
    ///
    /// Currently the [Nearest-neighbor] interpolation algorithm is used.
    ///
    /// [Nearest-neighbor]: https://en.wikipedia.org/wiki/Nearest-neighbor_interpolation
    Pixelated = 2,
}
impl From<ImageRendering> for webrender_api::ImageRendering {
    fn from(r: ImageRendering) -> Self {
        use webrender_api::ImageRendering::*;
        match r {
            ImageRendering::Auto => Auto,
            ImageRendering::CrispEdges => CrispEdges,
            ImageRendering::Pixelated => Pixelated,
        }
    }
}

macro_rules! expect_inner {
    ($self:ident.$fn_name:ident) => {
        if $self.is_outer() {
            tracing::error!("called `{}` in outer context of `{}`", stringify!($fn_name), $self.widget_id);
        }
    };
}

struct WidgetData {
    outer_offset: PxVector,
    inner_is_set: bool, // used to flag if frame is always 2d translate/scale.
    inner_transform: PxTransform,
    filter: RenderFilter,
}

/// A full frame builder.
pub struct FrameBuilder {
    frame_id: FrameId,
    pipeline_id: PipelineId,
    widget_id: WidgetId,
    transform: PxTransform,

    default_font_aa: FontRenderMode,

    renderer: Option<ViewRenderer>,

    scale_factor: Factor,

    display_list: DisplayListBuilder,

    hit_testable: bool,
    visible: bool,
    auto_hit_test: bool,
    hit_clips: HitTestClips,

    auto_hide_rect: PxRect,
    widget_data: Option<WidgetData>,
    child_offset: PxVector,
    parent_inner_bounds: Option<PxRect>,

    can_reuse: bool,
    open_reuse: Option<ReuseStart>,

    clear_color: RenderColor,

    render_index: ZIndex,
}
impl FrameBuilder {
    /// New builder.
    ///
    /// * `frame_id` - Id of the new frame.
    /// * `root_id` - Id of the window root widget.
    /// * `renderer` - Connection to the renderer connection that will render the frame, is `None` in renderless mode.
    /// * `scale_factor` - Scale factor that will be used to render the frame, usually the scale factor of the screen the window is at.
    /// * `default_font_aa` - Fallback font anti-aliasing used when the default value is requested.
    /// because WebRender does not let us change the initial clear color.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        frame_id: FrameId,
        root_id: WidgetId,
        root_bounds: &WidgetBoundsInfo,
        info_tree: &WidgetInfoTree,
        renderer: Option<ViewRenderer>,
        scale_factor: Factor,
        default_font_aa: FontAntiAliasing,
    ) -> Self {
        let pipeline_id = renderer
            .as_ref()
            .and_then(|r| r.pipeline_id().ok())
            .unwrap_or_else(PipelineId::dummy);

        let display_list = DisplayListBuilder::new(pipeline_id, frame_id);

        let root_size = root_bounds.outer_size();
        let auto_hide_rect = PxRect::from_size(root_size).inflate(root_size.width, root_size.height);
        root_bounds.set_outer_transform(PxTransform::identity(), info_tree);

        FrameBuilder {
            frame_id,
            pipeline_id,
            widget_id: root_id,
            transform: PxTransform::identity(),
            default_font_aa: match default_font_aa {
                FontAntiAliasing::Default | FontAntiAliasing::Subpixel => FontRenderMode::Subpixel,
                FontAntiAliasing::Alpha => FontRenderMode::Alpha,
                FontAntiAliasing::Mono => FontRenderMode::Mono,
            },
            renderer,
            scale_factor,
            display_list,
            hit_testable: true,
            visible: true,
            auto_hit_test: false,
            hit_clips: HitTestClips::default(),
            widget_data: Some(WidgetData {
                filter: vec![],
                outer_offset: PxVector::zero(),
                inner_is_set: false,
                inner_transform: PxTransform::identity(),
            }),
            child_offset: PxVector::zero(),
            parent_inner_bounds: None,
            can_reuse: true,
            open_reuse: None,
            auto_hide_rect,

            render_index: ZIndex(0),

            clear_color: color::rgba(0, 0, 0, 0).into(),
        }
    }

    /// [`new`](Self::new) with only the inputs required for renderless mode.
    pub fn new_renderless(
        frame_id: FrameId,
        root_id: WidgetId,
        root_bounds: &WidgetBoundsInfo,
        info_tree: &WidgetInfoTree,
        scale_factor: Factor,
        default_font_aa: FontAntiAliasing,
    ) -> Self {
        Self::new(frame_id, root_id, root_bounds, info_tree, None, scale_factor, default_font_aa)
    }

    /// Pixel scale factor used by the renderer.
    ///
    /// All layout values are scaled by this factor in the renderer.
    pub fn scale_factor(&self) -> Factor {
        self.scale_factor
    }

    /// If is building a frame for a headless and renderless window.
    ///
    /// In this mode only the meta and layout information will be used as a *frame*.
    pub fn is_renderless(&self) -> bool {
        self.renderer.is_none()
    }

    /// Set the color used to clear the pixel frame before drawing this frame.
    ///
    /// Note the default clear color is `rgba(0, 0, 0, 0)`, and it is not retained, a property
    /// that sets the clear color must set it every render.
    ///
    /// Note that the clear color is always *rendered* first before all other layers, if more then
    /// one layer sets the clear color only the value set on the top-most layer is used.
    pub fn set_clear_color(&mut self, color: RenderColor) {
        self.clear_color = color;
    }

    /// Connection to the renderer that will render this frame.
    ///
    /// Returns `None` when in [renderless](Self::is_renderless) mode.
    pub fn renderer(&self) -> Option<&ViewRenderer> {
        self.renderer.as_ref()
    }

    /// Id of the frame being build.
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Id of the current widget context.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Renderer pipeline ID or [`dummy`].
    ///
    /// [`dummy`]: PipelineId::dummy
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Current transform.
    pub fn transform(&self) -> &PxTransform {
        &self.transform
    }

    /// Returns `true` if hit-testing is enabled in the widget context, if `false` methods that push
    /// a hit-test silently skip.
    ///
    /// This can be set to `false` in a context using [`with_hit_tests_disabled`].
    ///
    /// [`with_hit_tests_disabled`]: Self::with_hit_tests_disabled
    pub fn is_hit_testable(&self) -> bool {
        self.hit_testable
    }

    /// Returns `true` if display items are actually generated, if `false` only transforms and hit-test are rendered.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Returns `true` if hit-tests are automatically pushed by `push_*` methods.
    ///
    /// Note that hit-tests are only added if [`is_hit_testable`] is `true`.
    ///
    /// [`is_hit_testable`]: Self::is_hit_testable
    pub fn auto_hit_test(&self) -> bool {
        self.auto_hit_test
    }

    /// Runs `render` with hit-tests disabled, inside `render` [`is_hit_testable`] is `false`, after
    /// it is the current value.
    ///
    /// [`is_hit_testable`]: Self::is_hit_testable
    pub fn with_hit_tests_disabled(&mut self, render: impl FnOnce(&mut Self)) {
        let prev = mem::replace(&mut self.hit_testable, false);
        render(self);
        self.hit_testable = prev;
    }

    /// Runs `render` with [`auto_hit_test`] set to a value for the duration of the `render` call.
    ///
    /// If this is used, [`FrameUpdate::with_auto_hit_test`] must also be used.
    ///
    /// [`auto_hit_test`]: Self::auto_hit_test
    pub fn with_auto_hit_test(&mut self, auto_hit_test: bool, render: impl FnOnce(&mut Self)) {
        let prev = mem::replace(&mut self.auto_hit_test, auto_hit_test);
        render(self);
        self.auto_hit_test = prev;
    }

    /// Current culling rect, widgets with outer-bounds that don't intersect this rect are rendered [hidden].
    ///
    /// [hidden]: Self::hide
    pub fn auto_hide_rect(&self) -> PxRect {
        self.auto_hide_rect
    }

    /// Runs `render` and [`hide`] all widgets with outer-bounds that don't intersect with the `auto_hide_rect`.
    ///
    /// [`hide`]: Self::hide
    pub fn with_auto_hide_rect(&mut self, auto_hide_rect: PxRect, render: impl FnOnce(&mut Self)) {
        let parent_rect = mem::replace(&mut self.auto_hide_rect, auto_hide_rect);
        render(self);
        self.auto_hide_rect = parent_rect;
    }

    /// Start a new widget outer context, this sets [`is_outer`] to `true` until an inner call to [`push_inner`],
    /// during this period properties can configure the widget stacking context and actual rendering and transforms
    /// are discouraged.
    ///
    /// If `reuse` is `Some` and the widget has been rendered before  and [`can_reuse`] allows reuse, the `render`
    /// closure is not called, an only a reference to the widget range in the previous frame is send.
    ///
    /// If the widget is collapsed during layout it is not rendered. See [`WidgetLayout::collapse`] for more details.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    /// [`can_reuse`]: Self::can_reuse
    /// [`WidgetLayout::collapse`]: crate::widget_info::WidgetLayout::collapse
    pub fn push_widget(&mut self, reuse: &mut Option<ReuseRange>, render: impl FnOnce(&mut Self)) {
        let id = WIDGET.id();

        if self.widget_data.is_some() {
            tracing::error!(
                "called `push_widget` for `{}` without calling `push_inner` for the parent `{}`",
                id,
                self.widget_id
            );
        }

        let bounds = WIDGET.bounds();
        let tree = WINDOW.widget_tree();

        if bounds.is_collapsed() {
            // collapse
            for info in tree.get(id).unwrap().self_and_descendants() {
                info.bounds_info().set_rendered(None, &tree);
            }
            return;
        }

        let prev_outer = bounds.outer_transform();
        let outer_transform = PxTransform::from(self.child_offset).then(&self.transform);
        if prev_outer != outer_transform {
            *reuse = None;
        }
        bounds.set_outer_transform(outer_transform, &tree);
        let outer_bounds = bounds.outer_bounds();

        let parent_visible = self.visible;

        if bounds.can_auto_hide() {
            match self.auto_hide_rect.intersection(&outer_bounds) {
                Some(cull) => {
                    let partial = cull != outer_bounds;
                    if partial || bounds.is_partially_culled() {
                        // partial cull, cannot reuse because descendant vis may have changed.
                        *reuse = None;
                        bounds.set_is_partially_culled(partial);
                    }
                }
                None => {
                    // full cull
                    self.visible = false;
                }
            }
        } else {
            bounds.set_is_partially_culled(false);
        }

        let can_reuse = match bounds.rendered() {
            Some(i) => i.visible == self.visible,
            // cannot reuse if the widget was not rendered in the previous frame (clear stale reuse ranges in descendants).
            None => false,
        };
        let parent_can_reuse = mem::replace(&mut self.can_reuse, can_reuse);

        self.render_index.0 += 1;
        let widget_z = self.render_index;

        let mut undo_prev_outer_transform = None;
        if reuse.is_some() {
            // check if is possible to reuse.

            if !self.can_reuse {
                *reuse = None; // reuse is stale because the widget was previously not rendered, or is disabled by user.
            } else if prev_outer != outer_transform {
                if let Some(undo_prev) = prev_outer.inverse() {
                    undo_prev_outer_transform = Some(undo_prev);
                } else {
                    *reuse = None; // cannot reuse because cannot undo prev-transform.
                }
            }
        }

        let index = self.hit_clips.push_child(id);
        bounds.set_hit_index(index);

        let mut reused = true;
        let display_count = self.display_list.len();

        let child_offset = mem::take(&mut self.child_offset);

        // try to reuse, or calls the closure and saves the reuse range.
        self.push_reuse(reuse, |frame| {
            // did not reuse, render widget.

            reused = false;
            undo_prev_outer_transform = None;

            frame.widget_data = Some(WidgetData {
                filter: vec![],
                outer_offset: child_offset,
                inner_is_set: false,
                inner_transform: PxTransform::identity(),
            });
            let parent_widget = mem::replace(&mut frame.widget_id, id);

            render(frame);

            frame.widget_id = parent_widget;
            frame.widget_data = None;
        });

        if reused {
            // if did reuse, patch transforms and z-indexes.

            let _span = tracing::trace_span!("reuse-descendants", ?id).entered();

            let transform_patch = undo_prev_outer_transform.and_then(|t| {
                let t = t.then(&outer_transform);
                if t != PxTransform::identity() {
                    Some(t)
                } else {
                    None
                }
            });
            let z_patch = bounds.rendered().map(|i| widget_z.0 as i64 - i.back.0 as i64).unwrap_or(0);

            let update_transforms = transform_patch.is_some();
            let update_z = z_patch != 0;

            // apply patches, only iterates over descendants once.
            if update_transforms && update_z {
                let transform_patch = transform_patch.unwrap();

                // patch current widget's inner.
                bounds.set_inner_transform(bounds.inner_transform().then(&transform_patch), &tree, id, self.parent_inner_bounds);

                // patch descendants outer and inner.
                for info in tree.get(id).unwrap().descendants() {
                    let bounds = info.bounds_info();

                    bounds.set_outer_transform(bounds.outer_transform().then(&transform_patch), &tree);
                    bounds.set_inner_transform(
                        bounds.inner_transform().then(&transform_patch),
                        &tree,
                        info.id(),
                        info.parent().map(|p| p.inner_bounds()),
                    );

                    if let Some(info) = bounds.rendered() {
                        let back = info.back.0 as i64 + z_patch;
                        let front = info.front.0 as i64 + z_patch;
                        bounds.set_rendered(
                            Some(WidgetRenderInfo {
                                visible: self.visible,
                                back: ZIndex(back as u32),
                                front: ZIndex(front as u32),
                            }),
                            &tree,
                        );
                    }
                }
            } else if update_transforms {
                let transform_patch = transform_patch.unwrap();

                bounds.set_inner_transform(bounds.inner_transform().then(&transform_patch), &tree, id, self.parent_inner_bounds);

                for info in tree.get(id).unwrap().descendants() {
                    let bounds = info.bounds_info();

                    bounds.set_outer_transform(bounds.outer_transform().then(&transform_patch), &tree);
                    bounds.set_inner_transform(
                        bounds.inner_transform().then(&transform_patch),
                        &tree,
                        info.id(),
                        info.parent().map(|p| p.inner_bounds()),
                    );
                }
            } else if update_z {
                for info in tree.get(id).unwrap().self_and_descendants() {
                    let bounds = info.bounds_info();

                    if let Some(info) = bounds.rendered() {
                        let back = info.back.0 as i64 + z_patch;
                        let front = info.front.0 as i64 + z_patch;
                        bounds.set_rendered(
                            Some(WidgetRenderInfo {
                                visible: info.visible,
                                back: ZIndex(back as u32),
                                front: ZIndex(front as u32),
                            }),
                            &tree,
                        );
                    }
                }
            }

            // increment by reused
            self.render_index = bounds.rendered().map(|i| i.front).unwrap_or(self.render_index);
        } else {
            // if did not reuse and rendered
            bounds.set_rendered(
                Some(WidgetRenderInfo {
                    visible: self.display_list.len() > display_count,
                    back: widget_z,
                    front: self.render_index,
                }),
                &tree,
            );
        }

        self.visible = parent_visible;
        self.can_reuse = parent_can_reuse;
    }

    /// If previously generated display list items are available for reuse.
    ///
    /// If `false` widgets must do a full render using [`push_widget`] even if they did not request a render.
    ///
    /// [`push_widget`]: Self::push_widget
    pub fn can_reuse(&self) -> bool {
        self.can_reuse
    }

    /// Calls `render` with [`can_reuse`] set to `false`.
    ///
    /// [`can_reuse`]: Self::can_reuse
    pub fn with_no_reuse(&mut self, render: impl FnOnce(&mut Self)) {
        let prev_can_reuse = self.can_reuse;
        self.can_reuse = false;
        render(self);
        self.can_reuse = prev_can_reuse;
    }

    /// If `group` has a range and [`can_reuse`] a reference to the items is added, otherwise `generate` is called and
    /// any display items generated by it are tracked in `group`.
    ///
    /// Note that hit-test items are not part of `group`, only display items are reused here, hit-test items for an widget are only reused if the entire
    /// widget is reused in [`push_widget`]. This method is recommended for widgets that render a large volume of display data that is likely to be reused
    /// even when the widget itself is not reused, an example is a widget that renders text and a background, the entire widget is invalidated when the
    /// background changes, but the text is the same, so placing the text in a reuse group avoids having to upload all glyphs again.
    ///
    /// [`can_reuse`]: Self::can_reuse
    /// [`push_widget`]: Self::push_widget
    pub fn push_reuse(&mut self, group: &mut Option<ReuseRange>, generate: impl FnOnce(&mut Self)) {
        if self.can_reuse {
            if let Some(g) = &group {
                if g.pipeline_id() == self.pipeline_id {
                    if self.visible {
                        self.display_list.push_reuse_range(g);
                    }
                    return;
                }
            }
        }
        *group = None;
        let parent_group = self.open_reuse.replace(self.display_list.start_reuse_range());

        generate(self);

        let start = self.open_reuse.take().unwrap();
        let range = self.display_list.finish_reuse_range(start);
        *group = Some(range);
        self.open_reuse = parent_group;
    }

    /// Calls `render` with [`is_visible`] set to `false`.
    ///
    /// Nodes that set the visibility to [`Hidden`] must render using this method and update using the [`FrameUpdate::hidden`] method.
    ///
    /// Note that for [`Collapsed`] the widget is automatically not rendered if [`WidgetLayout::collapse`] or other related
    /// collapse method was already called for it.
    ///
    /// [`is_visible`]: Self::is_visible
    /// [`Hidden`]: crate::widget_info::Visibility::Hidden
    /// [`Collapsed`]: crate::widget_info::Visibility::Collapsed
    /// [`WidgetLayout::collapse`]: crate::widget_info::WidgetLayout::collapse
    pub fn hide(&mut self, render: impl FnOnce(&mut Self)) {
        let parent_visible = mem::replace(&mut self.visible, false);
        render(self);
        self.visible = parent_visible;
    }

    /// Returns `true`  if the widget stacking context is still being build.
    ///
    /// This is `true` when inside an [`push_widget`] call but `false` when inside an [`push_inner`] call.
    ///
    /// [`push_widget`]: Self::push_widget
    /// [`push_inner`]: Self::push_inner
    pub fn is_outer(&self) -> bool {
        self.widget_data.is_some()
    }

    /// Includes a widget filter and continues the render build.
    ///
    /// This is valid only when [`is_outer`].
    ///
    /// When [`push_inner`] is called a stacking context is created for the widget that includes the `filter`.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    pub fn push_inner_filter(&mut self, filter: RenderFilter, render: impl FnOnce(&mut Self)) {
        if let Some(data) = self.widget_data.as_mut() {
            let mut filter = filter;
            filter.reverse(); // see `Self::open_widget_display` for why it is reversed.
            data.filter.extend(filter.iter().copied());

            render(self);
        } else {
            tracing::error!("called `push_inner_filter` inside inner context of `{}`", self.widget_id);
            render(self);
        }
    }

    /// Includes a widget opacity filter and continues the render build.
    ///
    /// This is valid only when [`is_outer`].
    ///
    /// When [`push_inner`] is called a stacking context is created for the widget that includes the opacity filter.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    pub fn push_inner_opacity(&mut self, bind: FrameValue<f32>, render: impl FnOnce(&mut Self)) {
        if let Some(data) = self.widget_data.as_mut() {
            data.filter.push(FilterOp::Opacity(bind));

            render(self);
        } else {
            tracing::error!("called `push_inner_opacity` inside inner context of `{}`", self.widget_id);
            render(self);
        }
    }

    /// Pre-starts the scope of an widget with `offset` set for the inner reference frame. The
    /// `render` closure must call [`push_widget`] before attempting to render.
    ///
    /// Nodes that use [`WidgetLayout::with_child`] to optimize reference frames must use this method when
    /// a reference frame was not created during render.
    ///
    /// Nodes that use this must also use [`FrameUpdate::with_child`].
    ///
    /// [`push_widget`]: Self::push_widget
    /// [`WidgetLayout::with_child`]: crate::widget_info::WidgetLayout::with_child
    pub fn push_child(&mut self, offset: PxVector, render: impl FnOnce(&mut Self)) {
        if self.widget_data.is_some() {
            tracing::error!("called `push_child` outside inner context of `{}`", self.widget_id);
        }

        self.child_offset = offset;
        render(self);
        self.child_offset = PxVector::zero();
    }

    /// Include the `transform` on the widget inner reference frame.
    ///
    /// This is valid only when [`is_outer`].
    ///
    /// When [`push_inner`] is called a reference frame is created for the widget that applies the layout transform then the `transform`.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    pub fn push_inner_transform(&mut self, transform: &PxTransform, render: impl FnOnce(&mut Self)) {
        if let Some(data) = &mut self.widget_data {
            let parent_transform = data.inner_transform;
            let parent_is_set = mem::replace(&mut data.inner_is_set, true);
            data.inner_transform = data.inner_transform.then(transform);

            render(self);

            if let Some(data) = &mut self.widget_data {
                data.inner_transform = parent_transform;
                data.inner_is_set = parent_is_set;
            }
        } else {
            tracing::error!("called `push_inner_transform` inside inner context of `{}`", self.widget_id);
            render(self);
        }
    }

    /// Push the widget reference frame and stacking context then call `render` inside of it.
    ///
    /// If `layout_translation_animating` is `false` the view-process can still be updated using [`FrameUpdate::update_inner`], but
    /// a full webrender frame will be generated for each update, if is `true` webrender frame updates are used, but webrender
    /// skips some optimizations, such as auto-merging transforms. When in doubt setting this to `true` is better than `false` as
    /// a webrender frame update is faster than a full frame, and the transform related optimizations don't gain much.
    pub fn push_inner(
        &mut self,
        layout_translation_key: FrameValueKey<PxTransform>,
        layout_translation_animating: bool,
        render: impl FnOnce(&mut Self),
    ) {
        if let Some(mut data) = self.widget_data.take() {
            let parent_transform = self.transform;
            let parent_hit_clips = mem::take(&mut self.hit_clips);

            let id = WIDGET.id();
            let bounds = WIDGET.bounds();
            let tree = WINDOW.widget_tree();

            let inner_offset = bounds.inner_offset();
            let inner_transform = data.inner_transform.then_translate((data.outer_offset + inner_offset).cast());
            self.transform = inner_transform.then(&parent_transform);
            bounds.set_inner_transform(self.transform, &tree, id, self.parent_inner_bounds);

            let parent_parent_inner_bounds = mem::replace(&mut self.parent_inner_bounds, Some(bounds.inner_bounds()));

            if self.visible {
                self.display_list.push_reference_frame(
                    SpatialFrameKey::from_widget(self.widget_id).to_wr(),
                    layout_translation_key.bind(inner_transform, layout_translation_animating),
                    !data.inner_is_set,
                );

                let has_stacking_ctx = !data.filter.is_empty();
                if has_stacking_ctx {
                    // we want to apply filters in the top-to-bottom, left-to-right order they appear in
                    // the widget declaration, but the widget declaration expands to have the top property
                    // node be inside the bottom property node, so the bottom property ends up inserting
                    // a filter first, because we cannot insert filters after the child node render is called
                    // so we need to reverse the filters here. Left-to-right sequences are reversed on insert
                    // so they get reversed again here and everything ends up in order.
                    data.filter.reverse();

                    self.display_list
                        .push_stacking_context(MixBlendMode::Normal, &data.filter, &[], &[]);
                }

                render(self);

                if has_stacking_ctx {
                    self.display_list.pop_stacking_context();
                }
                self.display_list.pop_reference_frame();
            } else {
                render(self);
            }

            self.transform = parent_transform;
            self.parent_inner_bounds = parent_parent_inner_bounds;

            let hit_clips = mem::replace(&mut self.hit_clips, parent_hit_clips);
            bounds.set_hit_clips(hit_clips);
        } else {
            tracing::error!("called `push_inner` more then once for `{}`", self.widget_id);
            render(self)
        }
    }

    /// Returns `true` if the widget reference frame and stacking context is pushed and now is time for rendering the widget.
    ///
    /// This is `true` when inside a [`push_inner`] call but `false` when inside a [`push_widget`] call.
    ///
    /// [`push_widget`]: Self::push_widget
    /// [`push_inner`]: Self::push_inner
    pub fn is_inner(&self) -> bool {
        self.widget_data.is_none()
    }

    /// Gets the inner-bounds hit-test shape builder.
    pub fn hit_test(&mut self) -> HitTestBuilder {
        expect_inner!(self.hit_test);

        HitTestBuilder {
            hit_clips: &mut self.hit_clips,
            is_hit_testable: self.hit_testable,
        }
    }

    /// Calls `render` with a new clip context that adds the `clip_rect`.
    ///
    /// If `clip_out` is `true` only pixels outside the rect are visible. If `hit_test` is `true` the hit-test shapes
    /// rendered inside `render` are also clipped.
    ///
    /// Note that [`auto_hit_test`] overwrites `hit_test` if it is `true`.
    ///
    /// [`auto_hit_test`]: Self::auto_hit_test
    pub fn push_clip_rect(&mut self, clip_rect: PxRect, clip_out: bool, hit_test: bool, render: impl FnOnce(&mut FrameBuilder)) {
        self.push_clips(move |c| c.push_clip_rect(clip_rect, clip_out, hit_test), render)
    }

    /// Calls `render` with a new clip context that adds  the `clip_rect` with rounded `corners`.
    ///
    /// If `clip_out` is `true` only pixels outside the rounded rect are visible. If `hit_test` is `true` the hit-test shapes
    /// rendered inside `render` are also clipped.
    ///
    /// Note that [`auto_hit_test`] overwrites `hit_test` if it is `true`.
    ///
    /// [`auto_hit_test`]: Self::auto_hit_test
    pub fn push_clip_rounded_rect(
        &mut self,
        clip_rect: PxRect,
        corners: PxCornerRadius,
        clip_out: bool,
        hit_test: bool,
        render: impl FnOnce(&mut FrameBuilder),
    ) {
        self.push_clips(move |c| c.push_clip_rounded_rect(clip_rect, corners, clip_out, hit_test), render)
    }

    /// Calls `clips` to push multiple clips that define a new clip context, then calls `render` in the clip context.
    pub fn push_clips(&mut self, clips: impl FnOnce(&mut ClipBuilder), render: impl FnOnce(&mut FrameBuilder)) {
        expect_inner!(self.push_clips);

        let (mut render_count, mut hit_test_count) = {
            let mut clip_builder = ClipBuilder {
                builder: self,
                render_count: 0,
                hit_test_count: 0,
            };
            clips(&mut clip_builder);
            (clip_builder.render_count, clip_builder.hit_test_count)
        };

        render(self);

        while hit_test_count > 0 {
            hit_test_count -= 1;

            self.hit_clips.pop_clip();
        }
        while render_count > 0 {
            render_count -= 1;

            self.display_list.pop_clip();
        }
    }

    /// Calls `render` inside a new reference frame transformed by `transform`.
    ///
    /// The `is_2d_scale_translation` flag optionally marks the `transform` as only ever having a simple 2D scale or translation,
    /// allowing for webrender optimizations.
    ///
    /// If `hit_test` is `true` the hit-test shapes rendered inside `render` for the same widget are also transformed.
    ///
    /// Note that [`auto_hit_test`] overwrites `hit_test` if it is `true`.
    ///
    /// [`push_inner`]: Self::push_inner
    /// [`WidgetLayout`]: crate::widget_info::WidgetLayout
    /// [`auto_hit_test`]: Self::auto_hit_test
    pub fn push_reference_frame(
        &mut self,
        key: SpatialFrameKey,
        transform: FrameValue<PxTransform>,
        is_2d_scale_translation: bool,
        hit_test: bool,
        render: impl FnOnce(&mut Self),
    ) {
        let transform_value = transform.value();

        let prev_transform = self.transform;
        self.transform = transform_value.then(&prev_transform);

        if self.visible {
            self.display_list
                .push_reference_frame(key.to_wr(), transform, is_2d_scale_translation);
        }

        let hit_test = hit_test || self.auto_hit_test;

        if hit_test {
            self.hit_clips.push_transform(transform);
        }

        render(self);

        if self.visible {
            self.display_list.pop_reference_frame();
        }
        self.transform = prev_transform;

        if hit_test {
            self.hit_clips.pop_transform();
        }
    }

    /// Calls `render` with added `filter` stacking context.
    ///
    /// Note that this introduces a new stacking context, you can use the [`push_inner_filter`] method to
    /// add to the widget stacking context.
    ///
    /// [`push_inner_filter`]: Self::push_inner_filter
    pub fn push_filter(&mut self, blend_mode: MixBlendMode, filter: &RenderFilter, render: impl FnOnce(&mut Self)) {
        expect_inner!(self.push_filter);

        if self.visible {
            self.display_list.push_stacking_context(blend_mode, filter, &[], &[]);

            render(self);

            self.display_list.pop_stacking_context();
        } else {
            render(self);
        }
    }

    /// Calls `render` with added opacity stacking context.
    pub fn push_opacity(&mut self, bind: FrameValue<f32>, render: impl FnOnce(&mut Self)) {
        expect_inner!(self.push_opacity);

        if self.visible {
            self.display_list
                .push_stacking_context(MixBlendMode::Normal, &[FilterOp::Opacity(bind)], &[], &[]);

            render(self);

            self.display_list.pop_stacking_context();
        } else {
            render(self);
        }
    }

    /// Push a border.
    pub fn push_border(&mut self, bounds: PxRect, widths: PxSideOffsets, sides: BorderSides, radius: PxCornerRadius) {
        expect_inner!(self.push_border);

        if self.visible {
            self.display_list.push_border(
                bounds,
                widths,
                sides.top.into(),
                sides.right.into(),
                sides.bottom.into(),
                sides.bottom.into(),
                radius,
            );
        }

        if self.auto_hit_test {
            self.hit_test().push_border(bounds, widths, radius);
        }
    }

    /// Push a text run.
    pub fn push_text(
        &mut self,
        clip_rect: PxRect,
        glyphs: &[GlyphInstance],
        font: &impl Font,
        color: FrameValue<RenderColor>,
        synthesis: FontSynthesis,
        aa: FontAntiAliasing,
    ) {
        expect_inner!(self.push_text);

        if let Some(r) = &self.renderer {
            if !glyphs.is_empty() && self.visible && !font.is_empty_fallback() {
                let (instance_key, flags) = font.instance_key(r, synthesis);

                let opts = GlyphOptions {
                    render_mode: match aa {
                        FontAntiAliasing::Default => self.default_font_aa,
                        FontAntiAliasing::Subpixel => FontRenderMode::Subpixel,
                        FontAntiAliasing::Alpha => FontRenderMode::Alpha,
                        FontAntiAliasing::Mono => FontRenderMode::Mono,
                    },
                    flags,
                };
                self.display_list.push_text(clip_rect, instance_key, glyphs, color, opts);
            }
        }

        if self.auto_hit_test {
            self.hit_test().push_rect(clip_rect);
        }
    }

    /// Push an image.
    pub fn push_image(&mut self, clip_rect: PxRect, img_size: PxSize, image: &impl Img, rendering: ImageRendering) {
        expect_inner!(self.push_image);

        if let Some(r) = &self.renderer {
            if self.visible {
                let image_key = image.image_key(r);
                self.display_list
                    .push_image(clip_rect, image_key, img_size, rendering.into(), image.alpha_type());
            }
        }

        if self.auto_hit_test {
            self.hit_test().push_rect(clip_rect);
        }
    }

    /// Push a color rectangle.
    ///
    /// The `color` can be bound and updated using [`FrameUpdate::update_color`], note that if the color binding or update
    /// is flagged as `animating` webrender frame updates are used when color updates are send, but webrender disables some
    /// caching for the entire `clip_rect` region, this can have a big performance impact in [`RenderMode::Software`] if a large
    /// part of the screen is affected, as the entire region is redraw every full frame even if the color did not actually change.
    pub fn push_color(&mut self, clip_rect: PxRect, color: FrameValue<RenderColor>) {
        expect_inner!(self.push_color);

        if self.visible {
            self.display_list.push_color(clip_rect, color);
        }

        if self.auto_hit_test {
            self.hit_test().push_rect(clip_rect);
        }
    }

    /// Push a repeating linear gradient rectangle.
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    #[allow(clippy::too_many_arguments)]
    pub fn push_linear_gradient(
        &mut self,
        clip_rect: PxRect,
        line: PxLine,
        stops: &[RenderGradientStop],
        extend_mode: RenderExtendMode,
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        debug_assert!(stops.len() >= 2);
        debug_assert!(stops[0].offset.abs() < 0.00001, "first color stop must be at offset 0.0");
        debug_assert!(
            (stops[stops.len() - 1].offset - 1.0).abs() < 0.00001,
            "last color stop must be at offset 1.0"
        );

        expect_inner!(self.push_linear_gradient);

        if !stops.is_empty() && self.visible {
            self.display_list.push_linear_gradient(
                clip_rect,
                webrender_api::Gradient {
                    start_point: line.start.to_wr(),
                    end_point: line.end.to_wr(),
                    extend_mode,
                },
                stops,
                tile_size,
                tile_spacing,
            );
        }

        if self.auto_hit_test {
            self.hit_test().push_rect(clip_rect);
        }
    }

    /// Push a repeating radial gradient rectangle.
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The  `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The `center` point is relative to the top-left of the tile, the `radius` is the distance between the first
    /// and last color stop in both directions and must be a non-zero positive value.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    #[allow(clippy::too_many_arguments)]
    pub fn push_radial_gradient(
        &mut self,
        clip_rect: PxRect,
        center: PxPoint,
        radius: PxSize,
        stops: &[RenderGradientStop],
        extend_mode: RenderExtendMode,
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        debug_assert!(stops.len() >= 2);
        debug_assert!(stops[0].offset.abs() < 0.00001, "first color stop must be at offset 0.0");
        debug_assert!(
            (stops[stops.len() - 1].offset - 1.0).abs() < 0.00001,
            "last color stop must be at offset 1.0"
        );

        expect_inner!(self.push_radial_gradient);

        if !stops.is_empty() && self.visible {
            self.display_list.push_radial_gradient(
                clip_rect,
                webrender_api::RadialGradient {
                    center: center.to_wr(),
                    radius: radius.to_wr(),
                    start_offset: 0.0,
                    end_offset: 1.0,
                    extend_mode,
                },
                stops,
                tile_size,
                tile_spacing,
            );
        }

        if self.auto_hit_test {
            self.hit_test().push_rect(clip_rect);
        }
    }

    /// Push a repeating conic gradient rectangle.
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The  `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    #[allow(clippy::too_many_arguments)]
    pub fn push_conic_gradient(
        &mut self,
        clip_rect: PxRect,
        center: PxPoint,
        angle: AngleRadian,
        stops: &[RenderGradientStop],
        extend_mode: RenderExtendMode,
        tile_size: PxSize,
        tile_spacing: PxSize,
    ) {
        debug_assert!(stops.len() >= 2);
        debug_assert!(stops[0].offset.abs() < 0.00001, "first color stop must be at offset 0.0");
        debug_assert!(
            (stops[stops.len() - 1].offset - 1.0).abs() < 0.00001,
            "last color stop must be at offset 1.0"
        );

        expect_inner!(self.push_conic_gradient);

        if !stops.is_empty() && self.visible {
            self.display_list.push_conic_gradient(
                clip_rect,
                webrender_api::ConicGradient {
                    center: center.to_wr(),
                    angle: angle.0,
                    start_offset: 0.0,
                    end_offset: 1.0,
                    extend_mode,
                },
                stops,
                tile_size,
                tile_spacing,
            );
        }

        if self.auto_hit_test {
            self.hit_test().push_rect(clip_rect);
        }
    }

    /// Push a styled vertical or horizontal line.
    pub fn push_line(
        &mut self,
        clip_rect: PxRect,
        orientation: crate::border::LineOrientation,
        color: RenderColor,
        style: crate::border::LineStyle,
    ) {
        expect_inner!(self.push_line);

        if self.visible {
            match style.render_command() {
                RenderLineCommand::Line(style, wavy_thickness) => {
                    self.display_list
                        .push_line(clip_rect, color, style, wavy_thickness, orientation.into());
                }
                RenderLineCommand::Border(style) => {
                    use crate::border::LineOrientation as LO;
                    let widths = match orientation {
                        LO::Vertical => PxSideOffsets::new(Px(0), Px(0), Px(0), clip_rect.width()),
                        LO::Horizontal => PxSideOffsets::new(clip_rect.height(), Px(0), Px(0), Px(0)),
                    };
                    self.display_list.push_border(
                        clip_rect,
                        widths,
                        webrender_api::BorderSide { color, style },
                        webrender_api::BorderSide {
                            color: RenderColor::TRANSPARENT,
                            style: webrender_api::BorderStyle::Hidden,
                        },
                        webrender_api::BorderSide {
                            color: RenderColor::TRANSPARENT,
                            style: webrender_api::BorderStyle::Hidden,
                        },
                        webrender_api::BorderSide { color, style },
                        PxCornerRadius::zero(),
                    );
                }
            }
        }

        if self.auto_hit_test {
            self.hit_test().push_rect(clip_rect);
        }
    }

    /// Push a `color` dot to mark the `offset`.
    ///
    /// The *dot* is a circle of the `color` highlighted by an white outline and shadow.
    pub fn push_debug_dot(&mut self, offset: PxPoint, color: impl Into<RenderColor>) {
        if !self.visible {
            return;
        }
        let scale = self.scale_factor();

        let radius = PxSize::splat(Px(6)) * scale;
        let color = color.into();

        let mut builder = webrender_api::GradientBuilder::new();
        builder.push(RenderGradientStop { offset: 0.0, color });
        builder.push(RenderGradientStop { offset: 0.5, color });
        builder.push(RenderGradientStop {
            offset: 0.6,
            color: RenderColor::WHITE,
        });
        builder.push(RenderGradientStop {
            offset: 0.7,
            color: RenderColor::WHITE,
        });
        builder.push(RenderGradientStop {
            offset: 0.8,
            color: RenderColor::BLACK,
        });
        builder.push(RenderGradientStop {
            offset: 1.0,
            color: RenderColor::TRANSPARENT,
        });

        let center = radius.to_vector().to_point();
        let gradient = builder.radial_gradient(center.to_wr(), radius.to_wr(), RenderExtendMode::Clamp);
        let stops = builder.into_stops();

        let bounds = radius * 2.0.fct();

        let offset = offset - radius.to_vector();

        self.display_list
            .push_radial_gradient(PxRect::new(offset, bounds), gradient, &stops, bounds, PxSize::zero());
    }

    /// Finalizes the build.
    pub fn finalize(self, info_tree: &WidgetInfoTree) -> BuiltFrame {
        info_tree.root().bounds_info().set_rendered(
            Some(WidgetRenderInfo {
                visible: self.visible,
                back: ZIndex(0),
                front: self.render_index,
            }),
            info_tree,
        );
        info_tree.after_render(self.frame_id, self.scale_factor);

        let display_list = self.display_list.finalize();

        let clear_color = self.clear_color;

        BuiltFrame { display_list, clear_color }
    }
}

/// Builder for a chain of render and hit-test clips.
///
/// The builder is available in [`FrameBuilder::push_clips`].
pub struct ClipBuilder<'a> {
    builder: &'a mut FrameBuilder,
    render_count: usize,
    hit_test_count: usize,
}
impl<'a> ClipBuilder<'a> {
    /// Pushes the `clip_rect`.
    ///
    /// If `clip_out` is `true` only pixels outside the rect are visible. If `hit_test` is `true` the hit-test shapes
    /// rendered inside `render` are also clipped.
    ///
    /// Note that [`auto_hit_test`] overwrites `hit_test` if it is `true`.
    ///
    /// [`auto_hit_test`]: FrameBuilder::auto_hit_test
    pub fn push_clip_rect(&mut self, clip_rect: PxRect, clip_out: bool, hit_test: bool) {
        if self.builder.visible {
            self.builder.display_list.push_clip_rect(clip_rect, clip_out);
            self.render_count += 1;
        }

        if hit_test || self.builder.auto_hit_test {
            self.builder.hit_clips.push_clip_rect(clip_rect.to_box2d(), clip_out);
            self.hit_test_count += 1;
        }
    }

    /// Push the `clip_rect` with rounded `corners`.
    ///
    /// If `clip_out` is `true` only pixels outside the rounded rect are visible. If `hit_test` is `true` the hit-test shapes
    /// rendered inside `render` are also clipped.
    ///
    /// Note that [`auto_hit_test`] overwrites `hit_test` if it is `true`.
    ///
    /// [`auto_hit_test`]: FrameBuilder::auto_hit_test
    pub fn push_clip_rounded_rect(&mut self, clip_rect: PxRect, corners: PxCornerRadius, clip_out: bool, hit_test: bool) {
        if self.builder.visible {
            self.builder.display_list.push_clip_rounded_rect(clip_rect, corners, clip_out);
            self.render_count += 1;
        }

        if hit_test || self.builder.auto_hit_test {
            self.builder
                .hit_clips
                .push_clip_rounded_rect(clip_rect.to_box2d(), corners, clip_out);
            self.hit_test_count += 1;
        }
    }
}

/// Builder for a chain of hit-test clips.
///
/// The build is available in [`HitTestBuilder::push_clips`].
pub struct HitTestClipBuilder<'a> {
    hit_clips: &'a mut HitTestClips,
    count: usize,
}
impl<'a> HitTestClipBuilder<'a> {
    /// Push a clip `rect`.
    ///
    /// If `clip_out` is `true` only hits outside the rect are valid.
    pub fn push_clip_rect(&mut self, rect: PxRect, clip_out: bool) {
        self.hit_clips.push_clip_rect(rect.to_box2d(), clip_out);
        self.count += 1;
    }

    /// Push a clip `rect` with rounded `corners`.
    ///
    /// If `clip_out` is `true` only hits outside the rect are valid.
    pub fn push_clip_rounded_rect(&mut self, rect: PxRect, corners: PxCornerRadius, clip_out: bool) {
        self.hit_clips.push_clip_rounded_rect(rect.to_box2d(), corners, clip_out);
        self.count += 1;
    }

    /// Push a clip ellipse.
    ///
    /// If `clip_out` is `true` only hits outside the ellipses are valid.
    pub fn push_clip_ellipse(&mut self, center: PxPoint, radii: PxSize, clip_out: bool) {
        self.hit_clips.push_clip_ellipse(center, radii, clip_out);
        self.count += 1;
    }
}

/// Builder for the hit-testable shape of the inner-bounds of a widget.
///
/// This builder is available in [`FrameBuilder::hit_test`] inside the inner-bounds of the rendering widget.
pub struct HitTestBuilder<'a> {
    hit_clips: &'a mut HitTestClips,
    is_hit_testable: bool,
}
impl<'a> HitTestBuilder<'a> {
    /// If the widget is hit-testable, if this is `false` all hit-test push methods are ignored.
    pub fn is_hit_testable(&self) -> bool {
        self.is_hit_testable
    }

    /// Push a hit-test `rect`.
    pub fn push_rect(&mut self, rect: PxRect) {
        if self.is_hit_testable && rect.size != PxSize::zero() {
            self.hit_clips.push_rect(rect.to_box2d());
        }
    }

    /// Push a hit-test `rect` with rounded `corners`.
    pub fn push_rounded_rect(&mut self, rect: PxRect, corners: PxCornerRadius) {
        if self.is_hit_testable && rect.size != PxSize::zero() {
            self.hit_clips.push_rounded_rect(rect.to_box2d(), corners);
        }
    }

    /// Push a hit-test ellipse.
    pub fn push_ellipse(&mut self, center: PxPoint, radii: PxSize) {
        if self.is_hit_testable && radii != PxSize::zero() {
            self.hit_clips.push_ellipse(center, radii);
        }
    }

    /// Push a clip `rect` that affects the `inner_hit_test`.
    pub fn push_clip_rect(&mut self, rect: PxRect, clip_out: bool, inner_hit_test: impl FnOnce(&mut Self)) {
        if !self.is_hit_testable {
            return;
        }

        self.hit_clips.push_clip_rect(rect.to_box2d(), clip_out);

        inner_hit_test(self);

        self.hit_clips.pop_clip();
    }

    /// Push a clip `rect` with rounded `corners` that affects the `inner_hit_test`.
    pub fn push_clip_rounded_rect(
        &mut self,
        rect: PxRect,
        corners: PxCornerRadius,
        clip_out: bool,
        inner_hit_test: impl FnOnce(&mut Self),
    ) {
        self.push_clips(move |c| c.push_clip_rounded_rect(rect, corners, clip_out), inner_hit_test);
    }

    /// Push a clip ellipse that affects the `inner_hit_test`.
    pub fn push_clip_ellipse(&mut self, center: PxPoint, radii: PxSize, clip_out: bool, inner_hit_test: impl FnOnce(&mut Self)) {
        self.push_clips(move |c| c.push_clip_ellipse(center, radii, clip_out), inner_hit_test);
    }

    /// Push clips that affect the `inner_hit_test`.
    pub fn push_clips(&mut self, clips: impl FnOnce(&mut HitTestClipBuilder), inner_hit_test: impl FnOnce(&mut Self)) {
        if !self.is_hit_testable {
            return;
        }

        let mut count = {
            let mut builder = HitTestClipBuilder {
                hit_clips: &mut *self.hit_clips,
                count: 0,
            };
            clips(&mut builder);
            builder.count
        };

        inner_hit_test(self);

        while count > 0 {
            count -= 1;
            self.hit_clips.pop_clip();
        }
    }

    /// Pushes a transform that affects the `inner_hit_test`.
    pub fn push_transform(&mut self, transform: PxTransform, inner_hit_test: impl FnOnce(&mut Self)) {
        if !self.is_hit_testable {
            return;
        }

        self.hit_clips.push_transform(FrameValue::Value(transform));

        inner_hit_test(self);

        self.hit_clips.pop_transform();
    }

    /// Pushes a composite hit-test that defines a border.
    pub fn push_border(&mut self, bounds: PxRect, widths: PxSideOffsets, corners: PxCornerRadius) {
        if !self.is_hit_testable {
            return;
        }

        let bounds = bounds.to_box2d();
        let mut inner_bounds = bounds;
        inner_bounds.min.x += widths.left;
        inner_bounds.min.y += widths.top;
        inner_bounds.max.x -= widths.right;
        inner_bounds.max.y -= widths.bottom;

        if inner_bounds.is_negative() {
            self.hit_clips.push_rounded_rect(bounds, corners);
        } else if corners == PxCornerRadius::zero() {
            self.hit_clips.push_clip_rect(inner_bounds, true);
            self.hit_clips.push_rect(bounds);
            self.hit_clips.pop_clip();
        } else {
            let inner_radii = corners.deflate(widths);

            self.hit_clips.push_clip_rounded_rect(inner_bounds, inner_radii, true);
            self.hit_clips.push_rounded_rect(bounds, corners);
            self.hit_clips.pop_clip();
        }
    }
}

/// Output of a [`FrameBuilder`].
pub struct BuiltFrame {
    /// Built display list.
    pub display_list: DisplayList,
    /// Clear color selected for the frame.
    pub clear_color: RenderColor,
}

enum RenderLineCommand {
    Line(webrender_api::LineStyle, f32),
    Border(webrender_api::BorderStyle),
}
impl crate::border::LineStyle {
    fn render_command(self) -> RenderLineCommand {
        use crate::border::LineStyle as LS;
        use RenderLineCommand::*;
        match self {
            LS::Solid => Line(webrender_api::LineStyle::Solid, 0.0),
            LS::Double => Border(webrender_api::BorderStyle::Double),
            LS::Dotted => Line(webrender_api::LineStyle::Dotted, 0.0),
            LS::Dashed => Line(webrender_api::LineStyle::Dashed, 0.0),
            LS::Groove => Border(webrender_api::BorderStyle::Groove),
            LS::Ridge => Border(webrender_api::BorderStyle::Ridge),
            LS::Wavy(thickness) => Line(webrender_api::LineStyle::Wavy, thickness),
            LS::Hidden => Border(webrender_api::BorderStyle::Hidden),
        }
    }
}

/// Represents a frame or update builder split from the main builder that must be folded back onto the
/// main builder after it is filled in a parallel task.
///
/// # Error
///
/// Traces an error on drop if it was not moved to the `B::parallel_fold` method.
#[must_use = "use in parallel task, then move it to `B::parallel_fold`"]
pub struct ParallelBuilder<B>(Option<B>);
impl<B> ParallelBuilder<B> {
    fn take(&mut self) -> B {
        self.0.take().expect("parallel builder finished")
    }
}
impl<B> ops::Deref for ParallelBuilder<B> {
    type Target = B;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("parallel builder finished")
    }
}
impl<B> ops::DerefMut for ParallelBuilder<B> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("parallel builder finished")
    }
}
impl<B> Drop for ParallelBuilder<B> {
    fn drop(&mut self) {
        if self.0.is_some() {
            tracing::error!("builder dropped without calling `{}::parallel_fold`", std::any::type_name::<B>())
        }
    }
}

/// A frame quick update.
///
/// A frame update causes a frame render without needing to fully rebuild the display list. It
/// is a more performant but also more limited way of generating a frame.
///
/// Any [`FrameValueKey`] used in the creation of the frame can be used for updating the frame.
pub struct FrameUpdate {
    pipeline_id: PipelineId,

    transforms: Vec<FrameValueUpdate<PxTransform>>,
    floats: Vec<FrameValueUpdate<f32>>,
    colors: Vec<FrameValueUpdate<RenderColor>>,

    current_clear_color: RenderColor,
    clear_color: Option<RenderColor>,
    frame_id: FrameId,

    widget_id: WidgetId,
    transform: PxTransform,
    outer_offset: PxVector,
    inner_transform: Option<PxTransform>,
    child_offset: PxVector,
    can_reuse_widget: bool,
    widget_bounds: WidgetBoundsInfo,
    parent_inner_bounds: Option<PxRect>,

    auto_hit_test: bool,
    visible: bool,
}
impl FrameUpdate {
    /// New frame update builder.
    ///
    /// * `frame_id` - Id of the frame that will be updated.
    /// * `root_id` - Id of the window root widget.
    /// * `renderer` - Reference to the renderer that will update.
    /// * `clear_color` - The current clear color.
    pub fn new(
        frame_id: FrameId,
        root_id: WidgetId,
        root_bounds: WidgetBoundsInfo,
        renderer: Option<&ViewRenderer>,
        clear_color: RenderColor,
    ) -> Self {
        // in case they add more dynamic property types.
        assert_size_of!(DynamicProperties, 72);

        let pipeline_id = renderer
            .as_ref()
            .and_then(|r| r.pipeline_id().ok())
            .unwrap_or_else(PipelineId::dummy);

        FrameUpdate {
            pipeline_id,
            widget_id: root_id,
            widget_bounds: root_bounds,
            transforms: vec![],
            floats: vec![],
            colors: vec![],
            clear_color: None,
            frame_id,
            current_clear_color: clear_color,

            transform: PxTransform::identity(),
            outer_offset: PxVector::zero(),
            inner_transform: Some(PxTransform::identity()),
            child_offset: PxVector::zero(),
            can_reuse_widget: true,

            auto_hit_test: false,
            parent_inner_bounds: None,
            visible: true,
        }
    }

    /// The frame that will be updated.
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Returns `true` if the widget inner transform update is still being build.
    ///
    /// This is `true` when inside an [`update_widget`] call but `false` when inside an [`update_inner`] call.
    ///
    /// [`update_widget`]: Self::update_widget
    /// [`update_inner`]: Self::update_inner
    pub fn is_outer(&self) -> bool {
        self.inner_transform.is_some()
    }

    /// Current transform.
    pub fn transform(&self) -> &PxTransform {
        &self.transform
    }

    /// Change the color used to clear the pixel buffer when redrawing the frame.
    pub fn set_clear_color(&mut self, color: RenderColor) {
        if self.visible {
            self.clear_color = Some(color);
        }
    }

    /// Returns `true` if all transform updates are also applied to hit-test transforms.
    pub fn auto_hit_test(&self) -> bool {
        self.auto_hit_test
    }
    /// Runs `render_update` with [`auto_hit_test`] set to a value for the duration of the `render` call.
    ///
    /// [`auto_hit_test`]: Self::auto_hit_test
    pub fn with_auto_hit_test(&mut self, auto_hit_test: bool, render_update: impl FnOnce(&mut Self)) {
        let prev = mem::replace(&mut self.auto_hit_test, auto_hit_test);
        render_update(self);
        self.auto_hit_test = prev;
    }

    /// Returns `true` if view updates are actually collected, if `false` only transforms and hit-test are updated.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Calls `update` with [`is_visible`] set to `false`.
    ///
    /// Nodes that set the visibility to [`Hidden`] must render using the [`FrameBuilder::hide`] method and update using this method.
    ///
    /// [`is_visible`]: Self::is_visible
    /// [`Hidden`]: crate::widget_info::Visibility::Hidden
    pub fn hidden(&mut self, update: impl FnOnce(&mut Self)) {
        let parent_visible = mem::replace(&mut self.visible, false);
        update(self);
        self.visible = parent_visible;
    }

    /// Update a transform value that does not potentially affect widget bounds.
    ///
    /// Use [`with_transform`] to update transforms that affect widget bounds.
    ///
    /// If `hit_test` is `true` the hit-test transform is also updated.
    ///
    /// [`with_transform`]: Self::with_transform
    pub fn update_transform(&mut self, new_value: FrameValueUpdate<PxTransform>, hit_test: bool) {
        if self.visible {
            self.transforms.push(new_value);
        }

        if hit_test || self.auto_hit_test {
            self.widget_bounds.update_hit_test_transform(new_value);
        }
    }

    /// Update a transform value, if there is one.
    pub fn update_transform_opt(&mut self, new_value: Option<FrameValueUpdate<PxTransform>>, hit_test: bool) {
        if let Some(value) = new_value {
            self.update_transform(value, hit_test)
        }
    }

    /// Update a transform that potentially affects widget bounds.
    ///
    /// The [`transform`] is updated to include this space for the call to the `render_update` closure. The closure
    /// must call render update on child nodes.
    ///
    /// If `hit_test` is `true` the hit-test transform is also updated.
    ///
    /// [`transform`]: Self::transform
    pub fn with_transform(&mut self, new_value: FrameValueUpdate<PxTransform>, hit_test: bool, render_update: impl FnOnce(&mut Self)) {
        self.with_transform_value(&new_value.value, render_update);
        self.update_transform(new_value, hit_test);
    }

    /// Update a transform that potentially affects widget bounds, if there is one.
    ///
    /// The `render_update` is always called.
    pub fn with_transform_opt(
        &mut self,
        new_value: Option<FrameValueUpdate<PxTransform>>,
        hit_test: bool,
        render_update: impl FnOnce(&mut Self),
    ) {
        match new_value {
            Some(value) => self.with_transform(value, hit_test, render_update),
            None => render_update(self),
        }
    }

    /// Calls `render_update` with an `offset` that affects the first inner child inner bounds.
    ///
    /// Nodes that used [`FrameBuilder::push_child`] during render must use this method to update the value.
    pub fn with_child(&mut self, offset: PxVector, render_update: impl FnOnce(&mut Self)) {
        self.child_offset = offset;
        render_update(self);
        self.child_offset = PxVector::zero();
    }

    /// Calls `render_update` while the [`transform`] is updated to include the `value` space.
    ///
    /// This is useful for cases where the inner transforms are affected by a `value` that is only rendered, never updated.
    ///
    /// [`transform`]: Self::transform
    pub fn with_transform_value(&mut self, value: &PxTransform, render_update: impl FnOnce(&mut Self)) {
        let parent_transform = self.transform;
        self.transform = value.then(&parent_transform);

        render_update(self);
        self.transform = parent_transform;
    }

    /// Update the transform applied after the inner bounds translate.
    ///
    /// This is only valid if [`is_outer`].
    ///
    /// [`is_outer`]: Self::is_outer
    pub fn with_inner_transform(&mut self, transform: &PxTransform, render_update: impl FnOnce(&mut Self)) {
        if let Some(inner_transform) = &mut self.inner_transform {
            let parent = *inner_transform;
            *inner_transform = inner_transform.then(transform);

            render_update(self);

            if let Some(inner_transform) = &mut self.inner_transform {
                *inner_transform = parent;
            }
        } else {
            tracing::error!("called `with_inner_transform` inside inner context of `{}`", self.widget_id);
            render_update(self);
        }
    }

    /// If widget update can be *skipped* by setting reuse in [`update_widget`].
    ///
    /// [`update_widget`]: Self::update_widget
    pub fn can_reuse_widget(&self) -> bool {
        self.can_reuse_widget
    }

    /// Calls `render_update` with [`can_reuse_widget`] set to `false`.
    ///
    /// [`can_reuse_widget`]: Self::can_reuse_widget
    pub fn with_no_reuse(&mut self, render_update: impl FnOnce(&mut Self)) {
        let prev_can_reuse = self.can_reuse_widget;
        self.can_reuse_widget = false;
        render_update(self);
        self.can_reuse_widget = prev_can_reuse;
    }

    /// Update the widget's outer transform.
    ///
    /// If the widget did not request render-update you can set `reuse` to try and only update outer/inner transforms of descendants.
    /// If the widget is reused the `render_update` is not called, the `reuse` flag can be ignored if [`can_reuse_widget`] does not allow
    /// it or if the previous transform is not invertible.
    ///
    /// [`can_reuse_widget`]: Self::can_reuse_widget
    pub fn update_widget(&mut self, reuse: bool, render_update: impl FnOnce(&mut Self)) {
        let id = WIDGET.id();

        if self.inner_transform.is_some() {
            tracing::error!(
                "called `update_widget` for `{}` without calling `update_inner` for the parent `{}`",
                id,
                self.widget_id
            );
        }

        let bounds = WIDGET.bounds();
        let tree = WINDOW.widget_tree();

        let outer_transform = PxTransform::from(self.child_offset).then(&self.transform);

        let parent_can_reuse = self.can_reuse_widget;
        let parent_bounds = mem::replace(&mut self.widget_bounds, bounds.clone());

        if self.can_reuse_widget && reuse {
            let _span = tracing::trace_span!("reuse-descendants", id=?self.widget_id).entered();

            let prev_outer = bounds.outer_transform();
            if prev_outer != outer_transform {
                if let Some(undo_prev) = prev_outer.inverse() {
                    let patch = undo_prev.then(&outer_transform);

                    for info in tree.get(id).unwrap().self_and_descendants() {
                        let bounds = info.bounds_info();
                        bounds.set_outer_transform(bounds.outer_transform().then(&patch), &tree);
                        bounds.set_inner_transform(
                            bounds.inner_transform().then(&patch),
                            &tree,
                            info.id(),
                            info.parent().map(|p| p.inner_bounds()),
                        );
                    }

                    return; // can reuse and patched.
                }
            } else {
                return; // can reuse and no change.
            }

            // actually cannot reuse because cannot undo prev-transform.
            self.can_reuse_widget = false;
        }

        bounds.set_outer_transform(outer_transform, &tree);
        self.outer_offset = mem::take(&mut self.child_offset);
        self.inner_transform = Some(PxTransform::identity());
        let parent_id = self.widget_id;
        self.widget_id = id;

        render_update(self);

        self.outer_offset = PxVector::zero();
        self.inner_transform = None;
        self.widget_id = parent_id;
        self.can_reuse_widget = parent_can_reuse;
        self.widget_bounds = parent_bounds;
    }

    /// Update the info transforms of the widget and descendants.
    ///
    /// Widgets that did not request render-update can use this method to update only the outer and inner transforms
    /// of itself and descendants as those values are global and the parent widget may have changed.
    pub fn reuse_widget(&mut self) {
        if self.inner_transform.is_some() {
            tracing::error!(
                "called `reuse_widget` for `{}` without calling `update_inner` for the parent `{}`",
                WIDGET.id(),
                self.widget_id
            );
        }
    }

    /// Update the widget's inner transform.
    ///
    /// The `layout_translation_animating` affects some webrender caches, see [`FrameBuilder::push_inner`] for details.
    pub fn update_inner(
        &mut self,
        layout_translation_key: FrameValueKey<PxTransform>,
        layout_translation_animating: bool,
        render_update: impl FnOnce(&mut Self),
    ) {
        let id = WIDGET.id();
        if let Some(inner_transform) = self.inner_transform.take() {
            let bounds = WIDGET.bounds();
            let tree = WINDOW.widget_tree();

            let inner_offset = bounds.inner_offset();
            let inner_transform = inner_transform.then_translate((self.outer_offset + inner_offset).cast());
            self.update_transform(layout_translation_key.update(inner_transform, layout_translation_animating), false);
            let parent_transform = self.transform;

            self.transform = inner_transform.then(&parent_transform);

            bounds.set_inner_transform(self.transform, &tree, id, self.parent_inner_bounds);
            let parent_inner_bounds = mem::replace(&mut self.parent_inner_bounds, Some(bounds.inner_bounds()));

            render_update(self);

            self.transform = parent_transform;
            self.parent_inner_bounds = parent_inner_bounds;
        } else {
            tracing::error!("called `update_inner` more then once for `{}`", id);
            render_update(self)
        }
    }

    /// Update a float value.
    pub fn update_f32(&mut self, new_value: FrameValueUpdate<f32>) {
        if self.visible {
            self.floats.push(new_value);
        }
    }

    /// Update a float value, if there is one.
    pub fn update_f32_opt(&mut self, new_value: Option<FrameValueUpdate<f32>>) {
        if let Some(value) = new_value {
            self.update_f32(value)
        }
    }

    /// Update a color value.
    ///
    /// See [`FrameBuilder::push_color`] for details.
    pub fn update_color(&mut self, new_value: FrameValueUpdate<RenderColor>) {
        if self.visible {
            self.colors.push(new_value)
        }
    }

    /// Update a color value, if there is one.
    pub fn update_color_opt(&mut self, new_value: Option<FrameValueUpdate<RenderColor>>) {
        if let Some(value) = new_value {
            self.update_color(value)
        }
    }

    /// Create a leaf update builder that can be send to a parallel task and must be folded back into this builder.
    ///
    /// This should be called just before the call to [`update_widget`], an error is traced if called inside an widget outer bounds.
    ///
    /// [`update_widget`]: Self::update_widget
    pub fn parallel_split(&self) -> ParallelBuilder<Self> {
        if self.inner_transform.is_some() {
            tracing::error!(
                "called `parallel_split` inside `{}` and before calling `update_inner`",
                self.widget_id
            );
        }

        ParallelBuilder(Some(Self {
            pipeline_id: self.pipeline_id,
            current_clear_color: self.current_clear_color,
            frame_id: self.frame_id,

            transforms: vec![],
            floats: vec![],
            colors: vec![],
            clear_color: None,

            widget_id: self.widget_id,
            transform: self.transform,
            outer_offset: self.outer_offset,
            inner_transform: self.inner_transform,
            child_offset: self.child_offset,
            can_reuse_widget: self.can_reuse_widget,
            widget_bounds: self.widget_bounds.clone(),
            parent_inner_bounds: self.parent_inner_bounds,
            auto_hit_test: self.auto_hit_test,
            visible: self.visible,
        }))
    }

    /// Collect updates from `split` into `self`.
    pub fn parallel_fold(&mut self, mut split: ParallelBuilder<Self>) {
        let mut split = split.take();

        debug_assert_eq!(self.pipeline_id, split.pipeline_id);
        debug_assert_eq!(self.frame_id, split.frame_id);
        debug_assert_eq!(self.widget_id, split.widget_id);

        self.transforms.append(&mut split.transforms);
        self.floats.append(&mut split.floats);
        self.colors.append(&mut split.colors);
        if let Some(c) = self.clear_color.take() {
            self.clear_color = Some(c);
        }
    }

    /// Finalize the update.
    ///
    /// Returns the property updates and the new clear color if any was set.
    pub fn finalize(mut self, info_tree: &WidgetInfoTree) -> BuiltFrameUpdate {
        info_tree.after_render_update(self.frame_id);

        if self.clear_color == Some(self.current_clear_color) {
            self.clear_color = None;
        }

        BuiltFrameUpdate {
            clear_color: self.clear_color,
            transforms: self.transforms,
            floats: self.floats,
            colors: self.colors,
        }
    }
}

/// Output of a [`FrameBuilder`].
pub struct BuiltFrameUpdate {
    /// Bound transforms update.
    pub transforms: Vec<FrameValueUpdate<PxTransform>>,
    /// Bound floats update.
    pub floats: Vec<FrameValueUpdate<f32>>,
    /// Bound colors update.
    pub colors: Vec<FrameValueUpdate<RenderColor>>,
    /// New clear color.
    pub clear_color: Option<RenderColor>,
}

unique_id_32! {
    #[derive(Debug)]
    struct FrameBindingKeyId;
}

unique_id_32! {
    /// Unique ID of a reference frame.
    ///
    /// See [`SpatialFrameKey`] for more details.
    #[derive(Debug)]
    pub struct SpatialFrameId;
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SpatialFrameKeyInner {
    Unique(SpatialFrameId),
    UniqueIndex(SpatialFrameId, u32),
    Widget(WidgetId),
    WidgetIndex(WidgetId, u32),
    FrameValue(FrameValueKey<PxTransform>),
    FrameValueIndex(FrameValueKey<PxTransform>, u32),
}
impl SpatialFrameKeyInner {
    const UNIQUE: u64 = 1 << 63;
    const WIDGET: u64 = 1 << 62;
    const FRAME_VALUE: u64 = 1 << 61;

    fn to_wr(self) -> SpatialTreeItemKey {
        match self {
            SpatialFrameKeyInner::UniqueIndex(id, index) => SpatialTreeItemKey::new(id.get() as u64, index as u64 | Self::UNIQUE),
            SpatialFrameKeyInner::WidgetIndex(id, index) => SpatialTreeItemKey::new(id.get(), index as u64 | Self::WIDGET),
            SpatialFrameKeyInner::FrameValue(key) => {
                SpatialTreeItemKey::new(((key.id.get() as u64) << 32) | u32::MAX as u64, Self::FRAME_VALUE)
            }
            SpatialFrameKeyInner::FrameValueIndex(key, index) => {
                SpatialTreeItemKey::new(((key.id.get() as u64) << 32) | index as u64, Self::FRAME_VALUE)
            }
            SpatialFrameKeyInner::Unique(id) => SpatialTreeItemKey::new(id.get() as u64, (u32::MAX as u64 + 1) | Self::UNIQUE),
            SpatialFrameKeyInner::Widget(id) => SpatialTreeItemKey::new(id.get(), (u32::MAX as u64 + 1) | Self::WIDGET),
        }
    }
}
/// Represents an unique key for a spatial reference frame that is recreated in multiple frames.
///
/// The key can be generated from [`WidgetId`], [`SpatialFrameId`] or [`FrameValueKey<PxTransform>`] all guaranteed
/// to be unique even if the inner value of IDs is the same.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SpatialFrameKey(SpatialFrameKeyInner);
impl SpatialFrameKey {
    /// Key used for the widget inner transform.
    ///
    /// See [`FrameBuilder::push_inner`].
    fn from_widget(widget_id: WidgetId) -> Self {
        Self(SpatialFrameKeyInner::Widget(widget_id))
    }

    /// Key from [`WidgetId`] and [`u32`] index.
    ///
    /// This can be used in nodes that know that they are the only one rendering children nodes.
    pub fn from_widget_child(parent_id: WidgetId, child_index: u32) -> Self {
        Self(SpatialFrameKeyInner::WidgetIndex(parent_id, child_index))
    }

    /// Key from [`SpatialFrameId`].
    pub fn from_unique(id: SpatialFrameId) -> Self {
        Self(SpatialFrameKeyInner::Unique(id))
    }

    /// Key from [`SpatialFrameId`] and [`u32`] index.
    pub fn from_unique_child(id: SpatialFrameId, child_index: u32) -> Self {
        Self(SpatialFrameKeyInner::UniqueIndex(id, child_index))
    }

    /// Key from a [`FrameValueKey<PxTransform>`].
    pub fn from_frame_value(frame_value_key: FrameValueKey<PxTransform>) -> Self {
        Self(SpatialFrameKeyInner::FrameValue(frame_value_key))
    }

    /// Key from a [`FrameValueKey<PxTransform>`] and [`u32`] index.
    pub fn from_frame_value_child(frame_value_key: FrameValueKey<PxTransform>, child_index: u32) -> Self {
        Self(SpatialFrameKeyInner::FrameValueIndex(frame_value_key, child_index))
    }

    /// To webrender key.
    pub fn to_wr(self) -> SpatialTreeItemKey {
        self.0.to_wr()
    }
}
impl From<FrameValueKey<PxTransform>> for SpatialFrameKey {
    fn from(value: FrameValueKey<PxTransform>) -> Self {
        Self::from_frame_value(value)
    }
}
impl From<SpatialFrameId> for SpatialFrameKey {
    fn from(id: SpatialFrameId) -> Self {
        Self::from_unique(id)
    }
}
impl From<(SpatialFrameId, u32)> for SpatialFrameKey {
    fn from((id, index): (SpatialFrameId, u32)) -> Self {
        Self::from_unique_child(id, index)
    }
}
impl From<(WidgetId, u32)> for SpatialFrameKey {
    fn from((id, index): (WidgetId, u32)) -> Self {
        Self::from_widget_child(id, index)
    }
}
impl From<(FrameValueKey<PxTransform>, u32)> for SpatialFrameKey {
    fn from((key, index): (FrameValueKey<PxTransform>, u32)) -> Self {
        Self::from_frame_value_child(key, index)
    }
}

/// Unique key of an updatable value in the view-process frame.
#[derive(Debug)]
pub struct FrameValueKey<T> {
    id: FrameBindingKeyId,
    _type: PhantomData<T>,
}
impl<T> PartialEq for FrameValueKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<T> Eq for FrameValueKey<T> {}
impl<T> Clone for FrameValueKey<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            _type: PhantomData,
        }
    }
}
impl<T> Copy for FrameValueKey<T> {}
impl<T> FrameValueKey<T> {
    /// Generates a new unique ID.
    pub fn new_unique() -> Self {
        FrameValueKey {
            id: FrameBindingKeyId::new_unique(),
            _type: PhantomData,
        }
    }

    /// To view key.
    pub fn to_wr(self) -> zero_ui_view_api::FrameValueKey<T> {
        Self::to_wr_child(self, u32::MAX)
    }

    /// To view key with an extra `index` modifier.
    pub fn to_wr_child(self, child_index: u32) -> zero_ui_view_api::FrameValueKey<T> {
        zero_ui_view_api::FrameValueKey::new(((self.id.get() as u64) << 32) | child_index as u64)
    }

    /// Create a binding with this key.
    ///
    /// The `animating` flag controls if the binding will propagate to webrender, if `true`
    /// webrender frame updates are generated for
    pub fn bind(self, value: T, animating: bool) -> FrameValue<T> {
        self.bind_child(u32::MAX, value, animating)
    }

    /// Like [`bind`] but the key is modified to include the `child_index`.
    ///
    /// [`bind`]: Self::bind
    pub fn bind_child(self, child_index: u32, value: T, animating: bool) -> FrameValue<T> {
        FrameValue::Bind {
            key: self.to_wr_child(child_index),
            value,
            animating,
        }
    }

    /// Create a value update with this key.
    pub fn update(self, value: T, animating: bool) -> FrameValueUpdate<T> {
        self.update_child(u32::MAX, value, animating)
    }

    /// Like [`update`] but the key is modified to include the `child_index`.
    ///
    /// [`update`]: Self::update
    pub fn update_child(self, child_index: u32, value: T, animating: bool) -> FrameValueUpdate<T> {
        FrameValueUpdate {
            key: self.to_wr_child(child_index),
            value,
            animating,
        }
    }

    /// Create a binding with this key and `var`.
    ///
    /// The `map` must produce a copy or clone of the frame value.
    pub fn bind_var<VT: var::VarValue>(self, var: &impl var::Var<VT>, map: impl FnOnce(&VT) -> T) -> FrameValue<T> {
        self.bind_var_child(u32::MAX, var, map)
    }

    /// Like [`bind_var`] but the key is modified to include the `child_index`.
    ///
    /// [`bind_var`]: Self::bind_var
    pub fn bind_var_child<VT: var::VarValue>(self, child_index: u32, var: &impl var::Var<VT>, map: impl FnOnce(&VT) -> T) -> FrameValue<T> {
        if var.capabilities().contains(var::VarCapabilities::NEW) {
            FrameValue::Bind {
                key: self.to_wr_child(child_index),
                value: var.with(map),
                animating: var.is_animating(),
            }
        } else {
            FrameValue::Value(var.with(map))
        }
    }

    /// Create a binding with this key, `var` and already mapped `value`.
    pub fn bind_var_mapped<VT: var::VarValue>(&self, var: &impl var::Var<VT>, value: T) -> FrameValue<T> {
        self.bind_var_mapped_child(u32::MAX, var, value)
    }

    /// Like [`bind_var_mapped`] but the key is modified to include the `child_index`.
    ///
    /// [`bind_var_mapped`]: Self::bind_var_mapped
    pub fn bind_var_mapped_child<VT: var::VarValue>(&self, child_index: u32, var: &impl var::Var<VT>, value: T) -> FrameValue<T> {
        if var.capabilities().contains(var::VarCapabilities::NEW) {
            FrameValue::Bind {
                key: self.to_wr_child(child_index),
                value,
                animating: var.is_animating(),
            }
        } else {
            FrameValue::Value(value)
        }
    }

    /// Create a value update with this key and `var`.
    pub fn update_var<VT: var::VarValue>(self, var: &impl var::Var<VT>, map: impl FnOnce(&VT) -> T) -> Option<FrameValueUpdate<T>> {
        self.update_var_child(u32::MAX, var, map)
    }

    /// Like [`update_var`] but the key is modified to include the `child_index`.
    ///
    /// [`update_var`]: Self::update_var
    pub fn update_var_child<VT: var::VarValue>(
        self,
        child_index: u32,
        var: &impl var::Var<VT>,
        map: impl FnOnce(&VT) -> T,
    ) -> Option<FrameValueUpdate<T>> {
        if var.capabilities().contains(var::VarCapabilities::NEW) {
            Some(FrameValueUpdate {
                key: self.to_wr_child(child_index),
                value: var.with(map),
                animating: var.is_animating(),
            })
        } else {
            None
        }
    }

    /// Create a value update with this key, `var` and already mapped `value`.
    pub fn update_var_mapped<VT: var::VarValue>(self, var: &impl var::Var<VT>, value: T) -> Option<FrameValueUpdate<T>> {
        self.update_var_mapped_child(u32::MAX, var, value)
    }

    /// Like [`update_var_mapped`] but the key is modified to include the `child_index`.
    ///
    /// [`update_var_mapped`]: Self::update_var_mapped
    pub fn update_var_mapped_child<VT: var::VarValue>(
        self,
        child_index: u32,
        var: &impl var::Var<VT>,
        value: T,
    ) -> Option<FrameValueUpdate<T>> {
        if var.capabilities().contains(var::VarCapabilities::NEW) {
            Some(FrameValueUpdate {
                key: self.to_wr_child(child_index),
                value,
                animating: var.is_animating(),
            })
        } else {
            None
        }
    }
}
assert_non_null!(FrameValueKey<RenderColor>);

bitflags! {
    /// Configure if a synthetic font is generated for fonts that do not implement **bold** or *oblique* variants.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct FontSynthesis: u8 {
        /// No synthetic font generated, if font resolution does not find a variant the matches the requested style and weight
        /// the request is ignored and the normal font is returned.
        const DISABLED = 0;
        /// Enable synthetic bold. Font resolution finds the closest bold variant, the difference added using extra stroke.
        const BOLD = 1;
        /// Enable synthetic oblique. If the font resolution does not find an oblique or italic variant a skew transform is applied.
        const STYLE = 2;
        /// Enabled all synthetic font possibilities.
        const ENABLED = Self::BOLD.bits() | Self::STYLE.bits();
    }
}
impl Default for FontSynthesis {
    /// [`FontSynthesis::ENABLED`]
    fn default() -> Self {
        FontSynthesis::ENABLED
    }
}
impl_from_and_into_var! {
    /// Convert to full [`ENABLED`](FontSynthesis::ENABLED) or [`DISABLED`](FontSynthesis::DISABLED).
    fn from(enabled: bool) -> FontSynthesis {
        if enabled { FontSynthesis::ENABLED } else { FontSynthesis::DISABLED }
    }
}

impl_from_and_into_var! {
    fn from(profiler: crate::text::Txt) -> RendererDebug {
        RendererDebug::profiler(profiler)
    }
}
impl var::IntoVar<RendererDebug> for bool {
    type Var = var::LocalVar<RendererDebug>;

    fn into_var(self) -> Self::Var {
        var::LocalVar(self.into())
    }
}
impl<'a> var::IntoVar<RendererDebug> for &'a str {
    type Var = var::LocalVar<RendererDebug>;

    fn into_var(self) -> Self::Var {
        var::LocalVar(self.into())
    }
}
impl var::IntoVar<RendererDebug> for String {
    type Var = var::LocalVar<RendererDebug>;

    fn into_var(self) -> Self::Var {
        var::LocalVar(self.into())
    }
}
impl var::IntoVar<RendererDebug> for webrender_api::DebugFlags {
    type Var = var::LocalVar<RendererDebug>;

    fn into_var(self) -> Self::Var {
        var::LocalVar(self.into())
    }
}
