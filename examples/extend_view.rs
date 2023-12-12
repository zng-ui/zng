use zero_ui::{color::filters::hue_rotate, layout::size, prelude::*};
use zero_ui_view::extensions::ViewExtensions;

// Examples of how to extend the view-process with custom renderers.
//
// This is an advanced API, use it only if you really can't render the effect you want
// using custom nodes/properties.

fn main() {
    examples_util::print_info();

    // zero_ui_view::init_extended(view_extensions);
    // app_main();

    zero_ui_view::run_same_process_extended(app_main, view_extensions);
}

fn app_main() {
    APP.defaults().run_window(async {
        Window! {
            // renderer_debug = {
            //     use zero_ui::core::render::webrender_api::DebugFlags;
            //     DebugFlags::TEXTURE_CACHE_DBG | DebugFlags::TEXTURE_CACHE_DBG_CLEAR_EVICTED
            // };

            title = "Extend-View Example";
            width = 900;

            child = Stack! {
                children_align = Align::CENTER;
                direction = StackDirection::left_to_right();
                spacing = 20;

                children = ui_vec![
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::CENTER;
                        spacing = 5;
                        children = ui_vec![
                            Text!("Using Display Items"),
                            Container! {
                                size = 30.vmin_pct();
                                child = using_display_items::app_side::custom_render_node();
                            },
                            Container! {
                                size = 30.vmin_pct();
                                hue_rotate = 180.deg();
                                child = using_display_items::app_side::custom_render_node();
                            },
                        ]
                    },
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::CENTER;
                        spacing = 5;
                        children = ui_vec![
                            Text!("Using Blob Images"),
                            Container! {
                                size = 30.vmin_pct();
                                child = using_blob::app_side::custom_render_node();
                            },
                            Container! {
                                size = 30.vmin_pct();
                                hue_rotate = 180.deg();
                                child = using_blob::app_side::custom_render_node();
                            },
                        ]
                    },
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::CENTER;
                        spacing = 5;
                        children = ui_vec![
                            Text!("Using GL Overlay"),
                            Container! {
                                size = 30.vmin_pct();
                                child = using_gl_overlay::app_side::custom_render_node();
                            },
                            Container! {
                                size = 30.vmin_pct();
                                hue_rotate = 180.deg(); // no effect
                                child = using_gl_overlay::app_side::custom_render_node();
                            },
                        ]
                    },
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        children_align = Align::CENTER;
                        spacing = 5;
                        children = ui_vec![
                            Text!("Using GL Texture"),
                            Container! {
                                size = 30.vmin_pct();
                                child = using_gl_texture::app_side::custom_render_node();
                            },
                            Container! {
                                size = 30.vmin_pct();
                                hue_rotate = 180.deg();
                                child = using_gl_texture::app_side::custom_render_node();
                            },
                        ]
                    },
                ]
            }
        }
    })
}

/// Called in the view-process to init extensions.
fn view_extensions() -> ViewExtensions {
    let mut exts = ViewExtensions::new();
    using_display_items::view_side::extend(&mut exts);
    using_blob::view_side::extend(&mut exts);
    using_gl_overlay::view_side::extend(&mut exts);
    using_gl_texture::view_side::extend(&mut exts);
    exts
}

/// Demo view extension renderer, using only Webrender display items.
pub mod using_display_items {
    /// App-process stuff, nodes.
    pub mod app_side {
        use zero_ui::core::app::view_process::{ApiExtensionId, VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT};
        use zero_ui::{
            mouse::{MOUSE_HOVERED_EVENT, MOUSE_MOVE_EVENT},
            wgt_prelude::*,
        };

