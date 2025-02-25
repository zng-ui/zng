use std::{cell::Cell, mem};

use rustc_hash::FxHashMap;
use webrender::api as wr;
use zng_unit::{Factor, Px, PxCornerRadius, PxRect, PxSideOffsets, PxSize, PxTransform, Rgba};
use zng_view_api::{
    GradientStop, ReferenceFrameId, RepeatMode,
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    display_list::{DisplayItem, DisplayList, FilterOp, FrameValue, FrameValueId, FrameValueUpdate, NinePatchSource, SegmentId},
    font::{GlyphIndex, GlyphInstance},
    window::FrameId,
};

use crate::px_wr::PxToWr;

pub fn display_list_to_webrender(
    list: DisplayList,
    ext: &mut dyn DisplayListExtension,
    cache: &mut DisplayListCache,
) -> wr::BuiltDisplayList {
    let r = display_list_build(&list, cache, ext, false);
    cache.insert(list);

    r
}

/// Handler for display list extension items.
///
/// Note that this is for extensions that still use Webrender, to generate normal
/// Webrender items or *blobs* that are Webrender's own extension mechanism.
/// Custom renderers can just inspect the display list directly.
///
/// This trait is implemented for `()` for view implementations that don't provide any extension.
pub trait DisplayListExtension {
    /// Handle new display list starting.
    ///
    /// This is called for every list, even if the list does not have any item for the extension.
    fn display_list_start(&mut self, args: &mut DisplayExtensionArgs) {
        let _ = args;
    }

    /// Handle extension push.
    ///
    /// This is only called for items addressing the extension.
    fn push_display_item(&mut self, args: &mut DisplayExtensionItemArgs);
    /// Handle extension pop.
    ///
    /// This is only called for items addressing the extension.
    fn pop_display_item(&mut self, args: &mut DisplayExtensionItemArgs) {
        let _ = args;
    }

    /// Handle display list finishing.
    fn display_list_end(&mut self, args: &mut DisplayExtensionArgs) {
        let _ = args;
    }

    /// Handle extension update.
    fn update(&mut self, args: &mut DisplayExtensionUpdateArgs) {
        let _ = args;
    }
}
impl DisplayListExtension for () {
    fn push_display_item(&mut self, args: &mut DisplayExtensionItemArgs) {
        let _ = args;
    }
}

/// Arguments for [`DisplayListExtension`] begin/end list.
pub struct DisplayExtensionArgs<'a> {
    /// The webrender display list.
    pub list: &'a mut wr::DisplayListBuilder,
    /// Space and clip tracker.
    pub sc: &'a mut SpaceAndClip,
}

/// Arguments for [`DisplayListExtension`] push and pop.
pub struct DisplayExtensionItemArgs<'a> {
    /// Extension index.
    pub extension_id: ApiExtensionId,
    /// Push payload, is empty for pop.
    pub payload: &'a ApiExtensionPayload,
    /// If the display item is reused.
    ///
    /// If `true` the payload is the same as received before any updates, the updated
    /// values must be applied to value deserialized from the payload.
    pub is_reuse: bool,
    /// The webrender display list.
    pub list: &'a mut wr::DisplayListBuilder,
    /// Space and clip tracker.
    pub sc: &'a mut SpaceAndClip,
}

/// Arguments for [`DisplayListExtension`] update.
pub struct DisplayExtensionUpdateArgs<'a> {
    /// Extension index.
    pub extension_id: ApiExtensionId,
    /// Update payload.
    pub payload: &'a ApiExtensionPayload,

    /// Set to `true` to rebuild the display list.
    ///
    /// The list will be rebuild using the last full payload received, the extension
    /// must patch in any subsequent updates onto this value.
    pub new_frame: bool,

    /// Webrender binding updates.
    ///
    /// If no other extension and update handlers request a new frame these properties
    /// will be send to Webrender to update the current frame.
    pub properties: &'a mut wr::DynamicProperties,
}

/// Tracks the current space & clip chain, and the backface visibility primitive flag.
pub struct SpaceAndClip {
    spatial_stack: Vec<wr::SpatialId>,
    clip_stack: Vec<wr::ClipId>,
    clip_chain_stack: Vec<(wr::ClipChainId, usize)>,
    prim_flags: wr::PrimitiveFlags,
    view_process_frame_id: u64,
}
impl SpaceAndClip {
    pub(crate) fn new(pipeline_id: wr::PipelineId) -> Self {
        let sid = wr::SpatialId::root_reference_frame(pipeline_id);
        SpaceAndClip {
            spatial_stack: vec![sid],
            clip_stack: vec![],
            clip_chain_stack: vec![],
            prim_flags: wr::PrimitiveFlags::IS_BACKFACE_VISIBLE,
            view_process_frame_id: 0,
        }
    }

    /// Current space.
    pub fn spatial_id(&self) -> wr::SpatialId {
        self.spatial_stack[self.spatial_stack.len() - 1]
    }

