//! Demo view extension custom renderer, integrated with Webrender using the blob API.

/// App-process stuff, nodes.
pub mod app_side {
    use zng::prelude::UiNode;
    use zng_app::view_process::VIEW_PROCESS;
    use zng_view_api::api_extension::ApiExtensionId;

    /// Node that sends external display item and updates.
    pub fn custom_render_node() -> impl UiNode {
        crate::using_display_items::app_side::custom_ext_node(extension_id)
    }

    pub fn extension_id() -> ApiExtensionId {
        VIEW_PROCESS
            .extension_id(super::api::extension_name())
            .ok()
            .flatten()
            .unwrap_or(ApiExtensionId::INVALID)
    }
}

/// View-process stuff, the actual extension.
pub mod view_side {
    use std::{collections::HashMap, sync::Arc};

    use zng::layout::{PxPoint, PxSize};
    use zng::prelude::task::parking_lot::Mutex;
    use zng_view::extensions::PxToWr as _;
    use zng_view::{
        extensions::{AsyncBlobRasterizer, BlobExtension, RenderItemArgs, RenderUpdateArgs, RendererExtension},
        webrender::{
            api::{
                BlobImageError, BlobImageKey, BlobImageParams, BlobImageResult, ColorF, CommonItemProperties, ImageDescriptor,
                ImageDescriptorFlags, ImageFormat, RasterizedBlobImage,
                units::{BlobDirtyRect, DeviceIntRect, DeviceIntSize, LayoutRect},
            },
            euclid,
        },
    };
    use zng_view_api::api_extension::ApiExtensionId;

    zng_view::view_process_extension!(|exts| {
        exts.renderer(super::api::extension_name(), CustomExtension::new);
    });

    struct CustomExtension {
        // id of this extension, for tracing.
        _id: ApiExtensionId,
        // renderer, shared between blob extensions.
        renderer: Arc<Mutex<CustomRenderer>>,
    }
    impl CustomExtension {
        fn new(id: ApiExtensionId) -> Self {
            Self {
                _id: id,
                renderer: Arc::default(),
            }
        }
    }
    impl RendererExtension for CustomExtension {
        fn is_init_only(&self) -> bool {
            false // retain the extension after renderer creation.
        }

        fn configure(&mut self, args: &mut zng_view::extensions::RendererConfigArgs) {
            // Blob entry point inside Webrender.
            args.blobs.push(Box::new(CustomBlobExtension {
                renderer: Arc::clone(&self.renderer),
            }));

            // Worker threads will be used during rasterization.
            //
            // This option is always already set by the window. Note that this thread pool
            // is also used by Webrender's glyph rasterizer, so be careful not to clog it.
            self.renderer.lock().workers.clone_from(&args.options.workers);
        }

        fn render_start(&mut self, _: &mut zng_view::extensions::RenderArgs) {
            let mut renderer = self.renderer.lock();
            for t in renderer.tasks.iter_mut() {
                if matches!(t.state, CustomRenderTaskState::Used) {
                    t.state = CustomRenderTaskState::Marked;
                }
            }
        }

        fn render_end(&mut self, args: &mut zng_view::extensions::RenderArgs) {
            let mut renderer = self.renderer.lock();
            for t in renderer.tasks.iter_mut() {
                match &mut t.state {
                    CustomRenderTaskState::Marked => t.state = CustomRenderTaskState::Free(0),
                    CustomRenderTaskState::Free(n) if *n < MAX_FREE => {
                        *n += 1;
                        if *n == MAX_FREE {
                            *n = MAX_FREE + 1;
                            args.transaction.delete_blob_image(t.key)
                        }
                    }
                    _ => {}
                }
            }
        }

