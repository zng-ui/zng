mod layout;
mod stack;
mod text;

pub use layout::*;
pub use stack::*;
pub use text::*;

use app_units::Au;
use font_loader::system_fonts;
use std::collections::HashMap;
use webrender::api::*;
pub use glutin::event::KeyboardInput;
pub use webrender::api::{LayoutPoint, LayoutRect, LayoutSize};

pub struct InitContext {
    pub api: RenderApi,
    pub document_id: DocumentId,
    fonts: HashMap<String, FontInstances>,
}

struct FontInstances {
    font_key: FontKey,
    instances: HashMap<u32, FontInstanceKey>,
}

#[derive(Clone)]
pub struct FontInstance {
    pub font_key: FontKey,
    pub instance_key: FontInstanceKey,
    pub size: u32,
}

impl InitContext {
    pub fn new(api: RenderApi, document_id: DocumentId) -> Self {
        InitContext {
            api,
            document_id,
            fonts: HashMap::new(),
        }
    }

    pub fn font(&mut self, family: &str, size: u32) -> FontInstance {
        let mut uncached_font = true;

        if let Some(font) = self.fonts.get(family) {
            if let Some(&instance_key) = font.instances.get(&size) {
                return FontInstance {
                    font_key: font.font_key,
                    instance_key,
                    size,
                };
            }
            uncached_font = false;
        }

        let mut txn = Transaction::new();

        if uncached_font {
            let property = system_fonts::FontPropertyBuilder::new().family(family).build();
            let (font, _) = system_fonts::get(&property).unwrap();

            let font_key = self.api.generate_font_key();
            txn.add_raw_font(font_key, font, 0);

            self.fonts.insert(
                family.to_owned(),
                FontInstances {
                    font_key,
                    instances: HashMap::new(),
                },
            );
        }

        let f = self.fonts.get_mut(family).unwrap();

        let instance_key = self.api.generate_font_instance_key();
        txn.add_font_instance(
            instance_key,
            f.font_key,
            Au::from_px(size as i32),
            None,
            None,
            Vec::new(),
        );
        f.instances.insert(size, instance_key);

        self.api.send_transaction(self.document_id, txn);

        FontInstance {
            font_key: f.font_key,
            instance_key,
            size,
        }
    }
}

pub struct RenderContext<'b> {
    builder: &'b mut DisplayListBuilder,
    spatial_id: SpatialId,
    final_size: LayoutSize,
}
impl<'b> RenderContext<'b> {
    pub fn new(builder: &'b mut DisplayListBuilder, spatial_id: SpatialId, final_size: LayoutSize) -> Self {
        RenderContext {
            builder,
            spatial_id,
            final_size,
        }
    }

    pub fn push_child(&mut self, child: &impl Ui, final_rect: &LayoutRect) {
        let spatial_id = self.builder.push_reference_frame(
            final_rect,
            self.spatial_id,
            TransformStyle::Flat,
            PropertyBinding::Value(LayoutTransform::default()),
            ReferenceFrameKind::Transform,
        );
        child.render(RenderContext::new(self.builder, spatial_id, final_rect.size));
        self.builder.pop_reference_frame();

        // about Stacking Contexts
        //https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Positioning/Understanding_z_index/The_stacking_context
    }

    fn layout_and_clip(&self, final_rect: LayoutRect) -> (LayoutPrimitiveInfo, SpaceAndClipInfo) {
        let lpi = LayoutPrimitiveInfo::new(final_rect);
        let sci = SpaceAndClipInfo {
            spatial_id: self.spatial_id,
            clip_id: ClipId::root(self.spatial_id.pipeline_id()),
        };

        (lpi, sci)
    }

    pub fn push_rect(&mut self, final_rect: LayoutRect, color: ColorF) {
        let (lpi, sci) = self.layout_and_clip(final_rect);
        self.builder.push_rect(&lpi, &sci, color);
    }

    pub fn push_gradient(
        &mut self,
        final_rect: LayoutRect,
        start: LayoutPoint,
        end: LayoutPoint,
        stops: Vec<GradientStop>,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect);

        let grad = self.builder.create_gradient(start, end, stops, ExtendMode::Clamp);
        self.builder
            .push_gradient(&lpi, &sci, grad, final_rect.size, LayoutSize::default());
    }

    pub fn push_text(
        &mut self,
        final_rect: LayoutRect,
        glyphs: &[GlyphInstance],
        font_instance_key: FontInstanceKey,
        color: ColorF,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect);

        self.builder
            .push_text(&lpi, &sci, &glyphs, font_instance_key, color, None);
    }
}

