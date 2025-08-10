//! Demo view extension renderer, using only Webrender display items.

/// App-process stuff, nodes.
pub mod app_side {
    use zng::{
        mouse::{MOUSE_HOVERED_EVENT, MOUSE_MOVE_EVENT},
        prelude_wgt::*,
    };
    use zng_app::view_process::{VIEW_PROCESS, VIEW_PROCESS_INITED_EVENT};
    use zng_view_api::api_extension::ApiExtensionId;

    /// Node that sends external display item and updates.
    pub fn custom_render_node() -> UiNode {
        custom_ext_node(extension_id)
    }
    // node that sends the cursor position, widget size and widget position in window to a view extension.
    // abstracted here to be reused by the other demos.
    pub(crate) fn custom_ext_node(extension_id: fn() -> ApiExtensionId) -> UiNode {
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
                    if cursor_px != cursor
                        && let Some(c) = frame.transform().inverse().and_then(|t| t.transform_point(cursor_px))
                    {
                        cursor = c;
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
                    if cursor_px != cursor
                        && let Some(c) = update.transform().inverse().and_then(|t| t.transform_point(cursor_px))
                    {
                        cursor = c;
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

    use zng::prelude_wgt::PxPoint;
    use zng_view::{
        extensions::{PxToWr as _, RenderItemArgs, RenderUpdateArgs, RendererExtension},
        webrender::{
            api::{
                ColorF, CommonItemProperties,
                units::{LayoutPoint, LayoutRect},
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
        fn is_init_only(&self) -> bool {
            false // retain the extension after renderer creation.
        }

        fn render_push(&mut self, args: &mut RenderItemArgs) {
            match args.payload.deserialize::<super::api::RenderPayload>() {
                Ok(mut p) => {
                    if let Some(binding) = p.cursor_binding {
                        // updatable item
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

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
}

/// Shared types.
pub mod api {
    use std::sync::atomic::{AtomicU32, Ordering};

    use zng::layout::{PxPoint, PxSize};
    use zng_view_api::api_extension::ApiExtensionName;

    pub fn extension_name() -> ApiExtensionName {
        ApiExtensionName::new("zng.examples.extend_renderer.using_display_items").unwrap()
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
