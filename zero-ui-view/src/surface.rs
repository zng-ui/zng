use std::{collections::VecDeque, fmt};

use glutin::event_loop::EventLoopWindowTarget;

use webrender::{
    api::{
        BuiltDisplayList, ColorF, DisplayListPayload, DocumentId, DynamicProperties, FontInstanceKey, FontInstanceOptions,
        FontInstancePlatformOptions, FontKey, FontVariation, HitTestResult, IdNamespace, ImageKey, PipelineId, SampledScrollOffset,
    },
    RenderApi, Renderer, RendererOptions, Transaction,
};
use zero_ui_view_api::{
    units::*, FrameId, FrameRequest, FrameUpdateRequest, HeadlessRequest, ImageId, ImageLoadedData, RenderMode, ViewProcessGen, WindowId,
};

use crate::{
    gl::{GlContext, GlContextManager},
    image_cache::{Image, ImageCache, ImageUseMap, WrImageCache},
    AppEvent, AppEventSender, FrameReadyMsg, WrNotifier,
};

/// A headless "window".
pub(crate) struct Surface {
    id: WindowId,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    api: RenderApi,
    size: DipSize,
    scale_factor: f32,

    context: GlContext,
    renderer: Option<Renderer>,
    image_use: ImageUseMap,

    pending_frames: VecDeque<(FrameId, bool)>,
    rendered_frame_id: FrameId,
    resized: bool,
}
impl fmt::Debug for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .field("size", &self.size)
            .field("scale_factor", &self.scale_factor)
            .finish_non_exhaustive()
    }
}
impl Surface {
    pub fn open(
        gen: ViewProcessGen,
        cfg: HeadlessRequest,
        window_target: &EventLoopWindowTarget<AppEvent>,
        gl_manager: &mut GlContextManager,
        event_sender: AppEventSender,
    ) -> Self {
        let id = cfg.id;

        let context = gl_manager.create_headless(id, window_target, cfg.render_mode);
        let size = cfg.size.to_px(cfg.scale_factor);
        context.resize(size.width.0, size.height.0);

        let opts = RendererOptions {
            // text-aa config from Firefox.
            enable_aa: true,
            force_subpixel_aa: false,
            enable_subpixel_aa: cfg!(not(target_os = "android")),

            renderer_id: Some((gen as u64) << 32 | id as u64),

            // this clear color paints over the one set using `Renderer::set_clear_color`.
            clear_color: ColorF::new(0.0, 0.0, 0.0, 0.0),

            allow_advanced_blend_equation: context.is_software(),
            clear_caches_with_quads: !context.is_software(),
            enable_gpu_markers: !context.is_software(),

            //panic_on_gl_error: true,
            ..Default::default()
        };

        let device_size = cfg.size.to_px(cfg.scale_factor).to_wr_device();

        let (mut renderer, sender) =
            webrender::Renderer::new(context.gl().clone(), WrNotifier::create(id, event_sender), opts, None).unwrap();
        renderer.set_external_image_handler(WrImageCache::new_boxed());

        let api = sender.create_api();
        let document_id = api.add_document(device_size);

        let pipeline_id = webrender::api::PipelineId(gen, id);

        Self {
            id,
            pipeline_id,
            document_id,
            api,
            size: cfg.size,
            scale_factor: cfg.scale_factor,

            context,
            renderer: Some(renderer),
            image_use: ImageUseMap::default(),

            pending_frames: VecDeque::new(),
            rendered_frame_id: FrameId::INVALID,
            resized: true,
        }
    }

    pub fn render_mode(&self) -> RenderMode {
        self.context.render_mode()
    }

    pub fn id(&self) -> WindowId {
        self.id
    }

    pub fn id_namespace(&self) -> IdNamespace {
        self.api.get_namespace_id()
    }

    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    /// Root document ID.
    pub fn document_id(&self) -> DocumentId {
        self.document_id
    }

    pub fn frame_id(&self) -> FrameId {
        self.rendered_frame_id
    }

    pub fn set_size(&mut self, document_id: DocumentId, size: DipSize, scale_factor: f32) {
        if self.size != size || (self.scale_factor - scale_factor).abs() > 0.001 {
            if self.document_id == document_id {
                self.size = size;
                self.scale_factor = scale_factor;
                self.context.make_current();
                let px_size = size.to_px(self.scale_factor);
                self.context.resize(px_size.width.0, px_size.height.0);
                self.resized = true;
            } else {
                todo!()
            }
        }
    }

    pub fn use_image(&mut self, image: &Image) -> ImageKey {
        self.image_use.new_use(image, self.document_id(), &mut self.api)
    }

    pub fn update_image(&mut self, key: ImageKey, image: &Image) {
        self.image_use.update_use(key, image, self.document_id(), &mut self.api);
    }

    pub fn delete_image(&mut self, key: ImageKey) {
        self.image_use.delete(key, self.document_id(), &mut self.api);
    }

    pub fn add_font(&mut self, font: Vec<u8>, index: u32) -> FontKey {
        let key = self.api.generate_font_key();
        let mut txn = webrender::Transaction::new();
        txn.add_raw_font(key, font, index);
        self.api.send_transaction(self.document_id(), txn);
        key
    }