impl<'b> RenderContext<'b> {
    pub fn final_size(&self) -> LayoutSize {
        self.final_size
    }
}

#[derive(Clone, Copy)]
pub enum Update {
    None,
    Render,
    Layout
}

pub trait Ui {
    fn on_keyboard_input(&mut self, _input: &KeyboardInput) -> Update { Update::None }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;
    fn arrange(&mut self, _final_size: LayoutSize) {}
    fn render(&self, c: RenderContext);


    fn into_box(self) -> Box<dyn Ui>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

impl Ui for Box<dyn Ui> {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.as_mut().measure(available_size)
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        self.as_mut().arrange(final_size)
    }
    fn render(&self, c: RenderContext) {
        self.as_ref().render(c)
    }
    fn into_box(self) -> Box<dyn Ui>
    where
        Self: Sized + 'static,
    {
        self
    }
}

impl Ui for () {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        LayoutSize::default()
    }

    fn render(&self, _: RenderContext) {}
}

pub fn rgbf(r: f32, g: f32, b: f32) -> ColorF {
    ColorF::new(r, g, b, 1.)
}

pub fn rgbaf(r: f32, g: f32, b: f32, a: f32) -> ColorF {
    ColorF::new(r, g, b, a)
}

pub fn rgb(r: u8, g: u8, b: u8) -> ColorF {
    ColorF::new(r as f32 / 255., g as f32 / 255., b as f32 / 255., 1.)
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> ColorF {
    ColorF::new(r as f32 / 255., g as f32 / 255., b as f32 / 255., a as f32 / 255.)
}

#[derive(Clone)]
pub struct FillColor {
    color: ColorF,
}

impl FillColor {
    pub fn new(color: ColorF) -> Self {
        FillColor { color }
    }
}

#[inline]
fn fill_measure(mut available_size: LayoutSize) -> LayoutSize {
    if available_size.width.is_infinite() {
        available_size.width = 0.;
    }

    if available_size.height.is_infinite() {
        available_size.height = 0.;
    }

    available_size
}

impl Ui for FillColor {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        fill_measure(available_size)
    }

    fn render(&self, mut c: RenderContext) {
        c.push_rect(LayoutRect::from_size(c.final_size()), self.color);
    }
}

pub fn fill_color(color: ColorF) -> FillColor {
    FillColor::new(color)
}

#[derive(Clone)]
pub struct FillGradient {
    start: LayoutPoint,
    end: LayoutPoint,
    stops: Vec<GradientStop>,
}

impl FillGradient {
    pub fn new(start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) -> Self {
        FillGradient { start, end, stops }
    }
}

impl Ui for FillGradient {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        fill_measure(available_size)
    }

    fn render(&self, mut c: RenderContext) {
        let final_size = c.final_size();
        let mut start = self.start;
        let mut end = self.end;

        start.x *= final_size.width;
        start.y *= final_size.height;
        end.x *= final_size.width;
        end.y *= final_size.height;

        c.push_gradient(LayoutRect::from_size(final_size), start, end, self.stops.clone());
    }
}

pub fn fill_gradient(start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) -> FillGradient {
    FillGradient::new(start, end, stops)
}

#[derive(Clone)]
pub struct BackgroundColor<T: Ui> {
    child: T,
    color: ColorF,
}

impl<T: Ui> BackgroundColor<T> {
    pub fn new(child: T, color: ColorF) -> Self {
        BackgroundColor { child, color }
    }
}

impl<T: Ui> Ui for BackgroundColor<T> {
    fn on_keyboard_input(&mut self, input: &KeyboardInput) -> Update {
        self.child.on_keyboard_input(input)
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(available_size)
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size)
    }
    fn render(&self, mut c: RenderContext) {
        c.push_rect(LayoutRect::from_size(c.final_size()), self.color);
        self.child.render(c)
    }
}
pub fn background_color<T: Ui>(child: T, color: ColorF) -> BackgroundColor<T> {
    BackgroundColor::new(child, color)
}
pub trait BackgroundColorExt: Ui + Sized {
    fn background_color(self, color: ColorF) -> BackgroundColor<Self> {
        BackgroundColor::new(self, color)
    }
}
impl<T: Ui> BackgroundColorExt for T {}
