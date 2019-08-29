#[macro_use]
mod macros;

mod color;
mod event;
mod layout;
mod stack;
mod text;

pub use crate::window::NextUpdate;
pub use color::*;
pub use event::*;
pub use layout::*;
pub use stack::*;
use std::iter::FromIterator;
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

pub struct NextFrame {
    builder: DisplayListBuilder,
    spatial_id: SpatialId,
    final_size: LayoutSize,
}

impl NextFrame {
    pub fn new(builder: DisplayListBuilder, spatial_id: SpatialId, final_size: LayoutSize) -> Self {
        NextFrame {
            builder,
            spatial_id,
            final_size,
        }
    }

    pub fn push_child(&mut self, child: &impl Ui, final_rect: &LayoutRect) {
        let final_size = self.final_size;
        let spatial_id = self.spatial_id;

        self.final_size = final_rect.size;
        self.spatial_id = self.builder.push_reference_frame(
            final_rect,
            self.spatial_id,
            TransformStyle::Flat,
            PropertyBinding::Value(LayoutTransform::default()),
            ReferenceFrameKind::Transform,
        );

        child.render(self);
        self.builder.pop_reference_frame();

        self.final_size = final_size;
        self.spatial_id = spatial_id;

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

    pub fn final_size(&self) -> LayoutSize {
        self.final_size
    }

    pub fn finalize(self) -> (PipelineId, LayoutSize, BuiltDisplayList) {
        self.builder.finalize()
    }
}

pub struct MouseInput {
    pub state: ElementState,
    pub button: MouseButton,
    pub modifiers: ModifiersState,
}

pub struct MouseMove {
    pub position: LayoutPoint,
    pub modifiers: ModifiersState,
}

/// An UI component.
///
/// # Implementers
/// This is usually not implemented directly, consider using [UiContainer], [UiMultiContainer], [UiLeaf] and [delegate_ui] first.
pub trait Ui {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;

    fn arrange(&mut self, final_size: LayoutSize);

    fn render(&self, f: &mut NextFrame);

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate);

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate);

    fn mouse_move(&mut self, input: &MouseMove, update: &mut NextUpdate);

    fn close_request(&mut self, update: &mut NextUpdate);

    /// Box this component, unless it is already `Box<dyn Ui>`.
    fn into_box(self) -> Box<dyn Ui>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

impl Ui for Box<dyn Ui> {
    fn into_box(self) -> Box<dyn Ui> {
        self
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.as_mut().measure(available_size)
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.as_mut().arrange(final_size);
    }

    fn render(&self, f: &mut NextFrame) {
        self.as_ref().render(f);
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        self.as_mut().keyboard_input(input, update);
    }

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
        self.as_mut().mouse_input(input, update);
    }

    fn mouse_move(&mut self, input: &MouseMove, update: &mut NextUpdate) {
        self.as_mut().mouse_move(input, update);
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        self.as_mut().close_request(update);
    }
}

/// An UI component that does not have a child component.
#[allow(unused_variables)]
pub trait UiLeaf {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut size = available_size;

        if size.width.is_infinite() {
            size.width = 0.0;
        }

        if size.height.is_infinite() {
            size.height = 0.0;
        }

        size
    }

    fn arrange(&mut self, final_size: LayoutSize) {}

    fn render(&self, f: &mut NextFrame);

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {}

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {}

    fn mouse_move(&mut self, input: &MouseMove, update: &mut NextUpdate) {}

    fn close_request(&mut self, update: &mut NextUpdate) {}
}

/// An UI component with a single child component.
pub trait UiContainer {
    type Child: Ui;

    fn child(&self) -> &Self::Child;

    fn child_mut(&mut self) -> &mut Self::Child;

    fn into_child(self) -> Self::Child;

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child_mut().measure(available_size)
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.child_mut().arrange(final_size);
    }

    fn render(&self, f: &mut NextFrame) {
        self.child().render(f);
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        self.child_mut().keyboard_input(input, update);
    }

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
        self.child_mut().mouse_input(input, update);
    }

    fn mouse_move(&mut self, input: &MouseMove, update: &mut NextUpdate) {
        self.child_mut().mouse_move(input, update);
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        self.child_mut().close_request(update);
    }
}

/// An UI Component with many child components.
pub trait UiMultiContainer<'a> {
    type Child: Ui + 'static;
    type Children: Iterator<Item = &'a Self::Child>;
    type ChildrenMut: Iterator<Item = &'a mut Self::Child>;

    fn children(&'a self) -> Self::Children;

    fn children_mut(&'a mut self) -> Self::ChildrenMut;

    fn collect_children<B: FromIterator<Self::Child>>(self) -> B;

    fn measure(&'a mut self, available_size: LayoutSize) -> LayoutSize {
        let mut size = LayoutSize::default();
        for c in self.children_mut() {
            size = c.measure(available_size).max(size);
        }
        size
    }

    fn arrange(&'a mut self, final_size: LayoutSize) {
        for c in self.children_mut() {
            c.arrange(final_size);
        }
    }

    fn render(&'a self, f: &mut NextFrame) {
        for c in self.children() {
            c.render(f);
        }
    }

    fn keyboard_input(&'a mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.keyboard_input(input, update);
        }
    }

    fn mouse_input(&'a mut self, input: &MouseInput, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.mouse_input(input, update);
        }
    }

    fn mouse_move(&'a mut self, input: &MouseMove, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.mouse_move(input, update);
        }
    }

    fn close_request(&'a mut self, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.close_request(update);
        }
    }
}

impl UiLeaf for () {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        LayoutSize::default()
    }
    fn render(&self, _: &mut NextFrame) {}
}
delegate_ui!(UiLeaf, ());
