//! Demo view extension custom renderer, integrated by drawing directly over the frame.

/// App-process stuff, nodes.
pub mod app_side {
    use zng::prelude::UiNode;
    use zng_app::view_process::VIEW_PROCESS;
    use zng_view_api::api_extension::ApiExtensionId;

    /// Node that sends external display item and updates.
    pub fn custom_render_node() -> UiNode {
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
    use zng::layout::{Px, PxPoint, PxRect, PxSize};
    use zng_view::{
        extensions::{RenderItemArgs, RenderUpdateArgs, RendererExtension},
        gleam::gl,
    };
    use zng_view_api::api_extension::ApiExtensionId;

    use super::api::BindingId;

    zng_view::view_process_extension!(|exts| {
        exts.renderer(super::api::extension_name(), CustomExtension::new);
    });

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
        fn is_init_only(&self) -> bool {
            false // retain the extension after renderer creation.
        }

        fn renderer_inited(&mut self, args: &mut zng_view::extensions::RendererInitedArgs) {
            // shaders/programs can be loaded here.
            self.renderer = Some(CustomRenderer::load(&**args.context.gl()));
        }
        fn renderer_deinited(&mut self, args: &mut zng_view::extensions::RendererDeinitedArgs) {
            // ..and unloaded here.
            if let Some(r) = self.renderer.take() {
                r.unload(&**args.context.gl());
            }
        }

        fn render_start(&mut self, _: &mut zng_view::extensions::RenderArgs) {
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

        fn redraw(&mut self, args: &mut zng_view::extensions::RedrawArgs) {
            if let Some(r) = &mut self.renderer {
                r.redraw(args.size, &**args.context.gl());
            }
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
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
                if task.cursor.x >= 0 && task.cursor.y < task.area.width() && task.cursor.y >= 0 && task.cursor.y < task.area.height() {
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
    use zng_view_api::api_extension::ApiExtensionName;

    pub use crate::using_display_items::api::*;

    pub fn extension_name() -> ApiExtensionName {
        ApiExtensionName::new("zng.examples.extend_renderer.using_gl_overlay").unwrap()
    }
}
