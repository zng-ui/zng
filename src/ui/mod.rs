#[macro_use]
mod macros;

#[cfg(test)]
#[macro_use]
pub mod test;

mod border;
mod color;
mod event;
mod layout;
mod log;
mod stack;
mod text;

use app_units::Au;
use fnv::FnvHashMap;
use font_loader::system_fonts;
use once_cell::sync::OnceCell;
use std::any::Any;
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::ops::Deref;
use std::rc::Rc;
use webrender::api as wapi;

pub use self::log::*;
pub use border::*;
pub use color::*;
pub use event::*;
pub use glutin::event::{ElementState, ModifiersState, MouseButton, ScanCode, VirtualKeyCode};
pub use glutin::window::CursorIcon;
pub use layout::*;
pub use stack::*;
pub use text::*;
pub use webrender::api::{
    BorderRadius, ColorF, FontInstanceKey, FontKey, GradientStop, LayoutPoint, LayoutRect, LayoutSideOffsets,
    LayoutSize,
};

#[doc(inline)]
pub use zero_ui_derive::impl_ui;
use zero_ui_derive::impl_ui_crate;

struct FontInstances {
    font_key: FontKey,
    instances: FnvHashMap<u32, FontInstanceKey>,
}

#[derive(Clone)]
pub struct FontInstance {
    pub font_key: FontKey,
    pub instance_key: FontInstanceKey,
    pub size: u32,
}

/// Declare and implement a unique ID type.
macro_rules! uid {
    ($(
        $(#[$outer:meta])*
        $vis:vis struct $Type:ident (_);
    )+) => {
        $(
            $(#[$outer])*
            /// # Details
            /// Underlying value is a `NonZeroU64` generated using a relaxed global atomic `fetch_add`,
            /// so IDs are unique for the process duration but order is not garanteed.
            ///
            /// Panics if you somehow reach `u64::max_value()` calls to `new`.
            #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
            $vis struct $Type(NonZeroU64);

            impl $Type {
                /// Generates a new unique ID.
                ///
                /// # Panics
                /// Panics if called more then `u64::max_value()` times.
                pub fn new() -> Self {
                    use std::sync::atomic::{AtomicU64, Ordering};
                    static NEXT: AtomicU64 = AtomicU64::new(1);

                    let id = NEXT.fetch_add(1, Ordering::Relaxed);

                    if let Some(id) = NonZeroU64::new(id) {
                        $Type(id)
                    } else {
                        NEXT.store(0, Ordering::SeqCst);
                        panic!("`{}` reached `u64::max_value()` IDs.",  stringify!($Type))
                    }
                }

                /// Retrieve the underlying `u64` value.
                #[allow(dead_code)]
                pub fn get(self) -> u64 {
                    self.0.get()
                }
            }
        )+
    };
}

uid! {
    /// Hit-test tag.
    pub struct HitTag(_);

    /// Focusable unique identifier.
    pub struct FocusKey(_);
}

mod private {
    pub trait Sealed {}
}

pub trait Value<T>: private::Sealed + Deref<Target = T> {
    fn changed(&self) -> bool;

    /// Gets if `self` and `other` derefs to the same data.
    fn is_same<O: Value<T>>(&self, other: &O) -> bool {
        std::ptr::eq(self.deref(), other.deref())
    }
}

#[derive(Clone)]
pub struct Owned<T>(T);

impl<T> private::Sealed for Owned<T> {}

impl<T> Deref for Owned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T: 'static> Value<T> for Owned<T> {
    fn changed(&self) -> bool {
        false
    }
}

struct VarData<T> {
    value: RefCell<T>,
    pending: Cell<Box<dyn FnOnce(&mut T)>>,
    changed: Cell<bool>,
}

pub struct Var<T> {
    r: Rc<VarData<T>>,
}

impl<T: 'static> Var<T> {
    pub fn new(value: T) -> Self {
        Var {
            r: Rc::new(VarData {
                value: RefCell::new(value),
                pending: Cell::new(Box::new(|_| {})),
                changed: Cell::new(false),
            }),
        }
    }

    fn change_value(&self, change: impl FnOnce(&mut T) + 'static) {
        self.r.pending.set(Box::new(change));
    }
}