    /// Current clip chain.
    pub fn clip_chain_id(&mut self, list: &mut wr::DisplayListBuilder) -> wr::ClipChainId {
        let mut start = 0;
        let mut parent = None;

        if let Some((id, i)) = self.clip_chain_stack.last().copied() {
            if i == self.clip_stack.len() {
                return id;
            } else {
                start = i;
                parent = Some(id);
            }
        }

        let clips = self.clip_stack[start..].iter().copied();
        let id = list.define_clip_chain(parent, clips);
        self.clip_chain_stack.push((id, self.clip_stack.len()));

        id
    }

    /// Push space.
    pub fn push_spatial(&mut self, spatial_id: wr::SpatialId) {
        self.spatial_stack.push(spatial_id);
    }

    /// Pop space.
    pub fn pop_spatial(&mut self) {
        self.spatial_stack.truncate(self.spatial_stack.len() - 1);
    }

    /// Push clip.
    pub fn push_clip(&mut self, clip: wr::ClipId) {
        self.clip_stack.push(clip);
    }

    /// Pop clip.
    pub fn pop_clip(&mut self) {
        self.clip_stack.truncate(self.clip_stack.len() - 1);

        if let Some((_, i)) = self.clip_chain_stack.last() {
            if *i > self.clip_stack.len() {
                self.clip_chain_stack.truncate(self.clip_chain_stack.len() - 1);
            }
        }
    }

    /// Gets the primitive flags for the item.
    pub fn primitive_flags(&self) -> wr::PrimitiveFlags {
        self.prim_flags
    }

    /// Set the `IS_BACKFACE_VISIBLE` flag to the next items.
    pub fn set_backface_visibility(&mut self, visible: bool) {
        self.prim_flags.set(wr::PrimitiveFlags::IS_BACKFACE_VISIBLE, visible);
    }

    /// Generate a reference frame ID, unique on this list.
    pub fn next_view_process_frame_id(&mut self) -> ReferenceFrameId {
        self.view_process_frame_id = self.view_process_frame_id.wrapping_add(1);
        ReferenceFrameId(self.view_process_frame_id, 1 << 63)
    }

    pub(crate) fn clear(&mut self, pipeline_id: wr::PipelineId) {
        #[cfg(debug_assertions)]
        {
            if self.clip_chain_stack.len() >= 2 {
                tracing::error!("found {} clip chains, expected 0 or 1", self.clip_chain_stack.len());
            }
            if !self.clip_stack.is_empty() {
                tracing::error!("found {} clips, expected 0", self.clip_stack.len());
            }
            if self.spatial_stack.len() != 1 {
                tracing::error!("found {} spatial, expected 1 root_reference_frame", self.spatial_stack.len());
            } else if self.spatial_stack[0].0 != 0 {
                tracing::error!("found other spatial id, expected root_reference_frame");
            }
        }

        self.clip_stack.clear();

        self.spatial_stack.clear();
        self.spatial_stack.push(wr::SpatialId::root_reference_frame(pipeline_id));

        self.clip_chain_stack.clear();

        self.prim_flags = wr::PrimitiveFlags::IS_BACKFACE_VISIBLE;
    }
}
struct CachedDisplayList {
    list: Vec<DisplayItem>,
    segments: Vec<(SegmentId, usize)>,
    used: Cell<bool>,
}

/// View process side cache of [`DisplayList`] frames for a pipeline.
pub struct DisplayListCache {
    pipeline_id: wr::PipelineId,
    id_namespace: wr::IdNamespace,
    lists: FxHashMap<FrameId, CachedDisplayList>,
    space_and_clip: Option<SpaceAndClip>,

    latest_frame: FrameId,
    bindings: FxHashMap<FrameValueId, (FrameId, usize)>,

    wr_list: Option<wr::DisplayListBuilder>,
}
impl DisplayListCache {
    /// New empty.
    pub fn new(pipeline_id: wr::PipelineId, id_namespace: wr::IdNamespace) -> Self {
        DisplayListCache {
            pipeline_id,
            id_namespace,
            lists: FxHashMap::default(),
            latest_frame: FrameId::INVALID,
            space_and_clip: Some(SpaceAndClip::new(pipeline_id)),
            bindings: FxHashMap::default(),
            wr_list: Some(wr::DisplayListBuilder::new(pipeline_id)),
        }
    }

    /// Keys namespace.
    pub fn id_namespace(&self) -> wr::IdNamespace {
        self.id_namespace
    }

    fn begin_wr(&mut self) -> (wr::DisplayListBuilder, SpaceAndClip) {
        let mut list = self.wr_list.take().unwrap();
        let sc = self.space_and_clip.take().unwrap();
        list.begin();
        (list, sc)
    }

    fn end_wr(&mut self, mut list: wr::DisplayListBuilder, mut sc: SpaceAndClip) -> wr::BuiltDisplayList {
        let r = list.end().1;
        self.wr_list = Some(list);
        sc.clear(self.pipeline_id);
        self.space_and_clip = Some(sc);
        r
    }