        /// Node that sends external display item and updates.
        pub fn custom_render_node() -> impl UiNode {
            custom_ext_node(extension_id)
        }
        // node that sends the cursor position, widget size and widget position in window to a view extension.
        // abstracted here to be reused by the other demos.
        pub(crate) fn custom_ext_node(extension_id: fn() -> ApiExtensionId) -> impl UiNode {
            let mut ext_id = ApiExtensionId::INVALID;
            let mut cursor = DipPoint::splat(Dip::MIN);
            let mut cursor_px = PxPoint::splat(Px::MIN);
            let mut render_size = PxSize::zero();

            // identifies this item in the view (for updates)
            let cursor_binding = super::api::BindingId::next_unique();

            match_node_leaf(move |op| match op {
                UiNodeOp::Init => {
                    WIDGET
                        .sub_event(&VIEW_PROCESS_INITED_EVENT)
                        .sub_event(&MOUSE_MOVE_EVENT)
                        .sub_event(&MOUSE_HOVERED_EVENT);
                    ext_id = extension_id();
                }
                UiNodeOp::Event { update } => {
                    if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                        if cursor != args.position {
                            cursor = args.position;
                            WIDGET.layout();
                        }
                    } else if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                        if args.is_mouse_leave() {
                            cursor = DipPoint::splat(Dip::MIN);
                            cursor_px = PxPoint::splat(Px::MIN);
                            WIDGET.render_update();
                        }
                    } else if VIEW_PROCESS_INITED_EVENT.on(update).is_some() {
                        ext_id = extension_id();
                        WIDGET.render();
                    }
                }
                UiNodeOp::Measure { desired_size, .. } => {
                    *desired_size = LAYOUT.constraints().fill_size();
                }
                UiNodeOp::Layout { final_size, .. } => {
                    *final_size = LAYOUT.constraints().fill_size();

                    if render_size != *final_size {
                        render_size = *final_size;
                        WIDGET.render();
                    }

                    if cursor != DipPoint::splat(Dip::MIN) {
                        let p = cursor.to_px(LAYOUT.scale_factor());
                        if cursor_px != p {
                            cursor_px = p;
                            WIDGET.render_update();
                        }
                    }
                }
                UiNodeOp::Render { frame } => {
                    // if extension is available
                    if ext_id != ApiExtensionId::INVALID {
                        let mut cursor = PxPoint::splat(Px::MIN);
                        if cursor_px != cursor {
                            if let Some(c) = frame.transform().inverse().and_then(|t| t.transform_point(cursor_px)) {
                                cursor = c;
                            }
                        }

                        let window_pos = frame.transform().transform_point(PxPoint::zero()).unwrap_or_default();

                        // push the entire custom item.
                        frame.push_extension_item(
                            ext_id,
                            &super::api::RenderPayload {
                                cursor_binding: Some(cursor_binding),
                                cursor,
                                size: render_size,
                                window_pos,
                            },
                        );
                    }
                }
                UiNodeOp::RenderUpdate { update } => {
                    // if extension is available
                    if ext_id != ApiExtensionId::INVALID {
                        let mut cursor = PxPoint::splat(Px::MIN);
                        if cursor_px != cursor {
                            if let Some(c) = update.transform().inverse().and_then(|t| t.transform_point(cursor_px)) {
                                cursor = c;
                            }
                        }

                        // push an update.
                        update.update_extension(ext_id, &super::api::RenderUpdatePayload { cursor_binding, cursor });
                    }
                }
                _ => {}
            })
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
        use std::collections::HashMap;

        use zero_ui::{
            core::app::view_process::{zero_ui_view_api::units::PxToWr, ApiExtensionId},
            core::units::PxPoint,
        };
        use zero_ui_view::{
            extensions::{RenderItemArgs, RenderUpdateArgs, RendererExtension, ViewExtensions},
            webrender::{
                api::{
                    units::{LayoutPoint, LayoutRect},
                    ColorF, CommonItemProperties,
                },
                euclid,
            },
        };

        pub fn extend(exts: &mut ViewExtensions) {
            exts.renderer(super::api::extension_name(), CustomExtension::new);
        }

        struct CustomExtension {
            // id of this extension, for tracing.
            _id: ApiExtensionId,
            // updated values
            updated: HashMap<super::api::BindingId, PxPoint>,
        }
        impl CustomExtension {
            fn new(id: ApiExtensionId) -> Self {
                Self {
                    _id: id,
                    updated: HashMap::new(),
                }
            }
        }
        impl RendererExtension for CustomExtension {
            fn is_config_only(&self) -> bool {
                false // retain the extension after renderer creation.
            }