impl<T> Clone for Var<T> {
    fn clone(&self) -> Self {
        Var { r: Rc::clone(&self.r) }
    }
}

impl<T> Deref for Var<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: This is safe because borrow_mut only occurs when committing a change
        // inside a FnOnce : 'static. Because it is 'static it cannot capture a unguarded
        // reference, but it can capture a Var clone, in that case we panic.
        unsafe {
            &self
                .r
                .value
                .try_borrow_unguarded()
                .expect("Cannot deref `Var` while changing the same `Var`")
        }
    }
}

impl<T> private::Sealed for Var<T> {}

impl<T> Value<T> for Var<T> {
    fn changed(&self) -> bool {
        self.r.changed.get()
    }
}

pub trait IntoValue<T> {
    type Value: Value<T>;

    fn into_value(self) -> Self::Value;
}

/// Does nothing. `Var<T>` already implements `Value<T>`.
impl<T> IntoValue<T> for Var<T> {
    type Value = Var<T>;

    fn into_value(self) -> Self::Value {
        self
    }
}

/// Wraps the value in an `Owned<T>` value.
impl<T: 'static> IntoValue<T> for T {
    type Value = Owned<T>;

    fn into_value(self) -> Owned<T> {
        Owned(self)
    }
}

impl<'s> IntoValue<String> for &'s str {
    type Value = Owned<String>;

    fn into_value(self) -> Owned<String> {
        Owned(self.to_owned())
    }
}

impl IntoValue<Cow<'static, str>> for &'static str {
    type Value = Owned<Cow<'static, str>>;

    fn into_value(self) -> Self::Value {
        Owned(self.into())
    }
}

impl IntoValue<Cow<'static, str>> for String {
    type Value = Owned<Cow<'static, str>>;

    fn into_value(self) -> Self::Value {
        Owned(self.into())
    }
}

impl IntoValue<LayoutPoint> for (f32, f32) {
    type Value = Owned<LayoutPoint>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutPoint::new(self.0, self.1))
    }
}

impl IntoValue<LayoutSize> for (f32, f32) {
    type Value = Owned<LayoutSize>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutSize::new(self.0, self.1))
    }
}

impl IntoValue<Vec<GradientStop>> for Vec<(f32, ColorF)> {
    type Value = Owned<Vec<GradientStop>>;

    fn into_value(self) -> Self::Value {
        Owned(
            self.into_iter()
                .map(|(offset, color)| GradientStop { offset, color })
                .collect(),
        )
    }
}

impl IntoValue<Vec<GradientStop>> for Vec<ColorF> {
    type Value = Owned<Vec<GradientStop>>;

    fn into_value(self) -> Self::Value {
        let point = 1. / (self.len() as f32 - 1.);
        Owned(
            self.into_iter()
                .enumerate()
                .map(|(i, color)| GradientStop {
                    offset: (i as f32) * point,
                    color,
                })
                .collect(),
        )
    }
}

impl IntoValue<LayoutSideOffsets> for f32 {
    type Value = Owned<LayoutSideOffsets>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutSideOffsets::new_all_same(self))
    }
}

///for (top-bottom, left-right)
impl IntoValue<LayoutSideOffsets> for (f32, f32) {
    type Value = Owned<LayoutSideOffsets>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutSideOffsets::new(self.0, self.1, self.0, self.1))
    }
}

///for (top, right, bottom, left)
impl IntoValue<LayoutSideOffsets> for (f32, f32, f32, f32) {
    type Value = Owned<LayoutSideOffsets>;

    fn into_value(self) -> Self::Value {
        Owned(LayoutSideOffsets::new(self.0, self.1, self.2, self.3))
    }
}

impl IntoValue<BorderDetails> for ColorF {
    type Value = Owned<BorderDetails>;

    fn into_value(self) -> Self::Value {
        let border_side = BorderSide {
            color: self,
            style: BorderStyle::Solid,
        };
        Owned(BorderDetails {
            left: border_side,
            right: border_side,
            top: border_side,
            bottom: border_side,
            radius: BorderRadius::zero(),
        })
    }
}

impl IntoValue<BorderDetails> for (ColorF, BorderStyle) {
    type Value = Owned<BorderDetails>;

