#[macro_use]
mod macros;

mod color;
mod event;
mod layout;
mod log;
mod stack;
mod text;

pub use self::log::*;
pub use color::*;
pub use event::*;
pub use layout::*;
pub use stack::*;
use std::any::Any;
use std::cell::Ref;
use std::cell::RefCell;
use std::iter::FromIterator;
use std::marker::PhantomData;
use std::ops::Deref;
use std::rc::Rc;
pub use text::*;

use app_units::Au;
use font_loader::system_fonts;
pub use glutin::event::{ElementState, ModifiersState, MouseButton, ScanCode, VirtualKeyCode};
pub use glutin::window::CursorIcon;
use std::collections::HashMap;
use webrender::api::*;
pub use webrender::api::{ColorF, LayoutPoint, LayoutRect, LayoutSize};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct HitTag(u64);

impl HitTag {
    /// Generates a new unique ID.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);

        HitTag(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

pub trait ReadValue<T> {
    fn value(&self) -> ValueRef<'_, T>;
    fn changed(&self) -> bool;
}

#[derive(Clone)]
pub struct Static<T>(T);

impl<T: 'static> ReadValue<T> for Static<T> {
    fn value(&self) -> ValueRef<'_, T> {
        ValueRef(ValueRefData::Static(&self.0))
    }
    fn changed(&self) -> bool {
        false
    }
}

struct VarData<T> {
    value: RefCell<T>,
    change: RefCell<VarChange<T>>,
}

struct VarChange<T> {
    pending: Option<Box<dyn FnOnce(&mut T)>>,
    changed: bool,
}

pub struct ValueRef<'a, T>(ValueRefData<'a, T>);

enum ValueRefData<'a, T> {
    Static(&'a T),
    Cell(Ref<'a, T>),
}

impl<'a, T> Deref for ValueRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match &self.0 {
            ValueRefData::Static(r) => r,
            ValueRefData::Cell(r) => &r,
        }
    }
}

pub struct Var<T> {
    r: Rc<VarData<T>>,
}

impl<T> Clone for Var<T> {
    fn clone(&self) -> Self {
        Var { r: Rc::clone(&self.r) }
    }
}

impl<T: 'static> Var<T> {
    pub fn new(value: T) -> Self {
        Var {
            r: Rc::new(VarData {
                value: RefCell::new(value),
                change: RefCell::new(VarChange {
                    pending: None,
                    changed: false,
                }),
            }),
        }
    }

    fn change_value(&self, change: impl FnOnce(&mut T) + 'static) {
        self.r.change.borrow_mut().pending = Some(Box::new(change));
    }
}

impl<T> ReadValue<T> for Var<T> {
    fn value(&self) -> ValueRef<'_, T> {
        ValueRef(ValueRefData::Cell(self.r.value.borrow()))
    }

    fn changed(&self) -> bool {
        self.r.change.borrow().changed
    }
}

pub(crate) trait ValueChange {
    fn commit(&mut self);
    fn reset_changed(&mut self);
}

impl<T> ValueChange for Var<T> {
    fn commit(&mut self) {
        let mut data = self.r.change.borrow_mut();
        if let Some(change) = data.pending.take() {
            change(&mut self.r.value.borrow_mut());
            data.changed = true;
        }
    }

    fn reset_changed(&mut self) {
        self.r.change.borrow_mut().changed = false;
    }
}

pub struct NewWindow {
    pub content: Box<dyn FnOnce(&mut NextUpdate) -> Box<dyn Ui>>,
    pub clear_color: ColorF,
    pub inner_size: LayoutSize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ContextValueId(u64);

impl ContextValueId {
    /// Generates a new unique ID.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(0);

        ContextValueId(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ContextValueKey<T> {
    id: ContextValueId,
    _data: PhantomData<T>,
}
impl<T> Clone for ContextValueKey<T> {
    fn clone(&self) -> Self {
        ContextValueKey {
            id: self.id,
            _data: self._data,
        }
    }
}
impl<T> Copy for ContextValueKey<T> {}

impl<T: 'static> ContextValueKey<T> {
    pub fn new() -> Self {
        ContextValueKey {
            id: ContextValueId::new(),
            _data: PhantomData,
        }
    }
}

pub struct NextUpdate {
    pub(crate) api: RenderApi,
    pub(crate) document_id: DocumentId,
    fonts: HashMap<String, FontInstances>,
    pub(crate) windows: Vec<NewWindow>,

