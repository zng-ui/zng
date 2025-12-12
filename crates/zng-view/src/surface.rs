use std::{collections::VecDeque, fmt, sync::Arc};

use tracing::span::EnteredSpan;
use webrender::{
    RenderApi, Renderer, Transaction,
    api::{DocumentId, DynamicProperties, FontInstanceKey, FontKey, FontVariation, PipelineId},
};
use winit::event_loop::ActiveEventLoop;
use zng_unit::{DipSize, DipToPx, Factor, Px, PxRect, Rgba};
use zng_view_api::{
    ViewProcessGen,
    api_extension::{ApiExtensionId, ApiExtensionPayload},
    font::{FontFaceId, FontId, FontOptions, FontVariationName, IpcFontBytes},
    image::{ImageDecoded, ImageId, ImageMaskMode, ImageTextureId},
    window::{FrameCapture, FrameId, FrameRequest, FrameUpdateRequest, HeadlessRequest, RenderMode, WindowId},
};

use crate::{
    AppEventSender, FrameReadyMsg, WrNotifier,
    display_list::{DisplayListCache, display_list_to_webrender},
    extensions::{
        self, BlobExtensionsImgHandler, DisplayListExtAdapter, FrameReadyArgs, RedrawArgs, RendererCommandArgs, RendererConfigArgs,
        RendererDeinitedArgs, RendererExtension, RendererInitedArgs, WindowConfigArgs, WindowExtension,
    },
    gl::{GlContext, GlContextManager},
    image_cache::{Image, ImageCache, ImageUseMap, ResizerCache, WrImageCache},
    px_wr::PxToWr as _,
    util::{PxToWinit, frame_render_reasons, frame_update_render_reasons},
};

/// A headless "window".
pub(crate) struct Surface {
    id: WindowId,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    api: RenderApi,
    size: DipSize,
    scale_factor: Factor,

    context: GlContext,
    renderer: Option<Renderer>,
    renderer_exts: Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
    external_images: extensions::ExternalImages,
    image_use: ImageUseMap,

    display_list_cache: DisplayListCache,
    clear_color: Option<Rgba>,