    fn into_value(self) -> Self::Value {
        let border_side = BorderSide {
            color: self.0,
            style: self.1,
        };
        Owned(BorderDetails {
            left: border_side,
            right: border_side,
            top: border_side,
            bottom: border_side,
            radius: BorderRadius::zero(),
        })
    }
}

pub(crate) trait VarChange {
    fn commit(&mut self);
    fn reset_changed(&mut self);
}

impl<T> VarChange for Var<T> {
    fn commit(&mut self) {
        let change = self.r.pending.replace(Box::new(|_| {}));
        change(&mut self.r.value.borrow_mut());
        self.r.changed.set(true);
    }

    fn reset_changed(&mut self) {
        self.r.changed.set(false);
    }
}

pub struct NewWindow {
    pub content: Box<dyn FnOnce(&mut NextUpdate) -> Box<dyn Ui>>,
    pub clear_color: ColorF,
    pub inner_size: LayoutSize,
}

macro_rules! ui_value_key {
    ($(
        $(#[$outer:meta])*
        pub struct $Key:ident (struct $Id:ident) { new_lazy() -> pub struct $KeyRef:ident };
    )+) => {$(
        uid! {struct $Id(_);}

        $(#[$outer])*
        /// # Example
        /// ```rust
        /// pub static FOO: $KeyRef<String> = $Key<String>::new_lazy();
        /// ```
        #[derive(Debug, PartialEq, Eq, Hash)]
        pub struct $Key<T> ($Id, PhantomData<T>);

        impl<T> Clone for $Key<T> {
            fn clone(&self) -> Self {
                $Key (self.0,self.1)
            }
        }

        impl<T> Copy for $Key<T> {}

        /// Dereferences to a $Key<T> that is generated on the first deref.
        pub struct $KeyRef<T> (OnceCell<$Key<T>>);

        impl<T: 'static> $Key<T> {
            /// New unique key.
            pub fn new() -> Self {
                $Key ($Id::new(), PhantomData)
            }

            /// New lazy initialized unique key. Use this for public static
            /// variables.
            ///
            /// # Example
            /// ```rust
            /// pub static FOO: $KeyRef<String> = $Key<String>::new_lazy();
            /// ```
            pub const fn new_lazy() -> $KeyRef<T> {
                $KeyRef(OnceCell::new())
            }

            fn id(&self) -> $Id {
                self.0
            }
        }

        impl<T: 'static> Deref for $KeyRef<T> {
            type Target = $Key<T>;
            fn deref(&self) -> &Self::Target {
                self.0.get_or_init(|| $Key::new())
            }
        }
    )+};
}

ui_value_key! {
    /// Unique key for a value set in a parent Ui to be read in a child Ui.
    pub struct ParentValueKey(struct ParentValueId) {
        new_lazy() -> pub struct ParentValueKeyRef
    };

    /// Unique key for a value set in a child Ui to be read in a parent Ui.
    pub struct ChildValueKey(struct ChildValueId) {
        new_lazy() -> pub struct ChildValueKeyRef
    };
}

enum UntypedRef {}

#[derive(new)]
pub struct UiValues {
    #[new(default)]
    parent_values: FnvHashMap<ParentValueId, *const UntypedRef>,
    #[new(default)]
    child_values: FnvHashMap<ChildValueId, Box<dyn Any>>,
}
impl UiValues {
    pub fn parent<T: 'static>(&self, key: ParentValueKey<T>) -> Option<&T> {
        // REFERENCE SAFETY: This is safe because parent_values are only inserted for the duration
        // of [with_parent_value] that holds the reference.
        //
        // TYPE SAFETY: This is safe because [ParentValueId::new] is always unique AND created by
        // [ParentValueKey::new] THAT can only be inserted in [with_parent_value].
        self.parent_values
            .get(&key.id())
            .map(|pointer| unsafe { &*(*pointer as *const T) })
    }

    pub fn with_parent_value<T: 'static>(
        &mut self,
        key: ParentValueKey<T>,
        value: &T,
        action: impl FnOnce(&mut UiValues),
    ) {
        let previous_value = self
            .parent_values
            .insert(key.id(), (value as *const T) as *const UntypedRef);

        action(self);

        if let Some(previous_value) = previous_value {
            self.parent_values.insert(key.id(), previous_value);
        } else {
            self.parent_values.remove(&key.id());
        }
    }

    pub fn child<T: 'static>(&self, key: ChildValueKey<T>) -> Option<&T> {
        self.child_values.get(&key.id()).map(|a| a.downcast_ref::<T>().unwrap())
    }

    pub fn set_child_value<T: 'static>(&mut self, key: ChildValueKey<T>, value: T) {
        self.child_values.insert(key.id(), Box::new(value));
    }

    pub(crate) fn clear_child_values(&mut self) {
        self.child_values.clear()
    }
}

