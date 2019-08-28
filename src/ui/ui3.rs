pub use crate::window::NextUpdate;
pub use glutin::event::{ElementState, KeyboardInput, ModifiersState, MouseButton, ScanCode, VirtualKeyCode};
use std::iter::FromIterator;
use webrender::api::*;
pub use webrender::api::{LayoutPoint, LayoutRect, LayoutSize};

pub struct MouseInput {
    pub state: ElementState,
    pub button: MouseButton,
    pub modifiers: ModifiersState,
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

pub trait Ui {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;

    fn arrange(&mut self, final_size: LayoutSize);

    fn render(&self, rc: &mut NextFrame);

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate);

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate);

    fn close_request(&mut self, update: &mut NextUpdate);

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

    fn render(&self, rc: &mut NextFrame) {
        self.as_ref().render(rc);
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        self.as_mut().keyboard_input(input, update);
    }

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
        self.as_mut().mouse_input(input, update);
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        self.as_mut().close_request(update);
    }
}

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

    fn render(&self, rc: &mut NextFrame);

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {}

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {}

    fn close_request(&mut self, update: &mut NextUpdate) {}
}

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

    fn render(&self, rc: &mut NextFrame) {
        self.child().render(rc);
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        self.child_mut().keyboard_input(input, update);
    }

    fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
        self.child_mut().mouse_input(input, update);
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        self.child_mut().close_request(update);
    }
}

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

    fn render(&'a self, rc: &mut NextFrame) {
        for c in self.children() {
            c.render(rc);
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

    fn close_request(&'a mut self, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.close_request(update);
        }
    }
}

#[doc(hidden)]
macro_rules! delegate_ui_methods {
    ($Del:ident, $T:ty) => {
        fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
            $Del::measure(self, available_size)
        }

        fn arrange(&mut self, final_size: LayoutSize) {
            $Del::arrange(self, final_size)
        }

        fn render(&self, rc: &mut NextFrame) {
            $Del::render(self, rc)
        }

        fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
            $Del::keyboard_input(self, input, update)
        }

        fn mouse_input(&mut self, input: &MouseInput, update: &mut NextUpdate) {
            $Del::mouse_input(self, input, update)
        }

        fn close_request(&mut self, update: &mut NextUpdate) {
            $Del::close_request(self, update)
        }
    };
}

/// Implements `[Ui]` for a type that implements `[UiLeaf]`, `[UiContainer]` or `[UiMultiContainer]`
/// by delegating all calls to the homonymous methods.
/// # Example
/// ```rust
/// pub struct Foo { }
///
/// impl UiLeaf for Foo {
///     fn render(&self, _: &mut NextFrame) { }
/// }
/// delegate_ui!(UiLeaf, Foo);
/// ```
///
/// You can also have a generic child type `TChild: Ui + 'static`.
///
/// ```rust
/// pub struct Bar<T> {
///     child: T
/// }
///
/// impl<T: Ui> UiContainer for Bar<T> {
///     type Child = T;
///
///      fn child(&self) -> &Self::Child {
///        &self.child
///    }
///
///    fn child_mut(&mut self) -> &mut Self::Child {
///        &mut self.child
///    }
///
///    fn into_child(self) -> Self::Child {
///        self.child
///    }
/// }
/// delegate_ui!(UiContainer, Bar<T>, T);
/// ```
#[macro_export]
macro_rules! delegate_ui {
    ($Del:ident, $T:ty) => {
        impl Ui for $T {
            delegate_ui_methods!($Del, $T);
        }
    };

    ($Del:ident, $T:ty, $TChild:ident) => {
        impl<$TChild: Ui + 'static> Ui for $T {
            delegate_ui_methods!($Del, $T);
        }
    };
}

impl UiLeaf for () {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        LayoutSize::default()
    }
    fn render(&self, _: &mut NextFrame) {}
}
delegate_ui!(UiLeaf, ());

/// A child in a stack munti-container.
pub struct StackSlot<T> {
    child: T,
    rect: LayoutRect,
}

impl<T> StackSlot<T> {
    pub fn new(child: T) -> Self {
        StackSlot {
            child,
            rect: LayoutRect::default(),
        }
    }
}

pub struct ZStack<T> {
    children: Vec<StackSlot<T>>,
}

impl<T: Ui> UiContainer for StackSlot<T> {
    type Child = T;

    fn child(&self) -> &Self::Child {
        &self.child
    }

    fn child_mut(&mut self) -> &mut Self::Child {
        &mut self.child
    }

    fn into_child(self) -> Self::Child {
        self.child
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.rect.size = self.child.measure(available_size);
        self.rect.size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.rect.size = final_size;
    }

    fn render(&self, rc: &mut NextFrame) {
        rc.push_child(&self.child, &self.rect);
    }
}
delegate_ui!(UiContainer, StackSlot<T>, T);

impl<'a, T: Ui + 'static> UiMultiContainer<'a> for ZStack<T> {
    type Child = StackSlot<T>;
    type Children = std::slice::Iter<'a, Self::Child>;
    type ChildrenMut = std::slice::IterMut<'a, Self::Child>;

    fn children(&'a self) -> Self::Children {
        self.children.iter()
    }

    fn children_mut(&'a mut self) -> Self::ChildrenMut {
        self.children.iter_mut()
    }

    fn collect_children<B: FromIterator<Self::Child>>(self) -> B {
        self.children.into_iter().collect()
    }
}
delegate_ui!(UiMultiContainer, ZStack<T>, T);

pub trait IntoStackSlots {
    type Child: Ui;
    fn into(self) -> Vec<StackSlot<Self::Child>>;
}

impl<T: Ui + 'static> IntoStackSlots for Vec<T> {
    type Child = T;
    fn into(self) -> Vec<StackSlot<T>> {
        self.into_iter().map(StackSlot::new).collect()
    }
}

macro_rules! impl_tuples {
    ($TH:ident, $TH2:ident, $($T:ident, )* ) => {
        impl<$TH, $TH2, $($T, )*> IntoStackSlots for ($TH, $TH2, $($T,)*)
        where $TH: Ui + 'static, $TH2: Ui + 'static, $($T: Ui + 'static, )*
        {
            type Child = Box<dyn Ui>;

            #[allow(non_snake_case)]
            fn into(self) -> Vec<StackSlot<Box<dyn Ui>>> {
                let ($TH, $TH2, $($T,)*) = self;
                vec![StackSlot::new($TH.into_box()), StackSlot::new($TH2.into_box()),  $(StackSlot::new($T.into_box()), )*]
            }
        }
        impl_tuples!($( $T, )*);
    };

    () => {};
}
impl_tuples!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z,);

impl<T: Ui> ZStack<T> {
    pub fn new<B: IntoStackSlots<Child = T>>(children: B) -> Self {
        ZStack {
            children: children.into(),
        }
    }
}

pub fn z_stack<B: IntoStackSlots>(children: B) -> ZStack<B::Child> {
    ZStack::new(children)
}