    pending_frames: VecDeque<(FrameId, FrameCapture, Option<EnteredSpan>)>,
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
    #[expect(clippy::too_many_arguments)]
    pub fn open(
        vp_gen: ViewProcessGen,
        cfg: HeadlessRequest,
        winit_loop: &ActiveEventLoop,
        gl_manager: &mut GlContextManager,
        mut window_exts: Vec<(ApiExtensionId, Box<dyn WindowExtension>)>,
        mut renderer_exts: Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
        event_sender: AppEventSender,
        resizer_cache: Arc<ResizerCache>,
    ) -> Self {
        let id = cfg.id;

        #[cfg(windows)]
        let mut prefer_egl = false;
        #[cfg(not(windows))]
        let prefer_egl = false;

        for (id, ext) in &mut window_exts {
            ext.configure(&mut WindowConfigArgs {
                config: cfg.extensions.iter().find(|(k, _)| k == id).map(|(_, p)| p),
                window: None,
            });

            #[cfg(windows)]
            if let Some(ext) = ext.as_any().downcast_ref::<crate::extensions::PreferAngleExt>() {
                prefer_egl = ext.prefer_egl;
            }
        }

        let mut context = gl_manager.create_headless(id, winit_loop, cfg.render_mode, &event_sender, prefer_egl);

        let size = cfg.size.to_px(cfg.scale_factor);
        context.resize(size.to_winit());

        let mut opts = webrender::WebRenderOptions {
            // text-aa config from Firefox.
            enable_aa: true,
            enable_subpixel_aa: cfg!(not(target_os = "android")),

            renderer_id: Some(((vp_gen.get() as u64) << 32) | id.get() as u64),

            // this clear color paints over the one set using `Renderer::set_clear_color`.
            clear_color: webrender::api::ColorF::new(0.0, 0.0, 0.0, 0.0),

            allow_advanced_blend_equation: context.is_software(),
            clear_caches_with_quads: !context.is_software(),
            enable_gpu_markers: !context.is_software(),

            // extensions expect this to be set.
            workers: Some(crate::util::wr_workers()),
            // optimize memory usage
            chunk_pool: Some(crate::util::wr_chunk_pool()),

            //panic_on_gl_error: true,
            ..Default::default()
        };
        let mut blobs = BlobExtensionsImgHandler(vec![]);
        for (id, ext) in &mut renderer_exts {
            ext.configure(&mut RendererConfigArgs {
                config: cfg.extensions.iter().find(|(k, _)| k == id).map(|(_, v)| v),
                options: &mut opts,
                blobs: &mut blobs.0,
                window: None,
                context: &mut context,
            });
        }
        if !opts.enable_multithreading {
            for b in &mut blobs.0 {
                b.enable_multithreading(false);
            }
        }
        opts.blob_image_handler = Some(Box::new(blobs));

        let device_size = cfg.size.to_px(cfg.scale_factor).to_wr_device();

        let (mut renderer, sender) =
            webrender::create_webrender_instance(context.gl().clone(), WrNotifier::create(id, event_sender), opts, None).unwrap();
        renderer.set_external_image_handler(WrImageCache::new_boxed());

        let mut external_images = extensions::ExternalImages::default();

        let mut api = sender.create_api();
        let document_id = api.add_document(device_size);
        let pipeline_id = webrender::api::PipelineId(vp_gen.get(), id.get());

        renderer_exts.retain_mut(|(_, ext)| {
            ext.renderer_inited(&mut RendererInitedArgs {
                renderer: &mut renderer,
                external_images: &mut external_images,
                api_sender: &sender,
                api: &mut api,
                document_id,
                pipeline_id,
                window: None,
                context: &mut context,
            });
            !ext.is_init_only()
        });

        Self {
            id,
            pipeline_id,
            document_id,
            display_list_cache: DisplayListCache::new(pipeline_id, api.get_namespace_id()),
            api,
            size: cfg.size,
            scale_factor: cfg.scale_factor,

            context,
            renderer: Some(renderer),
            renderer_exts,
            external_images,
            image_use: ImageUseMap::new(resizer_cache),

            clear_color: None,

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

    pub fn frame_id(&self) -> FrameId {
        self.rendered_frame_id
    }

    pub fn set_size(&mut self, size: DipSize, scale_factor: Factor) {
        if self.size != size || (self.scale_factor - scale_factor).abs().0 > 0.001 {
            self.size = size;
            self.scale_factor = scale_factor;
            self.context.make_current();
            let px_size = size.to_px(self.scale_factor);
            self.context.resize(px_size.to_winit());
            self.resized = true;
        }
    }

    pub fn use_image(&mut self, image: &Image) -> ImageTextureId {
        self.image_use.new_use(image, self.document_id, &mut self.api)
    }

    pub fn update_image(&mut self, texture_id: ImageTextureId, image: &Image, dirty_rect: Option<PxRect>) -> bool {
        self.image_use
            .update_use(texture_id, image, dirty_rect, self.document_id, &mut self.api)
    }

    pub fn delete_image(&mut self, texture_id: ImageTextureId) {
        self.image_use.delete(texture_id, self.document_id, &mut self.api);
    }

    pub fn add_font_face(&mut self, font: IpcFontBytes, index: u32) -> FontFaceId {
        #[cfg(target_os = "macos")]
        let index = {
            if index != 0 {
                tracing::error!("webrender does not support font index on macOS, ignoring `{index}` will use `0`");
            }
            0
        };

        let key = self.api.generate_font_key();
        let mut txn = webrender::Transaction::new();
        match font {
            IpcFontBytes::Bytes(b) => txn.add_raw_font(key, b.to_vec(), index),
            IpcFontBytes::System(p) => {
                #[cfg(not(any(target_os = "macos", target_os = "ios")))]
                txn.add_native_font(key, webrender::api::NativeFontHandle { path: p, index });

                #[cfg(any(target_os = "macos", target_os = "ios"))]
                match std::fs::read(p) {
                    Ok(d) => txn.add_raw_font(key, d, index),
                    Err(e) => {
                        tracing::error!("cannot load font, {e}");
                        return FontFaceId::INVALID;
                    }
                }
            }
        }
        self.api.send_transaction(self.document_id, txn);
        FontFaceId::from_raw(key.1)
    }

    pub fn delete_font_face(&mut self, font_face_id: FontFaceId) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font(FontKey(self.api.get_namespace_id(), font_face_id.get()));
        self.api.send_transaction(self.document_id, txn);
    }

    pub fn add_font(
        &mut self,
        font_face_id: FontFaceId,
        glyph_size: Px,
        options: FontOptions,
        variations: Vec<(FontVariationName, f32)>,
    ) -> FontId {
        let key = self.api.generate_font_instance_key();
        let mut txn = webrender::Transaction::new();
        txn.add_font_instance(
            key,
            FontKey(self.api.get_namespace_id(), font_face_id.get()),
            glyph_size.to_wr().get(),
            options.to_wr(),
            None,
            variations
                .into_iter()
                .map(|(n, v)| FontVariation {
                    tag: u32::from_be_bytes(n),
                    value: v,
                })
                .collect(),
        );
        self.api.send_transaction(self.document_id, txn);
        FontId::from_raw(key.1)
    }

    pub fn delete_font(&mut self, font_id: FontId) {
        let mut txn = webrender::Transaction::new();
        txn.delete_font_instance(FontInstanceKey(self.api.get_namespace_id(), font_id.get()));
        self.api.send_transaction(self.document_id, txn);
    }

    fn push_resize(&mut self, txn: &mut Transaction) {
        if self.resized {
            self.resized = false;
            let rect = PxRect::from_size(self.size.to_px(self.scale_factor)).to_wr_device();
            txn.set_document_view(rect);
        }
    }

    pub fn render(&mut self, frame: FrameRequest) {
        let _span = tracing::trace_span!("render").entered();

        let render_reasons = frame_render_reasons(&frame);

        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color.to_wr());

        let mut txn = Transaction::new();
        txn.reset_dynamic_properties();
        txn.append_dynamic_properties(DynamicProperties {
            transforms: vec![],
            floats: vec![],
            colors: vec![],
        });

        let display_list = display_list_to_webrender(
            frame.display_list,
            &mut DisplayListExtAdapter {
                frame_id: frame.id,
                extensions: &mut self.renderer_exts,
                transaction: &mut txn,
                document_id: self.document_id,
                renderer: self.renderer.as_mut().unwrap(),
                api: &mut self.api,
                external_images: &mut self.external_images,
            },
            &mut self.image_use,
            &mut self.display_list_cache,
        );

        self.renderer.as_mut().unwrap().set_clear_color(frame.clear_color.to_wr());
        self.clear_color = Some(frame.clear_color);

        txn.set_display_list(webrender::api::Epoch(frame.id.epoch()), (self.pipeline_id, display_list));

        txn.set_root_pipeline(self.pipeline_id);

        self.push_resize(&mut txn);

        txn.generate_frame(frame.id.get(), true, false, render_reasons);

        let frame_scope =
            tracing::trace_span!("<frame>", ?frame.id, capture = ?frame.capture, from_update = false, thread = "<webrender>").entered();
        self.pending_frames.push_back((frame.id, frame.capture, Some(frame_scope)));

        self.api.send_transaction(self.document_id, txn);
    }