#[cfg(test)]
mod ui_values {
    use super::*;

    #[test]
    fn with_parent_value() {
        let mut ui_values = UiValues::new();
        let key1 = ParentValueKey::new();
        let key2 = ParentValueKey::new();

        let val1: u32 = 10;
        let val2: u32 = 11;
        let val3: u32 = 12;

        assert_eq!(ui_values.parent(key1), None);
        assert_eq!(ui_values.parent(key2), None);

        ui_values.with_parent_value(key1, &val1, |ui_values| {
            assert_eq!(ui_values.parent(key1), Some(&val1));
            assert_eq!(ui_values.parent(key2), None);

            ui_values.with_parent_value(key2, &val2, |ui_values| {
                assert_eq!(ui_values.parent(key1), Some(&val1));
                assert_eq!(ui_values.parent(key2), Some(&val2));

                ui_values.with_parent_value(key1, &val3, |ui_values| {
                    assert_eq!(ui_values.parent(key1), Some(&val3));
                    assert_eq!(ui_values.parent(key2), Some(&val2));
                });

                assert_eq!(ui_values.parent(key1), Some(&val1));
                assert_eq!(ui_values.parent(key2), Some(&val2));
            });

            assert_eq!(ui_values.parent(key1), Some(&val1));
            assert_eq!(ui_values.parent(key2), None);
        });

        assert_eq!(ui_values.parent(key1), None);
        assert_eq!(ui_values.parent(key2), None);
    }
}

pub enum FocusRequest {
    /// Move focus to key.
    Direct(FocusKey),
    /// Move focus to next from current in screen, or to starting key.
    Next,
    /// Move focus to previous from current in screen, or to last in screen.
    Prev,
    Left,
    Right,
    Up,
    Down,
}

pub struct NextUpdate {
    pub(crate) api: wapi::RenderApi,
    pub(crate) document_id: wapi::DocumentId,
    fonts: FnvHashMap<String, FontInstances>,
    pub(crate) windows: Vec<NewWindow>,

    pub(crate) update_layout: bool,
    pub(crate) render_frame: bool,
    pub(crate) focus_request: Option<FocusRequest>,
    pub(crate) value_changes: Vec<Box<dyn VarChange>>,
    _request_close: bool,
}
impl NextUpdate {
    pub fn new(api: wapi::RenderApi, document_id: wapi::DocumentId) -> Self {
        NextUpdate {
            api,
            document_id,
            fonts: FnvHashMap::default(),
            windows: vec![],

            update_layout: true,
            render_frame: true,
            value_changes: vec![],
            focus_request: None,
            _request_close: false,
        }
    }

    pub fn create_window<C: Ui + 'static>(
        &mut self,
        clear_color: ColorF,
        inner_size: LayoutSize,
        content: impl FnOnce(&mut NextUpdate) -> C + 'static,
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

    pub fn focus(&mut self, request: FocusRequest) {
        self.focus_request = Some(request);
    }