            fn render_push(&mut self, args: &mut RenderItemArgs) {
                match args.payload.deserialize::<super::api::RenderPayload>() {
                    Ok(mut p) => {
                        if let Some(binding) = p.cursor_binding {
                            // updateable item
                            match self.updated.entry(binding) {
                                std::collections::hash_map::Entry::Occupied(e) => {
                                    if args.is_reuse {
                                        // item is old, use updated value
                                        p.cursor = *e.get();
                                    } else {
                                        // item is new, previous updated value invalid
                                        e.remove();
                                    }
                                }
                                std::collections::hash_map::Entry::Vacant(_) => {}
                            }
                        }

                        // render
                        let rect = LayoutRect::from_size(p.size.to_wr());
                        let part_size = rect.size() / 10.0;

                        let color = ColorF::new(0.5, 0.0, 1.0, 1.0);
                        let cursor = p.cursor.to_wr();

                        for y in 0..10 {
                            for x in 0..10 {
                                let part_pos = LayoutPoint::new(x as f32 * part_size.width, y as f32 * part_size.height);
                                let part_rect = euclid::Rect::new(part_pos, part_size).to_box2d();

                                let mut color = color;
                                let mid = part_pos.to_vector() + part_size.to_vector() / 2.0;
                                let dist = mid.to_point().distance_to(cursor).min(rect.width()) / rect.width();
                                color.g = 1.0 - dist;

                                let props = CommonItemProperties {
                                    clip_rect: part_rect,
                                    clip_chain_id: args.sc.clip_chain_id(args.list),
                                    spatial_id: args.sc.spatial_id(),
                                    flags: args.sc.primitive_flags(),
                                };
                                args.list.push_rect(&props, part_rect, color);
                            }
                        }
                    }
                    Err(e) => tracing::error!("invalid display item, {e}"),
                }
            }

            fn render_update(&mut self, args: &mut RenderUpdateArgs) {
                match args.payload.deserialize::<super::api::RenderUpdatePayload>() {
                    Ok(p) => {
                        self.updated.insert(p.cursor_binding, p.cursor);
                        // Request a full display list rebuild.
                        //
                        // This is optional because Webrender supports frame updates, using Webrender bindings,
                        // but just supporting render-updates is probably worth-it, if the full display-item payloads are large
                        // and update often.
                        //
                        // Note that even if you provide an optimal implementation and don't request a
                        // new_frame you still must handle the case when a display-item payload is reused
                        // after an update.
                        args.new_frame = true;

                        // For example we could have created a Webrender binding for each color square during
                        // `display_item_push`, then recomputed the colors and updated all here.
                        //
                        // args.properties.colors.push(..)
                        //
                        // Note that if you are going to do this you need to generate the binding keys in
                        // the app-process using the type `FrameValueKey<T>`, otherwise you will have key
                        // collisions with the normal animating properties.
                    }
                    Err(e) => tracing::error!("invalid update request, {e}"),
                }
            }
        }
    }

    /// Shared types.
    pub mod api {
        use std::sync::atomic::{AtomicU32, Ordering};

        use zero_ui::{
            core::app::view_process::ApiExtensionName,
            prelude::{PxPoint, PxSize},
        };

        pub fn extension_name() -> ApiExtensionName {
            ApiExtensionName::new("zero-ui.examples.extend_renderer.using_display_items").unwrap()
        }

        #[derive(serde::Serialize, serde::Deserialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub struct BindingId(u32);
        static ID_GEN: AtomicU32 = AtomicU32::new(0);
        impl BindingId {
            pub fn next_unique() -> Self {
                Self(ID_GEN.fetch_add(1, Ordering::Relaxed))
            }
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct RenderPayload {
            pub cursor_binding: Option<BindingId>,
            pub cursor: PxPoint,
            pub size: PxSize,
            pub window_pos: PxPoint,
        }

        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct RenderUpdatePayload {
            pub cursor_binding: BindingId,
            pub cursor: PxPoint,
        }
    }
}

