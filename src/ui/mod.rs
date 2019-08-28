mod color;
mod event;
mod layout;
mod stack;
mod text;
mod ui3;

pub use crate::window::NextUpdate;
pub use color::*;
pub use event::*;
pub use layout::*;
pub use stack::*;
pub use text::*;

use app_units::Au;
use font_loader::system_fonts;
pub use glutin::event::{ElementState, KeyboardInput, ModifiersState, MouseButton, ScanCode, VirtualKeyCode};
use std::collections::HashMap;
use webrender::api::*;
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

    pub fn push_child(&mut self, child: &mut impl Ui, final_rect: &LayoutRect) {
        let spatial_id = self.builder.push_reference_frame(
            final_rect,
            self.spatial_id,
            TransformStyle::Flat,
            PropertyBinding::Value(LayoutTransform::default()),
            ReferenceFrameKind::Transform,
        );
        child.render(&mut RenderContext::new(self.builder, spatial_id, final_rect.size));
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

pub struct MouseInput {
    pub state: ElementState,
    pub button: MouseButton,
    pub modifiers: ModifiersState,
}

pub trait Ui {
    type Child: Ui;

    fn for_each_child(&mut self, _action: impl FnMut(&mut Self::Child)) {}

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut desired_size = LayoutSize::default();
        let mut have_child = false;

        self.for_each_child(|c| {
            have_child = true;
            let child_desired_size = c.measure(available_size);
            desired_size = desired_size.max(child_desired_size);
        });

        if have_child {
            desired_size
        } else {
            desired_size = available_size;
            if desired_size.width.is_infinite() {
                desired_size.width = 0.;
            }
            if desired_size.height.is_infinite() {
                desired_size.height = 0.;
            }
            desired_size
        }
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        self.for_each_child(|c| c.arrange(final_size));
    }
    fn render(&mut self, rc: &mut RenderContext) {
        self.for_each_child(|c| c.render(rc));
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        self.for_each_child(|c| c.keyboard_input(input, update));
    }

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
        self.for_each_child(|c| c.mouse_input(input, update));
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        self.for_each_child(|c| c.close_request(update));
    }

    fn as_any(self) -> AnyUi
    where
        Self: Sized + 'static,
    {
        AnyUi::new(self)
    }
}

mod any_ui {
    use super::*;
    use std::any::Any;

    pub trait UiFns: Any {
        fn measure(&mut self, _: LayoutSize) -> LayoutSize;
        fn arrange(&mut self, _: LayoutSize);
        fn render(&mut self, _: &mut RenderContext);
        fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate);
        fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate);
        fn close_request(&mut self, update: &mut NextUpdate);
    }

    impl<T: Ui + 'static> UiFns for T {
        fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
            Ui::measure(self, available_size)
        }

        fn arrange(&mut self, final_size: LayoutSize) {
            Ui::arrange(self, final_size)
        }

        fn render(&mut self, rc: &mut RenderContext) {
            Ui::render(self, rc)
        }

        fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
            Ui::keyboard_input(self, input, update)
        }

        fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
            Ui::mouse_input(self, input, update)
        }

        fn close_request(&mut self, update: &mut NextUpdate) {
            Ui::close_request(self, update)
        }
    }
}

pub struct AnyUi {
    ui: Box<dyn any_ui::UiFns>,
}

impl AnyUi {
    fn new<T: any_ui::UiFns>(ui: T) -> Self {
        Self { ui: Box::new(ui) }
    }
}

impl Ui for AnyUi {
    type Child = ();

    fn for_each_child(&mut self, _: impl FnMut(&mut Self::Child)) {
        panic!("Ui::for_each_child must not be called in AnyUi")
    }

    fn as_any(self) -> AnyUi {
        self
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.ui.measure(available_size)
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.ui.arrange(final_size)
    }

    fn render(&mut self, rc: &mut RenderContext) {
        self.ui.render(rc)
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        self.ui.keyboard_input(input, update)
    }

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
        self.ui.mouse_input(input, update)
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        self.ui.close_request(update)
    }
}

impl Ui for () {
    type Child = ();

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        LayoutSize::default()
    }

    fn render(&mut self, _: &mut RenderContext) {}
}