    pub fn set<T: 'static>(&mut self, value: &Var<T>, new_value: T) {
        self.change(value, |v| *v = new_value);
    }

    pub fn change<T: 'static>(&mut self, value: &Var<T>, change: impl FnOnce(&mut T) + 'static) {
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

        let mut txn = wapi::Transaction::new();

        if uncached_font {
            let property = system_fonts::FontPropertyBuilder::new().family(family).build();
            let (font, _) = system_fonts::get(&property).unwrap();

            let font_key = self.api.generate_font_key();
            txn.add_raw_font(font_key, font, 0);

            self.fonts.insert(
                family.to_owned(),
                FontInstances {
                    font_key,
                    instances: FnvHashMap::default(),
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

#[derive(new)]
pub struct NextFrame {
    builder: wapi::DisplayListBuilder,
    spatial_id: wapi::SpatialId,
    final_size: LayoutSize,
    #[new(value = "CursorIcon::Default")]
    cursor: CursorIcon,
    #[new(value = "FocusMap::new()")]
    focus_map: FocusMap,
}

impl NextFrame {
    pub fn push_child(&mut self, child: &impl Ui, final_rect: &LayoutRect) {
        let final_size = self.final_size;
        let spatial_id = self.spatial_id;

        self.final_size = final_rect.size;
        self.spatial_id = self.builder.push_reference_frame(
            final_rect,
            self.spatial_id,
            wapi::TransformStyle::Flat,
            wapi::PropertyBinding::Value(wapi::LayoutTransform::default()),
            wapi::ReferenceFrameKind::Transform,
        );

        self.focus_map.push_reference_frame(final_rect);

        child.render(self);
        self.builder.pop_reference_frame();

        self.focus_map.pop_reference_frame(final_rect);

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
    ) -> (wapi::LayoutPrimitiveInfo, wapi::SpaceAndClipInfo) {
        let mut lpi = wapi::LayoutPrimitiveInfo::new(final_rect);
        lpi.tag = hit_tag.map(|v| (v.get(), self.cursor as u16));
        let sci = wapi::SpaceAndClipInfo {
            spatial_id: self.spatial_id,
            clip_id: wapi::ClipId::root(self.spatial_id.pipeline_id()),
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

        let grad = self.builder.create_gradient(start, end, stops, wapi::ExtendMode::Clamp);
        self.builder
            .push_gradient(&lpi, &sci, grad, final_rect.size, LayoutSize::default());
    }

    pub fn push_border(
        &mut self,
        final_rect: LayoutRect,
        widths: LayoutSideOffsets,
        details: wapi::BorderDetails,
        hit_tag: Option<HitTag>,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);

        self.builder.push_border(&lpi, &sci, widths, details);
    }

    pub fn push_text(
        &mut self,
        final_rect: LayoutRect,
        glyphs: &[wapi::GlyphInstance],
        font_instance_key: FontInstanceKey,
        color: ColorF,
        hit_tag: Option<HitTag>,
    ) {
        let (lpi, sci) = self.layout_and_clip(final_rect, hit_tag);

        self.builder
            .push_text(&lpi, &sci, &glyphs, font_instance_key, color, None);
    }

    pub fn push_focusable(&mut self, key: FocusKey, area: &LayoutRect) {
        self.focus_map.push_focusable(key, area);
    }

    pub fn push_focus_area(&mut self, _child: &impl Ui, _final_rect: &LayoutRect) {
        // within area navigation config as parameter.
        unimplemented!()
    }

    pub fn final_size(&self) -> LayoutSize {
        self.final_size
    }

    pub fn finalize(self) -> (wapi::PipelineId, LayoutSize, wapi::BuiltDisplayList) {
        self.builder.finalize()
    }
}

pub enum FocusState {
    NotFocused,
    NotActive,
    Active,
}

pub enum KeyNavigation {
    /// TAB goes to next in text reading order.
    /// Capture: TAB in last item goes back to first.
    /// Not capture: TAB in last item goes to next item after scope.
    Tab,
    /// Arrows goes to closest item in the arrow direction.
    /// Capture: Arrow press into edge of scope loops back to begining of the same line or column.
    ///    * Search next within a range to the same direction but in a parallel dimension?
    ///    * Remember dimension that entered item when going back (instead of using middle)?
    /// Not capture: Behaves like parent scope allows arrow navigation within this scope.
    Arrows,
    Both,
}

struct FocusScope {
        navigation: KeyNavigation,
        capture: bool,
        len: usize,
    }


struct FocusObject2{
    key: FocusKey,
    rect: LayoutRect,
    scope: Option<Box<FocusScope>>,
}

enum FocusObject {
    Focusable(FocusKey, LayoutRect),
    FocusScope {
        navigation: KeyNavigation,
        capture: bool,
        len: usize,
    },
}

//https://stackoverflow.com/questions/13420747/four-way-navigation-algorithm
#[derive(new)]
pub(crate) struct FocusMap {
    #[new(default)]
    current_scope: Vec<usize>,
    #[new(default)]
    offset: LayoutPoint,
    #[new(default)]
    objects: Vec<FocusObject>,
}
impl FocusMap {
    pub fn push_reference_frame(&mut self, final_rect: &LayoutRect) {
        self.offset += final_rect.origin.to_vector();
    }

    pub fn pop_reference_frame(&mut self, final_rect: &LayoutRect) {
        self.offset -= final_rect.origin.to_vector();
    }

    pub fn push_focus_scope(&mut self, navigation: KeyNavigation, capture: bool) {
        self.current_scope.push(self.objects.len());
        self.objects.push(FocusObject::FocusScope {
            navigation,
            capture,
            len: 0,
        });
    }

    pub fn pop_fucus_scope(&mut self) {
        let i = self.current_scope.pop().expect("Popped with no pushed FocusScope");
        let scope_len = self.objects.len() - i;
        if let FocusObject::FocusScope { len, .. } = &mut self.objects[i] {
            *len = scope_len;
        }
    }

    pub fn push_focusable(&mut self, key: FocusKey, area: &LayoutRect) {
        self.objects.push(FocusObject::Focusable(key, *area));
    }

    fn position(&self, focus_key: FocusKey) -> Option<usize> {
        self.objects.iter().position(|o| match o {
            FocusObject::Focusable(key, _) if *key == focus_key => true,
            _ => false,
        })
    }

    fn starting_point(&self) -> Option<FocusKey> {
        unimplemented!()
    }

    fn next_towards(&self, direction: (f32, f32), key: FocusKey) -> FocusKey {
        let i = self.position(key).unwrap();
        //self.objects[i].rect
        unimplemented!()
    }

    pub fn focus(&self, focused: Option<FocusKey>, r: FocusRequest) -> Option<FocusKey> {
        match (r, focused) {
            (FocusRequest::Direct(direct_key), _) => self.position(direct_key).map(|_| direct_key),
            (_, None) => self.starting_point(),
            //Tab - Shift+Tab
            (FocusRequest::Next, Some(key)) => unimplemented!(),
            (FocusRequest::Prev, Some(key)) => unimplemented!(),
            //Arrow Keys
            (FocusRequest::Left, Some(key)) => Some(self.next_towards((-1., 0.), key)),
            (FocusRequest::Right, Some(key)) => Some(self.next_towards((1., 0.), key)),
            (FocusRequest::Up, Some(key)) => Some(self.next_towards((0., -1.), key)),
            (FocusRequest::Down, Some(key)) => Some(self.next_towards((0., 1.), key)),
        }
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

#[derive(Debug, Clone, Copy)]
pub struct MouseInput {
    pub state: ElementState,
    pub button: MouseButton,
    pub modifiers: ModifiersState,
    pub position: LayoutPoint,
}

#[derive(Debug, Clone, Copy)]
pub struct UiMouseMove {
    pub position: LayoutPoint,
    pub modifiers: ModifiersState,
}

/// Hit test results.
#[derive(Default)]
pub struct Hits {
    points: FnvHashMap<HitTag, LayoutPoint>,
    cursor: CursorIcon,
}

impl Hits {
    pub fn new(hits: wapi::HitTestResult) -> Self {
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
                .map(|h| {
                    (
                        HitTag(NonZeroU64::new(h.tag.0).expect("Invalid tag: 0")),
                        h.point_relative_to_item,
                    )
                })
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

#[derive(Debug, PartialEq, Eq)]
pub enum FocusStatus {
    Focused,
    FocusWithin,
}

/// An UI component.
///
/// # Implementers
/// This is usually not implemented directly, consider using [impl_ui](attr.impl_ui.html) first.
pub trait Ui {
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize;

    fn arrange(&mut self, final_size: LayoutSize);

    fn render(&self, f: &mut NextFrame);

    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate);

    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate);

    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate);

    fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate);

    fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn focus_status(&self) -> Option<FocusStatus>;

    /// Gets the point over this UI element using a hit test result.
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint>;

    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate);

    /// Box this component, unless it is already `Box<dyn Ui>`.
    fn into_box(self) -> Box<dyn Ui>
    where
        Self: Sized + 'static,
    {
        Box::new(self)
    }
}

#[impl_ui_crate(delegate: self.as_ref(), delegate_mut: self.as_mut())]
impl Ui for Box<dyn Ui> {
    fn into_box(self) -> Box<dyn Ui> {
        self
    }
}

#[impl_ui_crate]
impl Ui for () {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        LayoutSize::default()
    }
}

// TODO
// https://github.com/servo/webrender/commit/717b1a272e8425d3952cc19f6d182b9087495c32
// https://doc.servo.org/webrender_api/struct.CommonItemProperties.html
// https://doc.servo.org/webrender_api/struct.DisplayListBuilder.html#method.push_hit_test

#[derive(new)]
pub struct UiCursor<T: Ui> {
    child: T,
    cursor: CursorIcon,
}

#[impl_ui_crate(child)]
impl<T: Ui + 'static> UiCursor<T> {
    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_cursor(self.cursor, &self.child)
    }
}

