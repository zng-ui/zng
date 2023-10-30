//! UI nodes used for building a text widget.

use std::{borrow::Cow, fmt, mem, ops, sync::Arc, time::Instant};

use atomic::{Atomic, Ordering};

use super::{
    commands::{TextEditOp, TextSelectOp, UndoTextEditOp, EDIT_CMD, SELECT_ALL_CMD, SELECT_CMD},
    text_properties::*,
};
use crate::{
    core::{
        access::{ACCESS_SELECTION_EVENT, ACCESS_TEXT_EVENT},
        clipboard::{CLIPBOARD, COPY_CMD, CUT_CMD, PASTE_CMD},
        focus::{FocusInfoBuilder, FOCUS, FOCUS_CHANGED_EVENT},
        keyboard::{Key, KeyState, KEYBOARD, KEY_INPUT_EVENT},
        mouse::{MOUSE, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT},
        pointer_capture::{POINTER_CAPTURE, POINTER_CAPTURE_EVENT},
        task::parking_lot::Mutex,
        text::{font_features::FontVariations, *},
        touch::{TOUCH_LONG_PRESS_EVENT, TOUCH_TAP_EVENT},
        window::WindowLoadingHandle,
    },
    prelude::{
        new_widget::*,
        scroll::{commands::ScrollToMode, SCROLL},
    },
};

/// Represents the caret position at the [`ResolvedText`] level.
#[derive(Clone)]
pub struct CaretInfo {
    /// Caret opacity.
    ///
    /// This variable is replaced often, the text resolver subscribes to it for
    /// [`UpdateOp::RenderUpdate`] automatically.
    pub opacity: ReadOnlyArcVar<Factor>,

    /// Caret byte offset in the text string.
    ///
    /// This is the insertion offset on the text, it can be the text length.
    pub index: Option<CaretIndex>,

    /// Second index that defines the start or end of a selection range.
    pub selection_index: Option<CaretIndex>,

    /// Selection by word or line sets this value, selection extend by word or line
    /// grows from this central selection. The value is `(selection, is_word)`.
    pub initial_selection: Option<(ops::Range<CaretIndex>, bool)>,

    /// Value incremented by one every time the `index` is set.
    ///
    /// This is used to signal interaction with the `index` value by [`TextEditOp`]
    /// even if the interaction only sets-it to the index same value.
    pub index_version: u8,

    /// If the index was set by using the [`caret_retained_x`].
    ///
    /// [`caret_retained_x`]: LayoutText::caret_retained_x
    pub used_retained_x: bool,

    /// Don't scroll to new caret position on the next update.
    ///
    /// If this is set to `true` the next time `index` or `index_version` changes auto-scroll is skipped once.
    pub skip_next_scroll: bool,
}
impl fmt::Debug for CaretInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CaretInfo")
            .field("opacity", &self.opacity.debug())
            .field("index", &self.index)
            .field("index_version", &self.index_version)
            .field("used_retained_x", &self.used_retained_x)
            .finish()
    }
}
impl CaretInfo {
    /// Set the index and update the index version.
    pub fn set_index(&mut self, index: CaretIndex) {
        self.index = Some(index);
        self.index_version = self.index_version.wrapping_add(1);
    }

    /// Sets the selection start, end and update the index version.
    ///
    /// The `end` is the caret position.
    pub fn set_selection(&mut self, start: CaretIndex, end: CaretIndex) {
        self.selection_index = Some(start);
        self.set_index(end);
    }

    /// Clears selection.
    pub fn clear_selection(&mut self) {
        self.selection_index = None;
        self.initial_selection = None;
        self.index_version = self.index_version.wrapping_add(1);
    }

    /// Set the char byte index and update the index version.
    ///
    /// The caret line is always snapped when the caret changes, so the line value will be updated.
    pub fn set_char_index(&mut self, index: usize) {
        if let Some(i) = &mut self.index {
            i.index = index;
        } else {
            self.index = Some(CaretIndex { index, line: 0 });
        }
        self.index_version = self.index_version.wrapping_add(1);
    }

    /// Set the char byte index of the selection start, end and update the index version.
    ///
    /// The `end` is the caret position.
    ///
    /// The caret and selection lines are always snapped when the caret changes, so the line values will be updated.
    pub fn set_char_selection(&mut self, start: usize, end: usize) {
        if let Some(s) = &mut self.selection_index {
            s.index = start;
        } else {
            self.selection_index = Some(CaretIndex { index: start, line: 0 });
        }
        self.set_char_index(end);
    }

    /// Gets the selection range if both [`index`] and [`selection_index`] are set.
    ///
    /// [`index`]: Self::index
    /// [`selection_index`]: Self::selection_index
    pub fn selection_range(&self) -> Option<ops::Range<CaretIndex>> {
        let a = self.index?;
        let b = self.selection_index?;

        use std::cmp::Ordering;
        match a.index.cmp(&b.index) {
            Ordering::Less => Some(a..b),
            Ordering::Equal => None,
            Ordering::Greater => Some(b..a),
        }
    }

    /// Gets the character range of the selection if both [`index`] and [`selection_index`] are set.
    ///
    /// [`index`]: Self::index
    /// [`selection_index`]: Self::selection_index
    pub fn selection_char_range(&self) -> Option<ops::Range<usize>> {
        self.selection_range().map(|r| r.start.index..r.end.index)
    }
}

/// Represents the resolved fonts and the transformed, white space corrected and segmented text.
#[derive(Debug)]
pub struct ResolvedText {
    /// Text transformed, white space corrected and segmented.
    pub text: SegmentedText,
    /// Queried font faces.
    pub faces: FontFaceList,
    /// Font synthesis allowed by the text context and required to render the best font match.
    pub synthesis: FontSynthesis,

    /// Layout that needs to be recomputed as identified by the text resolver node.
    ///
    /// This is added to the layout invalidation by the layout node itself. When set a layout must
    /// be requested for the widget.
    pub pending_layout: PendingLayout,

    /// Text modification is scheduled, caret info will only be valid after update.
    pub pending_edit: bool,

    /// Caret index and animation.
    pub caret: Mutex<CaretInfo>,

    /// Baseline set by `layout_text` during measure and used by `new_border` during arrange.
    baseline: Atomic<Px>,
}
impl Clone for ResolvedText {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            faces: self.faces.clone(),
            synthesis: self.synthesis,
            pending_layout: self.pending_layout,
            pending_edit: self.pending_edit,
            caret: Mutex::new(self.caret.lock().clone()),
            baseline: Atomic::new(self.baseline.load(Ordering::Relaxed)),
        }
    }
}
impl ResolvedText {
    fn no_context() -> Self {
        panic!("no `ResolvedText` in context, only available inside `resolve_text`")
    }

    /// Gets if the current code has resolved text in context.
    pub fn in_context() -> bool {
        !RESOLVED_TEXT.is_default()
    }

    /// Get the current contextual resolved text.
    ///
    /// # Panics
    ///
    /// Panics if requested in a node outside [`resolve_text`].
    pub fn get() -> Arc<ResolvedText> {
        RESOLVED_TEXT.get()
    }

    fn call_edit_op(ctx: &mut Option<Self>, op: impl FnOnce()) {
        RESOLVED_TEXT.with_context_opt(ctx, op);
        ctx.as_mut().unwrap().pending_edit = true;
    }
}

/// Info about the last text render or render update.
#[derive(Debug, Clone)]
pub struct RenderInfo {
    /// Render transform of the text, in  the window space.
    pub transform: PxTransform,
    /// Render scale factor of the text.
    pub scale_factor: Factor,
}
impl Default for RenderInfo {
    /// Identify, 1.fct()
    fn default() -> Self {
        Self {
            transform: PxTransform::identity(),
            scale_factor: 1.fct(),
        }
    }
}

/// Represents the layout text.
#[derive(Debug)]
pub struct LayoutText {
    /// Sized [`faces`].
    ///
    /// [`faces`]: ResolvedText::faces
    pub fonts: FontList,

    /// Layout text.
    pub shaped_text: ShapedText,

    /// Shaped text overflow info.
    pub overflow: Option<TextOverflowInfo>,

    /// Shaped text used as suffix when `shaped_text` overflows.
    pub overflow_suffix: Option<ShapedText>,

    /// Version updated every time the `shaped_text` is reshaped.
    pub shaped_text_version: u32,

    /// List of overline segments, defining origin and width of each line.
    ///
    /// Note that overlines are only computed if the `overline_thickness` is more than `0`.
    ///
    /// Default overlines are rendered by [`render_overlines`].
    pub overlines: Vec<(PxPoint, Px)>,

    /// Computed [`OVERLINE_THICKNESS_VAR`].
    pub overline_thickness: Px,

    /// List of strikethrough segments, defining origin and width of each line.
    ///
    /// Note that strikethroughs are only computed if the `strikethrough_thickness` is more than `0`.
    ///
    /// Default overlines are rendered by [`render_strikethroughs`].
    pub strikethroughs: Vec<(PxPoint, Px)>,
    /// Computed [`STRIKETHROUGH_THICKNESS_VAR`].
    pub strikethrough_thickness: Px,

    /// List of underline segments, defining origin and width of each line.
    ///
    /// Note that underlines are only computed if the `underline_thickness` is more than `0`.
    ///
    /// Default overlines are rendered by [`render_underlines`].
    ///
    /// Note that underlines trop down from these lines.
    pub underlines: Vec<(PxPoint, Px)>,
    /// Computed [`UNDERLINE_THICKNESS_VAR`].
    pub underline_thickness: Px,

    /// Top-middle offset of the caret in the shaped text.
    pub caret_origin: Option<PxPoint>,

    /// The x offset used when pressing up or down.
    pub caret_retained_x: Px,

    /// Info about the last text render or render update.
    pub render_info: Mutex<RenderInfo>,

    /// Latest layout viewport.
    pub viewport: PxSize,
}

impl Clone for LayoutText {
    fn clone(&self) -> Self {
        Self {
            fonts: self.fonts.clone(),
            shaped_text: self.shaped_text.clone(),
            overflow: self.overflow.clone(),
            overflow_suffix: self.overflow_suffix.clone(),
            shaped_text_version: self.shaped_text_version,
            overlines: self.overlines.clone(),
            overline_thickness: self.overline_thickness,
            strikethroughs: self.strikethroughs.clone(),
            strikethrough_thickness: self.strikethrough_thickness,
            underlines: self.underlines.clone(),
            underline_thickness: self.underline_thickness,
            caret_origin: self.caret_origin,
            caret_retained_x: self.caret_retained_x,
            render_info: Mutex::new(self.render_info.lock().clone()),
            viewport: PxSize::zero(),
        }
    }
}
impl LayoutText {
    fn no_context() -> Self {
        panic!("no `LayoutText` in context, only available inside `layout_text` during layout and render")
    }

    /// Gets if the current code has layout text in context.
    pub fn in_context() -> bool {
        !LAYOUT_TEXT.is_default()
    }

    /// Get the current contextual layout text.
    ///
    /// # Panics
    ///
    /// Panics if not available in context. Is only available inside [`layout_text`] after the first layout.
    pub fn get() -> Arc<LayoutText> {
        LAYOUT_TEXT.get()
    }

    fn call_select_op(ctx: &mut Option<Self>, op: impl FnOnce()) {
        LAYOUT_TEXT.with_context_opt(ctx, op);
    }
}

context_local! {
    /// Represents the contextual [`ResolvedText`] setup by the [`resolve_text`] node.
    static RESOLVED_TEXT: ResolvedText = ResolvedText::no_context();
    /// Represents the contextual [`LayoutText`] setup by the [`layout_text`] node.
    static LAYOUT_TEXT: LayoutText  = LayoutText::no_context();
}

