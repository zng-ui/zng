//! Frame render and metadata API.

use crate::{
    app::view_process::ViewRenderer,
    border::BorderSides,
    color::{RenderColor, RenderFilter},
    context::RenderContext,
    gradient::{RenderExtendMode, RenderGradientStop},
    units::*,
    var::impl_from_and_into_var,
    widget_info::WidgetRendered,
    window::WindowId,
    UiNode, WidgetId,
};
use derive_more as dm;
use linear_map::LinearMap;
use std::{collections::BTreeMap, marker::PhantomData, mem, ops};

pub use zero_ui_view_api::webrender_api;

use webrender_api::*;

pub use zero_ui_view_api::FrameId;

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
    fn instance_key(&self, renderer: &ViewRenderer, synthesis: FontSynthesis) -> webrender_api::FontInstanceKey;
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

/// A full frame builder.
pub struct FrameBuilder {
    frame_id: FrameId,
    window_id: WindowId,
    pipeline_id: PipelineId,

    renderer: Option<ViewRenderer>,

    scale_factor: Factor,

    // selected layer.
    layer_index: LayerIndex,
    display_list: DisplayListBuilder,

    widget_id: WidgetId,
    widget_rendered: bool,
    widget_transform_key: WidgetTransformKey,
    widget_stack_ctx_data: Option<(RenderTransform, Vec<FilterOp>, PrimitiveFlags)>,
    cancel_widget: bool,
    widget_display_mode: WidgetDisplayMode,

    clip_id: ClipId,
    spatial_id: SpatialId,
    parent_spatial_id: SpatialId,

    clear_color_layer: LayerIndex,
    clear_color: Option<RenderColor>,

    layers: BTreeMap<LayerIndex, Option<DisplayListBuilder>>,
    used_layers: LinearMap<LayerIndex, DisplayListBuilder>,
}