/// Demo view extension custom renderer, integrated with Webrender using the blob API.
pub mod using_blob {
    /// App-process stuff, nodes.
    pub mod app_side {
        use zero_ui::{
            core::app::view_process::{ApiExtensionId, VIEW_PROCESS},
            prelude::UiNode,
        };

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

        use zero_ui::{
            core::app::view_process::{zero_ui_view_api::units::PxToWr, ApiExtensionId},
            prelude::{task::parking_lot::Mutex, PxPoint, PxSize},
        };
        use zero_ui_view::{
            extensions::{AsyncBlobRasterizer, BlobExtension, RenderItemArgs, RenderUpdateArgs, RendererExtension, ViewExtensions},
            webrender::{
                api::{
                    units::{BlobDirtyRect, DeviceIntRect, DeviceIntSize, LayoutRect},
                    BlobImageError, BlobImageKey, BlobImageParams, BlobImageResult, ColorF, CommonItemProperties, ImageDescriptor,
                    ImageDescriptorFlags, ImageFormat, RasterizedBlobImage,
                },
                euclid,
            },
        };

        pub fn extend(exts: &mut ViewExtensions) {
            exts.renderer(super::api::extension_name(), CustomExtension::new);
        }

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
            fn is_config_only(&self) -> bool {
                false // retain the extension after renderer creation.
            }

            fn configure(&mut self, args: &mut zero_ui_view::extensions::RendererConfigArgs) {
                // Blob entry point inside Webrender.
                args.blobs.push(Box::new(CustomBlobExtension {
                    renderer: Arc::clone(&self.renderer),
                }));

                // Worker threads will be used during rasterization.
                //
                // This option is always already set by the window. Note that this thread pool
                // is also used by Webrender's glyph rasterizer, so be careful not to clog it.
                self.renderer.lock().workers = args.options.workers.clone();
            }

            fn render_start(&mut self, _: &mut zero_ui_view::extensions::RenderArgs) {
                let mut renderer = self.renderer.lock();
                for t in renderer.tasks.iter_mut() {
                    if matches!(t.state, CustomRenderTaskState::Used) {
                        t.state = CustomRenderTaskState::Marked;
                    }
                }
            }

            fn render_end(&mut self, args: &mut zero_ui_view::extensions::RenderArgs) {
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
                            // updateable item, gets own image
                            CustomTaskParams::Bound(binding)
                        } else {
                            // not updateable item, shares images of same params
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

                            let key = if let Some(i) = renderer
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
                            };

                            key
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
                            zero_ui_view::webrender::api::ImageRendering::Auto,
                            zero_ui_view::webrender::api::AlphaType::Alpha,
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
        }