/// An UI node that resolves the text context vars, applies the text transform and white space correction and segments the `text`.
///
/// This node setups the [`ResolvedText`] for all inner nodes, the `Text!` widget includes this node in the [`NestGroup::EVENT`] group,
/// so all properties except [`NestGroup::CONTEXT`] have access using the [`ResolvedText::get`] function.
///
/// This node also sets the accessibility label to the resolved text.
pub fn resolve_text(child: impl UiNode, text: impl IntoVar<Txt>) -> impl UiNode {
    struct LoadingFontFaceList {
        _var_handle: VarHandle,
        result: ResponseVar<FontFaceList>,
        _loading: Option<WindowLoadingHandle>,
    }
    impl LoadingFontFaceList {
        fn new(face: ResponseVar<FontFaceList>) -> Self {
            Self {
                _var_handle: face.subscribe(UpdateOp::Update, WIDGET.id()),
                result: face,
                _loading: WINDOW.loading_handle(1.secs()),
            }
        }
    }

    /// Data allocated only when `editable`.
    #[derive(Default)]
    struct EditData {
        events: [EventHandle; 5],
        caret_animation: VarHandle,
        cut: CommandHandle,
        copy: CommandHandle,
        paste: CommandHandle,
        edit: CommandHandle,
    }
    impl EditData {
        fn get(edit_data: &mut Option<Box<Self>>) -> &mut Self {
            &mut *edit_data.get_or_insert_with(Default::default)
        }

        fn subscribe(&mut self) {
            let editable = TEXT_EDITABLE_VAR.get();
            if editable {
                let id = WIDGET.id();

                self.events[0] = FOCUS_CHANGED_EVENT.subscribe(id);
                self.events[1] = INTERACTIVITY_CHANGED_EVENT.subscribe(id);
                self.events[2] = KEY_INPUT_EVENT.subscribe(id);
                self.events[3] = ACCESS_TEXT_EVENT.subscribe(id);

                self.paste = PASTE_CMD.scoped(id).subscribe(true);
                self.edit = EDIT_CMD.scoped(id).subscribe(true);
            }

            if TEXT_SELECTABLE_VAR.get() {
                let id = WIDGET.id();

                self.events[4] = ACCESS_SELECTION_EVENT.subscribe(id);

                self.copy = COPY_CMD.scoped(id).subscribe(true);
                if editable {
                    self.cut = CUT_CMD.scoped(id).subscribe(true);
                } else {
                    self.events[2] = KEY_INPUT_EVENT.subscribe(id);
                }
            }
        }
    }

    let text = text.into_var().boxed();
    let mut loading_faces = None;
    let mut resolved = None;

    // Use `EditData::get` to access.
    let mut edit_data = None;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            // for r.text
            WIDGET
                .sub_var(&text)
                .sub_var(&TEXT_TRANSFORM_VAR)
                .sub_var(&WHITE_SPACE_VAR)
                .sub_var(&DIRECTION_VAR);
            // for r.font_face & r.synthesis
            WIDGET
                .sub_var(&FONT_FAMILY_VAR)
                .sub_var(&FONT_STYLE_VAR)
                .sub_var(&FONT_WEIGHT_VAR)
                .sub_var(&FONT_STRETCH_VAR)
                .sub_var(&FONT_SYNTHESIS_VAR)
                .sub_var(&LANG_VAR);
            // for editable mode
            WIDGET.sub_var(&TEXT_EDITABLE_VAR).sub_var(&TEXT_SELECTABLE_VAR);

            let style = FONT_STYLE_VAR.get();
            let weight = FONT_WEIGHT_VAR.get();

            let f =
                FONT_FAMILY_VAR.with(|family| LANG_VAR.with(|lang| FONTS.list(family, style, weight, FONT_STRETCH_VAR.get(), lang.best())));

            let f = if f.is_done() {
                f.into_rsp().unwrap()
            } else {
                loading_faces = Some(LoadingFontFaceList::new(f));
                FontFaceList::empty()
            };

            let mut txt = text.get();

            let editable = TEXT_EDITABLE_VAR.get();

            if !editable {
                TEXT_TRANSFORM_VAR.with(|t| {
                    if let Cow::Owned(t) = t.transform(&txt) {
                        txt = t;
                    }
                });
                WHITE_SPACE_VAR.with(|t| {
                    if let Cow::Owned(t) = t.transform(&txt) {
                        txt = t;
                    }
                });
            }

            let caret_opacity = if editable && FOCUS.is_focused(WIDGET.id()).get() {
                let v = KEYBOARD.caret_animation();
                EditData::get(&mut edit_data).caret_animation = v.subscribe(UpdateOp::Update, WIDGET.id());
                v
            } else {
                var(0.fct()).read_only()
            };

            resolved = Some(ResolvedText {
                synthesis: FONT_SYNTHESIS_VAR.get() & f.best().synthesis_for(style, weight),
                faces: f,
                text: SegmentedText::new(txt, DIRECTION_VAR.get()),
                pending_layout: PendingLayout::empty(),
                pending_edit: false,
                baseline: Atomic::new(Px(0)),
                caret: Mutex::new(CaretInfo {
                    opacity: caret_opacity,
                    index: None,
                    selection_index: None,
                    initial_selection: None,
                    index_version: 0,
                    used_retained_x: false,
                    skip_next_scroll: false,
                }),
            });

            if editable || TEXT_SELECTABLE_VAR.get() {
                EditData::get(&mut edit_data).subscribe();
            }

            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.init());
        }
        UiNodeOp::Deinit => {
            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.deinit());
            edit_data = None;
            loading_faces = None;
            resolved = None;
        }
        UiNodeOp::Info { info } => {
            let editable = TEXT_EDITABLE_VAR.get();
            if editable || TEXT_SELECTABLE_VAR.get() {
                FocusInfoBuilder::new(info).focusable(true);
            }
            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.info(info));
            if !editable {
                if let Some(mut a) = info.access() {
                    a.set_label(resolved.as_ref().unwrap().text.text().clone());
                }
            }
        }
        UiNodeOp::Event { update } => {
            if let Some(_args) = FONT_CHANGED_EVENT.on(update) {
                // font query may return a different result.

                let style = FONT_STYLE_VAR.get();
                let weight = FONT_WEIGHT_VAR.get();

                let faces = FONT_FAMILY_VAR
                    .with(|family| LANG_VAR.with(|lang| FONTS.list(family, style, weight, FONT_STRETCH_VAR.get(), lang.best())));

                if faces.is_done() {
                    let faces = faces.rsp().unwrap();

                    let r = resolved.as_mut().unwrap();

                    if r.faces != faces {
                        r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight);
                        r.faces = faces;

                        r.pending_layout = PendingLayout::RESHAPE;
                        WIDGET.layout();
                    }
                } else {
                    loading_faces = Some(LoadingFontFaceList::new(faces));
                }
            } else if TEXT_EDITABLE_VAR.get() {
                let prev_caret = {
                    let caret = resolved.as_mut().unwrap().caret.get_mut();
                    (caret.index, caret.index_version, caret.selection_index)
                };

                let widget = WIDGET.info();

                if let Some(args) = INTERACTIVITY_CHANGED_EVENT.on(update) {
                    if args.is_disable(widget.id()) {
                        EditData::get(&mut edit_data).caret_animation = VarHandle::dummy();
                        resolved.as_mut().unwrap().caret.get_mut().opacity = var(0.fct()).read_only();
                    }
                }

                if !resolved.as_mut().unwrap().pending_edit && text.capabilities().can_modify() && widget.interactivity().is_enabled() {
                    if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                        if let KeyState::Pressed = args.state {
                            match &args.key {
                                Key::Backspace => {
                                    let caret = resolved.as_mut().unwrap().caret.get_mut();
                                    if caret.selection_index.is_some() || caret.index.unwrap_or(CaretIndex::ZERO).index > 0 {
                                        if args.modifiers.is_only_ctrl() {
                                            args.propagation().stop();
                                            ResolvedText::call_edit_op(&mut resolved, || TextEditOp::backspace_word().call(&text));
                                        } else if args.modifiers.is_empty() {
                                            args.propagation().stop();
                                            ResolvedText::call_edit_op(&mut resolved, || TextEditOp::backspace().call(&text));
                                        }
                                    }
                                }
                                Key::Delete => {
                                    let r = resolved.as_mut().unwrap();
                                    let caret = r.caret.get_mut();
                                    let caret_idx = caret.index.unwrap_or(CaretIndex::ZERO);
                                    if caret.selection_index.is_some() || caret_idx.index < r.text.text().len() {
                                        if args.modifiers.is_only_ctrl() {
                                            args.propagation().stop();
                                            ResolvedText::call_edit_op(&mut resolved, || TextEditOp::delete_word().call(&text));
                                        } else if args.modifiers.is_empty() {
                                            args.propagation().stop();
                                            ResolvedText::call_edit_op(&mut resolved, || TextEditOp::delete().call(&text));
                                        }
                                    }
                                }
                                _ => {
                                    let insert = args.insert_str();
                                    if !insert.is_empty() {
                                        let skip =
                                            (args.is_tab() && !ACCEPTS_TAB_VAR.get()) || (args.is_line_break() && !ACCEPTS_ENTER_VAR.get());
                                        if !skip {
                                            args.propagation().stop();
                                            ResolvedText::call_edit_op(&mut resolved, || {
                                                TextEditOp::insert(Txt::from_str(insert)).call(&text)
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                        let resolved = resolved.as_mut().unwrap();
                        let caret = resolved.caret.get_mut();
                        let caret_index = &mut caret.index;

                        if args.is_focused(widget.id()) {
                            if caret_index.is_none() {
                                *caret_index = Some(CaretIndex::ZERO);
                            } else {
                                // restore animation when the caret_index did not change
                                caret.opacity = KEYBOARD.caret_animation();
                                EditData::get(&mut edit_data).caret_animation =
                                    caret.opacity.subscribe(UpdateOp::RenderUpdate, widget.id());
                            }
                        } else {
                            EditData::get(&mut edit_data).caret_animation = VarHandle::dummy();
                            caret.opacity = var(0.fct()).read_only();
                        }
                    } else if let Some(args) = CUT_CMD.scoped(widget.id()).on_unhandled(update) {
                        let ctx = resolved.as_mut().unwrap();
                        if let Some(range) = ctx.caret.get_mut().selection_char_range() {
                            args.propagation().stop();
                            if CLIPBOARD.set_text(Txt::from_str(&ctx.text.text()[range])).is_ok() {
                                ResolvedText::call_edit_op(&mut resolved, || TextEditOp::delete().call(&text));
                            }
                        }
                    } else if let Some(args) = PASTE_CMD.scoped(widget.id()).on_unhandled(update) {
                        if let Some(paste) = CLIPBOARD.text().ok().flatten() {
                            if !paste.is_empty() {
                                args.propagation().stop();

                                ResolvedText::call_edit_op(&mut resolved, || {
                                    TextEditOp::insert(paste).call(&text);
                                });
                            }
                        }
                    } else if let Some(args) = EDIT_CMD.scoped(widget.id()).on_unhandled(update) {
                        if let Some(op) = args.param::<UndoTextEditOp>() {
                            args.propagation().stop();

                            ResolvedText::call_edit_op(&mut resolved, || op.call(&text));
                        } else if let Some(op) = args.param::<TextEditOp>() {
                            args.propagation().stop();

                            ResolvedText::call_edit_op(&mut resolved, || op.clone().call(&text));
                        }
                    } else if let Some(args) = ACCESS_TEXT_EVENT.on_unhandled(update) {
                        if args.widget_id == widget.id() {
                            args.propagation().stop();

                            ResolvedText::call_edit_op(&mut resolved, || {
                                if args.selection_only {
                                    TextEditOp::insert(args.txt.clone())
                                } else {
                                    let current_len = text.with(|t| t.len());
                                    let new_len = args.txt.len();
                                    TextEditOp::replace(0..current_len, args.txt.clone(), new_len..new_len)
                                }
                                .call(&text);
                            });
                        }
                    }

                    let resolved = resolved.as_mut().unwrap();
                    let caret = resolved.caret.get_mut();

                    if (caret.index, caret.index_version, caret.selection_index) != prev_caret {
                        caret.used_retained_x = false;
                        if caret.index.is_none() || !FOCUS.is_focused(widget.id()).get() {
                            EditData::get(&mut edit_data).caret_animation = VarHandle::dummy();
                            caret.opacity = var(0.fct()).read_only();
                        } else {
                            caret.opacity = KEYBOARD.caret_animation();
                            EditData::get(&mut edit_data).caret_animation = caret.opacity.subscribe(UpdateOp::RenderUpdate, widget.id());
                        }
                        resolved.pending_layout |= PendingLayout::CARET;
                        WIDGET.layout(); // update caret_origin
                    }
                }
            }
            if TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get() {
                let widget_id = WIDGET.id();

                if let Some(args) = COPY_CMD.scoped(widget_id).on_unhandled(update) {
                    let resolved = resolved.as_mut().unwrap();
                    if let Some(range) = resolved.caret.get_mut().selection_char_range() {
                        args.propagation().stop();
                        let _ = CLIPBOARD.set_text(Txt::from_str(&resolved.text.text()[range]));
                    }
                } else if let Some(args) = ACCESS_SELECTION_EVENT.on_unhandled(update) {
                    if args.start.0 == widget_id && args.caret.0 == widget_id {
                        args.propagation().stop();

                        let resolved = resolved.as_mut().unwrap();
                        let caret = resolved.caret.get_mut();

                        caret.set_char_selection(args.start.1, args.caret.1);

                        resolved.pending_layout |= PendingLayout::CARET;
                        WIDGET.layout();
                    }
                }
            }

            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.event(update));
        }
        UiNodeOp::Update { updates } => {
            let r = resolved.as_mut().unwrap();

            // update `r.text`, affects layout.
            if text.is_new() || TEXT_TRANSFORM_VAR.is_new() || WHITE_SPACE_VAR.is_new() || DIRECTION_VAR.is_new() {
                if text.is_new() {
                    if !r.pending_edit {
                        crate::core::undo::UNDO.clear();
                    }
                    r.pending_edit = false;
                }
                let mut text = text.get();

                if !TEXT_EDITABLE_VAR.get() {
                    TEXT_TRANSFORM_VAR.with(|t| {
                        if let Cow::Owned(t) = t.transform(&text) {
                            text = t;
                        }
                    });
                    WHITE_SPACE_VAR.with(|t| {
                        if let Cow::Owned(t) = t.transform(&text) {
                            text = t;
                        }
                    });
                }

                let direction = DIRECTION_VAR.get();
                if r.text.text() != &text || r.text.base_direction() != direction {
                    r.text = SegmentedText::new(text, direction);

                    // prevent invalid indexes
                    let caret = r.caret.get_mut();
                    if let Some(i) = &mut caret.index {
                        i.index = r.text.snap_grapheme_boundary(i.index);
                    }
                    if let Some(i) = &mut caret.selection_index {
                        i.index = r.text.snap_grapheme_boundary(i.index);
                    }
                    if let Some((cr, _)) = &mut caret.initial_selection {
                        cr.start.index = r.text.snap_grapheme_boundary(cr.start.index);
                        cr.end.index = r.text.snap_grapheme_boundary(cr.end.index);
                    }

                    if WINDOW.vars().access_enabled().get().is_enabled() {
                        WIDGET.info();
                    }

                    r.pending_layout = PendingLayout::RESHAPE;
                    WIDGET.layout();
                }
            }

            // update `r.font_face`, affects layout
            if FONT_FAMILY_VAR.is_new()
                || FONT_STYLE_VAR.is_new()
                || FONT_STRETCH_VAR.is_new()
                || FONT_WEIGHT_VAR.is_new()
                || LANG_VAR.is_new()
            {
                let style = FONT_STYLE_VAR.get();
                let weight = FONT_WEIGHT_VAR.get();

                let faces = FONT_FAMILY_VAR
                    .with(|family| LANG_VAR.with(|lang| FONTS.list(family, style, weight, FONT_STRETCH_VAR.get(), lang.best())));

                if faces.is_done() {
                    let faces = faces.rsp().unwrap();

                    if r.faces != faces {
                        r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(style, weight);
                        r.faces = faces;

                        r.pending_layout = PendingLayout::RESHAPE;
                        WIDGET.layout();
                    }
                } else {
                    loading_faces = Some(LoadingFontFaceList::new(faces));
                }
            }
            // update `r.synthesis`, affects render
            if FONT_SYNTHESIS_VAR.is_new() || FONT_STYLE_VAR.is_new() || FONT_WEIGHT_VAR.is_new() {
                let synthesis = FONT_SYNTHESIS_VAR.get() & r.faces.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());
                if r.synthesis != synthesis {
                    r.synthesis = synthesis;
                    WIDGET.render();
                }
            }

            if TEXT_EDITABLE_VAR.is_new() || TEXT_SELECTABLE_VAR.is_new() {
                edit_data = None;

                let editable = TEXT_EDITABLE_VAR.get();
                if editable || TEXT_SELECTABLE_VAR.get() {
                    EditData::get(&mut edit_data).subscribe();
                }

                let id = WIDGET.id();

                if editable && FOCUS.is_focused(id).get() {
                    let d = EditData::get(&mut edit_data);
                    let new_animation = KEYBOARD.caret_animation();
                    d.caret_animation = new_animation.subscribe(UpdateOp::RenderUpdate, id);
                    r.caret.get_mut().opacity = new_animation;
                } else {
                    r.caret.get_mut().opacity = var(0.fct()).read_only();
                }

                let mut text = text.get();
                if !editable {
                    // toggle text transforms
                    TEXT_TRANSFORM_VAR.with(|t| {
                        if let Cow::Owned(t) = t.transform(&text) {
                            text = t;
                        }
                    });
                    WHITE_SPACE_VAR.with(|t| {
                        if let Cow::Owned(t) = t.transform(&text) {
                            text = t;
                        }
                    });
                }

                if r.text.text() != &text {
                    r.text = SegmentedText::new(text, DIRECTION_VAR.get());
                    if let Some(i) = &mut r.caret.get_mut().index {
                        i.index = r.text.snap_grapheme_boundary(i.index);
                    }

                    r.pending_layout = PendingLayout::RESHAPE;
                    WIDGET.layout();
                }
            }

            if let Some(f) = loading_faces.take() {
                if f.result.is_done() {
                    let faces = f.result.into_rsp().unwrap();
                    if r.faces != faces {
                        r.synthesis = FONT_SYNTHESIS_VAR.get() & faces.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());
                        r.faces = faces;

                        r.pending_layout = PendingLayout::RESHAPE;
                        WIDGET.layout();
                    }
                } else {
                    loading_faces = Some(f);
                }
            }

            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.update(updates));
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = RESOLVED_TEXT.with_context_opt(&mut resolved, || child.layout(wl));
            resolved.as_mut().unwrap().pending_layout = PendingLayout::empty();
        }
        op => {
            RESOLVED_TEXT.with_context_opt(&mut resolved, || child.op(op));
        }
    })
}