bitflags! {
    struct WidgetDisplayMode: u8 {
        const REFERENCE_FRAME = 1;
        const STACKING_CONTEXT = 2;
    }
}
impl FrameBuilder {
    /// New builder.
    ///
    /// * `frame_id` - Id of the new frame.
    /// * `window_id` - Id of the window that will render the frame.
    /// * `renderer` - Connection to the renderer connection that will render the frame, is `None` in renderless mode.
    /// * `root_id` - Id of the root widget of the frame, usually the window root.
    /// * `root_transform_key` - Frame binding for the root widget layout transform.
    /// * `scale_factor` - Scale factor that will be used to render the frame, usually the scale factor of the screen the window is at.
    /// * `used_data` - Data generated by a previous frame buffer, if set is recycled for a performance boost.
    /// because WebRender does not let us change the initial clear color.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    pub fn new(
        frame_id: FrameId,
        window_id: WindowId,
        renderer: Option<ViewRenderer>,
        root_id: WidgetId,
        root_transform_key: WidgetTransformKey,
        scale_factor: Factor,
        used_data: Option<UsedFrameBuilder>,
    ) -> Self {
        let pipeline_id = renderer
            .as_ref()
            .and_then(|r| r.pipeline_id().ok())
            .unwrap_or_else(PipelineId::dummy);

        let mut display_list;
        let mut used_layers = LinearMap::new();

        if let Some(used) = used_data {
            if used.pipeline_id() == pipeline_id {
                used_layers = used.used_layers;
            }
        }

        if let Some(reuse) = used_layers.remove(&LayerIndex::DEFAULT) {
            display_list = reuse;
        } else {
            display_list = DisplayListBuilder::new(pipeline_id);
        }

        display_list.begin();

        let spatial_id = SpatialId::root_reference_frame(pipeline_id);
        let mut new = FrameBuilder {
            frame_id,
            window_id,
            pipeline_id,
            renderer,
            scale_factor,
            display_list,
            widget_id: root_id,
            widget_rendered: false,
            widget_transform_key: root_transform_key,
            widget_stack_ctx_data: None,
            cancel_widget: false,
            widget_display_mode: WidgetDisplayMode::empty(),

            clip_id: ClipId::root(pipeline_id),
            spatial_id,
            parent_spatial_id: spatial_id,

            layer_index: LayerIndex::DEFAULT,
            layers: Some((LayerIndex::DEFAULT, None)).into_iter().collect(),

            clear_color_layer: LayerIndex::DEFAULT,
            clear_color: None,

            used_layers,
        };
        new.widget_stack_ctx_data = Some((RenderTransform::identity(), Vec::default(), PrimitiveFlags::empty()));
        new
    }

    /// [`new`](Self::new) with only the inputs required for renderless mode.
    pub fn new_renderless(
        frame_id: FrameId,
        window_id: WindowId,
        root_id: WidgetId,
        root_transform_key: WidgetTransformKey,
        scale_factor: Factor,
        hint: Option<UsedFrameBuilder>,
    ) -> Self {
        Self::new(
            frame_id,
            window_id,
            None,
            root_id,
            root_transform_key,
            scale_factor,
            hint,
        )
    }

    /// Returns the current layer index.
    #[inline]
    pub fn layer_index(&self) -> LayerIndex {
        self.layer_index
    }

    /// Calls `action` with the builder set to the selected layer, creating it if it was not requested before.
    ///
    /// Every [`FrameBuilder`] is implicitly building a *layer* of display items, later added items
    /// will render on top of early added items, to ensure that an early added item is rendered on top
    /// of all later items you can request a layer that is a different [`FrameBuilder`] that will be composed
    /// at the right Z-order when the overall render builder is finalized.
    ///
    /// Note that layers don't retain any of the stacked context transforms, clips and effects at the position they are
    /// requested, the context must be recreated in the layer if you want to align display items with the parent
    /// display items in the parent layer. Some helper methods are provided for this, see [`with_context_in_layer`]
    /// for more details.
    ///
    /// [`with_context_in_layer`]: Self::with_context_in_layer
    pub fn with_layer(&mut self, index: LayerIndex, action: impl FnOnce(&mut Self)) {
        if self.layer_index == index {
            action(self)
        } else {
            let layer = self.layers.entry(index).or_insert_with(|| {
                Some({
                    let mut layer = self
                        .used_layers
                        .remove(&index)
                        .unwrap_or_else(|| DisplayListBuilder::new(self.pipeline_id));
                    layer.begin();
                    layer
                })
            });

            let prev_index = mem::replace(&mut self.layer_index, index);
            let prev_layer = mem::replace(&mut self.display_list, layer.take().unwrap());
            *self.layers.get_mut(&prev_index).unwrap() = Some(prev_layer);

            action(self);

            self.layer_index = prev_index;
            let layer = self.layers.get_mut(&prev_index).unwrap().take().unwrap();
            let layer = mem::replace(&mut self.display_list, layer);

            *self.layers.get_mut(&index).unwrap() = Some(layer);

            todo!("TODO, review Builder has some context values here, like the widget_id")
        }
    }

    /// Runs `action` in the selected layer with stacking contexts and widget metadata that recreate
    /// the current context of `self` in the selected layer.
    pub fn with_context_in_layer(&mut self, index: LayerIndex, action: impl FnOnce(&mut Self)) {
        self.with_layer(index, |layer| {
            action(layer);
            todo!();
        })
    }

    /// Pixel scale factor used by the renderer.
    ///
    /// All layout values are scaled by this factor in the renderer.
    #[inline]
    pub fn scale_factor(&self) -> Factor {
        self.scale_factor
    }

    /// Direct access to the current layer display list builder.
    ///
    /// # Careful
    ///
    /// This provides direct access to the underlying WebRender display list builder, modifying it
    /// can interfere with the working of the [`FrameBuilder`].
    ///
    /// Call [`open_widget_display`] before modifying the display list.
    ///
    /// Check the [`FrameBuilder`] source code before modifying the display list.
    ///
    /// Don't try to render using the [`FrameBuilder`] methods inside a custom clip or space, the methods will still
    /// use the [`clip_id`] and [`spatial_id`]. Custom items added to the display list
    /// should be self-contained and completely custom.
    ///
    /// If [`is_cancelling_widget`] don't modify the display list and try to
    /// early return pretending the operation worked.
    ///
    /// Call [`widget_rendered`] if you push anything to the display list.
    ///
    /// # WebRender
    ///
    /// The [`webrender`] crate used in the renderer is re-exported in `zero_ui_core::render::webrender`, and the
    /// [`webrender_api`] is re-exported in `webrender::api`.
    ///
    /// [`open_widget_display`]: Self::open_widget_display
    /// [`clip_id`]: Self::clip_id
    /// [`spatial_id`]: Self::spatial_id
    /// [`is_cancelling_widget`]: Self::is_cancelling_widget
    /// [`widget_rendered`]: Self::widget_rendered
    /// [`webrender`]: https://docs.rs/webrender
    /// [`webrender_api`]: https://docs.rs/webrender_api
    #[inline]
    pub fn display_list(&mut self) -> &mut DisplayListBuilder {
        &mut self.display_list
    }

    /// Indicates that something was rendered to [`display_list`].
    ///
    /// Note that only direct modification of [`display_list`] requires this method being called,
    /// the other rendering methods of this builder already flag this.
    ///
    /// [`display_list`]: Self::display_list
    pub fn widget_rendered(&mut self) {
        self.widget_rendered = true;
    }

    /// If is building a frame for a headless and renderless window.
    ///
    /// In this mode only the meta and layout information will be used as a *frame*. Methods still
    /// push to the [`display_list`](Self::display_list) when possible, custom methods should ignore this
    /// unless they need access to the [`renderer`](Self::renderer).
    #[inline]
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
    #[inline]
    pub fn set_clear_color(&mut self, color: RenderColor) {
        if self.clear_color.is_none() || self.clear_color_layer <= self.layer_index {
            self.clear_color = Some(color);
            self.clear_color_layer = self.layer_index;
        }
    }

    /// Connection to the renderer that will render this frame.
    ///
    /// Returns `None` when in [renderless](Self::is_renderless) mode.
    #[inline]
    pub fn renderer(&self) -> Option<&ViewRenderer> {
        self.renderer.as_ref()
    }

    /// Id of the frame being build.
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Renderer pipeline ID or [`dummy`].
    ///
    /// [`dummy`]: PipelineId::dummy
    #[inline]
    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Window that owns the frame.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Current widget.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Current clipping node.
    #[inline]
    pub fn clip_id(&self) -> ClipId {
        self.clip_id
    }

    /// Current spatial node.
    #[inline]
    pub fn spatial_id(&self) -> SpatialId {
        self.spatial_id
    }

    /// Current widget [`ItemTag`]. The first number is the raw [`widget_id`], the second number is reserved.
    ///
    /// For more details on how the ItemTag is used see [`FrameHitInfo::new`].
    ///
    /// [`widget_id`]: Self::widget_id
    #[inline]
    pub fn item_tag(&self) -> ItemTag {
        (self.widget_id.get(), 0)
    }

    /// Common item properties given a `clip_rect` and the current context.
    ///
    /// This is a common case helper, it also calls [`widget_rendered`].
    ///
    /// [`widget_rendered`]: Self::widget_rendered
    #[inline]
    pub fn common_item_ps(&mut self, clip_rect: PxRect) -> CommonItemProperties {
        self.widget_rendered();
        CommonItemProperties {
            clip_rect: clip_rect.to_wr(),
            clip_id: self.clip_id,
            spatial_id: self.spatial_id,
            flags: PrimitiveFlags::empty(),
        }
    }

    /// Generate a [`common_item_ps`] and pushes
    /// a hit-test [`item_tag`].
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    /// [`item_tag`]: FrameBuilder::item_tag
    #[inline]
    pub fn common_hit_item_ps(&mut self, clip_rect: PxRect) -> CommonItemProperties {
        let item = self.common_item_ps(clip_rect);
        self.display_list.push_hit_test(&item, self.item_tag());
        item
    }

    /// Includes a widget transform and continues the render build.
    ///
    /// This is `Ok(_)` only when a widget started, but [`open_widget_display`](Self::open_widget_display) was not called.
    ///
    /// In case of error the `child` render is still called just without the transform.
    #[inline]
    pub fn with_widget_transform(&mut self, transform: &RenderTransform, f: impl FnOnce(&mut Self)) -> Result<(), WidgetStartedError> {
        if self.cancel_widget {
            return Ok(());
        }
        if let Some((t, _, _)) = self.widget_stack_ctx_data.as_mut() {
            // we don't use post_transform here fore the same reason `Self::open_widget_display`
            // reverses filters, there is a detailed comment there.
            *t = transform.then(t);
            f(self);
            Ok(())
        } else {
            f(self);
            Err(WidgetStartedError)
        }
    }

    /// Includes a widget filter and continues the render build.
    ///
    /// This is `Ok(_)` only when a widget started, but [`open_widget_display`](Self::open_widget_display) was not called.
    ///
    /// In case of error the `child` render is still called just without the filter.
    #[inline]
    pub fn with_widget_filter(&mut self, filter: RenderFilter, f: impl FnOnce(&mut Self)) -> Result<(), WidgetStartedError> {
        if self.cancel_widget {
            return Ok(());
        }
        if let Some((_, fi, _)) = self.widget_stack_ctx_data.as_mut() {
            fi.extend(filter.into_iter().rev()); // see `Self::open_widget_display` for why it is reversed.
            f(self);
            Ok(())
        } else {
            f(self);
            Err(WidgetStartedError)
        }
    }

    /// Includes a widget opacity filter and continues the render build.
    ///
    /// This is `Ok(_)` only when a widget started, but [`open_widget_display`](Self::open_widget_display) was not called.
    ///
    /// In case of error the `child` render is still called just without the filter.
    #[inline]
    pub fn with_widget_opacity(&mut self, bind: FrameBinding<f32>, f: impl FnOnce(&mut Self)) -> Result<(), WidgetStartedError> {
        if self.cancel_widget {
            return Ok(());
        }
        if let Some((_, fi, _)) = self.widget_stack_ctx_data.as_mut() {
            let value = match &bind {
                PropertyBinding::Value(v) => *v,
                PropertyBinding::Binding(_, v) => *v,
            };
            fi.push(FilterOp::Opacity(bind, value));
            f(self);
            Ok(())
        } else {
            f(self);
            Err(WidgetStartedError)
        }
    }

    /// Include the `flags` on the widget stacking context flags.
    #[inline]
    pub fn width_widget_flags(&mut self, flags: PrimitiveFlags, f: impl FnOnce(&mut Self)) -> Result<(), WidgetStartedError> {
        if let Some((_, _, fl)) = self.widget_stack_ctx_data.as_mut() {
            *fl |= flags;
            f(self);
            Ok(())
        } else {
            f(self);
            Err(WidgetStartedError)
        }
    }

    /// Finish widget transform and filters by starting the widget reference frame and stacking context.
    #[inline]
    pub fn open_widget_display(&mut self) {
        if self.cancel_widget {
            return;
        }
        if let Some((transform, mut filters, flags)) = self.widget_stack_ctx_data.take() {
            if transform != RenderTransform::identity() {
                self.widget_display_mode |= WidgetDisplayMode::REFERENCE_FRAME;

                self.parent_spatial_id = self.spatial_id;
                self.spatial_id = self.display_list.push_reference_frame(
                    PxPoint::zero().to_wr(),
                    self.spatial_id,
                    TransformStyle::Flat,
                    self.widget_transform_key.bind(transform),
                    ReferenceFrameKind::Transform {
                        is_2d_scale_translation: false, // TODO track this
                        should_snap: false,
                    },
                    SpatialFrameId::widget_id_to_wr(self.widget_id, self.pipeline_id),
                );
            }

            if !filters.is_empty() || !flags.is_empty() {
                // we want to apply filters in the top-to-bottom, left-to-right order they appear in
                // the widget declaration, but the widget declaration expands to have the top property
                // node be inside the bottom property node, so the bottom property ends up inserting
                // a filter first, because we cannot insert filters after the child node render is called
                // so we need to reverse the filters here. Left-to-right sequences are reversed on insert
                // so they get reversed again here and everything ends up in order.
                filters.reverse();

                self.widget_display_mode |= WidgetDisplayMode::STACKING_CONTEXT;

                self.display_list.push_simple_stacking_context_with_filters(
                    PxPoint::zero().to_wr(),
                    self.spatial_id,
                    flags,
                    &filters,
                    &[],
                    &[],
                )
            }
        } // else already started widget display
    }

    fn close_widget_display(&mut self) {
        if self.widget_display_mode.contains(WidgetDisplayMode::STACKING_CONTEXT) {
            self.display_list.pop_stacking_context();
        }
        if self.widget_display_mode.contains(WidgetDisplayMode::REFERENCE_FRAME) {
            self.display_list.pop_reference_frame();
            self.spatial_id = self.parent_spatial_id;
        }
        self.widget_display_mode = WidgetDisplayMode::empty();
    }

    /// Cancel the current [`push_widget`](Self::push_widget) if we are
    /// still in before [`open_widget_display`](Self::open_widget_display).
    pub fn cancel_widget(&mut self) -> Result<(), WidgetStartedError> {
        if self.widget_stack_ctx_data.is_some() || self.cancel_widget {
            self.widget_stack_ctx_data = None;
            self.cancel_widget = true;
            Ok(())
        } else {
            Err(WidgetStartedError)
        }
    }

    /// Gets if [`cancel_widget`](Self::cancel_widget) was requested.
    ///
    /// When this is `true` all other methods just pretend to work until the [`push_widget`](Self::push_widget) ends.
    #[inline]
    pub fn is_cancelling_widget(&self) -> bool {
        self.cancel_widget
    }

    /// Calls `f` inside a new widget context.
    pub fn push_widget(
        &mut self,
        id: WidgetId,
        transform_key: WidgetTransformKey,
        rendered: &WidgetRendered,
        f: impl FnOnce(&mut Self),
    ) {
        if self.cancel_widget {
            rendered.set(false);
            return;
        }

        // NOTE: root widget is not processed by this method, if you add widget behavior here
        // similar behavior must be added in the `new` and `finalize` methods.

        self.widget_stack_ctx_data = Some((RenderTransform::identity(), Vec::default(), PrimitiveFlags::empty()));

        let parent_id = mem::replace(&mut self.widget_id, id);
        let parent_rendered = mem::take(&mut self.widget_rendered);
        let parent_transform_key = mem::replace(&mut self.widget_transform_key, transform_key);
        let parent_display_mode = mem::replace(&mut self.widget_display_mode, WidgetDisplayMode::empty());

        f(self);

        if self.cancel_widget {
            self.cancel_widget = false;
            self.widget_rendered = false;
        } else {
            self.close_widget_display();
        }

        rendered.set(self.widget_rendered);

        self.widget_id = parent_id;
        self.widget_rendered = parent_rendered;
        self.widget_transform_key = parent_transform_key;
        self.widget_display_mode = parent_display_mode;
    }

    /// Push a hit-test `rect` using [`common_item_ps`].
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    #[inline]
    pub fn push_hit_test(&mut self, rect: PxRect) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        let common_item_ps = self.common_item_ps(rect);
        self.display_list.push_hit_test(&common_item_ps, self.item_tag());
    }

    /// Calls `f` with a new [`clip_id`] that clips to `bounds`.
    ///
    /// [`clip_id`]: FrameBuilder::clip_id
    #[inline]
    pub fn push_simple_clip(&mut self, bounds: PxSize, f: impl FnOnce(&mut FrameBuilder)) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        let parent_clip_id = self.clip_id;

        self.clip_id = self.display_list.define_clip_rect(
            &SpaceAndClipInfo {
                spatial_id: self.spatial_id,
                clip_id: self.clip_id,
            },
            PxRect::from_size(bounds).to_wr(),
        );

        f(self);

        self.clip_id = parent_clip_id;
    }

    /// Calls `f` inside a scroll viewport space.
    pub fn push_scroll_frame(
        &mut self,
        scroll_id: ScrollId,
        viewport_size: PxSize,
        content_rect: PxRect,
        f: impl FnOnce(&mut FrameBuilder),
    ) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        let parent_spatial_id = self.spatial_id;

        self.spatial_id = self.display_list.define_scroll_frame(
            parent_spatial_id,
            scroll_id.to_wr(self.pipeline_id),
            content_rect.to_wr(),
            PxRect::from_size(viewport_size).to_wr(),
            content_rect.origin.to_vector().to_wr(),
            SpatialFrameId::scroll_id_to_wr(scroll_id, self.pipeline_id),
        );

        f(self);

        self.spatial_id = parent_spatial_id;
    }

    // TODO use the widget transform instead of calling this method.
    /// Calls `f` inside a new reference frame at `origin`.
    #[inline]
    pub fn push_reference_frame(&mut self, id: SpatialFrameId, origin: PxPoint, f: impl FnOnce(&mut Self)) {
        self.push_reference_frame_(id.to_wr(self.pipeline_id), origin, f)
    }

    /// Calls `f` inside a new reference frame at `origin`.
    ///
    /// The reference frame is identified by a [`SpatialFrameId`] and `index`. This method can be used
    /// to set the offset of multiple child nodes without needing to generate a full frame ID for each item.
    #[inline]
    pub fn push_reference_frame_item(&mut self, list_id: SpatialFrameId, index: usize, origin: PxPoint, f: impl FnOnce(&mut Self)) {
        self.push_reference_frame_(list_id.item_to_wr(index, self.pipeline_id), origin, f)
    }

    fn push_reference_frame_(&mut self, key: SpatialTreeItemKey, origin: PxPoint, f: impl FnOnce(&mut Self)) {
        if self.cancel_widget {
            return;
        }

        if origin == PxPoint::zero() {
            return f(self);
        }

        self.open_widget_display();

        let parent_spatial_id = self.spatial_id;
        self.spatial_id = self.display_list.push_reference_frame(
            origin.to_wr(),
            parent_spatial_id,
            TransformStyle::Flat,
            PropertyBinding::default(),
            ReferenceFrameKind::Transform {
                is_2d_scale_translation: false,
                should_snap: false,
            },
            key,
        );

        f(self);

        self.display_list.pop_reference_frame();
        self.spatial_id = parent_spatial_id;
    }

    /// Calls `f` inside a new reference frame transformed by `transform`.
    #[inline]
    pub fn push_transform(&mut self, id: SpatialFrameId, transform: FrameBinding<RenderTransform>, f: impl FnOnce(&mut Self)) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        let parent_spatial_id = self.spatial_id;
        self.spatial_id = self.display_list.push_reference_frame(
            PxPoint::zero().to_wr(),
            parent_spatial_id,
            TransformStyle::Flat,
            transform,
            ReferenceFrameKind::Transform {
                is_2d_scale_translation: false,
                should_snap: false,
            },
            id.to_wr(self.pipeline_id),
        );

        f(self);

        self.display_list.pop_reference_frame();
        self.spatial_id = parent_spatial_id;
    }

    /// Push a border using [`common_item_ps`].
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    #[inline]
    pub fn push_border(&mut self, bounds: PxRect, widths: PxSideOffsets, sides: BorderSides, radius: PxCornerRadius) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        let details = BorderDetails::Normal(NormalBorder {
            left: sides.left.into(),
            right: sides.right.into(),
            top: sides.top.into(),
            bottom: sides.bottom.into(),
            radius: radius.to_border_radius(),
            do_aa: true,
        });

        let info = self.common_hit_item_ps(bounds);

        self.display_list.push_border(&info, bounds.to_wr(), widths.to_wr(), details);
    }

    /// Push a text run using [`common_item_ps`].
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    #[inline]
    pub fn push_text(&mut self, rect: PxRect, glyphs: &[GlyphInstance], font: &impl Font, color: ColorF, synthesis: FontSynthesis) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        if let Some(r) = &self.renderer {
            let instance_key = font.instance_key(r, synthesis);

            let item = self.common_hit_item_ps(rect);
            self.display_list.push_text(&item, rect.to_wr(), glyphs, instance_key, color, None);
        } else {
            self.widget_rendered();
        }
    }

    /// Push an image using [`common_item_ps`].
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    pub fn push_image(&mut self, clip_rect: PxRect, img_size: PxSize, image: &impl Image, rendering: ImageRendering) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        if let Some(r) = &self.renderer {
            let image_key = image.image_key(r);
            let item = self.common_hit_item_ps(clip_rect);
            self.display_list.push_image(
                &item,
                PxRect::from_size(img_size).to_wr(),
                rendering.into(),
                image.alpha_type(),
                image_key,
                RenderColor::WHITE,
            )
        } else {
            self.widget_rendered();
        }
    }

    /// Push a color rectangle using [`common_item_ps`](FrameBuilder::common_item_ps).
    #[inline]
    pub fn push_color(&mut self, rect: PxRect, color: FrameBinding<RenderColor>) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        let item = self.common_hit_item_ps(rect);
        self.display_list.push_rect_with_animation(&item, rect.to_wr(), color);
    }

    /// Push a repeating linear gradient rectangle using [`common_item_ps`].
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn push_linear_gradient(
        &mut self,
        rect: PxRect,
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

        if self.cancel_widget {
            return;
        }

        self.open_widget_display();
        let item = self.common_hit_item_ps(rect);

        self.display_list.push_stops(stops);

        let gradient = Gradient {
            start_point: line.start.to_wr(),
            end_point: line.end.to_wr(),
            extend_mode,
        };
        self.display_list
            .push_gradient(&item, rect.to_wr(), gradient, tile_size.to_wr(), tile_spacing.to_wr());
    }

    /// Push a repeating radial gradient rectangle using [`common_item_ps`].
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The  `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The `center` point is relative to the top-left of the tile, the `radius` is the distance between the first
    /// and last color stop in both directions and must be a non-zero positive value.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn push_radial_gradient(
        &mut self,
        rect: PxRect,
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

        if self.cancel_widget {
            return;
        }

        self.open_widget_display();
        let item = self.common_hit_item_ps(rect);

        self.display_list.push_stops(stops);

        let gradient = RadialGradient {
            center: center.to_wr(),
            radius: radius.to_wr(),
            start_offset: 0.0, // TODO expose this?
            end_offset: 1.0,
            extend_mode,
        };
        self.display_list
            .push_radial_gradient(&item, rect.to_wr(), gradient, tile_size.to_wr(), tile_spacing.to_wr())
    }

    /// Push a repeating conic gradient rectangle using [`common_item_ps`].
    ///
    /// The gradient fills the `tile_size`, the tile is repeated to fill the `rect`.
    /// The  `extend_mode` controls how the gradient fills the tile after the last color stop is reached.
    ///
    /// The gradient `stops` must be normalized, first stop at 0.0 and last stop at 1.0, this
    /// is asserted in debug builds.
    ///
    /// [`common_item_ps`]: FrameBuilder::common_item_ps
    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn push_conic_gradient(
        &mut self,
        rect: PxRect,
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

        if self.cancel_widget {
            return;
        }

        self.open_widget_display();
        let item = self.common_hit_item_ps(rect);

        self.display_list.push_stops(stops);

        GradientBuilder::new();

        let gradient = ConicGradient {
            center: center.to_wr(),
            angle: angle.0,
            start_offset: 0.0, // TODO expose this?
            end_offset: 1.0,
            extend_mode,
        };
        self.display_list
            .push_conic_gradient(&item, rect.to_wr(), gradient, tile_size.to_wr(), tile_spacing.to_wr())
    }

    /// Push a styled vertical or horizontal line.
    #[inline]
    pub fn push_line(
        &mut self,
        bounds: PxRect,
        orientation: crate::border::LineOrientation,
        color: RenderColor,
        style: crate::border::LineStyle,
    ) {
        if self.cancel_widget {
            return;
        }

        self.open_widget_display();

        let item = self.common_hit_item_ps(bounds);

        match style.render_command() {
            RenderLineCommand::Line(style, thickness) => {
                self.display_list
                    .push_line(&item, &bounds.to_wr(), thickness, orientation.into(), &color, style)
            }
            RenderLineCommand::Border(style) => {
                use crate::border::LineOrientation as LO;
                let widths = match orientation {
                    LO::Vertical => PxSideOffsets::new(Px(0), Px(0), Px(0), bounds.width()),
                    LO::Horizontal => PxSideOffsets::new(bounds.height(), Px(0), Px(0), Px(0)),
                };
                let details = BorderDetails::Normal(NormalBorder {
                    left: BorderSide { color, style },
                    right: BorderSide {
                        color: RenderColor::TRANSPARENT,
                        style: BorderStyle::Hidden,
                    },
                    top: BorderSide { color, style },
                    bottom: BorderSide {
                        color: RenderColor::TRANSPARENT,
                        style: BorderStyle::Hidden,
                    },
                    radius: BorderRadius::uniform(0.0),
                    do_aa: false,
                });

                self.display_list.push_border(&item, bounds.to_wr(), widths.to_wr(), details);
            }
        }
    }

    /// Push a `color` dot to mark the `offset`.
    ///
    /// The *dot* is a circle of the `color` highlighted by an white outline and shadow.
    #[inline]
    pub fn push_debug_dot(&mut self, offset: PxPoint, color: impl Into<RenderColor>) {
        let scale = self.scale_factor();

        let radius = PxSize::splat(Px(6)) * scale;
        let color = color.into();

        let mut builder = GradientBuilder::new();
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

        let common_item_ps = self.common_item_ps(PxRect::new(offset, bounds));
        self.display_list.push_stops(&stops);
        self.display_list.push_radial_gradient(
            &common_item_ps,
            PxRect::new(offset, bounds).to_wr(),
            gradient,
            bounds.to_wr(),
            PxSize::zero().to_wr(),
        )
    }

    /// Finalizes the build.
    ///
    /// # Returns
    ///
    /// `(PipelineId, BuiltDisplayList)` : The display list finalize data.
    /// `RenderColor`: The clear color.
    /// `FrameInfo`: The built frame info.
    pub fn finalize(mut self, root_rendered: &WidgetRendered) -> (BuiltFrame, UsedFrameBuilder) {
        self.close_widget_display();

        root_rendered.set(self.widget_rendered);

        *self.layers.get_mut(&self.layer_index).unwrap() = Some(self.display_list);
        let mut used_layers = LinearMap::with_capacity(self.layers.len());

        let mut layers = self.layers.into_iter();

        let (bottom_index, final_list) = layers.next().unwrap();
        let mut final_list = final_list.unwrap();
        for (index, dl) in layers {
            let mut dl = dl.unwrap();
            todo!();
        }

        let (pipeline_id, display_list) = final_list.end();
        let (payload, descriptor) = display_list.into_data();
        let clear_color = self.clear_color.unwrap_or(RenderColor::WHITE);

        used_layers.insert(bottom_index, final_list);
        let reuse = UsedFrameBuilder { used_layers };

        let frame = BuiltFrame {
            id: self.frame_id,
            pipeline_id,
            display_list: (payload, descriptor),
            clear_color,
        };

        (frame, reuse)
    }
}

/// Output of a [`FrameBuilder`].
pub struct BuiltFrame {
    /// Frame id.
    pub id: FrameId,
    /// Pipeline.
    pub pipeline_id: PipelineId,
    /// Built display list.
    pub display_list: (DisplayListPayload, BuiltDisplayListDescriptor),
    /// Clear color selected for the frame.
    pub clear_color: RenderColor,
}

/// Data from a previous [`FrameBuilder`], can be reuse in the next frame for a performance boost.
pub struct UsedFrameBuilder {
    used_layers: LinearMap<LayerIndex, DisplayListBuilder>,
}
impl UsedFrameBuilder {
    /// Pipeline where this frame builder can be reused.
    pub fn pipeline_id(&self) -> PipelineId {
        self.used_layers
            .values()
            .next()
            .map(|p| p.pipeline_id)
            .unwrap_or_else(PipelineId::dummy)
    }
}
enum RenderLineCommand {
    Line(LineStyle, f32),
    Border(BorderStyle),
}
impl crate::border::LineStyle {
    fn render_command(self) -> RenderLineCommand {
        use crate::border::LineStyle as LS;
        use RenderLineCommand::*;
        match self {
            LS::Solid => Line(LineStyle::Solid, 0.0),
            LS::Double => Border(BorderStyle::Double),
            LS::Dotted => Line(LineStyle::Dotted, 0.0),
            LS::Dashed => Line(LineStyle::Dashed, 0.0),
            LS::Groove => Border(BorderStyle::Groove),
            LS::Ridge => Border(BorderStyle::Ridge),
            LS::Wavy(thickness) => Line(LineStyle::Wavy, thickness),
            LS::Hidden => Border(BorderStyle::Hidden),
        }
    }
}

/// Attempt to modify/cancel a widget transform or filters when it already started
/// pushing display items.
#[derive(Debug, dm::Display, dm::Error)]
#[display(fmt = "cannot modify widget transform or filters, widget display items already pushed")]
pub struct WidgetStartedError;

/// A frame quick update.
///
/// A frame update causes a frame render without needing to fully rebuild the display list. It
/// is a more performant but also more limited way of generating a frame.
///
/// Any [`FrameBindingKey`] used in the creation of the frame can be used for updating the frame.
pub struct FrameUpdate {
    bindings: DynamicProperties,
    current_clear_color: RenderColor,
    clear_color: Option<RenderColor>,
    scrolls: Vec<(ExternalScrollId, PxVector)>,
    frame_id: FrameId,
    window_id: WindowId,
    widget_id: WidgetId,
    widget_transform: RenderTransform,
    widget_transform_key: WidgetTransformKey,
    cancel_widget: bool,
}
impl FrameUpdate {
    /// New frame update builder.
    ///
    /// * `window_id` - Id of the window that owns the frame.
    /// * `root_id` - Id of the widget at the root of the frame.
    /// * `root_transform_key` - Frame binding for the root widget layout transform.
    /// * `frame_id` - Id of the frame that will be updated.
    /// * `clear_color` - The current clear color.
    /// * `used_data` - Data generated by a previous frame update, if set is recycled for a performance boost.
    pub fn new(
        window_id: WindowId,
        root_id: WidgetId,
        root_transform_key: WidgetTransformKey,
        frame_id: FrameId,
        clear_color: RenderColor,
        used_data: Option<UsedFrameUpdate>,
    ) -> Self {
        let hint = used_data.unwrap_or(UsedFrameUpdate {
            scrolls_capacity: 10,
            transforms_capacity: 100,
            floats_capacity: 100,
            colors_capacity: 100,
        });
        FrameUpdate {
            bindings: DynamicProperties {
                transforms: Vec::with_capacity(hint.transforms_capacity),
                floats: Vec::with_capacity(hint.floats_capacity),
                colors: Vec::with_capacity(hint.colors_capacity),
            },
            scrolls: Vec::with_capacity(hint.scrolls_capacity),
            window_id,
            clear_color: None,
            widget_id: root_id,
            widget_transform: RenderTransform::identity(),
            widget_transform_key: root_transform_key,
            frame_id,
            current_clear_color: clear_color,
            cancel_widget: false,
        }
    }

    /// Window that owns the frame.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Current widget.
    #[inline]
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// The frame that will be updated.
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// Change the color used to clear the pixel buffer when redrawing the frame.
    #[inline]
    pub fn set_clear_color(&mut self, color: RenderColor) {
        self.clear_color = Some(color);
    }

    /// Includes the widget transform.
    #[inline]
    pub fn with_widget_transform(&mut self, transform: &RenderTransform, f: impl FnOnce(&mut Self)) {
        self.widget_transform = self.widget_transform.then(transform);
        f(self);
    }

    /// Update a layout transform value.
    #[inline]
    pub fn update_transform(&mut self, new_value: FrameValue<RenderTransform>) {
        self.bindings.transforms.push(new_value);
    }

    /// Update a float value.
    #[inline]
    pub fn update_f32(&mut self, new_value: FrameValue<f32>) {
        self.bindings.floats.push(new_value);
    }

    /// Update a color value.
    #[inline]
    pub fn update_color(&mut self, new_value: FrameValue<RenderColor>) {
        self.bindings.colors.push(new_value)
    }

    /// Update a scroll frame offset.
    ///
    /// The `offset` is added to the offset used in the last full frame render.
    #[inline]
    pub fn update_scroll(&mut self, id: ExternalScrollId, offset: PxVector) {
        self.scrolls.push((id, offset))
    }

    /// Calls [`render_update`](UiNode::render_update) for `child` inside a new widget context.
    #[inline]
    pub fn update_widget(&mut self, id: WidgetId, transform_key: WidgetTransformKey, child: &impl UiNode, ctx: &mut RenderContext) {
        if self.cancel_widget {
            return;
        }

        // NOTE: root widget is not processed by this method, if you add widget behavior here
        // similar behavior must be added in the `new` and `finalize` methods.

        let parent_id = mem::replace(&mut self.widget_id, id);
        let parent_transform_key = mem::replace(&mut self.widget_transform_key, transform_key);
        let parent_transform = mem::replace(&mut self.widget_transform, RenderTransform::identity());
        let transforms_len = self.bindings.transforms.len();
        let floats_len = self.bindings.floats.len();
        let colors_len = self.bindings.colors.len();

        child.render_update(ctx, self);

        self.widget_id = parent_id;
        self.widget_transform_key = parent_transform_key;

        if self.cancel_widget {
            self.cancel_widget = false;
            self.widget_transform = parent_transform;
            self.bindings.transforms.truncate(transforms_len);
            self.bindings.floats.truncate(floats_len);
            self.bindings.colors.truncate(colors_len);
        } else {
            let widget_transform = mem::replace(&mut self.widget_transform, parent_transform);
            if widget_transform != RenderTransform::identity() {
                self.update_transform(self.widget_transform_key.update(widget_transform));
            }
        }
    }

    /// Rollback the current [`update_widget`](Self::update_widget).
    pub fn cancel_widget(&mut self) {
        self.cancel_widget = true;
    }

    /// Finalize the update.
    ///
    /// Returns the property updates, scroll updates and the new clear color if any was set.
    pub fn finalize(mut self) -> (BuiltFrameUpdate, UsedFrameUpdate) {
        if self.widget_transform != RenderTransform::identity() {
            self.update_transform(self.widget_transform_key.update(self.widget_transform));
        }

        if self.clear_color == Some(self.current_clear_color) {
            self.clear_color = None;
        }

        let used = UsedFrameUpdate {
            scrolls_capacity: self.scrolls.len(),
            transforms_capacity: self.bindings.transforms.len(),
            floats_capacity: self.bindings.floats.len(),
            colors_capacity: self.bindings.colors.len(),
        };

        let update = BuiltFrameUpdate {
            bindings: self.bindings,
            scrolls: self.scrolls,
            clear_color: self.clear_color,
        };

        (update, used)
    }
}

/// Output of a [`FrameBuilder`].
pub struct BuiltFrameUpdate {
    /// Webrender frame properties updates.
    pub bindings: DynamicProperties,
    /// Scroll updates.
    pub scrolls: Vec<(ExternalScrollId, PxVector)>,
    /// New clear color.
    pub clear_color: Option<RenderColor>,
}

/// Data from a previous [`FrameUpdate`], can be reuse in the next frame for a performance boost.
#[derive(Clone, Copy)]
pub struct UsedFrameUpdate {
    scrolls_capacity: usize,
    transforms_capacity: usize,
    floats_capacity: usize,
    colors_capacity: usize,
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

unique_id_64! {
    /// Unique ID of a scroll viewport.
    #[derive(Debug)]
    pub struct ScrollId;
}
unique_id_32! {
    /// Unique ID of a reference frame.
    #[derive(Debug)]
    pub struct SpatialFrameId;
}

impl ScrollId {
    /// Id of the implicit scroll ID at the root of all frames.
    ///
    /// This [`ExternalScrollId`] cannot be represented by [`ScrollId`] because
    /// it is the zero value.
    #[inline]
    pub fn wr_root(pipeline_id: PipelineId) -> ExternalScrollId {
        ExternalScrollId(0, pipeline_id)
    }

    /// To webrender [`ExternalScrollId`].
    #[inline]
    pub fn to_wr(self, pipeline_id: PipelineId) -> ExternalScrollId {
        ExternalScrollId(self.get(), pipeline_id)
    }
}

impl SpatialFrameId {
    const WIDGET_ID_FLAG: u64 = 1 << 63;
    const SCROLL_ID_FLAG: u64 = 1 << 62;
    const LIST_ID_FLAG: u64 = 1 << 61;

    /// Make a [`SpatialTreeItemKey`] from a [`WidgetId`], there is no collision
    /// with other keys generated.
    #[inline]
    pub fn widget_id_to_wr(self_: WidgetId, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        SpatialTreeItemKey::new(pipeline_id.0 as u64 | Self::WIDGET_ID_FLAG, self_.get())
    }

    /// Make a [`SpatialTreeItemKey`] from a [`ScrollId`], there is no collision
    /// with other keys generated.
    #[inline]
    pub fn scroll_id_to_wr(self_: ScrollId, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        SpatialTreeItemKey::new(pipeline_id.0 as u64 | Self::SCROLL_ID_FLAG, self_.get())
    }

    /// To webrender [`SpatialTreeItemKey`].
    #[inline]
    pub fn to_wr(self, pipeline_id: PipelineId) -> SpatialTreeItemKey {
        SpatialTreeItemKey::new(pipeline_id.0 as u64, self.get() as u64)
    }

    /// Make [`SpatialTreeItemKey`] from a a spatial parent + item index, there is no collision
    /// with other keys generated.
    #[inline]
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
    #[inline]
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
    #[inline]
    pub fn bind(self, value: T) -> FrameBinding<T> {
        FrameBinding::Binding(self.property_key(), value)
    }

    /// Create a value update with this key.
    #[inline]
    pub fn update(self, value: T) -> FrameValue<T> {
        FrameValue {
            key: self.property_key(),
            value,
        }
    }
}

/// `FrameBindingKey<LayoutTransform>`.
pub type WidgetTransformKey = FrameBindingKey<RenderTransform>;

/// A hit-test hit.
#[derive(Clone, Debug)]
pub struct HitInfo {
    /// ID of widget hit.
    pub widget_id: WidgetId,
}

/// A hit-test result.
#[derive(Clone, Debug)]
pub struct FrameHitInfo {
    window_id: WindowId,
    frame_id: FrameId,
    point: PxPoint,
    hits: Vec<HitInfo>,
}
impl FrameHitInfo {
    /// Initializes from a Webrender hit-test result.
    ///
    /// Only item tags produced by [`FrameBuilder`] are expected.
    ///
    /// The tag format is:
    ///
    /// * `u64`: Raw [`WidgetId`].
    /// * `u16`: Zero, reserved.
    #[inline]
    pub fn new(window_id: WindowId, frame_id: FrameId, point: PxPoint, hits: &HitTestResult) -> Self {
        let hits = hits
            .items
            .iter()
            .filter_map(|h| {
                if h.tag.0 == 0 || h.tag.1 != 0 {
                    None
                } else {
                    // SAFETY: we skip zero so the value is memory safe.
                    let widget_id = unsafe { WidgetId::from_raw(h.tag.0) };
                    Some(HitInfo { widget_id })
                }
            })
            .collect();

        FrameHitInfo {
            window_id,
            frame_id,
            point,
            hits,
        }
    }

    /// No hits info
    #[inline]
    pub fn no_hits(window_id: WindowId) -> Self {
        FrameHitInfo::new(window_id, FrameId::INVALID, PxPoint::new(Px(-1), Px(-1)), &HitTestResult::default())
    }

    /// The window that was hit-tested.
    #[inline]
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// The window frame that was hit-tested.
    #[inline]
    pub fn frame_id(&self) -> FrameId {
        self.frame_id
    }

    /// The point in the window that was hit-tested.
    #[inline]
    pub fn point(&self) -> PxPoint {
        self.point
    }

    /// All hits, from top-most.
    #[inline]
    pub fn hits(&self) -> &[HitInfo] {
        &self.hits
    }

    /// The top hit.
    #[inline]
    pub fn target(&self) -> Option<&HitInfo> {
        self.hits.first()
    }

    /// Finds the widget in the hit-test result if it was hit.
    #[inline]
    pub fn find(&self, widget_id: WidgetId) -> Option<&HitInfo> {
        self.hits.iter().find(|h| h.widget_id == widget_id)
    }

    /// If the widget is in was hit.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.hits.iter().any(|h| h.widget_id == widget_id)
    }

    /// Gets a clone of `self` that only contains the hits that also happen in `other`.
    #[inline]
    pub fn intersection(&self, other: &FrameHitInfo) -> FrameHitInfo {
        let mut hits: Vec<_> = self.hits.iter().filter(|h| other.contains(h.widget_id)).cloned().collect();
        hits.shrink_to_fit();

        FrameHitInfo {
            window_id: self.window_id,
            frame_id: self.frame_id,
            point: self.point,
            hits,
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
    #[inline]
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

/// Represents a layer in and of a [`FrameBuilder`].
///
/// See the [`with_layer`] method for more information.
///
/// [`with_layer`]: FrameBuilder::with_layer
#[derive(Default, Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
pub struct LayerIndex(pub u32);
impl LayerIndex {
    /// The top-most layer inside a [`FrameBuilder`].
    ///
    /// Only widgets that are pretending to be a child window should use this layer, including menus,
    /// drop-downs, pop-ups and tool-tips.
    ///
    /// This is the [`u32::MAX`] value.
    pub const TOP_MOST: LayerIndex = LayerIndex(u32::MAX);

    /// The layer for *adorner* display items.
    ///
    /// Adorner widgets are related to another widget but not as a visual part of it, examples of adorners
    /// are resize handles in a widget visual editor, or an interactive help/guide feature.
    ///
    /// This is the [`TOP_MOST - u16::MAX`] value.
    pub const ADORNERS: LayerIndex = LayerIndex(Self::TOP_MOST.0 - u16::MAX as u32);

    /// The default layer of a window or headless surface contents.
    ///
    /// This is the `1000` value.
    pub const DEFAULT: LayerIndex = LayerIndex(1000);

    /// The top-most layer inside a [`FrameBuilder`].
    ///
    /// Note that if any of the other layers fills the frame the contents of this
    /// layer are not visible, for example, in a window default layer with `background_color` set. This
    /// does not apply to the clear color, see the [`set_clear_color`] method for more details.
    ///
    /// This is the `0` value.
    ///
    /// [`set_clear_color`]: FrameBuilder::set_clear_color
    pub const BOTTOM_MOST: LayerIndex = LayerIndex(0);

    /// Compute `self + other` saturating at the [`TOP_MOST`] bound instead of overflowing.
    ///
    /// [`TOP_MOST`]: Self::TOP_MOST
    pub fn saturating_add(self, other: impl Into<LayerIndex>) -> Self {
        Self(self.0.saturating_add(other.into().0))
    }

    /// Compute `self - other` saturating at the [`BOTTOM_MOST`] bound instead of overflowing.
    ///
    /// [`BOTTOM_MOST`]: Self::BOTTOM_MOST
    pub fn saturating_sub(self, other: impl Into<LayerIndex>) -> Self {
        Self(self.0.saturating_sub(other.into().0))
    }
}
impl_from_and_into_var! {
    fn from(index: u32) -> LayerIndex {
        LayerIndex(index)
    }
}
/// Calls [`LayerIndex::saturating_add`].
impl<T: Into<Self>> ops::Add<T> for LayerIndex {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        self.saturating_add(rhs)
    }
}
/// Calls [`LayerIndex::saturating_sub`].
impl<T: Into<Self>> ops::Sub<T> for LayerIndex {
    type Output = Self;

    fn sub(self, rhs: T) -> Self::Output {
        self.saturating_sub(rhs)
    }
}
/// Calls [`LayerIndex::saturating_add`].
impl<T: Into<Self>> ops::AddAssign<T> for LayerIndex {
    fn add_assign(&mut self, rhs: T) {
        *self = *self + rhs;
    }
}
/// Calls [`LayerIndex::saturating_sub`].
impl<T: Into<Self>> ops::SubAssign<T> for LayerIndex {
    fn sub_assign(&mut self, rhs: T) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn layer_index_ops() {
        let idx = LayerIndex::DEFAULT;

        let p1 = idx + 1;
        let m1 = idx - 1;

        let mut idx = idx;

        idx += 1;
        assert_eq!(idx, p1);

        idx -= 2;
        assert_eq!(idx, m1);
    }
}