    pub(crate) update_layout: bool,
    pub(crate) render_frame: bool,
    pub(crate) value_changes: Vec<Box<dyn ValueChange>>,
    _request_close: bool,

    context_values: HashMap<ContextValueId, Box<dyn Any>>,
}
impl NextUpdate {
    pub fn new(api: RenderApi, document_id: DocumentId) -> Self {
        NextUpdate {
            api,
            document_id,
            fonts: HashMap::new(),
            windows: vec![],

            update_layout: true,
            render_frame: true,
            value_changes: vec![],
            _request_close: false,

            context_values: HashMap::new(),
        }
    }

    pub fn create_window<TContent: Ui + 'static>(
        &mut self,
        clear_color: ColorF,
        inner_size: LayoutSize,
        content: impl FnOnce(&mut NextUpdate) -> TContent + 'static,
    ) {
        self.windows.push(NewWindow {
            content: Box::new(move |c| content(c).into_box()),
            clear_color,
            inner_size,
        })
    }

    pub fn update_layout(&mut self) {
        self.update_layout = true;
    }
    pub fn render_frame(&mut self) {
        self.render_frame = true;
    }

    pub fn get<T: 'static>(&self, key: ContextValueKey<T>) -> Option<ValueRef<'_, T>> {
        self.context_values
            .get(&key.id)
            .map(|a| a.downcast_ref::<Box<dyn ReadValue<T>>>().unwrap().value())
    }

    pub fn set<T: 'static>(&mut self, value: &Var<T>, new_value: T) {
        self.change(value, |v| *v = new_value);
    }

    fn change<T: 'static>(&mut self, value: &Var<T>, change: impl FnOnce(&mut T) + 'static) {
        value.change_value(change);
        self.value_changes.push(Box::new(value.clone()));
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

    pub fn with_context_value<T: 'static>(
        &mut self,
        key: ContextValueKey<T>,
        value: impl ReadValue<T> + 'static,
        action: impl FnOnce(&mut NextUpdate),
    ) {
        let dyn_box: Box<dyn ReadValue<T>> = Box::new(value);
        let any_box = Box::new(dyn_box);
        let previous_value = self.context_values.insert(key.id, any_box);

        action(self);

        if let Some(value) = previous_value {
            self.context_values.insert(key.id, value);
        } else {
            self.context_values.remove(&key.id);
        }
    }

    //-------idea---------
    //
    //pub fn close_app(&mut self) {
    //    self.close = Some(CloseRequest::App);
    //}

    //pub fn cancel_close(&mut self) {
    //    self.cancel_close = true;
    //}

    //pub fn set_window_title(&mut self, title: String) {
    //    self.new_window_title = Some(title);
    //}

    //pub fn start_work(&mut self, work: impl FnOnce() + 'static) -> WorkKey {
    //    let key = self.next_work_key;
    //    self.new_work.push((key, Box::new(work)));
    //    self.next_work_key = WorkKey(key.0.wrapping_add(1));
    //    key
    //}

    //pub fn cancel_work(&mut self, work_key: WorkKey) {
    //    self.cancel_work.push(work_key)
    //}
}

pub struct NextFrame {
    builder: DisplayListBuilder,
    spatial_id: SpatialId,
    final_size: LayoutSize,
    cursor: CursorIcon,
}

impl NextFrame {
    pub fn new(builder: DisplayListBuilder, spatial_id: SpatialId, final_size: LayoutSize) -> Self {
        NextFrame {
            builder,
            spatial_id,
            final_size,
            cursor: CursorIcon::Default,
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

    pub fn push_cursor(&mut self, cursor: CursorIcon, child: &impl Ui) {
        let current_cursor = self.cursor;
        self.cursor = cursor;

        child.render(self);

        self.cursor = current_cursor;
    }

    fn layout_and_clip(
        &self,
        final_rect: LayoutRect,
        hit_tag: Option<HitTag>,
    ) -> (LayoutPrimitiveInfo, SpaceAndClipInfo) {
        let mut lpi = LayoutPrimitiveInfo::new(final_rect);
        lpi.tag = hit_tag.map(|v| (v.0, self.cursor as u16));
        let sci = SpaceAndClipInfo {
            spatial_id: self.spatial_id,
            clip_id: ClipId::root(self.spatial_id.pipeline_id()),
        };

        (lpi, sci)
    }

    pub fn push_color(&mut self, final_rect: LayoutRect, color: ColorF, hit_tag: Option<HitTag>) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);
        self.builder.push_rect(&lpi, &sci, color);
    }