    #[expect(clippy::too_many_arguments)]
    fn reuse(
        &self,
        frame_id: FrameId,
        seg_id: SegmentId,
        mut start: usize,
        mut end: usize,
        wr_list: &mut wr::DisplayListBuilder,
        ext: &mut dyn DisplayListExtension,
        sc: &mut SpaceAndClip,
    ) {
        if let Some(l) = self.lists.get(&frame_id) {
            l.used.set(true);

            let offset = l
                .segments
                .iter()
                .find_map(|&(id, o)| if id == seg_id { Some(o) } else { None })
                .unwrap_or_else(|| {
                    tracing::error!("unknown segment id {seg_id}");
                    l.list.len()
                });
            start += offset;
            end += offset;

            let range = l.list.get(start..end).unwrap_or_else(|| {
                tracing::error!("invalid reuse range ({start}..{end}) ignored, offset: {offset}");
                &[]
            });
            for item in range {
                display_item_to_webrender(item, wr_list, ext, sc, self, true);
            }
        } else {
            tracing::error!("did not find reuse frame {frame_id:?}");
        }
    }

    fn insert(&mut self, list: DisplayList) {
        self.lists.retain(|_, l| l.used.take());

        let (frame_id, list, segments) = list.into_parts();

        for (i, item) in list.iter().enumerate() {
            display_item_register_bindings(item, &mut self.bindings, (frame_id, i));
        }

        self.latest_frame = frame_id;
        self.lists.insert(
            frame_id,
            CachedDisplayList {
                list,
                segments,
                used: Cell::new(false),
            },
        );
    }

    fn get_update_target(&mut self, id: FrameValueId) -> Option<&mut DisplayItem> {
        if let Some((frame_id, i)) = self.bindings.get(&id) {
            if let Some(list) = self.lists.get_mut(frame_id) {
                if let Some(item) = list.list.get_mut(*i) {
                    return Some(item);
                }
            }
        }
        None
    }

    /// Apply updates, returns the webrender update if the renderer can also be updated and there are any updates,
    /// or returns a new frame if a new frame must be rendered.
    #[expect(clippy::result_large_err)] // both are large
    pub fn update(
        &mut self,
        ext: &mut dyn DisplayListExtension,
        transforms: Vec<FrameValueUpdate<PxTransform>>,
        floats: Vec<FrameValueUpdate<f32>>,
        colors: Vec<FrameValueUpdate<Rgba>>,
        extensions: Vec<(ApiExtensionId, ApiExtensionPayload)>,
        resized: bool,
    ) -> Result<Option<wr::DynamicProperties>, wr::BuiltDisplayList> {
        let mut new_frame = resized;

        for t in &transforms {
            if let Some(item) = self.get_update_target(t.id) {
                new_frame |= item.update_transform(t);
            }
        }
        for t in &floats {
            if let Some(item) = self.get_update_target(t.id) {
                new_frame |= item.update_float(t);
            }
        }
        for t in &colors {
            if let Some(item) = self.get_update_target(t.id) {
                new_frame |= item.update_color(t);
            }
        }

        let mut properties = wr::DynamicProperties::default();

        for (k, e) in &extensions {
            let mut args = DisplayExtensionUpdateArgs {
                extension_id: *k,
                payload: e,
                new_frame: false,
                properties: &mut properties,
            };
            ext.update(&mut args);
            new_frame |= args.new_frame;
        }

        if new_frame {
            let list = self.lists.get_mut(&self.latest_frame).expect("no frame to update");
            let list = mem::take(&mut list.list);
            let r = display_list_build(&list, self, ext, true);
            self.lists.get_mut(&self.latest_frame).unwrap().list = list;

            Err(r)
        } else {
            properties.transforms.extend(transforms.into_iter().filter_map(PxToWr::to_wr));
            properties.floats.extend(floats.into_iter().filter_map(PxToWr::to_wr));
            properties.colors.extend(colors.into_iter().filter_map(PxToWr::to_wr));

            if properties.transforms.is_empty() && properties.floats.is_empty() && properties.colors.is_empty() {
                Ok(None)
            } else {
                Ok(Some(properties))
            }
        }
    }
}

fn display_list_build(
    list: &[DisplayItem],
    cache: &mut DisplayListCache,
    ext: &mut dyn DisplayListExtension,
    is_reuse: bool,
) -> wr::BuiltDisplayList {
    let _s = tracing::trace_span!("DisplayList::build").entered();

    let (mut wr_list, mut sc) = cache.begin_wr();

    ext.display_list_start(&mut DisplayExtensionArgs {
        list: &mut wr_list,
        sc: &mut sc,
    });

    for item in list {
        display_item_to_webrender(item, &mut wr_list, ext, &mut sc, cache, is_reuse);
    }

    ext.display_list_end(&mut DisplayExtensionArgs {
        list: &mut wr_list,
        sc: &mut sc,
    });

    cache.end_wr(wr_list, sc)
}