    pub fn render_update(&mut self, frame: FrameUpdateRequest) {
        let _span = tracing::trace_span!("render_update").entered();

        let render_reasons = frame_update_render_reasons(&frame);

        if let Some(color) = frame.clear_color {
            self.clear_color = Some(color);
            self.renderer.as_mut().unwrap().set_clear_color(color.to_wr());
        }

        let resized = self.resized;

        let mut txn = Transaction::new();
        txn.set_root_pipeline(self.pipeline_id);
        self.push_resize(&mut txn);
        txn.generate_frame(self.frame_id().get(), true, false, render_reasons);

        let frame_scope = match self.display_list_cache.update(
            &mut DisplayListExtAdapter {
                frame_id: self.frame_id(),
                extensions: &mut self.renderer_exts,
                transaction: &mut txn,
                document_id: self.document_id,
                renderer: self.renderer.as_mut().unwrap(),
                api: &mut self.api,
                external_images: &mut self.external_images,
            },
            &mut self.image_use,
            frame.transforms,
            frame.floats,
            frame.colors,
            frame.extensions,
            resized,
        ) {
            Ok(p) => {
                if let Some(p) = p {
                    txn.append_dynamic_properties(p);
                }

                tracing::trace_span!("<frame-update>", ?frame.id, capture = ?frame.capture, thread = "<webrender>")
            }
            Err(d) => {
                txn.reset_dynamic_properties();
                txn.append_dynamic_properties(DynamicProperties {
                    transforms: vec![],
                    floats: vec![],
                    colors: vec![],
                });

                txn.set_display_list(webrender::api::Epoch(frame.id.epoch()), (self.pipeline_id, d));

                tracing::trace_span!("<frame>", ?frame.id, capture = ?frame.capture, from_update = true, thread = "<webrender>")
            }
        };

        self.pending_frames
            .push_back((frame.id, frame.capture, Some(frame_scope.entered())));

        self.api.send_transaction(self.document_id, txn);
    }

