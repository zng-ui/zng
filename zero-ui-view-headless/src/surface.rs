use std::{fmt, rc::Rc};

use gleam::gl;
use webrender::{RenderApi, Renderer, api::{self as webrender_api, ColorF, DocumentId, DynamicProperties, Epoch, FontInstanceKey, FontInstanceOptions, FontInstancePlatformOptions, FontKey, FontVariation, IdNamespace, ImageDescriptor, ImageKey, PipelineId}};
use zero_ui_view_api::{units::*, FrameRequest, HeadlessConfig, WinId};

/// A headless "window".
pub struct Surface {
    id: WinId,
    pipeline_id: PipelineId,
    document_id: DocumentId,
    api: RenderApi,
    size: DipSize,
    scale_factor: f32,

    //context: GlHeadlessContext,
    gl: Rc<dyn gl::Gl>,
    renderer: Option<Renderer>,
    rbos: [u32; 2],
    fbo: u32,

    frame_id: Epoch,
    resized: bool,
}
impl fmt::Debug for Surface {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Surface")
            .field("id", &self.id)
            .field("pipeline_id", &self.pipeline_id)
            .field("document_id", &self.document_id)
            .field("size", &self.size)
            .field("scale_factor", &self.scale_factor)
            .finish_non_exhaustive()
    }
}
impl Surface {
    pub fn open(id: WinId, config: HeadlessConfig) -> Self {
        todo!()
    }

    pub fn id(&self) -> WinId {
        self.id
    }

    pub fn namespace_id(&self) -> IdNamespace {
        self.api.get_namespace_id()
    }

    pub fn pipeline_id(&self) -> PipelineId {
        self.pipeline_id
    }

    pub fn scale_factor(&self) -> f32 {
        self.scale_factor
    }

    pub fn size(&self) -> DipSize {
        self.size
    }

    pub fn set_transparent(&mut self, transparent: bool) {
        todo!()
    }

    pub fn set_size(&mut self, size: DipSize, scale_factor: f32) {
        todo!()
    }

    pub fn add_image(&mut self, descriptor: ImageDescriptor, data: Vec<u8>) {
        todo!()
    }

    pub fn update_image(
        &mut self,
        key: ImageKey,
        descriptor: ImageDescriptor,
        data: Vec<u8>,
        dirty_rect: webrender_api::units::ImageDirtyRect,
    ) {
        todo!()
    }

    pub fn delete_image(&mut self, key: ImageKey) {
        todo!()
    }

    pub fn add_font(&mut self, font: Vec<u8>, index: u32) -> FontKey {
        todo!()
    }

    pub fn delete_font(&mut self, key: FontKey) {
        todo!()
    }

    pub fn add_font_instance(
        &mut self,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<FontInstanceOptions>,
        plataform_options: Option<FontInstancePlatformOptions>,
        variations: Vec<FontVariation>,
    ) -> FontInstanceKey {
        todo!()
    }

    pub fn delete_font_instance(&mut self, instance_key: FontInstanceKey) {
        todo!()
    }

    pub fn render(&mut self, frame: FrameRequest) {
        todo!()
    }

    pub fn render_update(&mut self, updates: DynamicProperties, clear_color: Option<ColorF>) {
        todo!()
    }

    
}