bitflags::bitflags! {
    /// Text layout parts that need rebuild.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PendingLayout: u8 {
        /// Underline size and position.
        const UNDERLINE     = 0b0000_0001;
        /// Strikethrough size and position.
        const STRIKETHROUGH = 0b0000_0010;
        /// Overline size and position.
        const OVERLINE      = 0b0000_0100;
        /// Caret origin.
        const CARET         = 0b0000_1000;
        /// Overflow.
        const OVERFLOW      = 0b0001_0000;
        /// Text lines position, retains line glyphs but reposition for new align and outer box.
        const RESHAPE_LINES = 0b0111_1111;
        /// Full reshape, re-compute all glyphs.
        const RESHAPE       = 0b1111_1111;
    }
}

/// An UI node that layouts the parent [`ResolvedText`] defined by the text context vars.
///
/// This node setups the [`LayoutText`] for all inner nodes, the `Text!` widget includes this
/// node in the `NestGroup::CHILD_LAYOUT + 100` nest group, so all properties in [`NestGroup::CHILD_LAYOUT`]
/// can affect the layout normally and custom properties can be created to be inside this group and have access
///  to the [`LayoutText::get`] function.
pub fn layout_text(child: impl UiNode) -> impl UiNode {
    struct FinalText {
        txt: Option<LayoutText>,
        shaping_args: TextShapingArgs,
        pending: PendingLayout,

        txt_is_measured: bool,
        last_layout: (LayoutMetrics, Option<InlineConstraintsMeasure>),
    }
    impl FinalText {
        fn measure(&mut self, metrics: &LayoutMetrics) -> Option<PxSize> {
            if metrics.inline_constraints().is_some() {
                return None;
            }

            metrics.constraints().fill_or_exact()
        }

        fn layout(&mut self, metrics: &LayoutMetrics, t: &ResolvedText, is_measure: bool) -> PxSize {
            self.pending |= t.pending_layout;

            let font_size = metrics.font_size();

            if self.txt.is_none() {
                let fonts = t.faces.sized(font_size, FONT_VARIATIONS_VAR.with(FontVariations::finalize));
                self.txt = Some(LayoutText {
                    shaped_text: ShapedText::new(fonts.best()),
                    overflow: None,
                    overflow_suffix: None,
                    shaped_text_version: 0,
                    fonts,
                    overlines: vec![],
                    overline_thickness: Px(0),
                    strikethroughs: vec![],
                    strikethrough_thickness: Px(0),
                    underlines: vec![],
                    underline_thickness: Px(0),
                    caret_origin: None,
                    caret_retained_x: Px(0),
                    render_info: Mutex::default(),
                    viewport: metrics.viewport(),
                });
                self.pending.insert(PendingLayout::RESHAPE);
            }

            let txt = self.txt.as_mut().unwrap();

            if font_size != txt.fonts.requested_size() || !txt.fonts.is_sized_from(&t.faces) {
                txt.fonts = t.faces.sized(font_size, FONT_VARIATIONS_VAR.with(FontVariations::finalize));
                self.pending.insert(PendingLayout::RESHAPE);
            }

            if TEXT_WRAP_VAR.get() && !metrics.constraints().x.is_unbounded() {
                let max_width = metrics.constraints().x.max().unwrap();
                if self.shaping_args.max_width != max_width {
                    self.shaping_args.max_width = max_width;

                    if !self.pending.contains(PendingLayout::RESHAPE) && txt.shaped_text.can_rewrap(max_width) {
                        self.pending.insert(PendingLayout::RESHAPE);
                    }
                }
            } else if self.shaping_args.max_width != Px::MAX {
                self.shaping_args.max_width = Px::MAX;
                if !self.pending.contains(PendingLayout::RESHAPE) && txt.shaped_text.can_rewrap(Px::MAX) {
                    self.pending.insert(PendingLayout::RESHAPE);
                }
            }

            if txt.caret_origin.is_none() {
                self.pending.insert(PendingLayout::CARET);
            }

            if let Some(inline) = metrics.inline_constraints() {
                match inline {
                    InlineConstraints::Measure(m) => {
                        if self.shaping_args.inline_constraints != Some(m) {
                            self.shaping_args.inline_constraints = Some(m);
                            self.pending.insert(PendingLayout::RESHAPE);
                        }
                    }
                    InlineConstraints::Layout(l) => {
                        if !self.pending.contains(PendingLayout::RESHAPE)
                            && (Some(l.first_segs.len()) != txt.shaped_text.line(0).map(|l| l.segs_len())
                                || Some(l.last_segs.len())
                                    != txt
                                        .shaped_text
                                        .line(txt.shaped_text.lines_len().saturating_sub(1))
                                        .map(|l| l.segs_len()))
                        {
                            self.pending.insert(PendingLayout::RESHAPE);
                        }

                        if !self.pending.contains(PendingLayout::RESHAPE_LINES)
                            && (txt.shaped_text.mid_clear() != l.mid_clear
                                || txt.shaped_text.line(0).map(|l| l.rect()) != Some(l.first)
                                || txt
                                    .shaped_text
                                    .line(txt.shaped_text.lines_len().saturating_sub(1))
                                    .map(|l| l.rect())
                                    != Some(l.last))
                        {
                            self.pending.insert(PendingLayout::RESHAPE_LINES);
                        }
                    }
                }
            } else if self.shaping_args.inline_constraints.is_some() {
                self.shaping_args.inline_constraints = None;
                self.pending.insert(PendingLayout::RESHAPE);
            }

            if !self.pending.contains(PendingLayout::RESHAPE_LINES) {
                let size = txt.shaped_text.size();
                if metrics.constraints().fill_size_or(size) != txt.shaped_text.align_size() {
                    self.pending.insert(PendingLayout::RESHAPE_LINES);
                }
            }

            let font = txt.fonts.best();

            let space_len = font.space_x_advance();
            let dft_tab_len = space_len * 3;

            let (letter_spacing, word_spacing, tab_length) = {
                LAYOUT.with_constraints(PxConstraints2d::new_exact(space_len, space_len), || {
                    (
                        LETTER_SPACING_VAR.layout_x(),
                        WORD_SPACING_VAR.layout_x(),
                        TAB_LENGTH_VAR.layout_dft_x(dft_tab_len),
                    )
                })
            };

            let dft_line_height = font.metrics().line_height();
            let line_height = {
                LAYOUT.with_constraints(PxConstraints2d::new_exact(dft_line_height, dft_line_height), || {
                    LINE_HEIGHT_VAR.layout_dft_y(dft_line_height)
                })
            };
            let line_spacing =
                { LAYOUT.with_constraints(PxConstraints2d::new_exact(line_height, line_height), || LINE_SPACING_VAR.layout_y()) };

            if !self.pending.contains(PendingLayout::RESHAPE)
                && (letter_spacing != self.shaping_args.letter_spacing
                    || word_spacing != self.shaping_args.word_spacing
                    || tab_length != self.shaping_args.tab_x_advance)
            {
                self.pending.insert(PendingLayout::RESHAPE);
            }
            if !self.pending.contains(PendingLayout::RESHAPE_LINES)
                && (line_spacing != self.shaping_args.line_spacing || line_height != self.shaping_args.line_height)
            {
                self.pending.insert(PendingLayout::RESHAPE_LINES);
            }

            self.shaping_args.letter_spacing = letter_spacing;
            self.shaping_args.word_spacing = word_spacing;
            self.shaping_args.tab_x_advance = tab_length;
            self.shaping_args.line_height = line_height;
            self.shaping_args.line_spacing = line_spacing;

            let dft_thickness = font.metrics().underline_thickness;
            let (overline, strikethrough, underline) = {
                LAYOUT.with_constraints(PxConstraints2d::new_exact(line_height, line_height), || {
                    (
                        OVERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                        STRIKETHROUGH_THICKNESS_VAR.layout_dft_y(dft_thickness),
                        UNDERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                    )
                })
            };

            if !self.pending.contains(PendingLayout::OVERLINE) && (txt.overline_thickness == Px(0) && overline > Px(0)) {
                self.pending.insert(PendingLayout::OVERLINE);
            }
            if !self.pending.contains(PendingLayout::STRIKETHROUGH) && (txt.strikethrough_thickness == Px(0) && strikethrough > Px(0)) {
                self.pending.insert(PendingLayout::STRIKETHROUGH);
            }
            if !self.pending.contains(PendingLayout::UNDERLINE) && (txt.underline_thickness == Px(0) && underline > Px(0)) {
                self.pending.insert(PendingLayout::UNDERLINE);
            }
            txt.overline_thickness = overline;
            txt.strikethrough_thickness = strikethrough;
            txt.underline_thickness = underline;

            let align = TEXT_ALIGN_VAR.get();
            let overflow_align = TEXT_OVERFLOW_ALIGN_VAR.get();
            if !self.pending.contains(PendingLayout::RESHAPE_LINES)
                && (align != txt.shaped_text.align() || overflow_align != txt.shaped_text.overflow_align())
            {
                self.pending.insert(PendingLayout::RESHAPE_LINES);
            }

            /*
                APPLY
            */

            if self.pending.contains(PendingLayout::RESHAPE) {
                txt.shaped_text = txt.fonts.shape_text(&t.text, &self.shaping_args);
                self.pending = self.pending.intersection(PendingLayout::RESHAPE_LINES);
            }

            if !self.pending.contains(PendingLayout::RESHAPE_LINES)
                && txt.shaped_text.align_size() != metrics.constraints().fill_size_or(txt.shaped_text.block_size())
            {
                self.pending.insert(PendingLayout::RESHAPE_LINES);
            }

            if !is_measure {
                self.last_layout = (metrics.clone(), self.shaping_args.inline_constraints);

                if self.pending.contains(PendingLayout::RESHAPE_LINES) {
                    txt.shaped_text.reshape_lines(
                        metrics.constraints(),
                        metrics.inline_constraints().map(|c| c.layout()),
                        align,
                        overflow_align,
                        line_height,
                        line_spacing,
                        metrics.direction(),
                    );
                    txt.shaped_text_version = txt.shaped_text_version.wrapping_add(1);
                    t.baseline.store(txt.shaped_text.baseline(), Ordering::Relaxed);
                    txt.caret_origin = None;
                }
                if self.pending.contains(PendingLayout::OVERFLOW) {
                    let txt_size = txt.shaped_text.size();
                    let max_size = metrics.constraints().fill_size_or(txt_size);
                    if txt_size.width > max_size.width || txt_size.height > max_size.height {
                        let suf_width = txt.overflow_suffix.as_ref().map(|s| s.size().width).unwrap_or(Px(0));
                        txt.overflow = txt.shaped_text.overflow_info(max_size, suf_width);

                        if txt.overflow.is_some() && txt.overflow_suffix.is_none() && !TEXT_EDITABLE_VAR.get() {
                            match TEXT_OVERFLOW_VAR.get() {
                                TextOverflow::Truncate(suf) if !suf.is_empty() => {
                                    let suf = SegmentedText::new(suf, self.shaping_args.direction);
                                    let suf = txt.fonts.shape_text(&suf, &self.shaping_args);

                                    txt.overflow = txt.shaped_text.overflow_info(max_size, suf.size().width);
                                    txt.overflow_suffix = Some(suf);
                                }
                                _ => {}
                            }
                        }
                    } else {
                        txt.overflow = None;
                    }
                }
                if self.pending.contains(PendingLayout::OVERLINE) {
                    if txt.overline_thickness > Px(0) {
                        txt.overlines = txt.shaped_text.lines().map(|l| l.overline()).collect();
                    } else {
                        txt.overlines = vec![];
                    }
                }
                if self.pending.contains(PendingLayout::STRIKETHROUGH) {
                    if txt.strikethrough_thickness > Px(0) {
                        txt.strikethroughs = txt.shaped_text.lines().map(|l| l.strikethrough()).collect();
                    } else {
                        txt.strikethroughs = vec![];
                    }
                }
                if self.pending.contains(PendingLayout::UNDERLINE) {
                    if txt.underline_thickness > Px(0) {
                        let skip = UNDERLINE_SKIP_VAR.get();
                        match UNDERLINE_POSITION_VAR.get() {
                            UnderlinePosition::Font => {
                                if skip == UnderlineSkip::GLYPHS | UnderlineSkip::SPACES {
                                    txt.underlines = txt
                                        .shaped_text
                                        .lines()
                                        .flat_map(|l| l.underline_skip_glyphs_and_spaces(txt.underline_thickness))
                                        .collect();
                                } else if skip.contains(UnderlineSkip::GLYPHS) {
                                    txt.underlines = txt
                                        .shaped_text
                                        .lines()
                                        .flat_map(|l| l.underline_skip_glyphs(txt.underline_thickness))
                                        .collect();
                                } else if skip.contains(UnderlineSkip::SPACES) {
                                    txt.underlines = txt.shaped_text.lines().flat_map(|l| l.underline_skip_spaces()).collect();
                                } else {
                                    txt.underlines = txt.shaped_text.lines().map(|l| l.underline()).collect();
                                }
                            }
                            UnderlinePosition::Descent => {
                                // descent clears all glyphs, so we only need to care about spaces
                                if skip.contains(UnderlineSkip::SPACES) {
                                    txt.underlines = txt.shaped_text.lines().flat_map(|l| l.underline_descent_skip_spaces()).collect();
                                } else {
                                    txt.underlines = txt.shaped_text.lines().map(|l| l.underline_descent()).collect();
                                }
                            }
                        }
                    } else {
                        txt.underlines = vec![];
                    }
                }

                if self.pending.contains(PendingLayout::CARET) {
                    let resolved_text = ResolvedText::get();
                    let mut caret = resolved_text.caret.lock();
                    let caret = &mut *caret;
                    if let Some(index) = &mut caret.index {
                        *index = txt.shaped_text.snap_caret_line(*index);
                        if let Some(sel) = &mut caret.selection_index {
                            *sel = txt.shaped_text.snap_caret_line(*sel);
                        }
                        let p = txt.shaped_text.caret_origin(*index, resolved_text.text.text());
                        if !caret.used_retained_x {
                            txt.caret_retained_x = p.x;
                        }
                        txt.caret_origin = Some(p);

                        if !mem::take(&mut caret.skip_next_scroll) && SCROLL.try_id().is_some() {
                            let line_height = txt
                                .shaped_text
                                .line(index.line)
                                .map(|l| l.rect().height())
                                .unwrap_or_else(|| txt.shaped_text.line_height());

                            if let Some(p) = txt.render_info.get_mut().transform.transform_point(p) {
                                let p = p - WIDGET.info().inner_bounds().origin;
                                let min_rect = Rect::new(p.to_point(), Size::new(Px(1), line_height * 2 + txt.shaped_text.line_spacing()));
                                SCROLL.scroll_to(ScrollToMode::minimal_rect(min_rect));
                            }
                        }
                    }
                }

                // self.pending is cleared in the node layout, after this method call
            }
            self.txt_is_measured = is_measure;

            metrics.constraints().fill_size_or(txt.shaped_text.size())
        }

        fn ensure_layout_for_render(&mut self) {
            if self.txt_is_measured {
                let metrics = self.last_layout.0.clone();
                self.shaping_args.inline_constraints = self.last_layout.1;
                LAYOUT.with_context(metrics.clone(), || {
                    self.layout(&metrics, &RESOLVED_TEXT.get(), false);
                });

                debug_assert!(!self.txt_is_measured);
            }
        }

        fn with(&mut self, f: impl FnOnce()) {
            if self.txt.is_some() {
                LAYOUT_TEXT.with_context_opt(&mut self.txt, f)
            }
        }
    }

    let mut txt = FinalText {
        txt: None,
        shaping_args: TextShapingArgs::default(),
        pending: PendingLayout::empty(),
        txt_is_measured: false,
        last_layout: (LayoutMetrics::new(1.fct(), PxSize::zero(), Px(0)), None),
    };

    /// Data allocated only when `editable`.
    #[derive(Default)]
    struct EditData {
        events: [EventHandle; 3],
        caret_animation: VarHandle,
        select: CommandHandle,
        select_all: CommandHandle,
    }
    impl EditData {
        fn get(edit_data: &mut Option<Box<Self>>) -> &mut Self {
            &mut *edit_data.get_or_insert_with(Default::default)
        }

        fn subscribe(&mut self) {
            let selectable = TEXT_SELECTABLE_VAR.get();

            if selectable || TEXT_EDITABLE_VAR.get() {
                let id = WIDGET.id();

                self.events[0] = MOUSE_INPUT_EVENT.subscribe(id);
                self.events[1] = TOUCH_TAP_EVENT.subscribe(id);
                self.events[2] = TOUCH_LONG_PRESS_EVENT.subscribe(id);
                // KEY_INPUT_EVENT subscribed by `resolve_text`.
            }

            if selectable {
                let id = WIDGET.id();

                self.select = SELECT_CMD.scoped(id).subscribe(true);
                self.select_all = SELECT_ALL_CMD.scoped(id).subscribe(true);
            }
        }
    }
    // Use `EditData::get` to access.
    let mut edit_data = None;

    // Used by selection by pointer (mouse or touch)
    let mut selection_move_handles = EventHandles::dummy();
    struct SelectionMouseDown {
        position: DipPoint,
        timestamp: Instant,
        count: u8,
    }
    let mut selection_mouse_down = None::<SelectionMouseDown>;
    let mut click_count = 0;

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&FONT_SIZE_VAR)
                .sub_var(&FONT_VARIATIONS_VAR)
                .sub_var(&LETTER_SPACING_VAR)
                .sub_var(&WORD_SPACING_VAR)
                .sub_var(&LINE_SPACING_VAR)
                .sub_var(&LINE_HEIGHT_VAR)
                .sub_var(&TAB_LENGTH_VAR);
            WIDGET
                .sub_var(&UNDERLINE_POSITION_VAR)
                .sub_var(&UNDERLINE_SKIP_VAR)
                .sub_var_layout(&OVERLINE_THICKNESS_VAR)
                .sub_var_layout(&STRIKETHROUGH_THICKNESS_VAR)
                .sub_var_layout(&UNDERLINE_THICKNESS_VAR);
            WIDGET
                .sub_var(&LINE_BREAK_VAR)
                .sub_var(&WORD_BREAK_VAR)
                .sub_var(&HYPHENS_VAR)
                .sub_var(&HYPHEN_CHAR_VAR)
                .sub_var(&TEXT_WRAP_VAR)
                .sub_var(&TEXT_OVERFLOW_VAR);
            WIDGET.sub_var_layout(&TEXT_ALIGN_VAR).sub_var_layout(&TEXT_OVERFLOW_ALIGN_VAR);

            WIDGET.sub_var(&FONT_FEATURES_VAR);
            // LANG_VAR already subscribed by `resolve_text`.

            txt.shaping_args.lang = LANG_VAR.with(|l| l.best().clone());
            txt.shaping_args.direction = txt.shaping_args.lang.character_direction().into();
            txt.shaping_args.line_break = LINE_BREAK_VAR.get();
            txt.shaping_args.word_break = WORD_BREAK_VAR.get();
            txt.shaping_args.hyphens = HYPHENS_VAR.get();
            txt.shaping_args.hyphen_char = HYPHEN_CHAR_VAR.get();
            txt.shaping_args.font_features = FONT_FEATURES_VAR.with(|f| f.finalize());

            if TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get() {
                EditData::get(&mut edit_data).subscribe();
            }

            // txt.txt not available yet.
            // txt.with(|| child.init());
        }
        UiNodeOp::Deinit => {
            txt.with(|| child.deinit());
            txt.txt = None;
            edit_data = None;
        }
        UiNodeOp::Event { update } => {
            let editable = TEXT_EDITABLE_VAR.get();
            let selectable = TEXT_SELECTABLE_VAR.get();
            if (editable || selectable) && WIDGET.info().interactivity().is_enabled() && txt.txt.is_some() {
                let resolved = RESOLVED_TEXT.get();
                let prev_caret_index = {
                    let caret = resolved.caret.lock();
                    (caret.index, caret.index_version, caret.selection_index)
                };

                if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                    if let KeyState::Pressed = args.state {
                        match &args.key {
                            Key::Tab => {
                                if editable && args.modifiers.is_empty() && ACCEPTS_TAB_VAR.get() {
                                    args.propagation().stop();
                                }
                            }
                            Key::Enter => {
                                if editable && args.modifiers.is_empty() && ACCEPTS_ENTER_VAR.get() {
                                    args.propagation().stop();
                                }
                            }
                            Key::ArrowRight => {
                                let mut modifiers = args.modifiers;
                                let select = selectable && modifiers.take_shift();
                                let word = modifiers.take_ctrl();
                                if modifiers.is_empty() && (editable || select) {
                                    args.propagation().stop();

                                    LayoutText::call_select_op(&mut txt.txt, || {
                                        if select {
                                            if word {
                                                TextSelectOp::select_next_word()
                                            } else {
                                                TextSelectOp::select_next()
                                            }
                                        } else if word {
                                            TextSelectOp::next_word()
                                        } else {
                                            TextSelectOp::next()
                                        }
                                        .call();
                                    });
                                }
                            }
                            Key::ArrowLeft => {
                                let mut modifiers = args.modifiers;
                                let select = selectable && modifiers.take_shift();
                                let word = modifiers.take_ctrl();
                                if modifiers.is_empty() && (editable || select) {
                                    args.propagation().stop();

                                    LayoutText::call_select_op(&mut txt.txt, || {
                                        if select {
                                            if word {
                                                TextSelectOp::select_prev_word()
                                            } else {
                                                TextSelectOp::select_prev()
                                            }
                                        } else if word {
                                            TextSelectOp::prev_word()
                                        } else {
                                            TextSelectOp::prev()
                                        }
                                        .call();
                                    });
                                }
                            }
                            Key::ArrowUp => {
                                if ACCEPTS_ENTER_VAR.get() || txt.txt.as_ref().unwrap().shaped_text.lines_len() > 1 {
                                    let mut modifiers = args.modifiers;
                                    let select = selectable && modifiers.take_shift();
                                    if modifiers.is_empty() && (editable || select) {
                                        args.propagation().stop();

                                        LayoutText::call_select_op(&mut txt.txt, || {
                                            if select {
                                                TextSelectOp::select_line_up()
                                            } else {
                                                TextSelectOp::line_up()
                                            }
                                            .call();
                                        });
                                    }
                                }
                            }
                            Key::ArrowDown => {
                                if ACCEPTS_ENTER_VAR.get() || txt.txt.as_ref().unwrap().shaped_text.lines_len() > 1 {
                                    let mut modifiers = args.modifiers;
                                    let select = selectable && modifiers.take_shift();
                                    if modifiers.is_empty() && (editable || select) {
                                        args.propagation().stop();

                                        LayoutText::call_select_op(&mut txt.txt, || {
                                            if select {
                                                TextSelectOp::select_line_down()
                                            } else {
                                                TextSelectOp::line_down()
                                            }
                                            .call();
                                        });
                                    }
                                }
                            }
                            Key::PageUp => {
                                if ACCEPTS_ENTER_VAR.get() || txt.txt.as_ref().unwrap().shaped_text.lines_len() > 1 {
                                    let mut modifiers = args.modifiers;
                                    let select = selectable && modifiers.take_shift();
                                    if modifiers.is_empty() && (editable || select) {
                                        args.propagation().stop();

                                        LayoutText::call_select_op(&mut txt.txt, || {
                                            if select {
                                                TextSelectOp::select_page_up()
                                            } else {
                                                TextSelectOp::page_up()
                                            }
                                            .call();
                                        });
                                    }
                                }
                            }
                            Key::PageDown => {
                                if ACCEPTS_ENTER_VAR.get() || txt.txt.as_ref().unwrap().shaped_text.lines_len() > 1 {
                                    let mut modifiers = args.modifiers;
                                    let select = selectable && modifiers.take_shift();
                                    if modifiers.is_empty() && (editable || select) {
                                        args.propagation().stop();

                                        LayoutText::call_select_op(&mut txt.txt, || {
                                            if select {
                                                TextSelectOp::select_page_down()
                                            } else {
                                                TextSelectOp::page_down()
                                            }
                                            .call();
                                        });
                                    }
                                }
                            }
                            Key::Home => {
                                let mut modifiers = args.modifiers;
                                let select = selectable && modifiers.take_shift();
                                let full_text = modifiers.take_ctrl();
                                if modifiers.is_empty() && (editable || select) {
                                    args.propagation().stop();

                                    LayoutText::call_select_op(&mut txt.txt, || {
                                        if select {
                                            if full_text {
                                                TextSelectOp::select_text_start()
                                            } else {
                                                TextSelectOp::select_line_start()
                                            }
                                        } else if full_text {
                                            TextSelectOp::text_start()
                                        } else {
                                            TextSelectOp::line_start()
                                        }
                                        .call();
                                    });
                                }
                            }
                            Key::End => {
                                let mut modifiers = args.modifiers;
                                let select = selectable && modifiers.take_shift();
                                let full_text = modifiers.take_ctrl();
                                if modifiers.is_empty() && (editable || select) {
                                    args.propagation().stop();

                                    LayoutText::call_select_op(&mut txt.txt, || {
                                        if select {
                                            if full_text {
                                                TextSelectOp::select_text_end()
                                            } else {
                                                TextSelectOp::select_line_end()
                                            }
                                        } else if full_text {
                                            TextSelectOp::text_end()
                                        } else {
                                            TextSelectOp::line_end()
                                        }
                                        .call();
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                } else if let Some(args) = MOUSE_INPUT_EVENT.on_unhandled(update) {
                    if args.is_primary() && args.is_mouse_down() {
                        let mut modifiers = args.modifiers;
                        let select = selectable && modifiers.take_shift();

                        if modifiers.is_empty() {
                            args.propagation().stop();

                            click_count = if let Some(info) = &mut selection_mouse_down {
                                let cfg = MOUSE.multi_click_config().get();

                                let double_allowed = args.timestamp.duration_since(info.timestamp) <= cfg.time && {
                                    let dist = (info.position.to_vector() - args.position.to_vector()).abs();
                                    let area = cfg.area;
                                    dist.x <= area.width && dist.y <= area.height
                                };

                                if double_allowed {
                                    info.timestamp = args.timestamp;
                                    info.count += 1;
                                    info.count = info.count.min(4);
                                } else {
                                    *info = SelectionMouseDown {
                                        position: args.position,
                                        timestamp: args.timestamp,
                                        count: 1,
                                    };
                                }

                                info.count
                            } else {
                                selection_mouse_down = Some(SelectionMouseDown {
                                    position: args.position,
                                    timestamp: args.timestamp,
                                    count: 1,
                                });
                                1
                            };

                            LayoutText::call_select_op(&mut txt.txt, || {
                                match click_count {
                                    1 => if select {
                                        TextSelectOp::select_nearest_to(args.position)
                                    } else {
                                        TextSelectOp::nearest_to(args.position)
                                    }
                                    .call(),
                                    2 => {
                                        if selectable {
                                            TextSelectOp::select_word_nearest_to(!select, args.position).call()
                                        }
                                    }
                                    3 => {
                                        if selectable {
                                            TextSelectOp::select_line_nearest_to(!select, args.position).call()
                                        }
                                    }
                                    4 => {
                                        if selectable {
                                            TextSelectOp::select_all().call()
                                        }
                                    }
                                    _ => unreachable!(),
                                };
                            });

                            if selectable {
                                let id = WIDGET.id();
                                selection_move_handles.push(MOUSE_MOVE_EVENT.subscribe(id));
                                selection_move_handles.push(POINTER_CAPTURE_EVENT.subscribe(id));
                                POINTER_CAPTURE.capture_widget(id);
                            }
                        }
                    } else {
                        selection_move_handles.clear();
                    }
                } else if let Some(args) = TOUCH_TAP_EVENT.on_unhandled(update) {
                    if args.modifiers.is_empty() {
                        args.propagation().stop();

                        LayoutText::call_select_op(&mut txt.txt, || {
                            TextSelectOp::nearest_to(args.position).call();
                        });
                    }
                } else if let Some(args) = TOUCH_LONG_PRESS_EVENT.on_unhandled(update) {
                    if args.modifiers.is_empty() && selectable {
                        args.propagation().stop();

                        LayoutText::call_select_op(&mut txt.txt, || {
                            TextSelectOp::select_word_nearest_to(true, args.position).call();
                        });
                    }
                } else if let Some(args) = MOUSE_MOVE_EVENT.on(update) {
                    if !selection_move_handles.is_dummy() && selectable {
                        args.propagation().stop();

                        LayoutText::call_select_op(&mut txt.txt, || match click_count {
                            1 => TextSelectOp::select_nearest_to(args.position).call(),
                            2 => TextSelectOp::select_word_nearest_to(false, args.position).call(),
                            3 => TextSelectOp::select_line_nearest_to(false, args.position).call(),
                            4 => {}
                            _ => unreachable!(),
                        });
                    }
                } else if let Some(args) = POINTER_CAPTURE_EVENT.on(update) {
                    if args.is_lost(WIDGET.id()) {
                        selection_move_handles.clear();
                    }
                } else if selectable {
                    if let Some(args) = SELECT_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                        if let Some(op) = args.param::<TextSelectOp>() {
                            args.propagation().stop();

                            LayoutText::call_select_op(&mut txt.txt, || op.clone().call());
                        }
                    } else if let Some(args) = SELECT_ALL_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                        args.propagation().stop();
                        LayoutText::call_select_op(&mut txt.txt, || TextSelectOp::select_all().call());
                    }
                }

                let mut caret = resolved.caret.lock();
                if (caret.index, caret.index_version, caret.selection_index) != prev_caret_index {
                    if !editable || caret.index.is_none() || !FOCUS.is_focused(WIDGET.id()).get() {
                        EditData::get(&mut edit_data).caret_animation = VarHandle::dummy();
                        caret.opacity = var(0.fct()).read_only();
                    } else {
                        caret.opacity = KEYBOARD.caret_animation();
                        EditData::get(&mut edit_data).caret_animation = caret.opacity.subscribe(UpdateOp::RenderUpdate, WIDGET.id());
                    }
                    txt.pending |= PendingLayout::CARET;
                    WIDGET.layout(); // update caret_origin
                }
            }

            txt.with(|| child.event(update));
        }
        UiNodeOp::Update { updates } => {
            if FONT_SIZE_VAR.is_new() || FONT_VARIATIONS_VAR.is_new() {
                txt.pending.insert(PendingLayout::RESHAPE);
                if let Some(t) = &mut txt.txt {
                    t.overflow_suffix = None;
                }
                WIDGET.layout();
            }

            if LETTER_SPACING_VAR.is_new()
                || WORD_SPACING_VAR.is_new()
                || LINE_SPACING_VAR.is_new()
                || LINE_HEIGHT_VAR.is_new()
                || TAB_LENGTH_VAR.is_new()
                || LANG_VAR.is_new()
            {
                txt.shaping_args.lang = LANG_VAR.with(|l| l.best().clone());
                txt.shaping_args.direction = txt.shaping_args.lang.character_direction().into(); // will be set in layout too.
                if let Some(t) = &mut txt.txt {
                    t.overflow_suffix = None;
                }
                txt.pending.insert(PendingLayout::RESHAPE);
                WIDGET.layout();
            }

            if UNDERLINE_POSITION_VAR.is_new() || UNDERLINE_SKIP_VAR.is_new() {
                txt.pending.insert(PendingLayout::UNDERLINE);
                WIDGET.layout();
            }

            if let Some(lb) = LINE_BREAK_VAR.get_new() {
                if txt.shaping_args.line_break != lb {
                    txt.shaping_args.line_break = lb;
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(wb) = WORD_BREAK_VAR.get_new() {
                if txt.shaping_args.word_break != wb {
                    txt.shaping_args.word_break = wb;
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(h) = HYPHENS_VAR.get_new() {
                if txt.shaping_args.hyphens != h {
                    txt.shaping_args.hyphens = h;
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if let Some(c) = HYPHEN_CHAR_VAR.get_new() {
                txt.shaping_args.hyphen_char = c;
                if Hyphens::None != txt.shaping_args.hyphens {
                    txt.pending.insert(PendingLayout::RESHAPE);
                    WIDGET.layout();
                }
            }
            if TEXT_WRAP_VAR.is_new() {
                txt.pending.insert(PendingLayout::RESHAPE);
                WIDGET.layout();
            }
            if TEXT_OVERFLOW_VAR.is_new() {
                if let Some(t) = &mut txt.txt {
                    t.overflow_suffix = None;
                }
                txt.pending.insert(PendingLayout::OVERFLOW);
                WIDGET.layout();
            }

            FONT_FEATURES_VAR.with_new(|f| {
                txt.shaping_args.font_features = f.finalize();
                txt.pending.insert(PendingLayout::RESHAPE);
                WIDGET.layout();
            });

            if TEXT_EDITABLE_VAR.is_new() || TEXT_SELECTABLE_VAR.is_new() {
                edit_data = None;

                if TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get() {
                    EditData::get(&mut edit_data).subscribe();
                }
            }

            if FONT_FAMILY_VAR.is_new()
                || FONT_STYLE_VAR.is_new()
                || FONT_STRETCH_VAR.is_new()
                || FONT_WEIGHT_VAR.is_new()
                || LANG_VAR.is_new()
            {
                // resolve_text already requests RESHAPE

                if let Some(t) = &mut txt.txt {
                    t.overflow_suffix = None;
                }
            }

            txt.with(|| child.update(updates));
        }
        UiNodeOp::Measure { wm, desired_size } => {
            child.delegated();

            let metrics = LAYOUT.metrics();

            *desired_size = if let Some(size) = txt.measure(&metrics) {
                size
            } else {
                let size = txt.layout(&metrics, &RESOLVED_TEXT.get(), true);

                if let (Some(inline), Some(l)) = (wm.inline(), txt.txt.as_ref()) {
                    if let Some(first_line) = l.shaped_text.line(0) {
                        inline.first = first_line.original_size();
                        inline.with_first_segs(|i| {
                            for seg in first_line.segs() {
                                i.push(InlineSegment {
                                    width: seg.advance(),
                                    kind: seg.kind(),
                                });
                            }
                        });
                    } else {
                        inline.first = PxSize::zero();
                        inline.with_first_segs(|i| i.clear());
                    }

                    if l.shaped_text.lines_len() == 1 {
                        inline.last = inline.first;
                        inline.last_segs = inline.first_segs.clone();
                    } else if let Some(last_line) = l.shaped_text.line(l.shaped_text.lines_len().saturating_sub(1)) {
                        inline.last = last_line.original_size();
                        inline.with_last_segs(|i| {
                            for seg in last_line.segs() {
                                i.push(InlineSegment {
                                    width: seg.advance(),
                                    kind: seg.kind(),
                                })
                            }
                        })
                    } else {
                        inline.last = PxSize::zero();
                        inline.with_last_segs(|i| i.clear());
                    }

                    inline.first_wrapped = l.shaped_text.first_wrapped();
                    inline.last_wrapped = l.shaped_text.lines_len() > 1;
                }
                size
            };

            LAYOUT.with_constraints(metrics.constraints().with_new_min_size(*desired_size), || {
                // foreign nodes in the CHILD_LAYOUT+100 ..= CHILD range may change the size
                txt.with(|| *desired_size = child.measure(wm))
            });
        }
        UiNodeOp::Layout { wl, final_size } => {
            child.delegated();

            let metrics = LAYOUT.metrics();

            if let Some(l) = &mut txt.txt {
                l.viewport = metrics.viewport();
            }

            let resolved_txt = RESOLVED_TEXT.get();
            *final_size = txt.layout(&metrics, &resolved_txt, false);

            if txt.pending != PendingLayout::empty() {
                WIDGET.render();
                txt.pending = PendingLayout::empty();
            }

            if let (Some(inline), Some(l)) = (wl.inline(), txt.txt.as_ref()) {
                let last_line = l.shaped_text.lines_len().saturating_sub(1);

                inline.first_segs.clear();
                inline.last_segs.clear();

                for (i, line) in l.shaped_text.lines().enumerate() {
                    if i == 0 {
                        let info = l.shaped_text.line(0).unwrap().segs().map(|s| s.inline_info());
                        if LAYOUT.direction().is_rtl() {
                            // help sort
                            inline.set_first_segs(info.rev());
                        } else {
                            inline.set_first_segs(info);
                        }
                    } else if i == last_line {
                        let info = l
                            .shaped_text
                            .line(l.shaped_text.lines_len().saturating_sub(1))
                            .unwrap()
                            .segs()
                            .map(|s| s.inline_info());
                        if LAYOUT.direction().is_rtl() {
                            // help sort
                            inline.set_last_segs(info.rev());
                        } else {
                            inline.set_last_segs(info);
                        }
                    }

                    inline.rows.push(line.rect());
                }
            }

            let baseline = resolved_txt.baseline.load(Ordering::Relaxed);
            wl.set_baseline(baseline);

            LAYOUT.with_constraints(metrics.constraints().with_new_min_size(*final_size), || {
                // foreign nodes in the CHILD_LAYOUT+100 ..= CHILD range may change the size
                txt.with(|| *final_size = child.layout(wl))
            });
        }
        UiNodeOp::Render { frame } => {
            txt.ensure_layout_for_render();
            txt.with(|| child.render(frame))
        }
        UiNodeOp::RenderUpdate { update } => {
            txt.ensure_layout_for_render();
            txt.with(|| child.render_update(update))
        }
        op => txt.with(|| child.op(op)),
    })
}

/// An Ui node that renders the default underline visual using the parent [`LayoutText`].
///
/// The lines are rendered before `child`, under it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_strikethroughs`] node.
pub fn render_underlines(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&UNDERLINE_STYLE_VAR).sub_var_render(&UNDERLINE_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = LayoutText::get();

            if !t.underlines.is_empty() {
                let style = UNDERLINE_STYLE_VAR.get();
                if style != LineStyle::Hidden {
                    let color = UNDERLINE_COLOR_VAR.get().into();
                    for &(origin, width) in &t.underlines {
                        frame.push_line(
                            PxRect::new(origin, PxSize::new(width, t.underline_thickness)),
                            LineOrientation::Horizontal,
                            color,
                            style,
                        );
                    }
                }
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the default strikethrough visual using the parent [`LayoutText`].
///
/// The lines are rendered after `child`, over it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_overlines`] node.
pub fn render_strikethroughs(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render(&STRIKETHROUGH_STYLE_VAR)
                .sub_var_render(&STRIKETHROUGH_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = LayoutText::get();
            if !t.strikethroughs.is_empty() {
                let style = STRIKETHROUGH_STYLE_VAR.get();
                if style != LineStyle::Hidden {
                    let color = STRIKETHROUGH_COLOR_VAR.get().into();
                    for &(origin, width) in &t.strikethroughs {
                        frame.push_line(
                            PxRect::new(origin, PxSize::new(width, t.strikethrough_thickness)),
                            LineOrientation::Horizontal,
                            color,
                            style,
                        );
                    }
                }
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the default overline visual using the parent [`LayoutText`].
///
/// The lines are rendered before `child`, under it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
pub fn render_overlines(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&OVERLINE_STYLE_VAR).sub_var_render(&OVERLINE_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = LayoutText::get();
            if !t.overlines.is_empty() {
                let style = OVERLINE_STYLE_VAR.get();
                if style != LineStyle::Hidden {
                    let color = OVERLINE_COLOR_VAR.get().into();
                    for &(origin, width) in &t.overlines {
                        frame.push_line(
                            PxRect::new(origin, PxSize::new(width, t.overline_thickness)),
                            LineOrientation::Horizontal,
                            color,
                            style,
                        );
                    }
                }
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the edit caret visual.
///
/// The caret is rendered after `child`, over it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
pub fn render_caret(child: impl UiNode) -> impl UiNode {
    let color_key = FrameValueKey::new_unique();

    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render_update(&CARET_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            child.render(frame);

            if TEXT_EDITABLE_VAR.get() {
                let t = LayoutText::get();

                if let Some(mut origin) = t.caret_origin {
                    let mut c = CARET_COLOR_VAR.get();
                    c.alpha = ResolvedText::get().caret.lock().opacity.get().0;

                    let caret_thickness = Dip::new(1).to_px(frame.scale_factor().0);
                    origin.x -= caret_thickness / 2;

                    let clip_rect = PxRect::new(origin, PxSize::new(caret_thickness, t.shaped_text.line_height()));
                    frame.push_color(clip_rect, color_key.bind(c.into(), true));
                }
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            child.render_update(update);

            let mut c = CARET_COLOR_VAR.get();
            c.alpha = ResolvedText::get().caret.lock().opacity.get().0;

            if TEXT_EDITABLE_VAR.get() {
                update.update_color(color_key.update(c.into(), true))
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the touch carets and implement interaction.
///
/// Caret visuals defined by [`CARET_TOUCH_SHAPE_VAR`].
pub fn touch_carets(child: impl UiNode) -> impl UiNode {
    // is [child] or [child, SelectionLeft, SelectionRight] or [child, Insert]
    let children = vec![child.boxed()];

    let mut sizes = [PxSize::zero(); 2];

    match_node_list(children, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&CARET_TOUCH_SHAPE_VAR);
        }
        UiNodeOp::Deinit => {
            c.deinit_all();
            c.children().truncate(1);
        }
        UiNodeOp::Update { .. } => {
            if CARET_TOUCH_SHAPE_VAR.is_new() {
                for caret in &mut c.children()[1..] {
                    caret.deinit();
                }
                c.children().truncate(1);

                WIDGET.layout();
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            let r_txt = ResolvedText::get();
            c.delegated();

            let children = c.children();
            *final_size = children[0].layout(wl);

            let caret = r_txt.caret.lock();
            let mut expected_len = 1; // 1 child
            if caret.index.is_some() && FOCUS.focused().with(|p| matches!(p, Some(p) if p.widget_id() == WIDGET.id())) {
                if caret.selection_index.is_some() {
                    expected_len = 3;
                } else {
                    expected_len = 2;
                }
            }

            if expected_len != children.len() {
                // children.len changed OR caret shape changed

                for caret in &mut children[1..] {
                    caret.deinit();
                }
                children.truncate(1);

                let shape = CARET_TOUCH_SHAPE_VAR.get();
                if expected_len == 2 {
                    children.push(shape(CaretShape::Insert));
                } else if expected_len == 3 {
                    children.push(shape(CaretShape::SelectionLeft));
                    children.push(shape(CaretShape::SelectionRight));
                }

                for caret in &mut children[1..] {
                    caret.init();
                }

                WIDGET.render();
            }

            for (caret, size) in children[1..].iter_mut().zip(&mut sizes) {
                *size = caret.layout(wl);
            }
        }
        UiNodeOp::Render { frame } => {
            c.delegated();

            let children = c.children();
            children[0].render(frame);
            if children.len() == 2 {
                let t = LayoutText::get();

                if let Some(mut origin) = t.caret_origin {
                    let id = WIDGET.id();
                    origin.x -= sizes[0].width / 2;
                    frame.push_reference_frame(
                        SpatialFrameKey::from_widget_child(id, 1),
                        FrameValue::Value(origin.to_vector().into()),
                        true,
                        true,
                        |frame| children[1].render(frame),
                    )
                }
            } else if children.len() == 3 {
                let t = LayoutText::get();
                let r_txt = ResolvedText::get();
                let caret = r_txt.caret.lock();

                if let (Some(index), Some(s_index), Some(mut origin)) = (caret.index, caret.selection_index, t.caret_origin) {
                    // !!: TODO right-to-left text
                    let index_is_left = index.index <= s_index.index;

                    let half_caret = Dip::new(1).to_px(frame.scale_factor().0) / 2; // !!: TODO position in shape, not here;

                    let id = WIDGET.id();
                    if index_is_left {
                        origin.x -= sizes[0].width - half_caret;
                    } else {
                        origin.x -= half_caret;
                    }
                    frame.push_reference_frame(
                        SpatialFrameKey::from_widget_child(id, 1),
                        FrameValue::Value(origin.to_vector().into()),
                        true,
                        true,
                        |frame| children[if index_is_left { 1 } else { 2 }].render(frame),
                    )
                    // !!: other end of the selection
                } else {
                    tracing::error!("touch caret instances do not match context caret")
                }
            }
        }
        _ => {}
    })
}

/// Default touch caret shape.
pub fn default_touch_caret(shape: CaretShape) -> impl UiNode {
    use zero_ui::prelude::new_property::*;
    match_node_leaf(move |op| match op {
        UiNodeOp::Layout { final_size, .. } => {
            let size = Dip::new(16).to_px(LAYOUT.scale_factor().0);
            *final_size = PxSize::splat(size);
            final_size.height += LayoutText::get().shaped_text.line_height();
            if !matches!(shape, CaretShape::Insert) {
                final_size.width *= 0.8;
            }
        }
        UiNodeOp::Render { frame } => {
            let size = Dip::new(16).to_px(frame.scale_factor().0);
            let mut size = PxSize::splat(size);

            let corners = match shape {
                CaretShape::SelectionLeft => PxCornerRadius::new(size, PxSize::zero(), PxSize::zero(), size),
                CaretShape::Insert => PxCornerRadius::new_all(size),
                CaretShape::SelectionRight => PxCornerRadius::new(PxSize::zero(), size, size, PxSize::zero()),
            };

            if !matches!(shape, CaretShape::Insert) {
                size.width *= 0.8;
            }

            let line_height = LayoutText::get().shaped_text.line_height();

            let rect = PxRect::new(PxPoint::new(Px(0), line_height), size);
            frame.push_clip_rounded_rect(rect, corners, false, false, |frame| {
                frame.push_color(rect, FrameValue::Value(colors::AZURE.into()));
            });

            let caret_thickness = Dip::new(1).to_px(frame.scale_factor().0);
            let line_pos = match shape {
                CaretShape::SelectionLeft => PxPoint::new(size.width - caret_thickness, Px(0)),
                CaretShape::Insert => PxPoint::new((size.width - caret_thickness) / Px(2), Px(0)),
                CaretShape::SelectionRight => PxPoint::zero(),
            };

            let rect = PxRect::new(line_pos, PxSize::new(caret_thickness, line_height));
            frame.push_color(rect, FrameValue::Value(colors::AZURE.into()));
        }
        _ => {}
    })
}

/// An Ui node that renders the text selection background.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
pub fn render_selection(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&SELECTION_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let r_txt = ResolvedText::get();

            if let Some(range) = r_txt.caret.lock().selection_range() {
                let l_txt = LayoutText::get();
                let r_txt = r_txt.text.text();

                for line_rect in l_txt.shaped_text.highlight_rects(range, r_txt) {
                    frame.push_color(line_rect, FrameValue::Value(SELECTION_COLOR_VAR.get().into()));
                }
            };
        }
        _ => {}
    })
}

/// An UI node that renders the parent [`LayoutText`].
///
/// This node renders the text only, decorators are rendered by other nodes.
///
/// This is the `Text!` widget inner most child node.
pub fn render_text() -> impl UiNode {
    #[derive(Clone, Copy, PartialEq)]
    struct RenderedText {
        version: u32,
        synthesis: FontSynthesis,
        color: Rgba,
        aa: FontAntiAliasing,
    }

    let mut reuse = None;
    let mut rendered = None;
    let mut color_key = None;

    match_node_leaf(move |op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_render_update(&FONT_COLOR_VAR)
                .sub_var_render(&FONT_AA_VAR)
                .sub_var(&FONT_PALETTE_VAR)
                .sub_var(&FONT_PALETTE_COLORS_VAR);

            if FONT_COLOR_VAR.capabilities().contains(VarCapabilities::NEW) {
                color_key = Some(FrameValueKey::new_unique());
            }
        }
        UiNodeOp::Deinit => {
            color_key = None;
            reuse = None;
            rendered = None;
        }
        UiNodeOp::Update { .. } => {
            if (FONT_PALETTE_VAR.is_new() || FONT_PALETTE_COLORS_VAR.is_new()) && LayoutText::in_context() {
                let t = LayoutText::get();
                if t.shaped_text.has_colored_glyphs() {
                    WIDGET.render();
                }
            }
        }
        UiNodeOp::Measure { desired_size, .. } => {
            let txt = LayoutText::get();
            *desired_size = LAYOUT.constraints().fill_size_or(txt.shaped_text.size())
        }
        UiNodeOp::Layout { final_size, .. } => {
            // layout implemented in `layout_text`, it sets the size as an exact size constraint, we return
            // the size here for foreign nodes in the CHILD_LAYOUT+100 ..= CHILD range.
            let txt = LayoutText::get();
            *final_size = LAYOUT.constraints().fill_size_or(txt.shaped_text.size())
        }
        UiNodeOp::Render { frame } => {
            let r = ResolvedText::get();
            let t = LayoutText::get();

            let lh = t.shaped_text.line_height();
            let clip = PxRect::from_size(t.shaped_text.align_size()).inflate(lh, lh); // clip inflated to allow some weird glyphs
            let color = FONT_COLOR_VAR.get();
            let color_value = if let Some(key) = color_key {
                key.bind(color.into(), FONT_COLOR_VAR.is_animating())
            } else {
                FrameValue::Value(color.into())
            };

            let aa = FONT_AA_VAR.get();

            let rt = Some(RenderedText {
                version: t.shaped_text_version,
                synthesis: r.synthesis,
                color,
                aa,
            });
            if rendered != rt {
                rendered = rt;
                reuse = None;
            }

            {
                let mut info = t.render_info.lock();
                info.transform = *frame.transform();
                info.scale_factor = frame.scale_factor();
            }

            frame.push_reuse(&mut reuse, |frame| {
                if t.shaped_text.has_colored_glyphs() || t.overflow_suffix.as_ref().map(|o| o.has_colored_glyphs()).unwrap_or(false) {
                    let palette_query = FONT_PALETTE_VAR.get();
                    FONT_PALETTE_COLORS_VAR.with(|palette_colors| {
                        let mut push_font_glyphs = |font: &crate::core::text::Font, glyphs, offset: Option<euclid::Vector2D<f32, Px>>| {
                            let mut palette = None;

                            match glyphs {
                                ShapedColoredGlyphs::Normal(glyphs) => {
                                    if let Some(offset) = offset {
                                        let mut glyphs = glyphs.to_vec();
                                        for g in &mut glyphs {
                                            g.point.x += offset.x;
                                            g.point.y += offset.y;
                                        }
                                        frame.push_text(clip, &glyphs, font, color_value, r.synthesis, aa);
                                    } else {
                                        frame.push_text(clip, glyphs, font, color_value, r.synthesis, aa);
                                    }
                                }
                                ShapedColoredGlyphs::Colored { point, glyphs, .. } => {
                                    for (index, color_i) in glyphs.iter() {
                                        let color = if let Some(color_i) = color_i {
                                            if let Some(i) = palette_colors.iter().position(|(ci, _)| *ci == color_i as u16) {
                                                palette_colors[i].1
                                            } else {
                                                // FontFace only parses colored glyphs if the font has at least one
                                                // palette, so it is safe to unwrap here
                                                let palette = palette
                                                    .get_or_insert_with(|| font.face().color_palettes().palette(palette_query).unwrap());

                                                // the font could have a bug and return an invalid palette index
                                                palette.colors.get(color_i).copied().unwrap_or(color)
                                            }
                                        } else {
                                            // color_i is None, meaning the base color.
                                            color
                                        };

                                        let mut g = GlyphInstance { point, index };
                                        if let Some(offset) = offset {
                                            g.point.x += offset.x;
                                            g.point.y += offset.y;
                                        }
                                        frame.push_text(clip, &[g], font, FrameValue::Value(color.into()), r.synthesis, aa);
                                    }
                                }
                            }
                        };

                        match (&t.overflow, TEXT_OVERFLOW_VAR.get(), TEXT_EDITABLE_VAR.get()) {
                            (Some(o), TextOverflow::Truncate(_), false) => {
                                for glyphs in &o.included_glyphs {
                                    for (font, glyphs) in t.shaped_text.colored_glyphs_slice(glyphs.clone()) {
                                        push_font_glyphs(font, glyphs, None)
                                    }
                                }

                                if let Some(suf) = &t.overflow_suffix {
                                    let suf_offset = o.suffix_origin.to_vector().cast_unit();
                                    for (font, glyphs) in suf.colored_glyphs() {
                                        push_font_glyphs(font, glyphs, Some(suf_offset))
                                    }
                                }
                            }
                            _ => {
                                // no overflow truncating
                                for (font, glyphs) in t.shaped_text.colored_glyphs() {
                                    push_font_glyphs(font, glyphs, None)
                                }
                            }
                        }
                    });
                } else {
                    // no colored glyphs

                    let mut push_font_glyphs = |font: &crate::core::text::Font, glyphs: Cow<[GlyphInstance]>| {
                        frame.push_text(clip, glyphs.as_ref(), font, color_value, r.synthesis, aa);
                    };

                    match (&t.overflow, TEXT_OVERFLOW_VAR.get(), !TEXT_EDITABLE_VAR.get()) {
                        (Some(o), TextOverflow::Truncate(_), false) => {
                            for glyphs in &o.included_glyphs {
                                for (font, glyphs) in t.shaped_text.glyphs_slice(glyphs.clone()) {
                                    push_font_glyphs(font, Cow::Borrowed(glyphs))
                                }
                            }

                            if let Some(suf) = &t.overflow_suffix {
                                let suf_offset = o.suffix_origin.to_vector().cast_unit();
                                for (font, glyphs) in suf.glyphs() {
                                    let mut glyphs = glyphs.to_vec();
                                    for g in &mut glyphs {
                                        g.point += suf_offset;
                                    }
                                    push_font_glyphs(font, Cow::Owned(glyphs))
                                }
                            }
                        }
                        _ => {
                            // no overflow truncating
                            for (font, glyphs) in t.shaped_text.glyphs() {
                                push_font_glyphs(font, Cow::Borrowed(glyphs))
                            }
                        }
                    }
                }
            });
        }
        UiNodeOp::RenderUpdate { update } => {
            {
                let t = LayoutText::get();
                let mut info = t.render_info.lock();
                info.transform = *update.transform();
            }

            if let Some(key) = color_key {
                let color = FONT_COLOR_VAR.get();

                update.update_color(key.update(color.into(), FONT_COLOR_VAR.is_animating()));

                let mut r = rendered.unwrap();
                r.color = color;
                rendered = Some(r);
            }
        }
        _ => {}
    })
}

/// Create a node that is sized one text line height by `width`.
///
/// This node can be used to reserve space for a full text in lazy loading contexts.
///
/// The contextual variables affect the layout size.
pub fn line_placeholder(width: impl IntoVar<Length>) -> impl UiNode {
    let child = layout_text(FillUiNode);
    let child = resolve_text(child, " ");
    crate::properties::width(child, width)
}

pub(super) fn get_caret_index(child: impl UiNode, index: impl IntoVar<Option<CaretIndex>>) -> impl UiNode {
    let index = index.into_var();
    match_node(child, move |c, op| {
        let mut u = false;
        match op {
            UiNodeOp::Init => {
                c.init();
                let _ = index.set(ResolvedText::get().caret.lock().index);
            }
            UiNodeOp::Deinit => {
                let _ = index.set(None);
            }
            UiNodeOp::Event { update } => {
                c.event(update);
                u = true;
            }
            UiNodeOp::Update { updates } => {
                c.update(updates);
                u = true;
            }
            _ => {}
        }
        if u {
            let t = ResolvedText::get();
            let idx = t.caret.lock().index;
            if !t.pending_edit && index.get() != idx {
                let _ = index.set(idx);
            }
        }
    })
}

pub(super) fn get_caret_status(child: impl UiNode, status: impl IntoVar<CaretStatus>) -> impl UiNode {
    let status = status.into_var();
    match_node(child, move |c, op| {
        let mut u = false;
        match op {
            UiNodeOp::Init => {
                c.init();
                let t = ResolvedText::get();
                let _ = status.set(match t.caret.lock().index {
                    None => CaretStatus::none(),
                    Some(i) => CaretStatus::new(i.index, &t.text),
                });
            }
            UiNodeOp::Deinit => {
                let _ = status.set(CaretStatus::none());
            }
            UiNodeOp::Event { update } => {
                c.event(update);
                u = true;
            }
            UiNodeOp::Update { updates } => {
                c.update(updates);
                u = true;
            }
            _ => {}
        }
        if u {
            let t = ResolvedText::get();
            let idx = t.caret.lock().index;
            if !t.pending_edit && status.get().index() != idx.map(|ci| ci.index) {
                let _ = status.set(match idx {
                    None => CaretStatus::none(),
                    Some(i) => CaretStatus::new(i.index, &t.text),
                });
            }
        }
    })
}

pub(super) fn get_lines_len(child: impl UiNode, len: impl IntoVar<usize>) -> impl UiNode {
    let len = len.into_var();
    match_node(child, move |c, op| match op {
        UiNodeOp::Deinit => {
            let _ = len.set(0usize);
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);
            let t = LayoutText::get();
            let l = t.shaped_text.lines_len();
            if l != len.get() {
                let _ = len.set(t.shaped_text.lines_len());
            }
        }
        _ => {}
    })
}

pub(super) fn get_lines_wrap_count(child: impl UiNode, lines: impl IntoVar<super::LinesWrapCount>) -> impl UiNode {
    let lines = lines.into_var();
    let mut version = 0;
    match_node(child, move |c, op| match op {
        UiNodeOp::Deinit => {
            let _ = lines.set(super::LinesWrapCount::NoWrap(0));
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);
            let t = LayoutText::get();
            if t.shaped_text_version != version {
                version = t.shaped_text_version;
                if let Some(update) = lines.with(|l| lines_wrap_count(l, &t.shaped_text)) {
                    let _ = lines.set(update);
                }
            }
        }
        _ => {}
    })
}
// Returns `Some(_)` if the current wrap count changed from `prev`. Only allocates if new count has wrapped lines.
fn lines_wrap_count(prev: &super::LinesWrapCount, txt: &ShapedText) -> Option<super::LinesWrapCount> {
    match prev {
        super::LinesWrapCount::NoWrap(len) => {
            let mut counter = lines_wrap_counter(txt);
            let mut l = 0;
            for c in &mut counter {
                if c != 1 {
                    // at least one line wraps now
                    let mut wrap = vec![1; l];
                    wrap.push(c);
                    wrap.extend(&mut counter);
                    return Some(super::LinesWrapCount::Wrap(wrap));
                }
                l += 1;
            }
            if l != *len {
                // no line wraps, but changed line count.
                Some(super::LinesWrapCount::NoWrap(l))
            } else {
                None
            }
        }
        super::LinesWrapCount::Wrap(counts) => {
            // find `counts[i]` that diverges from counts, OR
            // find if all new counts is now NoWrap
            let mut prev_counts = counts.iter();
            let mut new_counts = lines_wrap_counter(txt);
            let mut eq_l = 0;
            let mut eq_wrap = false;
            for c in &mut new_counts {
                if prev_counts.next() == Some(&c) {
                    eq_l += 1;
                    eq_wrap |= c != 1;
                } else if eq_wrap || c != 1 {
                    // not eq, and already found a wrap line
                    let mut wrap = counts[..eq_l].to_vec();
                    wrap.push(c);
                    wrap.extend(&mut new_counts);
                    return Some(super::LinesWrapCount::Wrap(wrap));
                } else {
                    // not eq, but maybe no wrap
                    let mut l = eq_l + 1; // +1 is +c
                    for c in &mut new_counts {
                        if c != 1 {
                            // nope, found a line wrap
                            let mut wrap = vec![1; l];
                            wrap.push(c);
                            wrap.extend(&mut new_counts);
                            return Some(super::LinesWrapCount::Wrap(wrap));
                        }
                        l += 1;
                    }
                    // changed to no wrap
                    return Some(super::LinesWrapCount::NoWrap(l));
                }
            }
            if prev_counts.next().is_some() {
                Some(super::LinesWrapCount::Wrap(counts[..eq_l].to_vec()))
            } else {
                None
            }
        }
    }
}
fn lines_wrap_counter(txt: &ShapedText) -> impl Iterator<Item = u32> + '_ {
    struct Counter<I> {
        lines: I,
        count: u32,
    }
    impl<'a, I: Iterator<Item = ShapedLine<'a>>> Iterator for Counter<I> {
        type Item = u32;

        fn next(&mut self) -> Option<u32> {
            loop {
                let line = self.lines.next()?;
                if line.ended_by_wrap() {
                    self.count += 1;
                    continue;
                }

                let c = self.count;
                self.count = 1;

                return Some(c);
            }
        }
    }
    Counter {
        lines: txt.lines(),
        count: 1,
    }
}