    pub fn push_hit_test(&mut self, hit_tag: HitTag, final_rect: LayoutRect) {
        let (lpi, sci) = self.layout_and_clip(final_rect, Some(hit_tag));
        self.builder.push_rect(&lpi, &sci, ColorF::TRANSPARENT);
    }

    pub fn push_gradient(
        &mut self,
        final_rect: LayoutRect,
        start: LayoutPoint,
        end: LayoutPoint,
        stops: Vec<GradientStop>,
        hit_tag: Option<HitTag>,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);

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
        hit_tag: Option<HitTag>,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);

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

/// Describes a keyboard input event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyboardInput {
    /// Identifies the physical key pressed
    ///
    /// This should not change if the user adjusts the host's keyboard map. Use when the physical location of the
    /// key is more important than the key's host GUI semantics, such as for movement controls in a first-person
    /// game.
    pub scancode: ScanCode,

    pub state: ElementState,

    /// Identifies the semantic meaning of the key
    ///
    /// Use when the semantics of the key are more important than the physical location of the key, such as when
    /// implementing appropriate behavior for "page up."
    pub virtual_keycode: Option<VirtualKeyCode>,

    /// Modifier keys active at the time of this input.
    ///
    /// This is tracked internally to avoid tracking errors arising from modifier key state changes when events from
    /// this device are not being delivered to the application, e.g. due to keyboard focus being elsewhere.
    pub modifiers: ModifiersState,

    ///  If the given key is being held down such that it is automatically repeating
    pub repeat: bool,
}

pub struct MouseInput {
    pub state: ElementState,
    pub button: MouseButton,
    pub modifiers: ModifiersState,
    pub position: LayoutPoint,
}

pub struct MouseMove {
    pub position: LayoutPoint,
    pub modifiers: ModifiersState,
}

/// Hit test results.
#[derive(Default)]
pub struct Hits {
    points: HashMap<HitTag, LayoutPoint>,
    cursor: CursorIcon,
}

impl Hits {
    pub fn new(hits: HitTestResult) -> Self {
        let cursor = hits
            .items
            .first()
            .map(|h| {
                if h.tag.1 <= CursorIcon::RowResize as u16 {
                    unsafe { std::mem::transmute(h.tag.1 as u8) }
                } else {
                    CursorIcon::Default
                }
            })
            .unwrap_or(CursorIcon::Default);

        Hits {
            points: hits
                .items
                .into_iter()
                .map(|h| (HitTag(h.tag.0), h.point_relative_to_item))
                .collect(),
            cursor,
        }
    }

    pub fn point_over(&self, tag: HitTag) -> Option<LayoutPoint> {
        self.points.get(&tag).cloned()
    }

    pub fn cursor(&self) -> CursorIcon {
        self.cursor
    }
}

/// An UI component.
///
/// # Implementers
/// This is usually not implemented directly, consider using [UiContainer], [UiMultiContainer], [UiLeaf] and [delegate_ui] first.
pub trait Ui {
    fn init(&mut self, update: &mut NextUpdate);

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;

    fn arrange(&mut self, final_size: LayoutSize);