pub fn cursor<T: Ui>(child: T, cursor: CursorIcon) -> UiCursor<T> {
    UiCursor::new(child, cursor)
}

pub trait Cursor: Ui + Sized {
    fn cursor(self, cursor: CursorIcon) -> UiCursor<Self> {
        UiCursor::new(self, cursor)
    }
}
impl<T: Ui> Cursor for T {}

#[derive(new)]
pub struct SetParentValue<T: Ui, V, R: Value<V>> {
    child: T,
    key: ParentValueKey<V>,
    value: R,
}

#[impl_ui_crate(child)]
impl<T: Ui, V: 'static, R: Value<V>> SetParentValue<T, V, R> {
    #[Ui]
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.init(v, update));
    }

    #[Ui]
    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;

        if self.value.changed() {
            values.with_parent_value(self.key, &self.value, |v| child.parent_value_changed(v, update));
        }

        values.with_parent_value(self.key, &self.value, |v| child.value_changed(v, update));
    }

    #[Ui]
    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.parent_value_changed(v, update));
    }

    #[Ui]
    fn keyboard_input(&mut self, input: &KeyboardInput, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.keyboard_input(input, v, update));
    }

    #[Ui]
    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.window_focused(focused, v, update));
    }

    #[Ui]
    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.mouse_input(input, hits, v, update));
    }

    #[Ui]
    fn mouse_move(&mut self, input: &UiMouseMove, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.mouse_move(input, hits, v, update));
    }

    #[Ui]
    fn mouse_entered(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.mouse_entered(v, update));
    }

    #[Ui]
    fn mouse_left(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.mouse_left(v, update));
    }

    #[Ui]
    fn close_request(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let child = &mut self.child;
        values.with_parent_value(self.key, &self.value, |v| child.close_request(v, update));
    }
}