        fn render_push(&mut self, args: &mut RenderItemArgs) {
            match args.payload.deserialize::<super::api::RenderPayload>() {
                Ok(p) => {
                    let mut renderer = self.renderer.lock();
                    let renderer = &mut *renderer;

                    let param = if let Some(binding) = p.cursor_binding {
                        // updatable item, gets own image
                        CustomTaskParams::Bound(binding)
                    } else {
                        // not updatable item, shares images of same params
                        CustomTaskParams::Params(p.size, p.cursor)
                    };

                    let mut key = None;
                    if let Some(i) = renderer.task_params.get(&param) {
                        let t = &mut renderer.tasks[*i];
                        if matches!(t.state, CustomRenderTaskState::Marked | CustomRenderTaskState::Used) {
                            // already rendering param
                            key = Some(t.key);
                            t.size = p.size;
                            t.state = CustomRenderTaskState::Used;
                        }
                    }
                    let blob_key = if let Some(k) = key {
                        k
                    } else {
                        // start rendering param
                        //
                        // the renderer will receive an async rasterize request from Webrender
                        // that is when we will actually render this.

                        if let Some(i) = renderer
                            .tasks
                            .iter()
                            .position(|t| matches!(t.state, CustomRenderTaskState::Free(n) if n < MAX_FREE))
                        {
                            // reuse blob key

                            let t = &mut renderer.tasks[i];
                            renderer.task_params.remove(&t.param());

                            if t.size != p.size {
                                let size = DeviceIntSize::new(p.size.width.0, p.size.height.0);
                                args.transaction.update_blob_image(
                                    t.key,
                                    ImageDescriptor {
                                        format: ImageFormat::BGRA8,
                                        size,
                                        stride: None,
                                        offset: 0,
                                        flags: ImageDescriptorFlags::IS_OPAQUE,
                                    },
                                    // we only need the params (size, cursor),
                                    // this can be used to store render commands.
                                    Arc::new(vec![]),
                                    DeviceIntRect::from_size(size),
                                    &BlobDirtyRect::All,
                                );
                            }

                            t.size = p.size;
                            t.cursor = p.cursor;
                            t.cursor_binding = p.cursor_binding;
                            t.state = CustomRenderTaskState::Used;

                            renderer.task_params.insert(t.param(), i);

                            t.key
                        } else {
                            // new blob key

                            let i = renderer.tasks.len();

                            let key = args.api.generate_blob_image_key();
                            let task = CustomRenderTask {
                                key,
                                size: p.size,
                                cursor: p.cursor,
                                cursor_binding: p.cursor_binding,
                                state: CustomRenderTaskState::Used,
                            };
                            renderer.tasks.push(task);
                            renderer.task_keys.insert(key, i);
                            renderer.task_params.insert(param, i);

                            let size = DeviceIntSize::new(p.size.width.0, p.size.height.0);
                            args.transaction.add_blob_image(
                                key,
                                ImageDescriptor {
                                    format: ImageFormat::BGRA8,
                                    size,
                                    stride: None,
                                    offset: 0,
                                    flags: ImageDescriptorFlags::IS_OPAQUE,
                                },
                                // we only need the params (size, cursor),
                                // this can be used to store render commands.
                                Arc::new(vec![]),
                                DeviceIntRect::from_size(size),
                                Some(128),
                            );

                            key
                        }
                    };

                    let rect = LayoutRect::from_size(p.size.to_wr());
                    let _cursor = p.cursor.to_wr();

                    let props = CommonItemProperties {
                        clip_rect: rect,
                        clip_chain_id: args.sc.clip_chain_id(args.list),
                        spatial_id: args.sc.spatial_id(),
                        flags: args.sc.primitive_flags(),
                    };
                    args.list.push_image(
                        &props,
                        rect,
                        zng_view::webrender::api::ImageRendering::Auto,
                        zng_view::webrender::api::AlphaType::Alpha,
                        blob_key.as_image(),
                        ColorF::WHITE,
                    )
                }
                Err(e) => tracing::error!("invalid display item, {e}"),
            }
        }

