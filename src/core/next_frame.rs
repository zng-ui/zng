use super::*;
use webrender::api::*;

pub struct NextFrame {
    builder: DisplayListBuilder,
    spatial_id: SpatialId,
    final_size: LayoutSize,
    cursor: CursorIcon,
    focus_map: FocusMap,
}

impl NextFrame {
    pub fn new(builder: DisplayListBuilder, root_spatial_id: SpatialId, final_size: LayoutSize) -> NextFrame {
        NextFrame {
            builder,
            spatial_id: root_spatial_id,
            final_size,
            cursor: CursorIcon::Default,
            focus_map: FocusMap::new(),
        }
    }

    pub fn push_child(&mut self, child: &impl Ui, final_rect: &LayoutRect) {
        let final_size = self.final_size;
        let spatial_id = self.spatial_id;

        self.final_size = final_rect.size;
        self.spatial_id = self.builder.push_reference_frame(
            final_rect,
            self.spatial_id,
            TransformStyle::Flat,
            PropertyBinding::Value(LayoutTransform::default()),
            ReferenceFrameKind::Transform,
        );

        self.focus_map.push_reference_frame(final_rect);

        child.render(self);
        self.builder.pop_reference_frame();

        self.focus_map.pop_reference_frame(final_rect);

        self.final_size = final_size;
        self.spatial_id = spatial_id;

        // about Stacking Contexts
        //https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Positioning/Understanding_z_index/The_stacking_context
    }

    pub fn push_cursor(&mut self, cursor: CursorIcon, child: &impl Ui) {
        let current_cursor = self.cursor;
        self.cursor = cursor;

        child.render(self);

        self.cursor = current_cursor;
    }

    fn layout_and_clip(
        &self,
        final_rect: LayoutRect,
        hit_tag: Option<HitTag>,
    ) -> (LayoutPrimitiveInfo, SpaceAndClipInfo) {
        let mut lpi = LayoutPrimitiveInfo::new(final_rect);
        lpi.tag = hit_tag.map(|v| (v.get(), self.cursor as u16));
        let sci = SpaceAndClipInfo {
            spatial_id: self.spatial_id,
            clip_id: ClipId::root(self.spatial_id.pipeline_id()),
        };

        (lpi, sci)
    }

    pub fn push_color(&mut self, final_rect: LayoutRect, color: ColorF, hit_tag: Option<HitTag>) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);
        self.builder.push_rect(&lpi, &sci, color);
    }

    pub fn push_hit_test(&mut self, hit_tag: HitTag, final_rect: LayoutRect) {
        let (lpi, sci) = self.layout_and_clip(final_rect, Some(hit_tag));
        self.builder.push_rect(&lpi, &sci, ColorF::TRANSPARENT);
    }

    pub fn push_gradient(
        &mut self,
        final_rect: LayoutRect,
        start: LayoutPoint,
        end: LayoutPoint,
        stops: Vec<GradientStop>,
        hit_tag: Option<HitTag>,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);

        let grad = self.builder.create_gradient(start, end, stops, ExtendMode::Clamp);
        self.builder
            .push_gradient(&lpi, &sci, grad, final_rect.size, LayoutSize::default());
    }

    pub fn push_border(
        &mut self,
        final_rect: LayoutRect,
        widths: LayoutSideOffsets,
        details: BorderDetails,
        hit_tag: Option<HitTag>,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);

        self.builder.push_border(&lpi, &sci, widths, details);
    }

    pub fn push_text(
        &mut self,
        final_rect: LayoutRect,
        glyphs: &[GlyphInstance],
        font_instance_key: FontInstanceKey,
        color: ColorF,
        hit_tag: Option<HitTag>,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);

        self.builder
            .push_text(&lpi, &sci, &glyphs, font_instance_key, color, None);
    }

    pub fn push_focusable(&mut self, key: FocusKey, rect: &LayoutRect) {
        self.focus_map.push_focusable(key, rect.center());
    }

    pub fn push_focus_scope(
        &mut self,
        key: FocusKey,
        rect: &LayoutRect,
        capture: bool,
        tab: Option<TabNav>,
        directional: Option<DirectionalNav>,
        child: &impl Ui,
    ) {
        self.focus_map.push_focus_scope(key, rect, capture, tab, directional);

        child.render(self);

        self.focus_map.pop_focus_scope();
    }

    pub fn final_size(&self) -> LayoutSize {
        self.final_size
    }

    pub(crate) fn finalize(self) -> ((PipelineId, LayoutSize, BuiltDisplayList), FocusMap) {
        (self.builder.finalize(), self.focus_map)
    }
}