    fn render(&self, f: &mut NextFrame);

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate);

    fn focused(&mut self, focused: bool, update: &mut NextUpdate);

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, update: &mut NextUpdate);

    fn mouse_move(&mut self, input: &MouseMove, hits: &Hits, update: &mut NextUpdate);

    fn mouse_entered(&mut self, update: &mut NextUpdate);

    fn mouse_left(&mut self, update: &mut NextUpdate);

    fn close_request(&mut self, update: &mut NextUpdate);

    /// Gets the point over this UI element using a hit test result.
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint>;

    fn value_changed(&mut self, update: &mut NextUpdate);

    fn context_value_changed(&mut self, update: &mut NextUpdate);

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

    fn init(&mut self, update: &mut NextUpdate) {
        self.as_mut().init(update);
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

    fn focused(&mut self, focused: bool, update: &mut NextUpdate) {
        self.as_mut().focused(focused, update);
    }

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, update: &mut NextUpdate) {
        self.as_mut().mouse_input(input, hits, update);
    }

    fn mouse_move(&mut self, input: &MouseMove, hits: &Hits, update: &mut NextUpdate) {
        self.as_mut().mouse_move(input, hits, update);
    }

    fn mouse_entered(&mut self, update: &mut NextUpdate) {
        self.as_mut().mouse_entered(update);
    }

    fn mouse_left(&mut self, update: &mut NextUpdate) {
        self.as_mut().mouse_left(update);
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        self.as_mut().close_request(update);
    }

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        self.as_ref().point_over(hits)
    }

    fn value_changed(&mut self, update: &mut NextUpdate) {
        self.as_mut().value_changed(update);
    }

    fn context_value_changed(&mut self, update: &mut NextUpdate) {
        self.as_mut().context_value_changed(update);
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

    fn init(&mut self, update: &mut NextUpdate) {}

    fn arrange(&mut self, final_size: LayoutSize) {}

    fn render(&self, f: &mut NextFrame);

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {}

    fn focused(&mut self, focused: bool, update: &mut NextUpdate) {}

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, update: &mut NextUpdate) {}

    fn mouse_move(&mut self, input: &MouseMove, hits: &Hits, update: &mut NextUpdate) {}

    fn mouse_entered(&mut self, update: &mut NextUpdate) {}

    fn mouse_left(&mut self, update: &mut NextUpdate) {}

    fn close_request(&mut self, update: &mut NextUpdate) {}

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        None
    }

    fn value_changed(&mut self, update: &mut NextUpdate) {}

    fn context_value_changed(&mut self, update: &mut NextUpdate) {}
}

/// An UI component with a single child component.
pub trait UiContainer {
    type Child: Ui;

    fn child(&self) -> &Self::Child;

    fn child_mut(&mut self) -> &mut Self::Child;

    fn into_child(self) -> Self::Child;

    fn init(&mut self, update: &mut NextUpdate) {
        self.child_mut().init(update);
    }

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

