//! Image cache API.

use std::{path::Path, sync::Arc};

use crate::{app::AppExtension, service::Service, task::http::TryUri};
use image::DynamicImage;
use webrender::api::*;

#[derive(Service)]
pub struct Images {}
impl Images {
    fn new() -> Self {
        Self {}
    }
}

#[derive(Default)]
pub struct ImageManager {}
impl AppExtension for ImageManager {
    fn init(&mut self, ctx: &mut crate::context::AppContext) {
        ctx.services.register(Images::new());
    }
}

pub struct Image {
    data: ImageData,
    descriptor: ImageDescriptor,
}

impl Image {
    pub fn from_file(file: impl AsRef<Path>) -> Self {
        todo!()
    }

    pub fn from_uri(uri: impl TryUri) -> Self {
        todo!()
    }

    pub fn from_decoded(image: DynamicImage) -> Self {
        todo!()
    }

    fn render_image(&self, api: &Arc<RenderApi>) -> ImageKey {
        api.generate_image_key()
    }
}

impl crate::render::Image for Image {
    fn image_key(&self, api: &Arc<RenderApi>) -> ImageKey {
        self.render_image(api)
    }
}