    pub fn on_frame_ready(&mut self, msg: FrameReadyMsg, images: &mut ImageCache) -> (FrameId, Option<ImageDecoded>) {
        let (frame_id, capture, _) = self
            .pending_frames
            .pop_front()
            .unwrap_or((self.rendered_frame_id, FrameCapture::None, None));
        self.rendered_frame_id = frame_id;

        let mut captured_data = None;

        let mut ext_args = FrameReadyArgs {
            frame_id,
            redraw: msg.composite_needed || capture != FrameCapture::None,
        };
        for (_, ext) in &mut self.renderer_exts {
            ext.frame_ready(&mut ext_args);
            ext_args.redraw |= msg.composite_needed || capture != FrameCapture::None;
        }

        if ext_args.redraw || msg.composite_needed || capture != FrameCapture::None {
            self.context.make_current();
            let renderer = self.renderer.as_mut().unwrap();

            let size = self.size.to_px(self.scale_factor);

            if msg.composite_needed {
                renderer.update();
                renderer.render(size.to_wr_device(), 0).unwrap();
                let _ = renderer.flush_pipeline_info();
            }

            for (_, ext) in &mut self.renderer_exts {
                ext.redraw(&mut RedrawArgs {
                    scale_factor: self.scale_factor,
                    size,
                    context: &mut self.context,
                });
            }

            let capture = match capture {
                FrameCapture::None => None,
                FrameCapture::Full => Some(None),
                FrameCapture::Mask(m) => Some(Some(m)),
                _ => None,
            };
            if let Some(mask) = capture {
                captured_data = Some(images.frame_image_data(
                    &**self.context.gl(),
                    PxRect::from_size(self.size.to_px(self.scale_factor)),
                    self.scale_factor,
                    mask,
                ));
            }
        }
        (frame_id, captured_data)
    }

    pub fn frame_image(&mut self, images: &mut ImageCache, mask: Option<ImageMaskMode>) -> ImageId {
        images.frame_image(
            &**self.context.gl(),
            PxRect::from_size(self.size.to_px(self.scale_factor)),
            self.id,
            self.rendered_frame_id,
            self.scale_factor,
            mask,
        )
    }

    pub fn frame_image_rect(&mut self, images: &mut ImageCache, rect: PxRect, mask: Option<ImageMaskMode>) -> ImageId {
        let rect = PxRect::from_size(self.size.to_px(self.scale_factor)).intersection(&rect).unwrap();
        images.frame_image(&**self.context.gl(), rect, self.id, self.rendered_frame_id, self.scale_factor, mask)
    }

    /// Calls the render extension command.
    pub fn render_extension(&mut self, extension_id: ApiExtensionId, request: ApiExtensionPayload) -> ApiExtensionPayload {
        let mut redraw = false;

        let mut r = None;

        for (id, ext) in &mut self.renderer_exts {
            if *id == extension_id {
                r = Some(ext.command(&mut RendererCommandArgs {
                    renderer: self.renderer.as_mut().unwrap(),
                    api: &mut self.api,
                    document_id: self.document_id,
                    request,
                    window: None,
                    redraw: &mut redraw,
                    context: &mut self.context,
                }));
                break;
            }
        }

        if redraw {
            let size = self.size.to_px(self.scale_factor);
            for (_, ext) in &mut self.renderer_exts {
                ext.redraw(&mut RedrawArgs {
                    scale_factor: self.scale_factor,
                    size,
                    context: &mut self.context,
                });
            }
        }

        r.unwrap_or_else(|| ApiExtensionPayload::unknown_extension(extension_id))
    }

    pub(crate) fn on_low_memory(&mut self) {
        self.api.notify_memory_pressure();

        for (_, ext) in &mut self.renderer_exts {
            ext.low_memory();
        }
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        self.context.make_current();
        self.renderer.take().unwrap().deinit();
        for (_, ext) in &mut self.renderer_exts {
            ext.renderer_deinited(&mut RendererDeinitedArgs {
                document_id: self.document_id,
                pipeline_id: self.pipeline_id,
                context: &mut self.context,
                window: None,
            })
        }
    }
}
