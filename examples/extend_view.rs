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
                mouse::MOUSE_MOVE_EVENT,
            },
            prelude::new_widget::*,
        };

        /// Node that generates display items and render updates for the custom renderer.
        pub fn custom_render_node() -> impl UiNode {
            let mut ext_id = ApiExtensionId::INVALID;
            let mut cursor = DipPoint::zero();
            let mut cursor_px = PxPoint::zero();
            let mut render_size = PxSize::zero();

            // identifies this item in the view (for updates)
            let cursor_binding = super::api::BindingId::next_unique();

            match_node_leaf(move |op| match op {
                UiNodeOp::Init => {
                    WIDGET.sub_event(&VIEW_PROCESS_INITED_EVENT).sub_event(&MOUSE_MOVE_EVENT);
                    ext_id = extension_id();
                }
                UiNodeOp::Event { update } => {
                    if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                        if cursor != args.position {
                            cursor = args.position;
                            WIDGET.layout();
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
                        if let Some(cursor) = frame.transform().transform_point(cursor_px) {
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
                        if let Some(cursor) = update.transform().transform_point(cursor_px) {
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
            core::app::view_process::{zero_ui_view_api::webrender_api::DisplayListBuilder, ApiExtensionId, ApiExtensionPayload},
            prelude::{PxPoint, PxSize},
        };
        use zero_ui_view::extensions::{RendererExtension, ViewExtensions};

        pub fn extend(exts: &mut ViewExtensions) {
            exts.renderer(super::api::extension_name(), CustomExtension::new);
        }

        struct CustomExtension {
            // id of this extension, for tracing.
            _id: ApiExtensionId,
            // updatable items
            bindings: HashMap<super::api::BindingId, ViewItem>,
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

            fn begin_display_list(&mut self) {
                self.bindings.clear();
            }

            fn finish_display_list(&mut self) {
                tracing::info!("finished rendering, ext: {:?}", self._id);
            }

            fn display_item_push(&mut self, payload: &ApiExtensionPayload, _wr_list: &mut DisplayListBuilder) {
                match payload.deserialize::<super::api::RenderPayload>() {
                    Ok(p) => {
                        // update bindings
                        let item = ViewItem {
                            cursor: p.cursor,
                            size: p.size,
                        };
                        if let Some(id) = p.cursor_binding {
                            if self.bindings.insert(id, item).is_some() {
                                tracing::error!("repeated binding id, {id:?}");
                            }
                        }

                        // render
                        tracing::info!("TODO, render, missing space&clip here, {:#?}", (item.cursor, item.size));
                    }
                    Err(e) => tracing::error!("invalid display item, {e}"),
                }
            }

            // TODO, update render, missing API
        }

        #[derive(Clone, Copy, Debug)]
        struct ViewItem {
            cursor: PxPoint,
            size: PxSize,
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
