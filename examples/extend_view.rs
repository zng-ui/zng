use zero_ui::prelude::*;
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
    App::default().run_window(async {
        Window! {
            // renderer_debug = {
            //     use zero_ui::core::render::webrender_api::DebugFlags;
            //     DebugFlags::TEXTURE_CACHE_DBG | DebugFlags::TEXTURE_CACHE_DBG_CLEAR_EVICTED
            // };

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
                    }
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
    exts
}

/// Demo view extension renderer, using only Webrender display items.
pub mod using_display_items {
    /// App-process stuff, nodes.
    pub mod app_side {
        use zero_ui::{
            core::{
                app::view_process::{ApiExtensionId, VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT},
                mouse::{MOUSE_HOVERED_EVENT, MOUSE_MOVE_EVENT},
            },
            prelude::new_widget::*,
        };

        /// Node that sends external display item and updates.
        pub fn custom_render_node() -> impl UiNode {
            custom_ext_node(extension_id)
        }
        // node that sends the cursor position and widget size to a view extension.
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
                        let p = cursor.to_px(LAYOUT.scale_factor().0);
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

                        // push the entire custom item.
                        frame.push_extension_item(
                            ext_id,
                            &super::api::RenderPayload {
                                cursor_binding: Some(cursor_binding),
                                cursor,
                                size: render_size,
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
            core::app::view_process::ApiExtensionId,
            prelude::{units::PxToWr, PxPoint},
        };
        use zero_ui_view::{
            extensions::{RenderItemArgs, RenderUpdateArgs, RendererExtension, ViewExtensions},
            webrender::{
                api::{
                    units::{LayoutPoint, LayoutRect},
                    ColorF, CommonItemProperties, PrimitiveFlags,
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
                                    flags: PrimitiveFlags::empty(),
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
            core::app::view_process::ApiExtensionId,
            prelude::{task::parking_lot::Mutex, units::PxToWr, Px, PxPoint, PxSize},
        };
        use zero_ui_view::{
            extensions::{AsyncBlobRasterizer, BlobExtension, RenderItemArgs, RenderUpdateArgs, RendererExtension, ViewExtensions},
            webrender::{
                api::{
                    units::{BlobDirtyRect, DeviceIntRect, DeviceIntSize, LayoutRect},
                    BlobImageError, BlobImageKey, BlobImageParams, BlobImageResult, ColorF, CommonItemProperties, ImageDescriptor,
                    ImageDescriptorFlags, ImageFormat, PrimitiveFlags, RasterizedBlobImage,
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
            // renderer, shared between blob extensions.
            renderer: Arc<Mutex<CustomRenderer>>,
        }
        impl CustomExtension {
            fn new(id: ApiExtensionId) -> Self {
                Self {
                    _id: id,
                    updated: HashMap::new(),
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
                    Ok(mut p) => {
                        let mut renderer = self.renderer.lock();
                        let renderer = &mut *renderer;

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

                        let mut key = None;
                        if let Some(i) = renderer.task_params.get(&(p.size, p.cursor)) {
                            let t = &mut renderer.tasks[*i];
                            if matches!(t.state, CustomRenderTaskState::Marked | CustomRenderTaskState::Used) {
                                // already rendering (size, cursor)
                                //
                                // in this demo we can identify the blob image by their parameters,
                                // this is not always possible, you may need to generate an unique
                                // id for each blob, either in the app-process or using the `RendererExtension::command`
                                // method to return an ID for the app-process.
                                key = Some(t.key);
                                t.state = CustomRenderTaskState::Used;
                            }
                        }
                        let blob_key = if let Some(k) = key {
                            k
                        } else {
                            // start rendering (size, cursor)
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
                                renderer.task_params.remove(&(t.size, t.cursor));

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

                                renderer.task_params.insert((t.size, t.cursor), i);

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
                                renderer.task_params.insert((p.size, p.cursor), i);
                                if let Some(b) = p.cursor_binding {
                                    renderer.task_binding.insert(b, i);
                                }

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
                            flags: PrimitiveFlags::empty(),
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
                        // update value for reuse patches (see the `using_display_items` demo)
                        self.updated.insert(p.cursor_binding, p.cursor);

                        // update the render task, in this demo this just means reassociating the blob key.
                        let mut renderer = self.renderer.lock();
                        let renderer = &mut *renderer;

                        if let Some(&i) = renderer.task_binding.get(&p.cursor_binding) {
                            let t = &mut renderer.tasks[i];
                            renderer.task_params.remove(&(t.size, t.cursor));

                            let was_shared = t.cursor == PxPoint::splat(Px::MIN);

                            t.cursor = p.cursor;
                            renderer.task_params.insert((t.size, t.cursor), i);

                            if !was_shared {
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

                                return;
                            }
                        }

                        // you can always just request a display-list rebuild.
                        args.new_frame = true;
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
                    renderer.task_params.remove(&(t.size, t.cursor));
                    if let Some(b) = &t.cursor_binding {
                        renderer.task_binding.remove(b);
                    }
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
            task_params: HashMap<(PxSize, PxPoint), usize>,
            task_binding: HashMap<super::api::BindingId, usize>,
            single_threaded: bool,
            workers: Option<Arc<zero_ui::prelude::task::rayon::ThreadPool>>,
        }

        #[derive(Clone)]
        struct CustomRenderTask {
            key: BlobImageKey,
            size: PxSize,
            cursor: PxPoint,
            cursor_binding: Option<super::api::BindingId>,
            state: CustomRenderTaskState,
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
