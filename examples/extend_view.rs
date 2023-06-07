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
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                children_align = Align::CENTER;
                children = ui_vec![
                    Text!("Using Display Items"),
                    Container! {
                        size = (500, 400);
                        child = using_display_items::app_side::custom_render_node();
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
    exts
}

/// Demo view extension renderer, using only Webrender display items.
pub mod using_display_items {
    /// App-process stuff, nodes.
    pub mod app_side {
        use zero_ui::{
            core::{
                app::view_process::{ApiExtensionId, ApiExtensionPayload, ViewProcessOffline, VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT},
                mouse::{MOUSE_HOVERED_EVENT, MOUSE_MOVE_EVENT},
            },
            prelude::new_widget::*,
        };

        /// Node that generates display items and render updates for the custom renderer.
        pub fn custom_render_node() -> impl UiNode {
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
                    let p = cursor.to_px(LAYOUT.scale_factor().0);
                    if cursor_px != p {
                        cursor_px = p;
                        WIDGET.render_update();
                    }
                }
                UiNodeOp::Render { frame } => {
                    // if extension is available
                    if ext_id != ApiExtensionId::INVALID {
                        if let Some(cursor) = frame.transform().inverse().and_then(|t| t.transform_point(cursor_px)) {
                            // push the entire custom item.
                            frame.push_extension_item(
                                ext_id,
                                ApiExtensionPayload::serialize(&super::api::RenderPayload {
                                    cursor_binding: Some(cursor_binding),
                                    cursor,
                                    size: render_size,
                                })
                                .unwrap(),
                            );
                        }
                    }
                }
                UiNodeOp::RenderUpdate { update } => {
                    // if extension is available
                    if ext_id != ApiExtensionId::INVALID {
                        if let Some(cursor) = update.transform().inverse().and_then(|t| t.transform_point(cursor_px)) {
                            // push an update.
                            update.update_extension(
                                ext_id,
                                ApiExtensionPayload::serialize(&super::api::RenderUpdatePayload { cursor_binding, cursor }).unwrap(),
                            );
                        }
                    }
                }
                _ => {}
            })
        }

        pub fn extension_id() -> ApiExtensionId {
            match VIEW_PROCESS.extensions() {
                Ok(exts) => {
                    let ext = super::api::extension_name();
                    match exts.id(&ext) {
                        Some(id) => id,
                        None => {
                            tracing::error!("extension {ext:?} not available");
                            ApiExtensionId::INVALID
                        }
                    }
                }
                Err(ViewProcessOffline) => ApiExtensionId::INVALID,
            }
        }
    }

    /// View-process stuff, the actual extension.
    pub mod view_side {
        use std::collections::HashMap;

        use zero_ui::{
            core::app::view_process::{
                zero_ui_view_api::{DisplayExtensionItemArgs, DisplayExtensionUpdateArgs},
                ApiExtensionId,
            },
            prelude::{units::PxToWr, PxPoint},
        };
        use zero_ui_view::{
            extensions::{RendererExtension, ViewExtensions},
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
            bindings: HashMap<super::api::BindingId, PxPoint>,
        }
        impl CustomExtension {
            fn new(id: ApiExtensionId) -> Self {
                Self {
                    _id: id,
                    bindings: HashMap::new(),
                }
            }
        }
        impl RendererExtension for CustomExtension {
            fn is_config_only(&self) -> bool {
                false // retain the extension after renderer creation.
            }

            fn display_item_push(&mut self, args: &mut DisplayExtensionItemArgs) {
                match args.payload.deserialize::<super::api::RenderPayload>() {
                    Ok(mut p) => {
                        if let Some(binding) = p.cursor_binding {
                            // updateable item
                            match self.bindings.entry(binding) {
                                std::collections::hash_map::Entry::Occupied(e) => {
                                    if *args.is_reuse {
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

                        for y in 0..10 {
                            for x in 0..10 {
                                let part_pos = LayoutPoint::new(x as f32 * part_size.width, y as f32 * part_size.height);
                                let part_rect = euclid::Rect::new(part_pos, part_size).to_box2d();

                                let cursor = p.cursor.to_wr();
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

            fn render_update(&mut self, args: &mut DisplayExtensionUpdateArgs) {
                match args.payload.deserialize::<super::api::RenderUpdatePayload>() {
                    Ok(p) => {
                        self.bindings.insert(p.cursor_binding, p.cursor);
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
