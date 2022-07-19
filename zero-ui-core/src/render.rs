//! Frame render and metadata API.

use crate::{
    app::view_process::ViewRenderer,
    border::BorderSides,
    color::{RenderColor, RenderFilter},
    context::RenderContext,
    gradient::{RenderExtendMode, RenderGradientStop},
    text::FontAntiAliasing,
    ui_list::ZIndex,
    units::*,
    var::impl_from_and_into_var,
    widget_info::{HitTestClips, WidgetInfoTree},
    WidgetId,
};

use std::{marker::PhantomData, mem};

use webrender_api::{FontRenderMode, PipelineId};
pub use zero_ui_view_api::{webrender_api, DisplayListBuilder, FrameId, RenderMode, ReuseRange};
use zero_ui_view_api::{
    webrender_api::{
        DynamicProperties, FilterOp, GlyphInstance, GlyphOptions, MixBlendMode, PropertyBinding, PropertyBindingKey, PropertyValue,
        SpatialTreeItemKey,
    },
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
/// in the app default extensions. The default image type is [`Image`] that implements this trait.
///
/// [`ImageManager`]: crate::image::ImageManager
/// [`Image`]: crate::image::Image
pub trait Image {
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
/// You can use the [`Image`] type to re-scale an image, image widgets probably can be configured to do this too.
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
    has_transform: bool,
    transform: RenderTransform,
    filter: RenderFilter,
}

/// A full frame builder.
pub struct FrameBuilder {
    frame_id: FrameId,
    pipeline_id: PipelineId,
    widget_id: WidgetId,
    transform: RenderTransform,

    default_font_aa: FontRenderMode,

    renderer: Option<ViewRenderer>,

    scale_factor: Factor,

    display_list: DisplayListBuilder,

    is_hit_testable: bool,
    auto_hit_test: bool,
    hit_clips: HitTestClips,

    widget_data: Option<WidgetData>,
    widget_rendered: bool,

    can_reuse: bool,
    open_reuse: Option<ReuseStart>,

    clear_color: Option<RenderColor>,

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
    /// * `used_data` - Data generated by a previous frame buffer, if set is recycled for a performance boost.
    /// because WebRender does not let us change the initial clear color.
    pub fn new(
        frame_id: FrameId,
        root_id: WidgetId,
        renderer: Option<ViewRenderer>,
        scale_factor: Factor,
        default_font_aa: FontAntiAliasing,
        used_data: Option<UsedFrameBuilder>,
    ) -> Self {
        let pipeline_id = renderer
            .as_ref()
            .and_then(|r| r.pipeline_id().ok())
            .unwrap_or_else(PipelineId::dummy);

        let mut used_dl = None;

        if let Some(u) = used_data {
            if u.pipeline_id() == pipeline_id {
                used_dl = Some(u.display_list);
            }
        }

        let display_list = if let Some(reuse) = used_dl {
            DisplayListBuilder::with_capacity(pipeline_id, frame_id, reuse)
        } else {
            DisplayListBuilder::new(pipeline_id, frame_id)
        };

        FrameBuilder {
            frame_id,
            pipeline_id,
            widget_id: root_id,
            transform: RenderTransform::identity(),
            default_font_aa: match default_font_aa {
                FontAntiAliasing::Default | FontAntiAliasing::Subpixel => FontRenderMode::Subpixel,
                FontAntiAliasing::Alpha => FontRenderMode::Alpha,
                FontAntiAliasing::Mono => FontRenderMode::Mono,
            },
            renderer,
            scale_factor,
            display_list,
            is_hit_testable: true,
            auto_hit_test: false,
            hit_clips: HitTestClips::default(),
            widget_data: Some(WidgetData {
                filter: vec![],
                has_transform: false,
                transform: RenderTransform::identity(),
            }),
            widget_rendered: false,
            can_reuse: true,
            open_reuse: None,

            render_index: ZIndex(0),

            clear_color: None,
        }
    }

    /// [`new`](Self::new) with only the inputs required for renderless mode.
    pub fn new_renderless(
        frame_id: FrameId,
        root_id: WidgetId,
        scale_factor: Factor,
        default_font_aa: FontAntiAliasing,
        hint: Option<UsedFrameBuilder>,
    ) -> Self {
        Self::new(frame_id, root_id, None, scale_factor, default_font_aa, hint)
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
    /// Note the default clear color is white, and it is not retained, a property
    /// that sets the clear color must set it every render.
    ///
    /// Note that the clear color is always *rendered* first before all other layers, if more then
    /// one layer sets the clear color only the value set on the top-most layer is used.
    pub fn set_clear_color(&mut self, color: RenderColor) {
        self.clear_color = Some(color);
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
    pub fn transform(&self) -> &RenderTransform {
        &self.transform
    }

    /// Returns `true` if hit-testing is enabled in the widget context, if `false` methods that push
    /// a hit-test silently skip.
    ///
    /// This can be set to `false` in a context using [`with_hit_tests_disabled`].
    ///
    /// [`with_hit_tests_disabled`]: Self::with_hit_tests_disabled
    pub fn is_hit_testable(&self) -> bool {
        self.is_hit_testable
    }

    /// Returns `true` if hit-tests are automatically pushed by `push_*` methods.
    ///
    /// Note that hit-tests are only added if [`is_hit_testable`] is `true`.
    ///
    /// [`is_hit_testable`]: Self::is_hit_testable
    pub fn auto_hit_test(&self) -> bool {
        self.auto_hit_test
    }

    /// Runs `render` while hit-tests are disabled, inside `render` [`is_hit_testable`] is `false`, after
    /// it is the current value.
    ///
    /// [`is_hit_testable`]: Self::is_hit_testable
    pub fn with_hit_tests_disabled(&mut self, render: impl FnOnce(&mut Self)) {
        let prev = mem::replace(&mut self.is_hit_testable, false);
        render(self);
        self.is_hit_testable = prev;
    }

    /// Runs `render` while [`auto_hit_test`] is set to a value.
    ///
    /// [`auto_hit_test`]: Self::auto_hit_test
    pub fn with_auto_hit_test(&mut self, auto_hit_test: bool, render: impl FnOnce(&mut Self)) {
        let prev = mem::replace(&mut self.auto_hit_test, auto_hit_test);
        render(self);
        self.auto_hit_test = prev;
    }

    /// Start a new widget outer context, this sets [`is_outer`] to `true` until an inner call to [`push_inner`],
    /// during this period properties can configure the widget stacking context and actual rendering and transforms
    /// are discouraged.
    ///
    /// If `reuse` is `Some` and the widget has been rendered before  and [`can_reuse`] allows reuse, the `render`
    /// closure is not called, an only a reference to the widget range in the previous frame is send.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    /// [`can_reuse`]: Self::can_reuse
    pub fn push_widget(
        &mut self,
        ctx: &mut RenderContext,
        reuse: &mut Option<ReuseRange>,
        render: impl FnOnce(&mut RenderContext, &mut Self),
    ) {
        if self.widget_data.is_some() {
            tracing::error!(
                "called `push_widget` for `{}` without calling `push_inner` for the parent `{}`",
                ctx.path.widget_id(),
                self.widget_id
            );
        }

        let parent_rendered = mem::take(&mut self.widget_rendered);
        // cannot reuse if the widget was not rendered in the previous frame (clear stale reuse ranges in descendants).
        let parent_can_reuse = mem::replace(&mut self.can_reuse, ctx.widget_info.bounds.rendered().is_some());

        let widget_z = self.render_index;
        self.render_index.0 += 1;

        let mut outer_transform = RenderTransform::identity();
        let mut undo_prev_outer_transform = None;
        if reuse.is_some() {
            // check if is possible to reuse.

            if !self.can_reuse {
                *reuse = None; // reuse is stale because the widget was previously not rendered, or is disabled by user.
            } else {
                outer_transform = RenderTransform::translation_px(ctx.widget_info.bounds.outer_offset()).then(&self.transform);

                let prev_outer = ctx.widget_info.bounds.outer_transform();
                if prev_outer != outer_transform {
                    if let Some(undo_prev) = prev_outer.inverse() {
                        undo_prev_outer_transform = Some(undo_prev);
                    } else {
                        *reuse = None; // cannot reuse because cannot undo prev-transform.
                    }
                }
            }
        }

        self.hit_clips.push_child(ctx.path.widget_id());

        let mut reused = true;

        // try to reuse, or calls the closure and saves the reuse range.
        self.push_reuse(reuse, |frame| {
            // did not reuse, render widget.

            reused = false;
            undo_prev_outer_transform = None;

            frame.widget_data = Some(WidgetData {
                filter: vec![],
                has_transform: false,
                transform: RenderTransform::identity(),
            });
            let parent_widget = mem::replace(&mut frame.widget_id, ctx.path.widget_id());

            render(ctx, frame);

            frame.widget_id = parent_widget;
            frame.widget_data = None;
        });

        if reused {
            // if did reuse, patch transforms and z-indexes.

            let transform_patch = undo_prev_outer_transform.and_then(|t| {
                let t = t.then(&outer_transform);
                if t != RenderTransform::identity() {
                    Some(t)
                } else {
                    None
                }
            });
            let z_patch = ctx
                .widget_info
                .bounds
                .rendered()
                .map(|(old_back, _)| widget_z.0 as i64 - old_back.0 as i64)
                .unwrap_or(0);

            let update_transforms = transform_patch.is_some();
            let update_z = z_patch != 0;

            // apply patches, only iterates over descendants once.
            if update_transforms && update_z {
                let transform_patch = transform_patch.unwrap();

                for info in ctx.info_tree.get(ctx.path.widget_id()).unwrap().self_and_descendants() {
                    let bounds = info.bounds_info();

                    bounds.set_outer_transform(bounds.outer_transform().then(&transform_patch), ctx.info_tree);
                    bounds.set_inner_transform(bounds.inner_transform().then(&transform_patch), ctx.info_tree);

                    if let Some((back, front)) = bounds.rendered() {
                        let back = back.0 as i64 + z_patch;
                        let front = front.0 as i64 + z_patch;
                        bounds.set_rendered(Some((ZIndex(back as u32), ZIndex(front as u32))), ctx.info_tree);
                    }
                }
            } else if update_transforms {
                let transform_patch = transform_patch.unwrap();

                for info in ctx.info_tree.get(ctx.path.widget_id()).unwrap().self_and_descendants() {
                    let bounds = info.bounds_info();

                    bounds.set_outer_transform(bounds.outer_transform().then(&transform_patch), ctx.info_tree);
                    bounds.set_inner_transform(bounds.inner_transform().then(&transform_patch), ctx.info_tree);
                }
            } else if update_z {
                for info in ctx.info_tree.get(ctx.path.widget_id()).unwrap().self_and_descendants() {
                    let bounds = info.bounds_info();

                    if let Some((back, front)) = bounds.rendered() {
                        let back = back.0 as i64 + z_patch;
                        let front = front.0 as i64 + z_patch;
                        bounds.set_rendered(Some((ZIndex(back as u32), ZIndex(front as u32))), ctx.info_tree);
                    }
                }
            }

            // increment by reused
            self.render_index = ctx.widget_info.bounds.rendered().map(|(_, f)| f).unwrap_or(self.render_index);
        } else {
            // if did not reuse

            // ensure  that if any item was pushed the widget is marked as rendered.
            self.widget_rendered |= !reuse.as_ref().unwrap().is_empty();

            if self.widget_rendered {
                ctx.widget_info
                    .bounds
                    .set_rendered(Some((widget_z, self.render_index)), ctx.info_tree);
            } else {
                ctx.widget_info.bounds.set_rendered(None, ctx.info_tree);
                self.render_index.0 -= 1;
            }
        }

        self.can_reuse = parent_can_reuse;
        self.widget_rendered |= parent_rendered;
    }

    /// Mark the widget as rendered even if it does not push any display or hit item.
    pub fn widget_rendered(&mut self) {
        self.widget_rendered = true;
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
    /// [`widget_rendered`]: Self::widget_rendered
    pub fn push_reuse(&mut self, group: &mut Option<ReuseRange>, generate: impl FnOnce(&mut Self)) {
        if self.can_reuse {
            if let Some(g) = &group {
                if g.pipeline_id() == self.pipeline_id {
                    self.display_list.push_reuse_range(g);
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

    /// Register that the current widget and descendants are not rendered in this frame.
    ///
    /// Nodes the set the visibility to the equivalent of [`Hidden`] or [`Collapsed`] must not call `render` and `render_update`
    /// for the descendant nodes and must call this method to update the rendered status of all descendant nodes.
    ///
    /// [`Hidden`]: crate::widget_info::Visibility::Hidden
    /// [`Collapsed`]: crate::widget_info::Visibility::Collapsed
    pub fn skip_render(&mut self, info_tree: &WidgetInfoTree) {
        if let Some(w) = info_tree.get(self.widget_id) {
            self.widget_rendered = false;
            for w in w.self_and_descendants() {
                w.bounds_info().set_rendered(None, info_tree);

                if w.path().contains(WidgetId::named("commands-stack")) {
                    println!("!! SK {:?}, {:?}", w.path(), w.bounds_info().rendered().is_some());
                }
            }
        } else {
            tracing::error!("skip_render did not find widget `{}` in info tree", self.widget_id)
        }
    }

    /// Register that all widget descendants are not rendered in this frame.
    ///
    /// Widgets that control the visibility of their children can call this and then, in the same frame, render
    /// only the children that should be visible.
    pub fn skip_render_descendants(&self, info_tree: &WidgetInfoTree) {
        if let Some(w) = info_tree.get(self.widget_id) {
            for w in w.descendants() {
                w.bounds_info().set_rendered(None, info_tree);
            }
        } else {
            tracing::error!("skip_render_descendants did not find widget `{}` in info tree", self.widget_id)
        }
    }

    /// Returns `true`  if the widget stacking context is still being build.
    ///
    /// This is `true` when inside a [`push_widget`] call but `false` when inside a [`push_inner`] call.
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
    pub fn push_inner_opacity(&mut self, bind: FrameBinding<f32>, render: impl FnOnce(&mut Self)) {
        if let Some(data) = self.widget_data.as_mut() {
            let value = match &bind {
                PropertyBinding::Value(v) => *v,
                PropertyBinding::Binding(_, v) => *v,
            };

            let filter = vec![FilterOp::Opacity(bind, value)];
            data.filter.push(filter[0]);

            render(self);
        } else {
            tracing::error!("called `push_inner_opacity` inside inner context of `{}`", self.widget_id);
            render(self);
        }
    }

    /// Include the `transform` on the widget inner reference frame.
    ///
    /// This is valid only when [`is_outer`].
    ///
    /// When [`push_inner`] is called a reference frame is created for the widget that applies the layout translate then the `transform`.
    ///
    /// [`is_outer`]: Self::is_outer
    /// [`push_inner`]: Self::push_inner
    pub fn push_inner_transform(&mut self, transform: &RenderTransform, render: impl FnOnce(&mut Self)) {
        if let Some(data) = &mut self.widget_data {
            let parent_has_transform = data.has_transform;
            let parent_transform = data.transform;
            data.has_transform = true;
            data.transform = data.transform.then(transform);

            render(self);

            if let Some(data) = &mut self.widget_data {
                data.has_transform = parent_has_transform;
                data.transform = parent_transform;
            }
        } else {
            tracing::error!("called `push_inner_transform` inside inner context of `{}`", self.widget_id);
            render(self);
        }
    }

    /// Push the widget reference frame and stacking context then call `render` inside of it.
    pub fn push_inner(
        &mut self,
        ctx: &mut RenderContext,
        layout_translation_key: FrameBindingKey<RenderTransform>,
        render: impl FnOnce(&mut RenderContext, &mut Self),
    ) {
        if let Some(mut data) = self.widget_data.take() {
            let parent_transform = self.transform;
            let parent_hit_clips = mem::take(&mut self.hit_clips);

            let outer_transform = RenderTransform::translation_px(ctx.widget_info.bounds.outer_offset()).then(&parent_transform);
            ctx.widget_info.bounds.set_outer_transform(outer_transform, ctx.info_tree);

            let translate = ctx.widget_info.bounds.inner_offset() + ctx.widget_info.bounds.outer_offset();
            let inner_transform = if data.has_transform {
                data.transform.then_translate_px(translate)
            } else {
                RenderTransform::translation_px(translate)
            };
            self.transform = inner_transform.then(&parent_transform);
            ctx.widget_info.bounds.set_inner_transform(self.transform, ctx.info_tree);

            self.display_list.push_reference_frame(
                SpatialFrameId::widget_id_to_wr(self.widget_id, self.pipeline_id),
                layout_translation_key.bind(inner_transform),
                !data.has_transform,
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

            render(ctx, self);

            if has_stacking_ctx {
                self.display_list.pop_stacking_context();
            }
            self.display_list.pop_reference_frame();

            self.transform = parent_transform;

            let hit_clips = mem::replace(&mut self.hit_clips, parent_hit_clips);
            ctx.widget_info.bounds.set_hit_clips(hit_clips);
        } else {
            tracing::error!("called `push_inner` more then once for `{}`", self.widget_id);
            render(ctx, self)
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
            is_hit_testable: self.is_hit_testable,
            widget_rendered: &mut self.widget_rendered,
        }
    }

    /// Calls `render` with a new clip context that adds the `clip_rect`.
    ///
    /// If `clip_out` is `true` only pixels outside the rect are visible.
    pub fn push_clip_rect(&mut self, clip_rect: PxRect, clip_out: bool, render: impl FnOnce(&mut FrameBuilder)) {
        expect_inner!(self.push_clip_rect);

        self.display_list.push_clip_rect(clip_rect, clip_out);

        render(self);

        self.display_list.pop_clip();
    }

    /// Calls `render` with a new clip context that adds  the `clip_rect` with rounded `corners`.
    ///
    /// If `clip_out` is `true` only pixels outside the rounded rect are visible.
    pub fn push_clip_rounded_rect(
        &mut self,
        clip_rect: PxRect,
        corners: PxCornerRadius,
        clip_out: bool,
        render: impl FnOnce(&mut FrameBuilder),
    ) {
        expect_inner!(self.push_clip_rounded_rect);

        self.display_list.push_clip_rounded_rect(clip_rect, corners, clip_out);

        render(self);

        self.display_list.pop_clip();
    }

    /// Calls `render` inside a new reference frame transformed by `transform`.
    ///
    /// Note that properties that use this method must also register the custom transform with the widget info, so that the widget
    /// can be found by decorator overlays or other features that depend on the info tree position.
    ///
    /// The `is_2d_scale_translation` flag optionally marks the `transform` as only ever having a simple 2D scale or translation,
    /// allowing for webrender optimizations.
    ///
    /// [`push_inner`]: Self::push_inner
    /// [`WidgetLayout`]: crate::widget_info::WidgetLayout
    pub fn push_reference_frame(
        &mut self,
        id: SpatialFrameId,
        transform: FrameBinding<RenderTransform>,
        is_2d_scale_translation: bool,
        render: impl FnOnce(&mut Self),
    ) {
        self.push_reference_frame_impl(id.to_wr(self.pipeline_id), transform, is_2d_scale_translation, render)
    }

    /// Pushes a custom `push_reference_frame` with an item [`SpatialFrameId`].
    pub fn push_reference_frame_item(
        &mut self,
        id: SpatialFrameId,
        item: usize,
        transform: FrameBinding<RenderTransform>,
        is_2d_scale_translation: bool,
        render: impl FnOnce(&mut Self),
    ) {
        self.push_reference_frame_impl(id.item_to_wr(item, self.pipeline_id), transform, is_2d_scale_translation, render)
    }
    fn push_reference_frame_impl(
        &mut self,
        id: SpatialTreeItemKey,
        transform: FrameBinding<RenderTransform>,
        is_2d_scale_translation: bool,
        render: impl FnOnce(&mut Self),
    ) {
        let parent_transform = self.transform;
        self.transform = match transform {
            PropertyBinding::Value(value) | PropertyBinding::Binding(_, value) => value,
        }
        .then(&parent_transform);

        self.display_list.push_reference_frame(id, transform, is_2d_scale_translation);

        render(self);

        self.display_list.pop_reference_frame();
        self.transform = parent_transform;
    }

    /// Calls `render` with added `filter` stacking context.
    ///
    /// Note that this introduces a new stacking context, you can use the [`push_inner_filter`] method to
    /// add to the widget stacking context.
    ///
    /// [`push_inner_filter`]: Self::push_inner_filter
    pub fn push_filter(&mut self, blend_mode: MixBlendMode, filter: &RenderFilter, render: impl FnOnce(&mut Self)) {
        expect_inner!(self.push_filter);

        self.display_list.push_stacking_context(blend_mode, filter, &[], &[]);

        render(self);

        self.display_list.pop_stacking_context();
    }

    /// Push a border.
    pub fn push_border(&mut self, bounds: PxRect, widths: PxSideOffsets, sides: BorderSides, radius: PxCornerRadius) {
        expect_inner!(self.push_border);

        self.display_list.push_border(
            bounds,
            widths,
            sides.top.into(),
            sides.right.into(),
            sides.bottom.into(),
            sides.bottom.into(),
            radius,
        );

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
        color: RenderColor,
        synthesis: FontSynthesis,
        aa: FontAntiAliasing,
    ) {
        expect_inner!(self.push_text);

        if let Some(r) = &self.renderer {
            if !glyphs.is_empty() {
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

            if self.auto_hit_test {
                self.hit_test().push_rect(clip_rect);
            }
        } else {
            self.widget_rendered = true;
        }
    }

    /// Push an image.
    pub fn push_image(&mut self, clip_rect: PxRect, img_size: PxSize, image: &impl Image, rendering: ImageRendering) {
        expect_inner!(self.push_image);

        if let Some(r) = &self.renderer {
            let image_key = image.image_key(r);
            self.display_list
                .push_image(clip_rect, image_key, img_size, rendering.into(), image.alpha_type());

            if self.auto_hit_test {
                self.hit_test().push_rect(clip_rect);
            }
        } else {
            self.widget_rendered = true;
        }
    }

    /// Push a color rectangle.
    pub fn push_color(&mut self, clip_rect: PxRect, color: FrameBinding<RenderColor>) {
        expect_inner!(self.push_color);

        self.display_list.push_color(clip_rect, color);

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

        if !stops.is_empty() {
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

        if !stops.is_empty() {
            self.display_list.push_radial_gradient(
                clip_rect,
                webrender_api::RadialGradient {
                    center: center.to_wr(),
                    radius: radius.to_wr(),
                    start_offset: 0.0, // TODO expose this?
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

        if !stops.is_empty() {
            self.display_list.push_conic_gradient(
                clip_rect,
                webrender_api::ConicGradient {
                    center: center.to_wr(),
                    angle: angle.0,
                    start_offset: 0.0, // TODO expose this?
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

        if self.auto_hit_test {
            self.hit_test().push_rect(clip_rect);
        }
    }

    /// Push a `color` dot to mark the `offset`.
    ///
    /// The *dot* is a circle of the `color` highlighted by an white outline and shadow.
    pub fn push_debug_dot(&mut self, offset: PxPoint, color: impl Into<RenderColor>) {
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
    pub fn finalize(self, info_tree: &WidgetInfoTree) -> (BuiltFrame, UsedFrameBuilder) {
        info_tree.root().bounds_info().set_rendered(
            if self.widget_rendered {
                Some((ZIndex(0), self.render_index))
            } else {
                None
            },
            info_tree,
        );
        info_tree.after_render(self.frame_id, self.scale_factor);

        let (display_list, capacity) = self.display_list.finalize();

        let clear_color = self.clear_color.unwrap_or(RenderColor::WHITE);

        let reuse = UsedFrameBuilder {
            pipeline_id: display_list.pipeline_id(),
            display_list: capacity,
        };

        let frame = BuiltFrame { display_list, clear_color };

        (frame, reuse)
    }
}

/// Builder for the hit-testable shape of the inner-bounds of a widget.
///
/// This builder is available in [`FrameBuilder::hit_test`] inside the inner-bounds of the rendering widget.
pub struct HitTestBuilder<'a> {
    hit_clips: &'a mut HitTestClips,
    is_hit_testable: bool,
    widget_rendered: &'a mut bool,
}
impl<'a> HitTestBuilder<'a> {
    /// If the widget is hit-testable, if this is `false` all hit-test push methods are ignored.
    pub fn is_hit_testable(&self) -> bool {
        self.is_hit_testable
    }

    /// Push a hit-test `rect`.
    pub fn push_rect(&mut self, rect: PxRect) {
        if self.is_hit_testable && rect.size != PxSize::zero() {
            *self.widget_rendered = true;
            self.hit_clips.push_rect(rect.to_box2d());
        }
    }

    /// Push a hit-test `rect` with rounded `corners`.
    pub fn push_rounded_rect(&mut self, rect: PxRect, corners: PxCornerRadius) {
        if self.is_hit_testable && rect.size != PxSize::zero() {
            *self.widget_rendered = true;
            self.hit_clips.push_rounded_rect(rect.to_box2d(), corners);
        }
    }

    /// Push a hit-test ellipse.
    pub fn push_ellipse(&mut self, center: PxPoint, radii: PxSize) {
        if self.is_hit_testable && radii != PxSize::zero() {
            *self.widget_rendered = true;
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
        if !self.is_hit_testable {
            return;
        }

        self.hit_clips.push_clip_rounded_rect(rect.to_box2d(), corners, clip_out);

        inner_hit_test(self);

        self.hit_clips.pop_clip();
    }

    /// Push a clip ellipse that affects the `inner_hit_test`.
    pub fn push_clip_ellipse(&mut self, center: PxPoint, radii: PxSize, clip_out: bool, inner_hit_test: impl FnOnce(&mut Self)) {
        if self.is_hit_testable && radii != PxSize::zero() {
            self.hit_clips.push_clip_ellipse(center, radii, clip_out);

            inner_hit_test(self);

            self.hit_clips.pop_clip();
        }
    }

    /// Pushes a transform that affects the `inner_hit_test`.
    pub fn push_transform(&mut self, transform: RenderTransform, inner_hit_test: impl FnOnce(&mut Self)) {
        if !self.is_hit_testable {
            return;
        }

        self.hit_clips.push_transform(transform);

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

/// Data from a previous [`FrameBuilder`], can be reuse in the next frame for a performance boost.
pub struct UsedFrameBuilder {
    pipeline_id: PipelineId,
    display_list: usize,
}
impl UsedFrameBuilder {
    /// Pipeline where this frame builder can be reused.
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }
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

/// A frame quick update.
///
/// A frame update causes a frame render without needing to fully rebuild the display list. It
/// is a more performant but also more limited way of generating a frame.
///
/// Any [`FrameBindingKey`] used in the creation of the frame can be used for updating the frame.
pub struct FrameUpdate {
    pipeline_id: PipelineId,
    bindings: DynamicProperties,
    current_clear_color: RenderColor,
    clear_color: Option<RenderColor>,
    frame_id: FrameId,

    widget_id: WidgetId,
    transform: RenderTransform,
    inner_transform: Option<RenderTransform>,
    can_reuse_widget: bool,
}
impl FrameUpdate {
    /// New frame update builder.
    ///
    /// * `frame_id` - Id of the frame that will be updated.
    /// * `root_id` - Id of the window root widget.
    /// * `renderer` - Reference to the renderer that will update.
    /// * `clear_color` - The current clear color.
    /// * `used_data` - Data generated by a previous frame update, if set is recycled for a performance boost.
    pub fn new(
        frame_id: FrameId,
        root_id: WidgetId,
        renderer: Option<&ViewRenderer>,
        clear_color: RenderColor,
        used_data: Option<UsedFrameUpdate>,
    ) -> Self {
        // in case they add more dynamic property types.
        assert_size_of!(DynamicProperties, 72);

        let pipeline_id = renderer
            .as_ref()
            .and_then(|r| r.pipeline_id().ok())
            .unwrap_or_else(PipelineId::dummy);

        let mut hint = None;
        if let Some(h) = used_data {
            if h.pipeline_id == pipeline_id {
                hint = Some(h);
            }
        }
        let hint = hint.unwrap_or(UsedFrameUpdate {
            pipeline_id,
            transforms_capacity: 100,
            floats_capacity: 100,
            colors_capacity: 100,
        });
        FrameUpdate {
            pipeline_id,
            widget_id: root_id,
            bindings: DynamicProperties {
                transforms: Vec::with_capacity(hint.transforms_capacity),
                floats: Vec::with_capacity(hint.floats_capacity),
                colors: Vec::with_capacity(hint.colors_capacity),
            },
            clear_color: None,
            frame_id,
            current_clear_color: clear_color,

            transform: RenderTransform::identity(),
            inner_transform: Some(RenderTransform::identity()),
            can_reuse_widget: true,
        }
    }

    /// The frame that will be updated.
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Returns `true` if the widget inner transform update is still being build.
    ///
    /// This is `true` when inside a [`update_widget`] call but `false` when inside a [`update_inner`] call.
    ///
    /// [`update_widget`]: Self::update_widget
    /// [`update_inner`]: Self::update_inner
    pub fn is_outer(&self) -> bool {
        self.inner_transform.is_some()
    }

    /// Current transform.
    pub fn transform(&self) -> &RenderTransform {
        &self.transform
    }

    /// Change the color used to clear the pixel buffer when redrawing the frame.
    pub fn set_clear_color(&mut self, color: RenderColor) {
        self.clear_color = Some(color);
    }

    /// Update a transform value that does not potentially affect widget bounds.
    ///
    /// Use [`with_transform`] to update transforms that affect widget bounds.
    ///
    /// [`with_transform`]: Self::with_transform
    pub fn update_transform(&mut self, new_value: FrameValue<RenderTransform>) {
        self.bindings.transforms.push(new_value);
    }

    /// Update a transform that potentially affects widget bounds.
    ///
    /// The [`transform`] is updated to include this space for the call to the `render_update` closure. The closure
    /// must call render update on child nodes.
    ///
    /// [`transform`]: Self::transform
    pub fn with_transform(&mut self, new_value: FrameValue<RenderTransform>, render_update: impl FnOnce(&mut Self)) {
        let parent_transform = self.transform;
        self.transform = new_value.value.then(&parent_transform);

        self.update_transform(new_value);

        render_update(self);
        self.transform = parent_transform;
    }

    /// Update the transform applied after the inner bounds translate.
    ///
    /// This is only valid if [`is_outer`].
    ///
    /// [`is_outer`]: Self::is_outer
    pub fn with_inner_transform(&mut self, transform: &RenderTransform, render_update: impl FnOnce(&mut Self)) {
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
    pub fn update_widget(&mut self, ctx: &mut RenderContext, reuse: bool, render_update: impl FnOnce(&mut RenderContext, &mut Self)) {
        if self.inner_transform.is_some() {
            tracing::error!(
                "called `update_widget` for `{}` without calling `update_inner` for the parent `{}`",
                ctx.path.widget_id(),
                self.widget_id
            );
        }

        let outer_transform = RenderTransform::translation_px(ctx.widget_info.bounds.outer_offset()).then(&self.transform);

        let parent_can_reuse = self.can_reuse_widget;

        if self.can_reuse_widget && reuse {
            let prev_outer = ctx.widget_info.bounds.outer_transform();
            if prev_outer != outer_transform {
                if let Some(undo_prev) = prev_outer.inverse() {
                    let patch = undo_prev.then(&outer_transform);

                    for info in ctx.info_tree.get(ctx.path.widget_id()).unwrap().self_and_descendants() {
                        let bounds = info.bounds_info();
                        bounds.set_outer_transform(bounds.outer_transform().then(&patch), ctx.info_tree);
                        bounds.set_inner_transform(bounds.inner_transform().then(&patch), ctx.info_tree);
                    }

                    return; // can reuse and patched.
                }
            } else {
                return; // can reuse and no change.
            }

            // actually cannot reuse because cannot undo prev-transform.
            self.can_reuse_widget = false;
        }

        ctx.widget_info.bounds.set_outer_transform(outer_transform, ctx.info_tree);
        self.inner_transform = Some(RenderTransform::identity());
        let parent_id = self.widget_id;
        self.widget_id = ctx.path.widget_id();
        render_update(ctx, self);
        self.inner_transform = None;
        self.widget_id = parent_id;
        self.can_reuse_widget = parent_can_reuse;
    }

    /// Update the info transforms of the widget and descendants.
    ///
    /// Widgets that did not request render-update can use this method to update only the outer and inner transforms
    /// of itself and descendants as those values are global and the parent widget may have changed.
    pub fn reuse_widget(&mut self, ctx: &mut RenderContext) {
        if self.inner_transform.is_some() {
            tracing::error!(
                "called `reuse_widget` for `{}` without calling `update_inner` for the parent `{}`",
                ctx.path.widget_id(),
                self.widget_id
            );
        }
    }

    /// Update the widget's inner transform.
    pub fn update_inner(
        &mut self,
        ctx: &mut RenderContext,
        layout_translation_key: FrameBindingKey<RenderTransform>,
        render_update: impl FnOnce(&mut RenderContext, &mut Self),
    ) {
        if let Some(inner_transform) = self.inner_transform.take() {
            let translate = ctx.widget_info.bounds.inner_offset() + ctx.widget_info.bounds.outer_offset();
            let inner_transform = inner_transform.then_translate_px(translate);
            self.update_transform(layout_translation_key.update(inner_transform));
            let parent_transform = self.transform;

            self.transform = inner_transform.then(&parent_transform);
            ctx.widget_info.bounds.set_inner_transform(self.transform, ctx.info_tree);

            render_update(ctx, self);
            self.transform = parent_transform;
        } else {
            tracing::error!("called `update_inner` more then once for `{}`", self.widget_id);
            render_update(ctx, self)
        }
    }

    /// Update a float value.
    pub fn update_f32(&mut self, new_value: FrameValue<f32>) {
        self.bindings.floats.push(new_value);
    }

    /// Update a color value.
    pub fn update_color(&mut self, new_value: FrameValue<RenderColor>) {
        self.bindings.colors.push(new_value)
    }

    /// Finalize the update.
    ///
    /// Returns the property updates and the new clear color if any was set.
    pub fn finalize(mut self, info_tree: &WidgetInfoTree) -> (BuiltFrameUpdate, UsedFrameUpdate) {
        info_tree.after_render_update(self.frame_id);

        if self.clear_color == Some(self.current_clear_color) {
            self.clear_color = None;
        }

        let used = UsedFrameUpdate {
            pipeline_id: self.pipeline_id,
            transforms_capacity: self.bindings.transforms.len(),
            floats_capacity: self.bindings.floats.len(),
            colors_capacity: self.bindings.colors.len(),
        };

        let update = BuiltFrameUpdate {
            bindings: self.bindings,
            clear_color: self.clear_color,
        };

        (update, used)
    }
}

/// Output of a [`FrameBuilder`].
pub struct BuiltFrameUpdate {
    /// Webrender frame properties updates.
    pub bindings: DynamicProperties,
    /// New clear color.
    pub clear_color: Option<RenderColor>,
}

/// Data from a previous [`FrameUpdate`], can be reuse in the next frame for a performance boost.
#[derive(Clone, Copy)]
pub struct UsedFrameUpdate {
    pipeline_id: PipelineId,
    transforms_capacity: usize,
    floats_capacity: usize,
    colors_capacity: usize,
}
impl UsedFrameUpdate {
    /// Pipeline where this frame builder can be reused.
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }
}

/// A frame value that can be updated without regenerating the full frame.
///
/// Use `FrameBinding::Value(value)` to not use the quick update feature.
///
/// Create a [`FrameBindingKey`] and use its [`bind`] method to setup a frame binding.
///
/// [`bind`]: FrameBindingKey::bind
pub type FrameBinding<T> = PropertyBinding<T>; // we rename this to not conflict with the zero_ui property terminology.

/// A frame value update.
pub type FrameValue<T> = PropertyValue<T>;

unique_id_64! {
    #[derive(Debug)]
    struct FrameBindingKeyId;
}

unique_id_32! {
    /// Unique ID of a reference frame.
    #[derive(Debug)]
    pub struct SpatialFrameId;
}
impl SpatialFrameId {
    const WIDGET_ID_FLAG: u64 = 1 << 62;
    const LIST_ID_FLAG: u64 = 1 << 61;

    /// Make a [`SpatialTreeItemKey`] from a [`WidgetId`], there is no collision
    /// with other keys generated.
    ///
    /// This is the spatial id used for the widget's inner bounds offset.
    pub fn widget_id_to_wr(self_: WidgetId, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        SpatialTreeItemKey::new(pipeline_id.0 as u64 | Self::WIDGET_ID_FLAG, self_.get())
    }

    /// To webrender [`SpatialTreeItemKey`].
    pub fn to_wr(self, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        SpatialTreeItemKey::new(pipeline_id.0 as u64, self.get() as u64)
    }

    /// Make [`SpatialTreeItemKey`] from a a spatial parent + item index, there is no collision
    /// with other keys generated.
    pub fn item_to_wr(self, index: usize, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        let item = (index as u64) << 32;
        SpatialTreeItemKey::new(pipeline_id.0 as u64 | Self::LIST_ID_FLAG, self.get() as u64 | item)
    }
}

/// Unique key of a [`FrameBinding`] value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameBindingKey<T> {
    id: FrameBindingKeyId,
    _type: PhantomData<T>,
}
impl<T> FrameBindingKey<T> {
    /// Generates a new unique ID.
    ///
    /// # Panics
    /// Panics if called more then `u64::MAX` times.
    pub fn new_unique() -> Self {
        FrameBindingKey {
            id: FrameBindingKeyId::new_unique(),
            _type: PhantomData,
        }
    }

    fn property_key(&self) -> PropertyBindingKey<T> {
        PropertyBindingKey::new(self.id.get())
    }

    /// Create a binding with this key.
    pub fn bind(self, value: T) -> FrameBinding<T> {
        FrameBinding::Binding(self.property_key(), value)
    }

    /// Create a value update with this key.
    pub fn update(self, value: T) -> FrameValue<T> {
        FrameValue {
            key: self.property_key(),
            value,
        }
    }
}

bitflags! {
    /// Configure if a synthetic font is generated for fonts that do not implement **bold** or *oblique* variants.
    pub struct FontSynthesis: u8 {
        /// No synthetic font generated, if font resolution does not find a variant the matches the requested propertied
        /// the properties are ignored and the normal font is returned.
        const DISABLED = 0;
        /// Enable synthetic bold. Font resolution finds the closest bold variant, the difference added using extra stroke.
        const BOLD = 1;
        /// Enable synthetic oblique. If the font resolution does not find an oblique or italic variant a skew transform is applied.
        const STYLE = 2;
        /// Enabled all synthetic font possibilities.
        const ENABLED = Self::BOLD.bits | Self::STYLE.bits;
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