fn display_item_to_webrender(
    item: &DisplayItem,
    wr_list: &mut wr::DisplayListBuilder,
    ext: &mut dyn DisplayListExtension,
    sc: &mut SpaceAndClip,
    cache: &DisplayListCache,
    is_reuse: bool,
) {
    match item {
        DisplayItem::Reuse {
            frame_id,
            seg_id,
            start,
            end,
        } => cache.reuse(*frame_id, *seg_id, *start, *end, wr_list, ext, sc),

        DisplayItem::PushReferenceFrame {
            id,
            transform,
            transform_style,
            is_2d_scale_translation,
        } => {
            let spatial_id = wr_list.push_reference_frame(
                wr::units::LayoutPoint::zero(),
                sc.spatial_id(),
                transform_style.to_wr(),
                transform.to_wr(),
                wr::ReferenceFrameKind::Transform {
                    is_2d_scale_translation: *is_2d_scale_translation,
                    should_snap: false,
                    paired_with_perspective: false,
                },
                id.to_wr(),
            );
            sc.push_spatial(spatial_id);
        }
        DisplayItem::PopReferenceFrame => {
            wr_list.pop_reference_frame();
            sc.pop_spatial();
        }

        DisplayItem::PushStackingContext {
            blend_mode,
            transform_style,
            filters,
        } => {
            let clip = sc.clip_chain_id(wr_list);
            wr_list.push_stacking_context(
                wr::units::LayoutPoint::zero(),
                sc.spatial_id(),
                sc.primitive_flags(),
                Some(clip),
                transform_style.to_wr(),
                blend_mode.to_wr(),
                &filters.iter().map(|f| f.to_wr()).collect::<Vec<_>>(),
                &[],
                &[],
                wr::RasterSpace::Screen, // Local disables sub-pixel AA for performance (future perf.)
                wr::StackingContextFlags::empty(),
            )
        }
        DisplayItem::PopStackingContext => wr_list.pop_stacking_context(),

        DisplayItem::PushClipRect { clip_rect, clip_out } => {
            let clip_id = if *clip_out {
                wr_list.define_clip_rounded_rect(
                    sc.spatial_id(),
                    wr::ComplexClipRegion::new(clip_rect.to_wr(), PxCornerRadius::zero().to_wr(), wr::ClipMode::ClipOut),
                )
            } else {
                wr_list.define_clip_rect(sc.spatial_id(), clip_rect.to_wr())
            };

            sc.push_clip(clip_id);
        }
        DisplayItem::PushClipRoundedRect {
            clip_rect,
            corners,
            clip_out,
        } => {
            let clip_id = wr_list.define_clip_rounded_rect(
                sc.spatial_id(),
                wr::ComplexClipRegion::new(
                    clip_rect.to_wr(),
                    corners.to_wr(),
                    if *clip_out { wr::ClipMode::ClipOut } else { wr::ClipMode::Clip },
                ),
            );
            sc.push_clip(clip_id);
        }
        DisplayItem::PopClip => sc.pop_clip(),

        DisplayItem::PushMask { image_id, rect } => {
            let clip_id = wr_list.define_clip_image_mask(
                sc.spatial_id(),
                wr::ImageMask {
                    image: wr::ImageKey(cache.id_namespace(), image_id.get()),
                    rect: rect.to_wr(),
                },
                &[],
                wr::FillRule::Nonzero,
            );
            sc.push_clip(clip_id);
            let clip = sc.clip_chain_id(wr_list);
            wr_list.push_stacking_context(
                wr::units::LayoutPoint::zero(),
                sc.spatial_id(),
                sc.primitive_flags(),
                Some(clip),
                wr::TransformStyle::Flat,
                wr::MixBlendMode::Normal,
                &[],
                &[],
                &[],
                wr::RasterSpace::Screen,
                wr::StackingContextFlags::empty(),
            );
        }
        DisplayItem::PopMask => {
            wr_list.pop_stacking_context();
            sc.pop_clip();
        }

        DisplayItem::SetBackfaceVisibility { visible } => {
            sc.set_backface_visibility(*visible);
        }

        DisplayItem::Text {
            clip_rect,
            font_id,
            glyphs,
            color,
            options,
        } => {
            let bounds = clip_rect.to_wr();
            let clip = sc.clip_chain_id(wr_list);
            wr_list.push_text(
                &wr::CommonItemProperties {
                    clip_rect: bounds,
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                bounds,
                cast_glyphs_to_wr(glyphs),
                wr::FontInstanceKey(cache.id_namespace(), font_id.get()),
                color.into_value().to_wr(),
                options.clone().to_wr_world(),
            );
        }

        DisplayItem::Color { clip_rect, color } => {
            let bounds = clip_rect.to_wr();
            let clip = sc.clip_chain_id(wr_list);
            wr_list.push_rect_with_animation(
                &wr::CommonItemProperties {
                    clip_rect: bounds,
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                bounds,
                color.to_wr(),
            )
        }
        DisplayItem::BackdropFilter { clip_rect, filters } => {
            let bounds = clip_rect.to_wr();
            let clip = sc.clip_chain_id(wr_list);
            wr_list.push_backdrop_filter(
                &wr::CommonItemProperties {
                    clip_rect: bounds,
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                &filters.iter().map(|f| f.to_wr()).collect::<Vec<_>>(),
                &[],
                &[],
            )
        }

        DisplayItem::Border {
            bounds,
            widths,
            sides: [top, right, bottom, left],
            radius,
        } => {
            let bounds = bounds.to_wr();
            let clip = sc.clip_chain_id(wr_list);
            wr_list.push_border(
                &wr::CommonItemProperties {
                    clip_rect: bounds,
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                bounds,
                widths.to_wr(),
                wr::BorderDetails::Normal(wr::NormalBorder {
                    left: left.to_wr(),
                    right: right.to_wr(),
                    top: top.to_wr(),
                    bottom: bottom.to_wr(),
                    radius: radius.to_wr(),
                    do_aa: true,
                }),
            );
        }
        DisplayItem::NinePatchBorder {
            source,
            bounds,
            widths,
            img_size,
            fill,
            slice,
            repeat_horizontal,
            repeat_vertical,
        } => {
            nine_patch_border_to_webrender(
                sc,
                wr_list,
                source,
                cache,
                *bounds,
                *widths,
                *repeat_horizontal,
                *slice,
                *img_size,
                *repeat_vertical,
                *fill,
            );
        }

        DisplayItem::Image {
            clip_rect,
            image_id,
            image_size,
            rendering,
            alpha_type,
            tile_size,
            tile_spacing,
        } => {
            let bounds = clip_rect.to_wr();
            let clip = sc.clip_chain_id(wr_list);
            let props = wr::CommonItemProperties {
                clip_rect: bounds,
                clip_chain_id: clip,
                spatial_id: sc.spatial_id(),
                flags: sc.primitive_flags(),
            };

            if tile_spacing.is_empty() && tile_size == image_size {
                wr_list.push_image(
                    &props,
                    PxRect::from_size(*image_size).to_wr(),
                    rendering.to_wr(),
                    alpha_type.to_wr(),
                    wr::ImageKey(cache.id_namespace(), image_id.get()),
                    wr::ColorF::WHITE,
                );
            } else {
                wr_list.push_repeating_image(
                    &props,
                    PxRect::from_size(*image_size).to_wr(),
                    tile_size.to_wr(),
                    tile_spacing.to_wr(),
                    rendering.to_wr(),
                    alpha_type.to_wr(),
                    wr::ImageKey(cache.id_namespace(), image_id.get()),
                    wr::ColorF::WHITE,
                );
            }
        }

        DisplayItem::LinearGradient {
            clip_rect,
            start_point,
            end_point,
            extend_mode,
            stops,
            tile_origin,
            tile_size,
            tile_spacing,
        } => {
            let mut tile_origin = *tile_origin;
            tile_origin.x.0 = tile_origin.x.0.rem_euclid(tile_size.width.0);
            tile_origin.y.0 = tile_origin.y.0.rem_euclid(tile_size.height.0);
            let bounds = PxRect::new(
                -tile_origin + clip_rect.origin.to_vector(),
                clip_rect.size + tile_origin.to_vector().to_size(),
            )
            .to_wr();

            let clip = sc.clip_chain_id(wr_list);
            // stops needs to be immediately followed by the gradient, if the clip-chain item
            // is inserted in the between the stops are lost.
            wr_list.push_stops(cast_gradient_stops_to_wr(stops));
            wr_list.push_gradient(
                &wr::CommonItemProperties {
                    clip_rect: clip_rect.to_wr(),
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                bounds,
                wr::Gradient {
                    start_point: start_point.cast_unit(),
                    end_point: end_point.cast_unit(),
                    extend_mode: extend_mode.to_wr(),
                },
                tile_size.to_wr(),
                tile_spacing.to_wr(),
            )
        }
        DisplayItem::RadialGradient {
            clip_rect,
            center,
            radius,
            start_offset,
            end_offset,
            extend_mode,
            stops,
            tile_origin,
            tile_size,
            tile_spacing,
        } => {
            let mut tile_origin = *tile_origin;
            tile_origin.x.0 = tile_origin.x.0.rem_euclid(tile_size.width.0);
            tile_origin.y.0 = tile_origin.y.0.rem_euclid(tile_size.height.0);
            let bounds = PxRect::new(
                -tile_origin + clip_rect.origin.to_vector(),
                clip_rect.size + tile_origin.to_vector().to_size(),
            )
            .to_wr();

            let clip = sc.clip_chain_id(wr_list);
            wr_list.push_stops(cast_gradient_stops_to_wr(stops));
            wr_list.push_radial_gradient(
                &wr::CommonItemProperties {
                    clip_rect: clip_rect.to_wr(),
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                bounds,
                wr::RadialGradient {
                    center: center.cast_unit(),
                    radius: radius.cast_unit(),
                    start_offset: *start_offset,
                    end_offset: *end_offset,
                    extend_mode: extend_mode.to_wr(),
                },
                tile_size.to_wr(),
                tile_spacing.to_wr(),
            )
        }
        DisplayItem::ConicGradient {
            clip_rect,
            center,
            angle,
            start_offset,
            end_offset,
            extend_mode,
            stops,
            tile_origin,
            tile_size,
            tile_spacing,
        } => {
            let mut tile_origin = *tile_origin;
            tile_origin.x.0 = tile_origin.x.0.rem_euclid(tile_size.width.0);
            tile_origin.y.0 = tile_origin.y.0.rem_euclid(tile_size.height.0);
            let bounds = PxRect::new(
                -tile_origin + clip_rect.origin.to_vector(),
                clip_rect.size + tile_origin.to_vector().to_size(),
            )
            .to_wr();

            let clip = sc.clip_chain_id(wr_list);
            wr_list.push_stops(cast_gradient_stops_to_wr(stops));
            wr_list.push_conic_gradient(
                &wr::CommonItemProperties {
                    clip_rect: clip_rect.to_wr(),
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                bounds,
                wr::ConicGradient {
                    center: center.cast_unit(),
                    angle: angle.0,
                    start_offset: *start_offset,
                    end_offset: *end_offset,
                    extend_mode: extend_mode.to_wr(),
                },
                tile_size.to_wr(),
                tile_spacing.to_wr(),
            )
        }
        DisplayItem::Line {
            clip_rect,
            color,
            style,
            orientation,
        } => {
            let bounds = clip_rect.to_wr();
            let clip = sc.clip_chain_id(wr_list);
            let (line_style, wavy_line_thickness) = style.to_wr();
            wr_list.push_line(
                &wr::CommonItemProperties {
                    clip_rect: bounds,
                    clip_chain_id: clip,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                &bounds,
                wavy_line_thickness,
                orientation.to_wr(),
                &color.to_wr(),
                line_style,
            );
        }
        DisplayItem::PushExtension { extension_id, payload } => ext.push_display_item(&mut DisplayExtensionItemArgs {
            extension_id: *extension_id,
            payload,
            is_reuse,
            list: wr_list,
            sc,
        }),
        DisplayItem::PopExtension { extension_id } => ext.pop_display_item(&mut DisplayExtensionItemArgs {
            extension_id: *extension_id,
            payload: &ApiExtensionPayload::empty(),
            is_reuse,
            list: wr_list,
            sc,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn nine_patch_border_to_webrender(
    sc: &mut SpaceAndClip,
    wr_list: &mut wr::DisplayListBuilder,
    source: &NinePatchSource,
    cache: &DisplayListCache,
    mut bounds: PxRect,
    mut widths: PxSideOffsets,
    repeat_horizontal: RepeatMode,
    slice: PxSideOffsets,
    img_size: PxSize,
    repeat_vertical: RepeatMode,
    fill: bool,
) {
    let clip = sc.clip_chain_id(wr_list);

    let source = match source {
        NinePatchSource::Image { image_id, rendering } => {
            wr::NinePatchBorderSource::Image(wr::ImageKey(cache.id_namespace(), image_id.get()), rendering.to_wr())
        }
        NinePatchSource::LinearGradient {
            start_point,
            end_point,
            extend_mode,
            stops,
        } => {
            wr_list.push_stops(cast_gradient_stops_to_wr(stops));
            wr::NinePatchBorderSource::Gradient(wr::Gradient {
                start_point: start_point.cast_unit(),
                end_point: end_point.cast_unit(),
                extend_mode: extend_mode.to_wr(),
            })
        }
        NinePatchSource::RadialGradient {
            center,
            radius,
            start_offset,
            end_offset,
            extend_mode,
            stops,
        } => {
            wr_list.push_stops(cast_gradient_stops_to_wr(stops));
            wr::NinePatchBorderSource::RadialGradient(wr::RadialGradient {
                center: center.cast_unit(),
                radius: radius.cast_unit(),
                start_offset: *start_offset,
                end_offset: *end_offset,
                extend_mode: extend_mode.to_wr(),
            })
        }
        NinePatchSource::ConicGradient {
            center,
            angle,
            start_offset,
            end_offset,
            extend_mode,
            stops,
        } => {
            wr_list.push_stops(cast_gradient_stops_to_wr(stops));
            wr::NinePatchBorderSource::ConicGradient(wr::ConicGradient {
                center: center.cast_unit(),
                angle: angle.0,
                start_offset: *start_offset,
                end_offset: *end_offset,
                extend_mode: extend_mode.to_wr(),
            })
        }
    };

    // webrender does not implement RepeatMode::Space, so we hide the space lines (and corners) and do a manual repeat
    let actual_bounds = bounds;
    let actual_widths = widths;
    let mut render_corners = false;
    if let wr::NinePatchBorderSource::Image(image_key, rendering) = source {
        use wr::euclid::rect as r;

        if matches!(repeat_horizontal, zng_view_api::RepeatMode::Space) {
            bounds.origin.y += widths.top;
            bounds.size.height -= widths.vertical();
            widths.top = Px(0);
            widths.bottom = Px(0);
            render_corners = true;

            for (bounds, slice) in [
                // top
                (
                    r::<_, Px>(
                        actual_widths.left,
                        Px(0),
                        actual_bounds.width() - actual_widths.horizontal(),
                        actual_widths.top,
                    ),
                    r::<_, Px>(slice.left, Px(0), img_size.width - slice.horizontal(), slice.top),
                ),
                // bottom
                (
                    r(
                        actual_widths.left,
                        actual_bounds.height() - actual_widths.bottom,
                        actual_bounds.width() - actual_widths.horizontal(),
                        actual_widths.bottom,
                    ),
                    r(
                        slice.left,
                        img_size.height - slice.bottom,
                        img_size.width - slice.horizontal(),
                        slice.bottom,
                    ),
                ),
            ] {
                let scale = Factor(bounds.height().0 as f32 / slice.height().0 as f32);

                let size = PxRect::from_size(img_size * scale).to_wr();
                let clip = slice * scale;

                let mut offset_x = (bounds.origin.x - clip.origin.x).0 as f32;
                let offset_y = (bounds.origin.y - clip.origin.y).0 as f32;

                let bounds_width = bounds.width().0 as f32;
                let clip = clip.to_wr();
                let n = (bounds_width / clip.width()).floor();
                let space = bounds_width - clip.width() * n;
                let space = space / (n + 1.0);

                offset_x += space;
                let advance = clip.width() + space;
                for _ in 0..n as u32 {
                    let spatial_id = wr_list.push_reference_frame(
                        wr::units::LayoutPoint::zero(),
                        sc.spatial_id(),
                        wr::TransformStyle::Flat,
                        wr::PropertyBinding::Value(wr::units::LayoutTransform::translation(offset_x, offset_y, 0.0)),
                        wr::ReferenceFrameKind::Transform {
                            is_2d_scale_translation: true,
                            should_snap: false,
                            paired_with_perspective: false,
                        },
                        sc.next_view_process_frame_id().to_wr(),
                    );
                    sc.push_spatial(spatial_id);

                    let clip_id = sc.clip_chain_id(wr_list);
                    wr_list.push_image(
                        &wr::CommonItemProperties {
                            clip_rect: clip,
                            clip_chain_id: clip_id,
                            spatial_id: sc.spatial_id(),
                            flags: sc.primitive_flags(),
                        },
                        size,
                        rendering,
                        wr::AlphaType::Alpha,
                        image_key,
                        wr::ColorF::WHITE,
                    );

                    wr_list.pop_reference_frame();
                    sc.pop_spatial();

                    offset_x += advance;
                }
            }
        }
        if matches!(repeat_vertical, zng_view_api::RepeatMode::Space) {
            bounds.origin.x += widths.left;
            bounds.size.width -= widths.horizontal();
            widths.left = Px(0);
            widths.right = Px(0);
            render_corners = true;

            for (bounds, slice) in [
                // left
                (
                    r::<_, Px>(
                        Px(0),
                        actual_widths.top,
                        actual_widths.left,
                        actual_bounds.height() - actual_widths.vertical(),
                    ),
                    r::<_, Px>(Px(0), slice.top, slice.left, img_size.height - slice.vertical()),
                ),
                // right
                (
                    r(
                        actual_bounds.width() - actual_widths.right,
                        actual_widths.top,
                        actual_widths.right,
                        actual_bounds.height() - actual_widths.vertical(),
                    ),
                    r(
                        img_size.width - slice.right,
                        slice.left,
                        slice.right,
                        img_size.height - slice.vertical(),
                    ),
                ),
            ] {
                let scale = Factor(bounds.width().0 as f32 / slice.width().0 as f32);

                let size = PxRect::from_size(img_size * scale).to_wr();
                let clip = slice * scale;

                let offset_x = (bounds.origin.x - clip.origin.x).0 as f32;
                let mut offset_y = (bounds.origin.y - clip.origin.y).0 as f32;

                let bounds_height = bounds.height().0 as f32;
                let clip = clip.to_wr();
                let n = (bounds_height / clip.height()).floor();
                let space = bounds_height - clip.height() * n;
                let space = space / (n + 1.0);

                offset_y += space;
                let advance = clip.height() + space;
                for _ in 0..n as u32 {
                    let spatial_id = wr_list.push_reference_frame(
                        wr::units::LayoutPoint::zero(),
                        sc.spatial_id(),
                        wr::TransformStyle::Flat,
                        wr::PropertyBinding::Value(wr::units::LayoutTransform::translation(offset_x, offset_y, 0.0)),
                        wr::ReferenceFrameKind::Transform {
                            is_2d_scale_translation: true,
                            should_snap: false,
                            paired_with_perspective: false,
                        },
                        sc.next_view_process_frame_id().to_wr(),
                    );
                    sc.push_spatial(spatial_id);

                    let clip_id = sc.clip_chain_id(wr_list);
                    wr_list.push_image(
                        &wr::CommonItemProperties {
                            clip_rect: clip,
                            clip_chain_id: clip_id,
                            spatial_id: sc.spatial_id(),
                            flags: sc.primitive_flags(),
                        },
                        size,
                        rendering,
                        wr::AlphaType::Alpha,
                        image_key,
                        wr::ColorF::WHITE,
                    );

                    wr_list.pop_reference_frame();
                    sc.pop_spatial();

                    offset_y += advance;
                }
            }
        }
    }

    let wr_bounds = bounds.to_wr();

    wr_list.push_border(
        &wr::CommonItemProperties {
            clip_rect: wr_bounds,
            clip_chain_id: clip,
            spatial_id: sc.spatial_id(),
            flags: sc.primitive_flags(),
        },
        wr_bounds,
        widths.to_wr(),
        wr::BorderDetails::NinePatch(wr::NinePatchBorder {
            source,
            width: img_size.width.0,
            height: img_size.height.0,
            slice: slice.to_wr_device(),
            fill,
            repeat_horizontal: repeat_horizontal.to_wr(),
            repeat_vertical: repeat_vertical.to_wr(),
        }),
    );

    // if we rendered RepeatMode::Space
    if render_corners {
        let wr::NinePatchBorderSource::Image(image_key, rendering) = source else {
            unreachable!()
        };

        use wr::euclid::rect as r;

        for (bounds, slice) in [
            // top-left
            (
                r::<_, Px>(Px(0), Px(0), actual_widths.left, actual_widths.top),
                r::<_, Px>(Px(0), Px(0), slice.left, slice.top),
            ),
            // top-right
            (
                r(
                    actual_bounds.width() - actual_widths.right,
                    Px(0),
                    actual_widths.right,
                    actual_widths.top,
                ),
                r(img_size.width - slice.right, Px(0), slice.right, slice.top),
            ),
            // bottom-right
            (
                r(
                    actual_bounds.width() - actual_widths.right,
                    actual_bounds.height() - actual_widths.bottom,
                    actual_widths.right,
                    actual_widths.bottom,
                ),
                r(
                    img_size.width - slice.right,
                    img_size.height - slice.bottom,
                    slice.right,
                    slice.bottom,
                ),
            ),
            // bottom-left
            (
                r(
                    Px(0),
                    actual_bounds.height() - actual_widths.bottom,
                    actual_widths.left,
                    actual_widths.bottom,
                ),
                r(Px(0), img_size.height - slice.bottom, slice.left, slice.bottom),
            ),
        ] {
            let scale_x = bounds.size.width.0 as f32 / slice.size.width.0 as f32;
            let scale_y = bounds.size.height.0 as f32 / slice.size.height.0 as f32;

            let mut size = img_size;
            size.width *= scale_x;
            size.height *= scale_y;

            let mut clip = slice;
            clip.origin.x *= scale_x;
            clip.origin.y *= scale_y;
            clip.size.width *= scale_x;
            clip.size.height *= scale_y;

            let offset_x = bounds.origin.x - clip.origin.x;
            let offset_y = bounds.origin.y - clip.origin.y;

            let spatial_id = wr_list.push_reference_frame(
                wr::units::LayoutPoint::zero(),
                sc.spatial_id(),
                wr::TransformStyle::Flat,
                wr::PropertyBinding::Value(wr::units::LayoutTransform::translation(offset_x.0 as _, offset_y.0 as _, 0.0)),
                wr::ReferenceFrameKind::Transform {
                    is_2d_scale_translation: true,
                    should_snap: false,
                    paired_with_perspective: false,
                },
                sc.next_view_process_frame_id().to_wr(),
            );
            sc.push_spatial(spatial_id);

            let clip_id = sc.clip_chain_id(wr_list);
            wr_list.push_image(
                &wr::CommonItemProperties {
                    clip_rect: clip.to_wr(),
                    clip_chain_id: clip_id,
                    spatial_id: sc.spatial_id(),
                    flags: sc.primitive_flags(),
                },
                PxRect::from_size(size).to_wr(),
                rendering,
                wr::AlphaType::Alpha,
                image_key,
                wr::ColorF::WHITE,
            );

            wr_list.pop_reference_frame();
            sc.pop_spatial();
        }
    }
}

fn display_item_register_bindings(item: &DisplayItem, bindings: &mut FxHashMap<FrameValueId, (FrameId, usize)>, value: (FrameId, usize)) {
    match item {
        DisplayItem::PushReferenceFrame {
            transform: FrameValue::Bind { id, .. },
            ..
        } => {
            bindings.insert(*id, value);
        }
        DisplayItem::PushStackingContext { filters, .. } => {
            for filter in filters.iter() {
                if let FilterOp::Opacity(FrameValue::Bind { id, .. }) = filter {
                    bindings.insert(*id, value);
                }
            }
        }
        DisplayItem::Color {
            color: FrameValue::Bind { id, .. },
            ..
        } => {
            bindings.insert(*id, value);
        }
        DisplayItem::Text {
            color: FrameValue::Bind { id, .. },
            ..
        } => {
            bindings.insert(*id, value);
        }
        _ => {}
    }
}

pub(crate) fn cast_glyphs_to_wr(glyphs: &[GlyphInstance]) -> &[wr::GlyphInstance] {
    debug_assert_eq!(std::mem::size_of::<GlyphInstance>(), std::mem::size_of::<wr::GlyphInstance>());
    debug_assert_eq!(std::mem::size_of::<GlyphIndex>(), std::mem::size_of::<wr::GlyphIndex>());
    debug_assert_eq!(
        std::mem::size_of::<wr::euclid::Point2D<f32, zng_unit::Px>>(),
        std::mem::size_of::<wr::units::LayoutPoint>()
    );

    // SAFETY: GlyphInstance is a copy of the webrender_api
    unsafe { std::mem::transmute(glyphs) }
}

fn cast_gradient_stops_to_wr(stops: &[GradientStop]) -> &[wr::GradientStop] {
    debug_assert_eq!(std::mem::size_of::<GradientStop>(), std::mem::size_of::<wr::GradientStop>());
    debug_assert_eq!(std::mem::size_of::<Rgba>(), std::mem::size_of::<wr::ColorF>());

    // SAFETY: GradientStop has the same layout as webrender_api (f32, [f32; 4])
    unsafe { std::mem::transmute(stops) }
}