        struct CustomBlobExtension {
            renderer: Arc<Mutex<CustomRenderer>>,
        }
        impl BlobExtension for CustomBlobExtension {
            fn create_blob_rasterizer(&mut self) -> Box<dyn zero_ui_view::extensions::AsyncBlobRasterizer> {
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

            fn add(&mut self, _args: &zero_ui_view::extensions::BlobAddArgs) {
                // Webrender received the add request
                //
                // In this demo we already added from the display item.
            }

            fn update(&mut self, _args: &zero_ui_view::extensions::BlobUpdateArgs) {
                // Webrender received the update request
            }

            fn delete(&mut self, key: zero_ui::core::app::view_process::zero_ui_view_api::webrender_api::BlobImageKey) {
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
            fn rasterize(&mut self, args: &mut zero_ui_view::extensions::BlobRasterizerArgs) {
                if !self.snapshot.single_threaded {
                    // rasterize all tiles in parallel, Webrender also uses Rayon
                    // but the `rasterize` call is made in the SceneBuilderThread
                    // so we mount the workers thread-pool here.

                    use zero_ui::prelude::task::rayon::prelude::*;

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
            workers: Option<Arc<zero_ui::prelude::task::rayon::ThreadPool>>,
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
        use zero_ui::core::app::view_process::ApiExtensionName;

        pub use crate::using_display_items::api::*;

        pub fn extension_name() -> ApiExtensionName {
            ApiExtensionName::new("zero-ui.examples.extend_renderer.using_blob").unwrap()
        }
    }
}

/// Demo view extension custom renderer, integrated by drawing directly over the frame.
pub mod using_gl_overlay {
    /// App-process stuff, nodes.
    pub mod app_side {
        use zero_ui::{
            core::app::view_process::{ApiExtensionId, VIEW_PROCESS},
            prelude::UiNode,
        };

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
        use zero_ui::{
            core::app::view_process::ApiExtensionId,
            prelude::{units::PxRect, Px, PxPoint, PxSize},
        };
        use zero_ui_view::{
            extensions::{RenderItemArgs, RenderUpdateArgs, RendererExtension, ViewExtensions},
            gleam::gl,
        };

        use super::api::BindingId;

        pub fn extend(exts: &mut ViewExtensions) {
            exts.renderer(super::api::extension_name(), CustomExtension::new);
        }

        struct CustomExtension {
            // id of this extension, for tracing.
            _id: ApiExtensionId,
            renderer: Option<CustomRenderer>,
        }
        impl CustomExtension {
            fn new(id: ApiExtensionId) -> Self {
                Self { _id: id, renderer: None }
            }
        }
        impl RendererExtension for CustomExtension {
            fn is_config_only(&self) -> bool {
                false // retain the extension after renderer creation.
            }

            fn renderer_inited(&mut self, args: &mut zero_ui_view::extensions::RendererInitedArgs) {
                // shaders/programs can be loaded here.
                self.renderer = Some(CustomRenderer::load(args.gl));
            }
            fn renderer_deinited(&mut self, args: &mut zero_ui_view::extensions::RendererDeinitedArgs) {
                // ..and unloaded here.
                if let Some(r) = self.renderer.take() {
                    r.unload(args.gl);
                }
            }

            fn render_start(&mut self, _: &mut zero_ui_view::extensions::RenderArgs) {
                if let Some(r) = &mut self.renderer {
                    r.clear();
                }
            }

            fn render_push(&mut self, args: &mut RenderItemArgs) {
                match args.payload.deserialize::<super::api::RenderPayload>() {
                    Ok(p) => {
                        // we use the display list for convenience only, each render/update
                        // is paired with Webrender updates, this extension does not actually
                        // use Webrender.
                        if let Some(r) = &mut self.renderer {
                            r.push_task(&p);
                        }
                    }
                    Err(e) => tracing::error!("invalid display item, {e}"),
                }
            }

            fn render_update(&mut self, args: &mut RenderUpdateArgs) {
                match args.payload.deserialize::<super::api::RenderUpdatePayload>() {
                    Ok(p) => {
                        if let Some(r) = &mut self.renderer {
                            r.update_task(&p);
                        }
                    }
                    Err(e) => tracing::error!("invalid update request, {e}"),
                }
            }

            fn redraw(&mut self, args: &mut zero_ui_view::extensions::RedrawArgs) {
                if let Some(r) = &mut self.renderer {
                    r.redraw(args.size, args.gl);
                }
            }
        }

        struct CustomRenderer {
            tasks: Vec<DrawTask>,
            unloaded_ok: bool,
        }
        impl CustomRenderer {
            pub fn load(_gl: &dyn gl::Gl) -> Self {
                // let vao = gl.gen_vertex_arrays(1);

                Self {
                    tasks: vec![],
                    unloaded_ok: false,
                }
            }
            fn unload(mut self, _gl: &dyn gl::Gl) {
                // gl.delete_vertex_arrays(&self.vao);

                self.tasks.clear();
                self.unloaded_ok = true;
            }

            pub fn clear(&mut self) {
                self.tasks.clear();
            }

            pub fn push_task(&mut self, task: &super::api::RenderPayload) {
                self.tasks.push(DrawTask {
                    area: PxRect::new(task.window_pos, task.size),
                    cursor: task.cursor,
                    cursor_binding: task.cursor_binding,
                });
            }

            pub fn update_task(&mut self, p: &super::api::RenderUpdatePayload) {
                if let Some(i) = self.tasks.iter_mut().find(|i| i.cursor_binding == Some(p.cursor_binding)) {
                    i.cursor = p.cursor;
                }
            }

            pub fn redraw(&mut self, canvas_size: PxSize, gl: &dyn gl::Gl) {
                // gl (0, 0) is the bottom-left corner not the top-left.
                let gl_y = |max_y: Px| canvas_size.height - max_y;

                for task in &self.tasks {
                    // gl.bind_vertex_array(vao);

                    gl.enable(gl::SCISSOR_TEST);
                    gl.scissor(
                        task.area.origin.x.0,
                        gl_y(task.area.max_y()).0,
                        task.area.size.width.0,
                        task.area.size.height.0,
                    );
                    gl.clear_color(0.0, 0.0, 0.0, 1.0);
                    gl.clear(gl::COLOR_BUFFER_BIT);
                }
                for task in &self.tasks {
                    if task.cursor.x >= Px(0)
                        && task.cursor.y < task.area.width()
                        && task.cursor.y >= Px(0)
                        && task.cursor.y < task.area.height()
                    {
                        let r = task.cursor.x.0 as f32 / task.area.width().0 as f32;
                        let b = task.cursor.y.0 as f32 / task.area.height().0 as f32;

                        let x = task.area.origin.x + task.cursor.x - Px(50);
                        let y = task.area.origin.y + task.cursor.y - Px(50);
                        let cursor = PxRect::new(PxPoint::new(x, y), PxSize::splat(Px(100)));

                        gl.enable(gl::SCISSOR_TEST);
                        gl.scissor(cursor.origin.x.0, gl_y(cursor.max_y()).0, cursor.size.width.0, cursor.size.height.0);
                        gl.clear_color(r, 0.5, b, 1.0);
                        gl.clear(gl::COLOR_BUFFER_BIT);
                    }
                }
            }
        }
        impl Drop for CustomRenderer {
            fn drop(&mut self) {
                if !self.unloaded_ok {
                    tracing::error!("CustomRenderer::unload was not used to drop the renderer");
                }
            }
        }

        #[derive(Debug)]
        struct DrawTask {
            area: PxRect,
            cursor: PxPoint,
            cursor_binding: Option<BindingId>,
        }
    }

    pub mod api {
        use zero_ui::core::app::view_process::ApiExtensionName;

        pub use crate::using_display_items::api::*;

        pub fn extension_name() -> ApiExtensionName {
            ApiExtensionName::new("zero-ui.examples.extend_renderer.using_gl_overlay").unwrap()
        }
    }
}

/// Demo view extension custom renderer, integrated by drawing to a texture uses as an image.
pub mod using_gl_texture {
    /// App-process stuff, nodes.
    pub mod app_side {
        use zero_ui::{
            core::app::view_process::{ApiExtensionId, VIEW_PROCESS},
            prelude::UiNode,
        };

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
        use zero_ui::{
            core::app::view_process::{zero_ui_view_api::units::PxToWr, ApiExtensionId},
            prelude::units::PxRect,
        };
        use zero_ui_view::{
            extensions::{RenderItemArgs, RendererExtension, ViewExtensions},
            gleam::gl,
            webrender::api::{
                units::{DeviceIntSize, TexelRect},
                AlphaType, ColorF, CommonItemProperties, ExternalImageData, ExternalImageId, ExternalImageType, ImageDescriptor,
                ImageDescriptorFlags, ImageFormat, ImageKey, ImageRendering,
            },
        };

        pub fn extend(exts: &mut ViewExtensions) {
            exts.renderer(super::api::extension_name(), CustomExtension::new);
        }

        struct TextureInfo {
            // texture in OpenGL.
            texture: gl::GLuint,
            // texture in external image registry (for webrender).
            external_id: ExternalImageId,
            // texture in renderer (and display lists).
            image_key: ImageKey,
        }

        struct CustomExtension {
            // id of this extension, for tracing.
            _id: ApiExtensionId,

            texture: Option<TextureInfo>,
        }
        impl CustomExtension {
            fn new(id: ApiExtensionId) -> Self {
                Self { _id: id, texture: None }
            }
        }
        impl RendererExtension for CustomExtension {
            fn is_config_only(&self) -> bool {
                false // retain the extension after renderer creation.
            }

            fn renderer_inited(&mut self, args: &mut zero_ui_view::extensions::RendererInitedArgs) {
                // gl available here and in `redraw`.
                //
                // dynamic textures can be generated by collecting request on `command` or on `render_push` and
                // generating on the next `redraw` that will happen after `render_push` or on request after `command`.

                let size = DeviceIntSize::splat(100);

                // OpenGL
                let texture = args.gl.gen_textures(1)[0];
                args.gl.bind_texture(gl::TEXTURE_2D, texture);
                let mut img = vec![0u8; size.width as usize * size.height as usize * 4];
                let mut line = 0u8;
                let mut col = 0u8;
                for rgba in img.chunks_exact_mut(4) {
                    rgba[0] = 255;
                    rgba[1] = 10 + line * 3;
                    rgba[2] = 10 + line * 3;
                    rgba[3] = 255;

                    col = col.wrapping_add(1);
                    if col == 0 {
                        line = line.wrapping_add(1);
                    }
                }
                args.gl.tex_image_2d(
                    gl::TEXTURE_2D,
                    0,
                    gl::RGBA as _,
                    size.width,
                    size.height,
                    0,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    Some(&img),
                );

                // Webrender
                let external_id = args
                    .external_images
                    .register_texture(TexelRect::new(0.0, 0.0, size.width as f32, size.height as f32), texture);

                let image_key = args.api.generate_image_key();
                let mut txn = zero_ui_view::webrender::Transaction::new();
                txn.add_image(
                    image_key,
                    ImageDescriptor {
                        format: ImageFormat::RGBA8,
                        size,
                        stride: None,
                        offset: 0,
                        flags: ImageDescriptorFlags::IS_OPAQUE,
                    },
                    zero_ui_view::webrender::api::ImageData::External(ExternalImageData {
                        id: external_id,
                        channel_index: 0,
                        image_type: ExternalImageType::TextureHandle(zero_ui_view::webrender::api::ImageBufferKind::Texture2D),
                    }),
                    None,
                );
                args.api.send_transaction(args.document_id, txn);

                self.texture = Some(TextureInfo {
                    texture,
                    external_id,
                    image_key,
                });
            }

            fn renderer_deinited(&mut self, args: &mut zero_ui_view::extensions::RendererDeinitedArgs) {
                if let Some(t) = self.texture.take() {
                    let _ = t.external_id; // already cleanup by renderer deinit.
                    args.gl.delete_textures(&[t.texture]);
                }
            }

            fn render_push(&mut self, args: &mut RenderItemArgs) {
                match args.payload.deserialize::<super::api::RenderPayload>() {
                    Ok(p) => {
                        if let Some(t) = &self.texture {
                            let rect = PxRect::from_size(p.size).to_wr();
                            let props = CommonItemProperties {
                                clip_rect: rect,
                                clip_chain_id: args.sc.clip_chain_id(args.list),
                                spatial_id: args.sc.spatial_id(),
                                flags: args.sc.primitive_flags(),
                            };
                            args.list
                                .push_image(&props, rect, ImageRendering::Auto, AlphaType::Alpha, t.image_key, ColorF::WHITE);
                        }
                    }
                    Err(e) => tracing::error!("invalid display item, {e}"),
                }
            }
        }
    }

    pub mod api {
        use zero_ui::core::app::view_process::ApiExtensionName;

        pub use crate::using_display_items::api::*;

        pub fn extension_name() -> ApiExtensionName {
            ApiExtensionName::new("zero-ui.examples.extend_renderer.using_gl_texture").unwrap()
        }
    }
}