    fn focused(&mut self, focused: bool, update: &mut NextUpdate) {
        self.child_mut().focused(focused, update);
    }

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, update: &mut NextUpdate) {
        self.child_mut().mouse_input(input, hits, update);
    }

    fn mouse_move(&mut self, input: &MouseMove, hits: &Hits, update: &mut NextUpdate) {
        self.child_mut().mouse_move(input, hits, update);
    }

    fn mouse_entered(&mut self, update: &mut NextUpdate) {
        self.child_mut().mouse_entered(update);
    }

    fn mouse_left(&mut self, update: &mut NextUpdate) {
        self.child_mut().mouse_left(update);
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        self.child_mut().close_request(update);
    }

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        self.child().point_over(hits)
    }

    fn value_changed(&mut self, update: &mut NextUpdate) {
        self.child_mut().value_changed(update);
    }

    fn context_value_changed(&mut self, update: &mut NextUpdate) {
        self.child_mut().context_value_changed(update);
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

    fn init(&'a mut self, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.init(update);
        }
    }

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

    fn focused(&'a mut self, focused: bool, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.focused(focused, update);
        }
    }

    fn mouse_input(&'a mut self, input: &MouseInput, hits: &Hits, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.mouse_input(input, hits, update);
        }
    }

    fn mouse_move(&'a mut self, input: &MouseMove, hits: &Hits, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.mouse_move(input, hits, update);
        }
    }

    fn mouse_entered(&'a mut self, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.mouse_entered(update);
        }
    }

    fn mouse_left(&'a mut self, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.mouse_left(update);
        }
    }

    fn close_request(&'a mut self, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.close_request(update);
        }
    }

    fn point_over(&'a self, hits: &Hits) -> Option<LayoutPoint> {
        for c in self.children() {
            if let Some(point) = c.point_over(hits) {
                return Some(point);
            }
        }
        None
    }

    fn value_changed(&'a mut self, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.value_changed(update);
        }
    }

    fn context_value_changed(&'a mut self, update: &mut NextUpdate) {
        for c in self.children_mut() {
            c.context_value_changed(update);
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

// TODO
// https://github.com/servo/webrender/commit/717b1a272e8425d3952cc19f6d182b9087495c32
// https://doc.servo.org/webrender_api/struct.CommonItemProperties.html
// https://doc.servo.org/webrender_api/struct.DisplayListBuilder.html#method.push_hit_test

pub struct UiCursor<T: Ui> {
    child: T,
    cursor: CursorIcon,
}

impl<T: Ui> UiCursor<T> {
    pub fn new(child: T, cursor: CursorIcon) -> Self {
        UiCursor { child, cursor }
    }
}

impl<T: Ui + 'static> UiContainer for UiCursor<T> {
    delegate_child!(child, T);

    fn render(&self, f: &mut NextFrame) {
        f.push_cursor(self.cursor, &self.child)
    }
}

delegate_ui!(UiContainer, UiCursor<T>, T);

pub fn cursor<T: Ui>(child: T, cursor: CursorIcon) -> UiCursor<T> {
    UiCursor::new(child, cursor)
}

pub trait Cursor: Ui + Sized {
    fn cursor(self, cursor: CursorIcon) -> UiCursor<Self> {
        UiCursor::new(self, cursor)
    }
}
impl<T: Ui> Cursor for T {}

pub struct SetContextValue<T: Ui, V, R: ReadValue<V> + Clone> {
    child: T,
    value: R,
    key: ContextValueKey<V>,
}
impl<T: Ui, V: 'static, R: ReadValue<V> + Clone + 'static> SetContextValue<T, V, R> {
    fn new(child: T, key: ContextValueKey<V>, value: R) -> Self {
        SetContextValue { child, key, value }
    }
}
impl<T: Ui, V: 'static, R: ReadValue<V> + Clone + 'static> UiContainer for SetContextValue<T, V, R> {
    delegate_child!(child, T);

    fn init(&mut self, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| Ui::init(self.child_mut(), u));
    }

    fn value_changed(&mut self, update: &mut NextUpdate) {
        if self.value.changed() {
            update.with_context_value(self.key, self.value.clone(), |u| {
                Ui::context_value_changed(self.child_mut(), u)
            });
        }

        update.with_context_value(self.key, self.value.clone(), |u| Ui::value_changed(self.child_mut(), u));
    }

    fn context_value_changed(&mut self, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| {
            Ui::context_value_changed(self.child_mut(), u)
        });
    }

    fn keyboard_input(&mut self, input: &KeyboardInput, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| {
            Ui::keyboard_input(self.child_mut(), input, u)
        });
    }

    fn focused(&mut self, focused: bool, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| {
            Ui::focused(self.child_mut(), focused, u)
        });
    }

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| {
            Ui::mouse_input(self.child_mut(), input, hits, u)
        });
    }

    fn mouse_move(&mut self, input: &MouseMove, hits: &Hits, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| {
            Ui::mouse_move(self.child_mut(), input, hits, u)
        });
    }

    fn mouse_entered(&mut self, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| Ui::mouse_entered(self.child_mut(), u));
    }

    fn mouse_left(&mut self, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| Ui::mouse_left(self.child_mut(), u));
    }

    fn close_request(&mut self, update: &mut NextUpdate) {
        update.with_context_value(self.key, self.value.clone(), |u| Ui::close_request(self.child_mut(), u));
    }
}
impl<T: Ui, V: 'static, R: ReadValue<V> + Clone + 'static> Ui for SetContextValue<T, V, R> {
    delegate_ui_methods!(UiContainer);
}

pub trait ContextValue: Ui + Sized {
    fn set_ctx_val<T: 'static, R: ReadValue<T> + Clone + 'static>(
        self,
        key: ContextValueKey<T>,
        value: R,
    ) -> SetContextValue<Self, T, R> {
        SetContextValue::new(self, key, value)
    }
}
impl<T: Ui> ContextValue for T {}