    pub fn delete_font(&mut self, key: FontKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font(key);
        self.api.send_transaction(self.document_id(), txn);
    }

    pub fn add_font_instance(
        &mut self,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<FontInstanceOptions>,
        plataform_options: Option<FontInstancePlatformOptions>,
        variations: Vec<FontVariation>,
    ) -> FontInstanceKey {
        let key = self.api.generate_font_instance_key();
        let mut txn = webrender::Transaction::new();
        txn.add_font_instance(key, font_key, glyph_size.to_wr().get(), options, plataform_options, variations);
        self.api.send_transaction(self.document_id(), txn);
        key
    }

    pub fn delete_font_instance(&mut self, instance_key: FontInstanceKey) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font_instance(instance_key);
        self.api.send_transaction(self.document_id(), txn);
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;
            let rect = PxRect::from_size(self.size.to_px(self.scale_factor)).to_wr_device();
            txn.set_document_view(rect);
        }
    }

    pub fn render(&mut self, frame: FrameRequest) {
        let render_reasons = frame.render_reasons();

        self.pending_frames.push_back((frame.id, frame.capture_image));
        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color);

        let mut txn = Transaction::new();
        txn.reset_dynamic_properties();
        txn.append_dynamic_properties(DynamicProperties {
            transforms: vec![],
            floats: vec![],
            colors: vec![],
        });

        let display_list = BuiltDisplayList::from_data(
            DisplayListPayload {
                items_data: frame.display_list.0.to_vec(),
                cache_data: frame.display_list.1.to_vec(),
                spatial_tree: frame.display_list.2.to_vec(),
            },
            frame.display_list.3,
        );
        let viewport_size = self.size.to_px(self.scale_factor).to_wr();
        txn.set_display_list(
            frame.id.epoch(),
            Some(frame.clear_color),
            viewport_size,
            (frame.pipeline_id, display_list),
        );

        for (scroll_id, offset) in frame.scrolls {
            txn.set_scroll_offsets(
                scroll_id,
                vec![SampledScrollOffset {
                    offset: offset.to_wr(),
                    generation: 0,
                }],
            );
        }

        txn.set_root_pipeline(self.pipeline_id);

        self.push_resize(&mut txn);

        txn.generate_frame(frame.id.get(), render_reasons);
        self.api.send_transaction(self.document_id(), txn);
    }

    pub fn render_update(&mut self, frame: FrameUpdateRequest) {
        let render_reasons = frame.render_reasons();

        if let Some(color) = frame.clear_color {
            self.renderer.as_mut().unwrap().set_clear_color(color);
        }

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);

        txn.append_dynamic_properties(frame.updates);
        for (scroll_id, offset) in frame.scroll_updates {
            txn.set_scroll_offsets(
                scroll_id,
                vec![SampledScrollOffset {
                    offset: offset.to_wr(),
                    generation: 0,
                }],
            );
        }

        self.push_resize(&mut txn);

        txn.generate_frame(self.frame_id().get(), render_reasons);
        self.api.send_transaction(self.document_id(), txn);
    }

    pub fn on_frame_ready(&mut self, msg: FrameReadyMsg, images: &mut ImageCache) -> (FrameId, Option<ImageLoadedData>) {
        debug_assert_eq!(self.document_id, msg.document_id);

        if self.document_id != msg.document_id {
            todo!("document rendering is not implemented in WR");
        }

        let (frame_id, capture) = self.pending_frames.pop_front().unwrap_or((self.rendered_frame_id, false));
        self.rendered_frame_id = frame_id;

        let mut captured_data = None;

        if msg.composite_needed || capture {
            self.context.make_current();
            let renderer = self.renderer.as_mut().unwrap();

            if msg.composite_needed {
                renderer.update();
                renderer.render((self.size.to_px(self.scale_factor)).to_wr_device(), 0).unwrap();
                let _ = renderer.flush_pipeline_info();
            }
            if capture {
                captured_data = Some(images.frame_image_data(
                    renderer,
                    PxRect::from_size(self.size.to_px(self.scale_factor)),
                    true,
                    self.scale_factor,
                ));
            }
        }
        (frame_id, captured_data)
    }

    pub fn frame_image(&mut self, images: &mut ImageCache) -> ImageId {
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            PxRect::from_size(self.size.to_px(self.scale_factor)),
            true,
            self.id,
            self.rendered_frame_id,
            self.scale_factor,
        )
    }

    pub fn frame_image_rect(&mut self, images: &mut ImageCache, rect: PxRect) -> ImageId {
        let rect = PxRect::from_size(self.size.to_px(self.scale_factor)).intersection(&rect).unwrap();
        images.frame_image(
            self.renderer.as_mut().unwrap(),
            rect,
            true,
            self.id,
            self.rendered_frame_id,
            self.scale_factor,
        )
    }

    pub fn hit_test(&mut self, point: DipPoint) -> (FrameId, PxPoint, HitTestResult) {
        let _p = tracing::trace_span!("hit_test").entered();
        let point = point.to_px(self.scale_factor);
        (
            self.rendered_frame_id,
            point,
            self.api.hit_test(self.document_id(), point.to_wr_world()),
        )
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        self.context.make_current();
        self.renderer.take().unwrap().deinit();
    }
}