        fn render_update(&mut self, args: &mut RenderUpdateArgs) {
            match args.payload.deserialize::<super::api::RenderUpdatePayload>() {
                Ok(p) => {
                    let mut renderer = self.renderer.lock();
                    let renderer = &mut *renderer;

                    if let Some(&i) = renderer.task_params.get(&CustomTaskParams::Bound(p.cursor_binding)) {
                        // update the render task

                        let t = &mut renderer.tasks[i];
                        t.cursor = p.cursor;

                        let size = DeviceIntSize::new(t.size.width.0, t.size.height.0);
                        args.transaction.update_blob_image(
                            t.key,
                            ImageDescriptor {
                                format: ImageFormat::BGRA8,
                                size,
                                stride: None,
                                offset: 0,
                                flags: ImageDescriptorFlags::IS_OPAQUE,
                            },
                            Arc::new(vec![]),
                            DeviceIntRect::from_size(size),
                            &BlobDirtyRect::All,
                        );
                    } else {
                        // or rebuilds the display list
                        args.new_frame = true;
                    }
                }
                Err(e) => tracing::error!("invalid update request, {e}"),
            }
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    struct CustomBlobExtension {
        renderer: Arc<Mutex<CustomRenderer>>,
    }
    impl BlobExtension for CustomBlobExtension {
        fn create_blob_rasterizer(&mut self) -> Box<dyn zng_view::extensions::AsyncBlobRasterizer> {
            Box::new(CustomBlobRasterizer {
                // rasterizer is a snapshot of the current state
                snapshot: self.renderer.lock().clone(),
            })
        }

        fn create_similar(&self) -> Box<dyn BlobExtension> {
            Box::new(CustomBlobExtension {
                renderer: Arc::clone(&self.renderer),
            })
        }

        fn add(&mut self, _args: &zng_view::extensions::BlobAddArgs) {
            // Webrender received the add request
            //
            // In this demo we already added from the display item.
        }

        fn update(&mut self, _args: &zng_view::extensions::BlobUpdateArgs) {
            // Webrender received the update request
        }

        fn delete(&mut self, key: BlobImageKey) {
            // Webrender requested cleanup.

            let mut renderer = self.renderer.lock();
            let renderer = &mut *renderer;
            if let Some(i) = renderer.task_keys.remove(&key) {
                let t = &mut renderer.tasks[i];
                renderer.task_params.remove(&t.param());
                t.state = CustomRenderTaskState::Free(MAX_FREE + 1);
            }
        }

        fn enable_multithreading(&mut self, enable: bool) {
            self.renderer.lock().single_threaded = !enable;
        }
    }

    struct CustomBlobRasterizer {
        snapshot: CustomRenderer,
    }
    impl CustomBlobRasterizer {
        fn rasterize_tile(task: &CustomRenderTask, r: &BlobImageParams) -> BlobImageResult {
            if r.descriptor.format != ImageFormat::BGRA8 {
                // you must always respond, if no extension responds Webrender will panic.
                return Err(BlobImageError::Other(format!("format {:?} is not supported", r.descriptor.format)));
            }

            // draw the requested tile
            let size = r.descriptor.rect.size();
            let offset = r.descriptor.rect.min.to_f32().to_vector();
            let cursor = task.cursor.to_wr();
            let max_dist = task.size.width.0 as f32;

            let mut texels = Vec::with_capacity(size.area() as usize * 4);

            for y in 0..size.height {
                for x in 0..size.width {
                    let t = euclid::Point2D::new(x, y).to_f32() + offset;
                    let dist = t.distance_to(cursor).min(max_dist);

                    let d = 1.0 - dist / max_dist;
                    let d = (255.0 * d).round() as u8;

                    let r = if (dist % 5.0).abs() < 1.0 { d.max(50) } else { 50 };

                    texels.extend([d, d, r, 255]);
                }
            }

            Ok(RasterizedBlobImage {
                rasterized_rect: DeviceIntRect::from_size(DeviceIntSize::new(size.width, size.height)),
                data: Arc::new(texels),
            })
        }
    }
    impl AsyncBlobRasterizer for CustomBlobRasterizer {
        fn rasterize(&mut self, args: &mut zng_view::extensions::BlobRasterizerArgs) {
            if !self.snapshot.single_threaded {
                // rasterize all tiles in parallel, Webrender also uses Rayon
                // but the `rasterize` call is made in the SceneBuilderThread
                // so we mount the workers thread-pool here.

                use zng::prelude::task::rayon::prelude::*;

                let tiles = self.snapshot.workers.as_ref().unwrap().install(|| {
                    args.requests.par_iter().filter_map(|r| {
                        let i = *self.snapshot.task_keys.get(&r.request.key)?;
                        // request is for us
                        let task = &self.snapshot.tasks[i];
                        Some((r.request, Self::rasterize_tile(task, r)))
                    })
                });

                args.responses.par_extend(tiles);
            } else {
                // single-threaded mode is only for testing
                let tiles = args.requests.iter().filter_map(|r| {
                    let i = *self.snapshot.task_keys.get(&r.request.key)?;
                    let task = &self.snapshot.tasks[i];
                    Some((r.request, Self::rasterize_tile(task, r)))
                });
                args.responses.extend(tiles);
            }
        }
    }

    #[derive(Clone, Default)]
    struct CustomRenderer {
        tasks: Vec<CustomRenderTask>,
        task_keys: HashMap<BlobImageKey, usize>,
        task_params: HashMap<CustomTaskParams, usize>,
        single_threaded: bool,
        workers: Option<Arc<zng::prelude::task::rayon::ThreadPool>>,
    }

    #[derive(PartialEq, Eq, Hash, Clone, Copy)]
    enum CustomTaskParams {
        Bound(super::api::BindingId),
        Params(PxSize, PxPoint),
    }

    #[derive(Clone)]
    struct CustomRenderTask {
        key: BlobImageKey,
        size: PxSize,
        cursor: PxPoint,
        cursor_binding: Option<super::api::BindingId>,
        state: CustomRenderTaskState,
    }

    impl CustomRenderTask {
        fn param(&self) -> CustomTaskParams {
            match self.cursor_binding {
                Some(id) => CustomTaskParams::Bound(id),
                None => CustomTaskParams::Params(self.size, self.cursor),
            }
        }
    }
    #[derive(Clone, Copy, Debug)]
    enum CustomRenderTaskState {
        Used,
        Marked,
        Free(u8),
    }
    /// Maximum display-list rebuilds that a unused BlobImageKey is retained.
    const MAX_FREE: u8 = 5;
}

pub mod api {
    use zng_view_api::api_extension::ApiExtensionName;

    pub use crate::using_display_items::api::*;

    pub fn extension_name() -> ApiExtensionName {
        ApiExtensionName::new("zng.examples.extend_renderer.using_blob").unwrap()
    }
}