pub trait ParentValue: Ui + Sized {
    fn set_ctx_val<T: 'static, V: IntoValue<T>>(
        self,
        key: ParentValueKey<T>,
        value: V,
    ) -> SetParentValue<Self, T, V::Value> {
        SetParentValue::new(self, key, value.into_value())
    }

    //TODO alias value
}
impl<T: Ui> ParentValue for T {}

#[derive(new)]
pub struct Focusable<C: Ui> {
    child: C,
    focused: bool,
}
#[impl_ui_crate(child)]
impl<C: Ui> Focusable<C> {
    #[Ui]
    fn window_focused(&mut self, focused: bool, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.window_focused(focused, values, update);
    }

    #[Ui]
    fn mouse_input(&mut self, input: &MouseInput, hits: &Hits, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.mouse_input(input, hits, values, update);

        if input.state == ElementState::Pressed {
            self.focused = self.child.focus_status().is_none() && self.point_over(hits).is_some();
        }
    }

    #[Ui]
    fn focus_status(&self) -> Option<FocusStatus> {
        if self.focused {
            Some(FocusStatus::Focused)
        } else {
            match self.child.focus_status() {
                None => None,
                _ => Some(FocusStatus::FocusWithin),
            }
        }
    }
}

pub trait FocusableExt: Ui + Sized {
    fn focusable(self) -> Focusable<Self> {
        Focusable::new(self, false)
    }
}
impl<T: Ui> FocusableExt for T {}
