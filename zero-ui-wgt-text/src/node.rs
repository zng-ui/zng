//! UI nodes used for building a text widget.

use std::{
    borrow::Cow,
    fmt, mem, ops,
    sync::{atomic::AtomicBool, Arc},
    time::Instant,
};

use super::{
    cmd::{TextEditOp, TextSelectOp, UndoTextEditOp, EDIT_CMD, SELECT_ALL_CMD, SELECT_CMD},
    text_properties::*,
};
use atomic::{Atomic, Ordering};
use parking_lot::Mutex;
use zero_ui_app::{
    access::{ACCESS_SELECTION_EVENT, ACCESS_TEXT_EVENT},
    render::FontSynthesis,
    widget::info::INTERACTIVITY_CHANGED_EVENT,
};
use zero_ui_ext_clipboard::{CLIPBOARD, COPY_CMD, CUT_CMD, PASTE_CMD};
use zero_ui_ext_font::{font_features::FontVariations, *};
use zero_ui_ext_input::{
    focus::{FocusInfoBuilder, WidgetInfoFocusExt as _, FOCUS, FOCUS_CHANGED_EVENT},
    keyboard::{Key, KeyState, KEYBOARD, KEY_INPUT_EVENT},
    mouse::{MOUSE, MOUSE_INPUT_EVENT, MOUSE_MOVE_EVENT},
    pointer_capture::{POINTER_CAPTURE, POINTER_CAPTURE_EVENT},
    touch::{TOUCH_INPUT_EVENT, TOUCH_LONG_PRESS_EVENT, TOUCH_MOVE_EVENT, TOUCH_TAP_EVENT},
};
use zero_ui_ext_l10n::LANG_VAR;
use zero_ui_ext_undo::UNDO;
use zero_ui_ext_window::{cmd::CANCEL_IME_CMD, WINDOW_Ext as _, WidgetInfoBuilderImeArea as _, WindowLoadingHandle, IME_EVENT};
use zero_ui_layout::context::{InlineConstraints, InlineConstraintsMeasure, InlineSegment};
use zero_ui_view_api::{
    config::FontAntiAliasing,
    mouse::ButtonState,
    touch::{TouchId, TouchPhase},
    webrender_api::GlyphInstance,
};
use zero_ui_wgt::prelude::*;
use zero_ui_wgt_data::{DataNoteHandle, DATA};
use zero_ui_wgt_layer::{
    popup::{ContextCapture, PopupState, POPUP},
    AnchorMode, AnchorTransform, LayerIndex, LAYERS,
};
use zero_ui_wgt_scroll::{cmd::ScrollToMode, SCROLL};

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

/// IME text edit that is not committed yet.
#[derive(Clone)]
pub struct ImePreview {
    /// The inserted text.
    pub txt: Txt,

    /// Caret index when IME started.
    pub prev_caret: CaretIndex,
    /// Selection index when IME started.
    ///
    /// If set defines a selection of the text variable that is replaced with the `txt`.
    pub prev_selection: Option<CaretIndex>,
}

/// Represents the resolved fonts and the transformed, white space corrected and segmented text.
pub struct ResolvedText {
    /// The text source variable.
    pub txt: BoxedVar<Txt>,
    /// IME text edit that is not committed yet. Only the text in the segmented and shaped text is edited,
    /// the text variable is not updated yet and undo is not tracking these changes.
    pub ime_preview: Option<ImePreview>,

    /// Text transformed, white space corrected and segmented.
    pub segmented_text: SegmentedText,
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

    /// Show touch carets.
    ///
    /// Set to `true` on touch interactions and set to `false` on other interactions.
    pub touch_carets: AtomicBool,

    /// Baseline set by `layout_text` during measure and used by `new_border` during arrange.
    baseline: Atomic<Px>,
}

impl fmt::Debug for ResolvedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResolvedText")
            .field("segmented_text", &self.segmented_text)
            .field("faces", &self.faces)
            .field("synthesis", &self.synthesis)
            .field("pending_layout", &self.pending_layout)
            .field("pending_edit", &self.pending_edit)
            .field("caret", &self.caret)
            .field("touch_carets", &self.touch_carets)
            .finish_non_exhaustive()
    }
}
impl Clone for ResolvedText {
    fn clone(&self) -> Self {
        Self {
            txt: self.txt.clone(),
            ime_preview: self.ime_preview.clone(),
            segmented_text: self.segmented_text.clone(),
            faces: self.faces.clone(),
            synthesis: self.synthesis,
            pending_layout: self.pending_layout,
            pending_edit: self.pending_edit,
            caret: Mutex::new(self.caret.lock().clone()),
            touch_carets: AtomicBool::new(self.touch_carets.load(Ordering::Relaxed)),
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

    fn call_edit_op(ctx: &mut Option<Self>, op: impl FnOnce() -> bool) {
        let registered = RESOLVED_TEXT.with_context_opt(ctx, op);
        if registered {
            let ctx = ctx.as_mut().unwrap();
            if !ctx.pending_edit {
                ctx.pending_edit = true;
                WIDGET.update(); // in case the edit does not actually change the text.
            }
        }
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
    /// Note that underlines are only computed if the `underline_thickness` is more than `0`. These
    /// underlines never cover the IME review text range.
    ///
    /// Default underlines are rendered by [`render_underlines`].
    pub underlines: Vec<(PxPoint, Px)>,
    /// Computed [`UNDERLINE_THICKNESS_VAR`].
    pub underline_thickness: Px,

    /// List of underline segments for IME preview text, defining origin and width of each line.
    ///
    /// Note that underlines are only computed if the `ime_underline_thickness` is more than `0`.
    ///
    /// Default underlines are rendered by [`render_underlines`].
    pub ime_underlines: Vec<(PxPoint, Px)>,
    /// Computed [`IME_UNDERLINE_THICKNESS_VAR`].
    pub ime_underline_thickness: Px,

    /// Top-middle offset of the caret index in the shaped text.
    pub caret_origin: Option<PxPoint>,

    /// Top-middle offset of the caret selection_index in the shaped text.
    pub caret_selection_origin: Option<PxPoint>,

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
            ime_underlines: self.ime_underlines.clone(),
            ime_underline_thickness: self.ime_underline_thickness,
            caret_origin: self.caret_origin,
            caret_selection_origin: self.caret_selection_origin,
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
        events: [EventHandle; 6],
        caret_animation: VarHandle,
        max_count: VarHandle,
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
                self.events[5] = IME_EVENT.subscribe(id);

                self.paste = PASTE_CMD.scoped(id).subscribe(true);
                self.edit = EDIT_CMD.scoped(id).subscribe(true);

                self.max_count = MAX_CHARS_COUNT_VAR.subscribe(UpdateOp::Update, id);
            }

            if TEXT_SELECTABLE_VAR.get() {
                let id = WIDGET.id();

                self.events[4] = ACCESS_SELECTION_EVENT.subscribe(id);

                let obscure = OBSCURE_TXT_VAR.get();
                self.copy = COPY_CMD.scoped(id).subscribe(!obscure);
                if editable {
                    self.cut = CUT_CMD.scoped(id).subscribe(!obscure);
                } else {
                    // used in `render_selection`
                    self.events[0] = FOCUS_CHANGED_EVENT.subscribe(id);

                    self.events[2] = KEY_INPUT_EVENT.subscribe(id);
                }
            }
        }
    }
    fn enforce_max_count(text: &BoxedVar<Txt>) {
        let max_count = MAX_CHARS_COUNT_VAR.get();
        if max_count > 0 {
            let count = text.with(|t| t.chars().count());
            if count > max_count {
                tracing::debug!("txt var set to text longer than can be typed");
                let _ = text.modify(move |t| {
                    if let Some((i, _)) = t.as_str().char_indices().nth(max_count) {
                        t.to_mut().truncate(i);
                    }
                });
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
                txt: text.clone(),
                ime_preview: None,
                synthesis: FONT_SYNTHESIS_VAR.get() & f.best().synthesis_for(style, weight),
                faces: f,
                segmented_text: SegmentedText::new(txt, DIRECTION_VAR.get()),
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
                touch_carets: AtomicBool::new(false),
            });

            if editable || TEXT_SELECTABLE_VAR.get() {
                EditData::get(&mut edit_data).subscribe();
            }

            if editable {
                enforce_max_count(&text);
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
            if !editable && !OBSCURE_TXT_VAR.get() {
                if let Some(mut a) = info.access() {
                    a.set_label(resolved.as_ref().unwrap().segmented_text.text().clone());
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
            } else if TEXT_EDITABLE_VAR.get() && text.capabilities().can_modify() {
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

                if !resolved.as_mut().unwrap().pending_edit && widget.interactivity().is_enabled() {
                    if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                        let ctx = resolved.as_mut().unwrap();
                        if let KeyState::Pressed = args.state {
                            match &args.key {
                                Key::Backspace => {
                                    let caret = ctx.caret.get_mut();
                                    if caret.selection_index.is_some() || caret.index.unwrap_or(CaretIndex::ZERO).index > 0 {
                                        if args.modifiers.is_only_ctrl() {
                                            args.propagation().stop();
                                            *ctx.touch_carets.get_mut() = false;
                                            ResolvedText::call_edit_op(&mut resolved, || TextEditOp::backspace_word().call(&text));
                                        } else if args.modifiers.is_empty() {
                                            args.propagation().stop();
                                            *ctx.touch_carets.get_mut() = false;
                                            ResolvedText::call_edit_op(&mut resolved, || TextEditOp::backspace().call(&text));
                                        }
                                    }
                                }
                                Key::Delete => {
                                    let caret = ctx.caret.get_mut();
                                    let caret_idx = caret.index.unwrap_or(CaretIndex::ZERO);
                                    if caret.selection_index.is_some() || caret_idx.index < ctx.segmented_text.text().len() {
                                        if args.modifiers.is_only_ctrl() {
                                            args.propagation().stop();
                                            *ctx.touch_carets.get_mut() = false;
                                            ResolvedText::call_edit_op(&mut resolved, || TextEditOp::delete_word().call(&text));
                                        } else if args.modifiers.is_empty() {
                                            args.propagation().stop();
                                            *ctx.touch_carets.get_mut() = false;
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
                                            *ctx.touch_carets.get_mut() = false;
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

                        let auto_select = match AUTO_SELECTION_VAR.get() {
                            AutoSelection::Disabled => false,
                            AutoSelection::Enabled => true,
                            AutoSelection::Auto => !ACCEPTS_ENTER_VAR.get(),
                        };
                        if auto_select && TEXT_SELECTABLE_VAR.get() {
                            if args.is_blur(widget.id()) {
                                // deselect if the widget is not the ALT return focus and is not the parent scope return focus.

                                let us = Some(widget.id());
                                let alt_return = FOCUS.alt_return().with(|p| p.as_ref().map(|p| p.widget_id()));
                                if alt_return != us {
                                    if let Some(info) = WIDGET.info().into_focusable(true, true) {
                                        if let Some(scope) = info.scope() {
                                            let parent_return =
                                                FOCUS.return_focused(scope.info().id()).with(|p| p.as_ref().map(|p| p.widget_id()));
                                            if parent_return != us {
                                                SELECT_CMD.scoped(widget.id()).notify_param(TextSelectOp::next());
                                            }
                                        }
                                    }
                                }
                            } else if args.highlight && args.is_focus(widget.id()) {
                                SELECT_ALL_CMD.scoped(widget.id()).notify();
                            }
                        }
                    } else if let Some(args) = CUT_CMD.scoped(widget.id()).on_unhandled(update) {
                        let ctx = resolved.as_mut().unwrap();
                        if let Some(range) = ctx.caret.get_mut().selection_char_range() {
                            args.propagation().stop();
                            *ctx.touch_carets.get_mut() = false;
                            if CLIPBOARD.set_text(Txt::from_str(&ctx.segmented_text.text()[range])).is_ok() {
                                ResolvedText::call_edit_op(&mut resolved, || TextEditOp::delete().call(&text));
                            }
                        }
                    } else if let Some(args) = PASTE_CMD.scoped(widget.id()).on_unhandled(update) {
                        if let Some(paste) = CLIPBOARD.text().ok().flatten() {
                            if !paste.is_empty() {
                                args.propagation().stop();
                                *resolved.as_mut().unwrap().touch_carets.get_mut() = false;
                                ResolvedText::call_edit_op(&mut resolved, || TextEditOp::insert(paste).call(&text));
                            }
                        }
                    } else if let Some(args) = EDIT_CMD.scoped(widget.id()).on_unhandled(update) {
                        if let Some(op) = args.param::<UndoTextEditOp>() {
                            args.propagation().stop();

                            ResolvedText::call_edit_op(&mut resolved, || {
                                op.call(&text);
                                true
                            });
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
                                .call(&text)
                            });
                        }
                    } else if let Some(args) = IME_EVENT.on_unhandled(update) {
                        let mut resegment = false;

                        if let Some((start, end)) = args.preview_caret {
                            // update preview txt

                            let resolved = resolved.as_mut().unwrap();

                            if args.txt.is_empty() {
                                if let Some(preview) = resolved.ime_preview.take() {
                                    resegment = true;
                                    let caret = resolved.caret.get_mut();
                                    caret.set_index(preview.prev_caret);
                                    caret.selection_index = preview.prev_selection;
                                }
                            } else if let Some(preview) = &mut resolved.ime_preview {
                                resegment = preview.txt != args.txt;
                                if resegment {
                                    preview.txt = args.txt.clone();
                                }
                            } else {
                                resegment = true;
                                let caret = resolved.caret.get_mut();
                                resolved.ime_preview = Some(ImePreview {
                                    txt: args.txt.clone(),
                                    prev_caret: caret.index.unwrap_or(CaretIndex::ZERO),
                                    prev_selection: caret.selection_index,
                                });
                            }

                            // update preview caret/selection indexes.
                            if let Some(preview) = &resolved.ime_preview {
                                let caret = resolved.caret.get_mut();
                                let ime_start = if let Some(s) = preview.prev_selection {
                                    preview.prev_caret.index.min(s.index)
                                } else {
                                    preview.prev_caret.index
                                };
                                if start != end {
                                    let start = ime_start + start;
                                    let end = ime_start + end;
                                    resegment |= caret.selection_char_range() != Some(start..end);
                                    caret.set_char_selection(start, end);
                                } else {
                                    let start = ime_start + start;
                                    resegment |= caret.selection_index.is_some() || caret.index.map(|c| c.index) != Some(start);
                                    caret.set_char_index(start);
                                    caret.selection_index = None;
                                }
                            }
                        } else {
                            // commit IME insert

                            args.propagation().stop();
                            {
                                let resolved = resolved.as_mut().unwrap();
                                if let Some(preview) = resolved.ime_preview.take() {
                                    // restore caret
                                    let caret = resolved.caret.get_mut();
                                    caret.set_index(preview.prev_caret);
                                    caret.selection_index = preview.prev_selection;

                                    if args.txt.is_empty() {
                                        // the actual insert already re-segments, except in this case
                                        // where there is nothing to insert.
                                        resegment = true;
                                    }
                                }
                            }

                            if !args.txt.is_empty() {
                                // actual insert
                                {
                                    let resolved = resolved.as_mut().unwrap();
                                    *resolved.touch_carets.get_mut() = false;

                                    // if the committed text is equal the last preview reshape is skipped
                                    // leaving behind the IME underline highlight.
                                    resolved.pending_layout |= PendingLayout::UNDERLINE;
                                    WIDGET.layout();
                                }
                                ResolvedText::call_edit_op(&mut resolved, || TextEditOp::insert(args.txt.clone()).call(&text));
                            }
                        }

                        if resegment {
                            let resolved = resolved.as_mut().unwrap();

                            // re-segment text to insert or remove the preview
                            let mut text = resolved.txt.get();
                            if let Some(preview) = &resolved.ime_preview {
                                if let Some(s) = preview.prev_selection {
                                    let range = if preview.prev_caret.index < s.index {
                                        preview.prev_caret.index..s.index
                                    } else {
                                        s.index..preview.prev_caret.index
                                    };
                                    text.to_mut().replace_range(range, preview.txt.as_str());
                                } else {
                                    text.to_mut().insert_str(preview.prev_caret.index, preview.txt.as_str());
                                }
                                text.end_mut();
                            }
                            resolved.segmented_text = SegmentedText::new(text, DIRECTION_VAR.get());

                            resolved.pending_layout |= PendingLayout::RESHAPE;
                            WIDGET.layout();
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
                        let _ = CLIPBOARD.set_text(Txt::from_str(&resolved.segmented_text.text()[range]));
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
            if let Some(edit_data) = &mut edit_data {
                let enable = !OBSCURE_TXT_VAR.get() && resolved.as_mut().unwrap().caret.get_mut().selection_range().is_some();
                edit_data.cut.set_enabled(enable);
                edit_data.copy.set_enabled(enable);
            }
        }
        UiNodeOp::Update { updates } => {
            let r = resolved.as_mut().unwrap();

            // update `r.text`, affects layout.
            if text.is_new() || TEXT_TRANSFORM_VAR.is_new() || WHITE_SPACE_VAR.is_new() || DIRECTION_VAR.is_new() {
                if text.is_new() {
                    if !r.pending_edit && UNDO.scope() == Some(WIDGET.id()) {
                        UNDO.clear();
                    }
                    if let Some(p) = r.ime_preview.take() {
                        let c = r.caret.get_mut();
                        c.index = Some(p.prev_caret);
                        c.selection_index = p.prev_selection;

                        CANCEL_IME_CMD.scoped(WINDOW.id()).notify();
                    }

                    r.pending_edit = false;

                    enforce_max_count(&text);
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
                if r.segmented_text.text() != &text || r.segmented_text.base_direction() != direction {
                    r.segmented_text = SegmentedText::new(text, direction);

                    // prevent invalid indexes
                    let caret = r.caret.get_mut();
                    if let Some(i) = &mut caret.index {
                        i.index = r.segmented_text.snap_grapheme_boundary(i.index);
                    }
                    if let Some(i) = &mut caret.selection_index {
                        i.index = r.segmented_text.snap_grapheme_boundary(i.index);
                    }
                    if let Some((cr, _)) = &mut caret.initial_selection {
                        cr.start.index = r.segmented_text.snap_grapheme_boundary(cr.start.index);
                        cr.end.index = r.segmented_text.snap_grapheme_boundary(cr.end.index);
                    }

                    if WINDOW.vars().access_enabled().get().is_enabled() {
                        WIDGET.info();
                    }

                    r.pending_layout = PendingLayout::RESHAPE;
                    WIDGET.layout();
                }
            }

            r.pending_edit = false; // in case the edit did not actually change the text

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

                if editable {
                    enforce_max_count(&text);
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

                if r.segmented_text.text() != &text {
                    r.segmented_text = SegmentedText::new(text, DIRECTION_VAR.get());
                    if let Some(i) = &mut r.caret.get_mut().index {
                        i.index = r.segmented_text.snap_grapheme_boundary(i.index);
                    }

                    r.pending_layout = PendingLayout::RESHAPE;
                    WIDGET.layout();
                }
            }

            if TEXT_EDITABLE_VAR.get() && MAX_CHARS_COUNT_VAR.is_new() {
                enforce_max_count(&text);
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

bitflags! {
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
                    ime_underlines: vec![],
                    underline_thickness: Px(0),
                    ime_underline_thickness: Px(0),
                    caret_origin: None,
                    caret_selection_origin: None,
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
            let (overline, strikethrough, underline, ime_underline) = {
                LAYOUT.with_constraints(PxConstraints2d::new_exact(line_height, line_height), || {
                    (
                        OVERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                        STRIKETHROUGH_THICKNESS_VAR.layout_dft_y(dft_thickness),
                        UNDERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                        IME_UNDERLINE_THICKNESS_VAR.layout_dft_y(dft_thickness),
                    )
                })
            };

            if !self.pending.contains(PendingLayout::OVERLINE) && (txt.overline_thickness == Px(0)) != (overline == Px(0)) {
                self.pending.insert(PendingLayout::OVERLINE);
            }
            if !self.pending.contains(PendingLayout::STRIKETHROUGH) && (txt.strikethrough_thickness == Px(0)) != (strikethrough == Px(0)) {
                self.pending.insert(PendingLayout::STRIKETHROUGH);
            }
            if !self.pending.contains(PendingLayout::UNDERLINE)
                && ((txt.underline_thickness == Px(0)) != (underline == Px(0))
                    || (txt.ime_underline_thickness != Px(0)) != (ime_underline != Px(0)))
            {
                self.pending.insert(PendingLayout::UNDERLINE);
            }
            txt.overline_thickness = overline;
            txt.strikethrough_thickness = strikethrough;
            txt.underline_thickness = underline;
            txt.ime_underline_thickness = ime_underline;

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
                txt.shaped_text = txt.fonts.shape_text(&t.segmented_text, &self.shaping_args);
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
                    txt.caret_selection_origin = None;
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
                    let ime_range = if let Some(ime) = &t.ime_preview {
                        let start = ime.prev_selection.unwrap_or(ime.prev_caret).index.min(ime.prev_caret.index);
                        start..start + ime.txt.len()
                    } else {
                        0..0
                    };
                    let caret_ime_range =
                        if !ime_range.is_empty() && (txt.underline_thickness > Px(0) || txt.ime_underline_thickness > Px(0)) {
                            let start = txt.shaped_text.snap_caret_line(CaretIndex {
                                index: ime_range.start,
                                line: 0,
                            });
                            let end = txt.shaped_text.snap_caret_line(CaretIndex {
                                index: ime_range.end,
                                line: 0,
                            });

                            start..end
                        } else {
                            CaretIndex::ZERO..CaretIndex::ZERO
                        };

                    if txt.underline_thickness > Px(0) {
                        let mut underlines = vec![];

                        let skip = UNDERLINE_SKIP_VAR.get();
                        match UNDERLINE_POSITION_VAR.get() {
                            UnderlinePosition::Font => {
                                if skip == UnderlineSkip::GLYPHS | UnderlineSkip::SPACES {
                                    for line in txt.shaped_text.lines() {
                                        for und in line.underline_skip_glyphs_and_spaces(txt.underline_thickness) {
                                            underlines.push(und);
                                        }
                                    }
                                } else if skip.contains(UnderlineSkip::GLYPHS) {
                                    for line in txt.shaped_text.lines() {
                                        for und in line.underline_skip_glyphs(txt.underline_thickness) {
                                            underlines.push(und);
                                        }
                                    }
                                } else if skip.contains(UnderlineSkip::SPACES) {
                                    for line in txt.shaped_text.lines() {
                                        for und in line.underline_skip_spaces() {
                                            underlines.push(und);
                                        }
                                    }
                                } else {
                                    for line in txt.shaped_text.lines() {
                                        let und = line.underline();
                                        underlines.push(und);
                                    }
                                }
                            }
                            UnderlinePosition::Descent => {
                                // descent clears all glyphs, so we only need to care about spaces
                                if skip.contains(UnderlineSkip::SPACES) {
                                    for line in txt.shaped_text.lines() {
                                        for und in line.underline_descent_skip_spaces() {
                                            underlines.push(und);
                                        }
                                    }
                                } else {
                                    for line in txt.shaped_text.lines() {
                                        let und = line.underline_descent();
                                        underlines.push(und);
                                    }
                                }
                            }
                        }

                        if !ime_range.is_empty() {
                            underlines =
                                txt.shaped_text
                                    .clip_lines(caret_ime_range.clone(), true, t.segmented_text.text(), underlines.into_iter());
                        }

                        txt.underlines = underlines;
                    } else {
                        txt.underlines = vec![];
                    }

                    if txt.ime_underline_thickness > Px(0) && !ime_range.is_empty() {
                        let mut ime_underlines = vec![];

                        // collects underlines for all segments that intersect with the IME text.
                        for line in txt.shaped_text.lines() {
                            let line_range = line.text_range();
                            if line_range.start < ime_range.end && line_range.end > ime_range.start {
                                for seg in line.segs() {
                                    let seg_range = seg.text_range();
                                    if seg_range.start < ime_range.end && seg_range.end > ime_range.start {
                                        for und in seg.underline_skip_glyphs(txt.ime_underline_thickness) {
                                            ime_underlines.push(und);
                                        }
                                    }
                                }
                            }
                        }

                        txt.ime_underlines =
                            txt.shaped_text
                                .clip_lines(caret_ime_range, false, t.segmented_text.text(), ime_underlines.into_iter());
                    } else {
                        txt.ime_underlines = vec![];
                    }
                }

                if self.pending.contains(PendingLayout::CARET) {
                    let resolved_text = ResolvedText::get();
                    let mut caret = resolved_text.caret.lock();
                    let caret = &mut *caret;
                    if let Some(index) = &mut caret.index {
                        *index = txt.shaped_text.snap_caret_line(*index);

                        let p = txt.shaped_text.caret_origin(*index, resolved_text.segmented_text.text());
                        if !caret.used_retained_x {
                            txt.caret_retained_x = p.x;
                        }
                        txt.caret_origin = Some(p);

                        if let Some(sel) = &mut caret.selection_index {
                            *sel = txt.shaped_text.snap_caret_line(*sel);
                            txt.caret_selection_origin = Some(txt.shaped_text.caret_origin(*sel, resolved_text.segmented_text.text()));
                        }

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
        ime_area: Arc<Atomic<PxRect>>,
    }
    impl EditData {
        fn get(edit_data: &mut Option<Box<Self>>) -> &mut Self {
            &mut *edit_data.get_or_insert_with(Default::default)
        }

        fn subscribe(&mut self) {
            let editable = TEXT_EDITABLE_VAR.get();
            let selectable = TEXT_SELECTABLE_VAR.get();

            if selectable || editable {
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

        fn update_ime(&self, txt: &mut LayoutText) {
            let transform = txt.render_info.get_mut().transform;
            let area;

            if let Some(a) = txt.caret_origin {
                let (ac, bc) = {
                    let ctx = ResolvedText::get();
                    let c = ctx.caret.lock();
                    (c.index, c.selection_index)
                };
                let ac = ac.unwrap_or(CaretIndex::ZERO);
                let mut a_line = PxRect::new(a, PxSize::new(Px(1), txt.shaped_text.line(ac.line).unwrap().height())).to_box2d();

                if let Some(b) = txt.caret_selection_origin {
                    let bc = bc.unwrap_or(CaretIndex::ZERO);
                    let b_line = PxRect::new(b, PxSize::new(Px(1), txt.shaped_text.line(bc.line).unwrap().height())).to_box2d();

                    a_line.min = a_line.min.min(b_line.min);
                    a_line.max = a_line.max.min(b_line.max);
                }
                area = a_line;
            } else {
                area = PxBox::from_size(txt.shaped_text.size());
            }

            if let Some(area) = transform.outer_transformed(area) {
                self.ime_area.store(area.to_rect(), atomic::Ordering::Relaxed);
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

            WIDGET.sub_var(&OBSCURE_TXT_VAR).sub_var(&OBSCURING_CHAR_VAR);

            // LANG_VAR already subscribed by `resolve_text`.

            txt.shaping_args.lang = LANG_VAR.with(|l| l.best().clone());
            txt.shaping_args.direction = txt.shaping_args.lang.direction();
            txt.shaping_args.line_break = LINE_BREAK_VAR.get();
            txt.shaping_args.word_break = WORD_BREAK_VAR.get();
            txt.shaping_args.hyphens = HYPHENS_VAR.get();
            txt.shaping_args.hyphen_char = HYPHEN_CHAR_VAR.get();
            txt.shaping_args.font_features = FONT_FEATURES_VAR.with(|f| f.finalize());

            if OBSCURE_TXT_VAR.get() {
                txt.shaping_args.obscuring_char = Some(OBSCURING_CHAR_VAR.get());
            }

            if TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get() {
                EditData::get(&mut edit_data).subscribe();
            }

            // txt.txt not available yet.
            // txt.with(|| child.init());
        }
        UiNodeOp::Deinit => {
            txt.with(|| child.deinit());
            txt.txt = None;
            txt.shaping_args = TextShapingArgs::default();
            edit_data = None;
        }
        UiNodeOp::Info { info } => {
            if let Some(data) = &edit_data {
                info.set_ime_area(data.ime_area.clone());
            }
        }
        UiNodeOp::Event { update } => {
            let resolved = RESOLVED_TEXT.get();
            let editable = TEXT_EDITABLE_VAR.get() && resolved.txt.capabilities().can_modify();
            let selectable = TEXT_SELECTABLE_VAR.get();
            if (editable || selectable) && WIDGET.info().interactivity().is_enabled() && txt.txt.is_some() {
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
                                    resolved.touch_carets.store(false, Ordering::Relaxed);
                                }
                            }
                            Key::Enter => {
                                if editable && args.modifiers.is_empty() && ACCEPTS_ENTER_VAR.get() {
                                    args.propagation().stop();
                                    resolved.touch_carets.store(false, Ordering::Relaxed);
                                }
                            }
                            Key::ArrowRight => {
                                let mut modifiers = args.modifiers;
                                let select = selectable && modifiers.take_shift();
                                let word = modifiers.take_ctrl();
                                if modifiers.is_empty() && (editable || select) {
                                    args.propagation().stop();

                                    resolved.touch_carets.store(false, Ordering::Relaxed);

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

                                    resolved.touch_carets.store(false, Ordering::Relaxed);

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

                                        resolved.touch_carets.store(false, Ordering::Relaxed);

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

                                        resolved.touch_carets.store(false, Ordering::Relaxed);

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

                                        resolved.touch_carets.store(false, Ordering::Relaxed);

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

                                        resolved.touch_carets.store(false, Ordering::Relaxed);

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

                                    resolved.touch_carets.store(false, Ordering::Relaxed);

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

                                    resolved.touch_carets.store(false, Ordering::Relaxed);

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

                            resolved.touch_carets.store(false, Ordering::Relaxed);

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

                        resolved.touch_carets.store(true, Ordering::Relaxed);

                        LayoutText::call_select_op(&mut txt.txt, || {
                            TextSelectOp::nearest_to(args.position).call();
                        });
                    }
                } else if let Some(args) = TOUCH_LONG_PRESS_EVENT.on_unhandled(update) {
                    if args.modifiers.is_empty() && selectable {
                        args.propagation().stop();

                        resolved.touch_carets.store(true, Ordering::Relaxed);

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
                txt.shaping_args.direction = txt.shaping_args.lang.direction(); // will be set in layout too.
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
            if OBSCURE_TXT_VAR.is_new() || OBSCURING_CHAR_VAR.is_new() {
                if let Some(obscure) = OBSCURE_TXT_VAR.get_new() {
                    if edit_data.is_none() && WINDOW.info().access_enabled().is_enabled() {
                        WIDGET.info();
                    }

                    if obscure {
                        UNDO.clear();
                    }
                }

                let c = if OBSCURE_TXT_VAR.get() {
                    Some(OBSCURING_CHAR_VAR.get())
                } else {
                    None
                };
                if txt.shaping_args.obscuring_char != c {
                    txt.shaping_args.obscuring_char = c;
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
                if TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get() {
                    if edit_data.is_none() {
                        EditData::get(&mut edit_data).subscribe();
                        WIDGET.info();
                    }
                } else {
                    edit_data = None;
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
            txt.with(|| child.render(frame));

            if let Some(data) = &edit_data {
                let txt = txt.txt.as_mut().unwrap();
                data.update_ime(txt);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            txt.ensure_layout_for_render();
            txt.with(|| child.render_update(update));

            if let Some(data) = &edit_data {
                let txt = txt.txt.as_mut().unwrap();
                data.update_ime(txt);
            }
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

/// An Ui node that renders the default IME preview underline visual using the parent [`LayoutText`].
///
///
/// The lines are rendered before `child`, under it.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_underlines`] node.
pub fn render_ime_preview_underlines(child: impl UiNode) -> impl UiNode {
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&IME_UNDERLINE_STYLE_VAR).sub_var_render(&FONT_COLOR_VAR);
        }
        UiNodeOp::Render { frame } => {
            let t = LayoutText::get();

            if !t.ime_underlines.is_empty() {
                let style = IME_UNDERLINE_STYLE_VAR.get();
                if style != LineStyle::Hidden {
                    let color = FONT_COLOR_VAR.get().into();
                    for &(origin, width) in &t.ime_underlines {
                        frame.push_line(
                            PxRect::new(origin, PxSize::new(width, t.ime_underline_thickness)),
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
                let resolved = ResolvedText::get();

                if let (false, Some(mut origin)) = (resolved.touch_carets.load(Ordering::Relaxed), t.caret_origin) {
                    let mut c = CARET_COLOR_VAR.get();
                    c.alpha = resolved.caret.lock().opacity.get().0;

                    let caret_thickness = Dip::new(1).to_px(frame.scale_factor());
                    origin.x -= caret_thickness / 2;

                    let clip_rect = PxRect::new(origin, PxSize::new(caret_thickness, t.shaped_text.line_height()));
                    frame.push_color(clip_rect, color_key.bind(c.into(), true));
                }
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            child.render_update(update);

            if TEXT_EDITABLE_VAR.get() {
                let resolved = ResolvedText::get();

                if !resolved.touch_carets.load(Ordering::Relaxed) {
                    let mut c = CARET_COLOR_VAR.get();
                    c.alpha = ResolvedText::get().caret.lock().opacity.get().0;

                    update.update_color(color_key.update(c.into(), true))
                }
            }
        }
        _ => {}
    })
}

/// An Ui node that renders the touch carets and implement interaction.
///
/// Caret visuals defined by [`CARET_TOUCH_SHAPE_VAR`].
pub fn touch_carets(child: impl UiNode) -> impl UiNode {
    let mut carets: Vec<Caret> = vec![];
    struct Caret {
        id: WidgetId,
        layout: Arc<Mutex<CaretLayout>>,
    }
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&CARET_TOUCH_SHAPE_VAR);
        }
        UiNodeOp::Deinit => {
            for caret in carets.drain(..) {
                LAYERS.remove(caret.id);
            }
        }
        UiNodeOp::Update { .. } => {
            if !carets.is_empty() && CARET_TOUCH_SHAPE_VAR.is_new() {
                for caret in carets.drain(..) {
                    LAYERS.remove(caret.id);
                }
                WIDGET.layout();
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = c.layout(wl);

            let r_txt = ResolvedText::get();

            let caret = r_txt.caret.lock();
            let mut expected_len = 0;
            if caret.index.is_some()
                && FOCUS.focused().with(|p| matches!(p, Some(p) if p.widget_id() == WIDGET.id()))
                && r_txt.touch_carets.load(Ordering::Relaxed)
            {
                if caret.selection_index.is_some() {
                    if r_txt.segmented_text.is_bidi() {
                        expected_len = 4;
                    } else {
                        expected_len = 2;
                    }
                } else {
                    expected_len = 1;
                }
            }

            if expected_len != carets.len() {
                for caret in carets.drain(..) {
                    LAYERS.remove(caret.id);
                }

                // caret shape node, inserted as ADORNER+1, anchored, propagates LocalContext and collects size+caret mid
                let mut open_caret = |s| {
                    let c_layout = Arc::new(Mutex::new(CaretLayout::default()));
                    let id = WidgetId::new_unique();

                    let caret = TouchCaret! {
                        id;
                        touch_caret_input = TouchCaretInput {
                            ctx: LocalContext::capture(),
                            layout: c_layout.clone(),
                            parent_id: WIDGET.id(),
                            shape: s,
                            shape_fn: CARET_TOUCH_SHAPE_VAR.get(),
                        };
                    };

                    LAYERS.insert_anchored(LayerIndex::ADORNER + 1, WIDGET.id(), AnchorMode::foreground(), caret);
                    carets.push(Caret { id, layout: c_layout })
                };

                if expected_len == 1 {
                    open_caret(CaretShape::Insert);
                } else if expected_len == 2 {
                    open_caret(CaretShape::SelectionLeft);
                    open_caret(CaretShape::SelectionRight);
                } else if expected_len == 4 {
                    open_caret(CaretShape::SelectionLeft);
                    open_caret(CaretShape::SelectionRight);
                    open_caret(CaretShape::SelectionLeft);
                    open_caret(CaretShape::SelectionRight);
                }
            }

            if !carets.is_empty() {
                if carets.len() == 1 {
                    let t = LayoutText::get();
                    if let Some(mut origin) = t.caret_origin {
                        let mut l = carets[0].layout.lock();
                        if l.width == Px::MIN {
                            // wait caret's first layout.
                            return;
                        }

                        origin.x -= l.width / 2;
                        if l.x != origin.x || l.y != origin.y {
                            l.x = origin.x;
                            l.y = origin.y;

                            UPDATES.render(carets[0].id);
                        }
                    }
                } else if carets.len() == 2 || carets.len() == 4 {
                    let t = LayoutText::get();

                    if let (Some(index), Some(s_index), Some(mut origin), Some(mut s_origin)) =
                        (caret.index, caret.selection_index, t.caret_origin, t.caret_selection_origin)
                    {
                        let mut l = [carets[0].layout.lock(), carets[1].layout.lock()];
                        if l[0].width == Px::MIN && l[1].width == Px::MIN {
                            return;
                        }

                        let mut index_is_left = index.index <= s_index.index;
                        let seg_txt = &r_txt.segmented_text;
                        if let Some((_, seg)) = seg_txt.get(seg_txt.seg_from_char(index.index)) {
                            if seg.direction().is_rtl() {
                                index_is_left = !index_is_left;
                            }
                        }

                        let mut s_index_is_left = s_index.index < index.index;
                        if let Some((_, seg)) = seg_txt.get(seg_txt.seg_from_char(s_index.index)) {
                            if seg.direction().is_rtl() {
                                s_index_is_left = !s_index_is_left;
                            }
                        }

                        if index_is_left {
                            origin.x -= l[0].mid;
                        } else {
                            origin.x -= l[1].mid;
                        }
                        if s_index_is_left {
                            s_origin.x -= l[0].mid;
                        } else {
                            s_origin.x -= l[1].mid;
                        }

                        let changed;

                        if index_is_left == s_index_is_left {
                            let i = if index_is_left { 0 } else { 1 };

                            changed = l[i].x != origin.x || l[i].y != origin.y || l[i + 2].x != s_origin.x || l[i + 2].y != s_origin.y;

                            for l in &mut l {
                                l.x = Px::MIN;
                                l.y = Px::MIN;
                                l.is_selection_index = false;
                            }

                            l[i].x = origin.x;
                            l[i].y = origin.y;
                            l[i + 2].x = s_origin.x;
                            l[i + 2].y = s_origin.y;
                            l[i + 2].is_selection_index = true;
                        } else {
                            let (lft, rgt) = if index_is_left { (0, 1) } else { (1, 0) };

                            changed = l[lft].x != origin.x || l[lft].y != origin.y || l[rgt].x != s_origin.x || l[rgt].y != s_origin.y;

                            for l in &mut l {
                                l.x = Px::MIN;
                                l.y = Px::MIN;
                                l.is_selection_index = false;
                            }

                            l[lft].x = origin.x;
                            l[lft].y = origin.y;
                            l[rgt].x = s_origin.x;
                            l[rgt].y = s_origin.y;
                            l[rgt].is_selection_index = true;
                        }

                        if changed {
                            for c in &carets {
                                UPDATES.render(c.id);
                            }
                        }
                    } else {
                        tracing::error!("touch caret instances do not match context caret")
                    }
                }
            }
        }
        UiNodeOp::Render { .. } | UiNodeOp::RenderUpdate { .. } => {
            if let Some(inner_rev) = WIDGET.info().inner_transform().inverse() {
                let text = LayoutText::get().render_info.lock().transform.then(&inner_rev);

                for c in &carets {
                    let mut l = c.layout.lock();
                    if l.inner_text != text {
                        l.inner_text = text;

                        if l.x > Px::MIN && l.y > Px::MIN {
                            UPDATES.render(c.id);
                        }
                    }
                }
            }
        }
        _ => {}
    })
}
struct CaretLayout {
    // set by caret
    width: Px,
    mid: Px,
    // set by Text
    inner_text: PxTransform,
    x: Px,
    y: Px,
    is_selection_index: bool,
}
impl Default for CaretLayout {
    fn default() -> Self {
        Self {
            width: Px::MIN,
            mid: Px::MIN,
            inner_text: Default::default(),
            x: Px::MIN,
            y: Px::MIN,
            is_selection_index: false,
        }
    }
}

#[derive(Clone)]
struct TouchCaretInput {
    shape: CaretShape,
    shape_fn: WidgetFn<CaretShape>,
    layout: Arc<Mutex<CaretLayout>>,
    ctx: LocalContext,
    parent_id: WidgetId,
}
impl fmt::Debug for TouchCaretInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TouchCaretInput")
    }
}
impl PartialEq for TouchCaretInput {
    fn eq(&self, other: &Self) -> bool {
        self.shape == other.shape && self.shape_fn == other.shape_fn && Arc::ptr_eq(&self.layout, &other.layout)
    }
}

#[widget($crate::node::TouchCaret)]
struct TouchCaret(WidgetBase);
impl TouchCaret {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|b| {
            let input = b.capture_value::<TouchCaretInput>(property_id!(touch_caret_input)).unwrap();

            let shape = (input.shape_fn)(input.shape);
            b.set_child(shape);

            let ctx = input.ctx.clone();
            let shape = input.shape;
            let c_layout = input.layout.clone();
            let parent_id = input.parent_id;
            b.push_intrinsic(NestGroup::SIZE, "touch_caret", move |c| {
                Self::touch_caret(c, ctx, c_layout, shape, parent_id)
            });
        });
    }

    fn touch_caret(
        child: impl UiNode,
        mut ctx: LocalContext,
        c_layout: Arc<Mutex<CaretLayout>>,
        shape: CaretShape,
        parent_id: WidgetId,
    ) -> impl UiNode {
        let mut caret_mid_buf = Some(Arc::new(Atomic::new(Px(0))));
        let mut touch_move = None::<(TouchId, EventHandles)>;
        let mut touch_area = PxSize::zero();

        match_node(child, move |c, op| {
            ctx.with_context_blend(false, || match op {
                UiNodeOp::Init => {
                    WIDGET.sub_event(&TOUCH_INPUT_EVENT);
                }
                UiNodeOp::Deinit => {
                    touch_move = None;
                }
                UiNodeOp::Event { update } => {
                    c.event(update);

                    if let Some(args) = TOUCH_INPUT_EVENT.on_unhandled(update) {
                        if args.is_touch_start() {
                            let mut handles = EventHandles::dummy();
                            handles.push(TOUCH_MOVE_EVENT.subscribe(WIDGET.id()));
                            handles.push(POINTER_CAPTURE_EVENT.subscribe(WIDGET.id()));
                            touch_move = Some((args.touch, handles));
                            POINTER_CAPTURE.capture_subtree(WIDGET.id());
                        } else {
                            touch_move = None;
                        }
                    } else if let Some(args) = TOUCH_MOVE_EVENT.on_unhandled(update) {
                        if let Some((id, _)) = &touch_move {
                            for t in &args.touches {
                                if t.touch == *id {
                                    let pos = t.position();
                                    let op = match shape {
                                        CaretShape::Insert => TextSelectOp::nearest_to(pos),
                                        _ => TextSelectOp::select_index_nearest_to(pos, c_layout.lock().is_selection_index),
                                    };
                                    SELECT_CMD.scoped(parent_id).notify_param(op);
                                    break;
                                }
                            }
                        }
                    } else if let Some(args) = POINTER_CAPTURE_EVENT.on(update) {
                        if args.is_lost(WIDGET.id()) {
                            touch_move = None;
                        }
                    }
                }
                UiNodeOp::Layout { wl, final_size } => {
                    *final_size = TOUCH_CARET_MID.with_context(&mut caret_mid_buf, || c.layout(wl));
                    touch_area = *final_size;
                    let mid = caret_mid_buf.as_ref().unwrap().load(Ordering::Relaxed);

                    let mut c_layout = c_layout.lock();

                    if c_layout.width != final_size.width || c_layout.mid != mid {
                        UPDATES.layout(parent_id);
                        c_layout.width = final_size.width;
                        c_layout.mid = mid;
                    }
                }
                UiNodeOp::Render { frame } => {
                    let l = c_layout.lock();

                    c.delegated();

                    let mut transform = l.inner_text;

                    if l.x > Px::MIN && l.y > Px::MIN {
                        transform = transform.then(&PxTransform::from(PxVector::new(l.x, l.y)));
                        frame.push_inner_transform(&transform, |frame| {
                            c.render(frame);
                            frame.hit_test().push_rect(PxRect::from_size(touch_area));
                        });
                    }
                }
                op => c.op(op),
            })
        })
    }
}
#[property(CONTEXT, capture, widget_impl(TouchCaret))]
fn touch_caret_input(input: impl IntoValue<TouchCaretInput>) {}

/// Default touch caret shape.
///
/// See [`caret_touch_shape`] for more details.
///
/// [`caret_touch_shape`]: fn@super::caret_touch_shape
pub fn default_touch_caret(shape: CaretShape) -> impl UiNode {
    match_node_leaf(move |op| match op {
        UiNodeOp::Layout { final_size, .. } => {
            let factor = LAYOUT.scale_factor();
            let size = Dip::new(16).to_px(factor);
            *final_size = PxSize::splat(size);
            final_size.height += LayoutText::get().shaped_text.line_height();

            let caret_thickness = Dip::new(1).to_px(factor);

            let caret_offset = match shape {
                CaretShape::SelectionLeft => {
                    final_size.width *= 0.8;
                    final_size.width - caret_thickness / 2.0 // rounds .5 to 1, to match `render_caret`
                }
                CaretShape::SelectionRight => {
                    final_size.width *= 0.8;
                    caret_thickness / 2 // rounds .5 to 0
                }
                CaretShape::Insert => final_size.width / 2 - caret_thickness / 2,
            };
            set_touch_caret_mid(caret_offset);
        }
        UiNodeOp::Render { frame } => {
            let size = Dip::new(16).to_px(frame.scale_factor());
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

            let caret_thickness = Dip::new(1).to_px(frame.scale_factor());

            let line_pos = match shape {
                CaretShape::SelectionLeft => PxPoint::new(size.width - caret_thickness, Px(0)),
                CaretShape::Insert => PxPoint::new(size.width / 2 - caret_thickness / 2, Px(0)),
                CaretShape::SelectionRight => PxPoint::zero(),
            };

            let rect = PxRect::new(line_pos, PxSize::new(caret_thickness, line_height));
            frame.push_color(rect, FrameValue::Value(colors::AZURE.into()));
        }
        _ => {}
    })
}

context_local! {
    static TOUCH_CARET_MID: Atomic<Px> = Atomic::new(Px(0));
}

/// Set the ***x*** offset to the middle of the caret line in the touch caret shape.
/// ///
/// See [`caret_touch_shape`] for more details.
///
/// [`caret_touch_shape`]: fn@super::caret_touch_shape
pub fn set_touch_caret_mid(caret_line_middle: Px) {
    TOUCH_CARET_MID.get().store(caret_line_middle, Ordering::Relaxed);
}

/// An Ui node that renders the text selection background.
///
/// The `Text!` widgets introduces this node in `new_child`, around the [`render_text`] node.
pub fn render_selection(child: impl UiNode) -> impl UiNode {
    let mut is_focused = false;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_render(&SELECTION_COLOR_VAR);
            is_focused = false;
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                let new_is_focused = args.is_focused(WIDGET.id());
                if is_focused != new_is_focused {
                    WIDGET.render();
                    is_focused = new_is_focused;
                }
            }
        }
        UiNodeOp::Render { frame } => {
            let r_txt = ResolvedText::get();

            if let Some(range) = r_txt.caret.lock().selection_range() {
                let l_txt = LayoutText::get();
                let r_txt = r_txt.segmented_text.text();

                let mut selection_color = SELECTION_COLOR_VAR.get();
                if !is_focused {
                    selection_color = selection_color.desaturate(100.pct());
                }

                for line_rect in l_txt.shaped_text.highlight_rects(range, r_txt) {
                    if !line_rect.size.is_empty() {
                        frame.push_color(line_rect, FrameValue::Value(selection_color.into()));
                    }
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
                        let mut push_font_glyphs = |font: &Font, glyphs, offset: Option<euclid::Vector2D<f32, Px>>| {
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

                    let mut push_font_glyphs = |font: &Font, glyphs: Cow<[GlyphInstance]>| {
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
    zero_ui_wgt_size_offset::width(child, width)
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
                    Some(i) => CaretStatus::new(i.index, &t.segmented_text),
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
                    Some(i) => CaretStatus::new(i.index, &t.segmented_text),
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

pub(super) fn parse_text<T>(child: impl UiNode, value: impl IntoVar<T>) -> impl UiNode
where
    T: super::TxtParseValue,
{
    let value = value.into_var();

    let error = var(Txt::from_static(""));
    let mut _error_note = DataNoteHandle::dummy();

    #[derive(Clone, Copy, bytemuck::NoUninit)]
    #[repr(u8)]
    enum State {
        Sync,
        Requested,
        Pending,
    }
    let state = Arc::new(Atomic::new(State::Sync));

    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            let ctx = ResolvedText::get();

            // initial T -> Txt sync
            let _ = ctx.txt.set_from_map(&value, |val| val.to_txt());

            // bind `TXT_PARSE_LIVE_VAR` <-> `value` using `bind_filter_map_bidi`:
            // - in case of parse error, it is set in `error` variable, that is held by the binding.
            // - on error update the DATA note is updated.
            // - in case parse is not live, ignores updates (Txt -> None), sets `state` to `Pending`.
            // - in case of Pending and `PARSE_CMD` state is set to `Requested` and `TXT_PARSE_LIVE_VAR.update()`.
            // - the pending state is also tracked in `TXT_PARSE_PENDING_VAR` and the `PARSE_CMD` handle.

            let live = TXT_PARSE_LIVE_VAR.actual_var();
            let is_pending = TXT_PARSE_PENDING_VAR.actual_var();
            let cmd_handle = Arc::new(super::cmd::PARSE_CMD.scoped(WIDGET.id()).subscribe(false));

            let binding = ctx.txt.bind_filter_map_bidi(
                &value,
                clmv!(state, error, is_pending, cmd_handle, |txt| {
                    if live.get() || matches!(state.load(Ordering::Relaxed), State::Requested) {
                        // can try parse

                        if !matches!(state.swap(State::Sync, Ordering::Relaxed), State::Sync) {
                            // exit pending state, even if it parse fails
                            let _ = is_pending.set(false);
                            cmd_handle.set_enabled(false);
                        }

                        // try parse
                        match T::from_txt(txt) {
                            Ok(val) => {
                                error.set(Txt::from_static(""));
                                Some(val)
                            }
                            Err(e) => {
                                error.set(e);
                                None
                            }
                        }
                    } else {
                        // cannot try parse

                        if !matches!(state.swap(State::Pending, Ordering::Relaxed), State::Pending) {
                            // enter pending state
                            let _ = is_pending.set(true);
                            cmd_handle.set_enabled(true);
                        }

                        // does not update the value
                        None
                    }
                }),
                clmv!(state, error, |val| {
                    // value updated externally, exit error, exit pending.

                    error.set(Txt::from_static(""));

                    if !matches!(state.swap(State::Sync, Ordering::Relaxed), State::Sync) {
                        let _ = is_pending.set(false);
                        cmd_handle.set_enabled(false);
                    }

                    Some(val.to_txt())
                }),
            );

            // cmd_handle is held by the binding

            WIDGET.sub_var(&TXT_PARSE_LIVE_VAR).sub_var(&error).push_var_handles(binding);
        }
        UiNodeOp::Deinit => {
            _error_note = DataNoteHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = super::cmd::PARSE_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                if matches!(state.load(Ordering::Relaxed), State::Pending) {
                    // requested parse and parse is pending

                    state.store(State::Requested, Ordering::Relaxed);
                    let _ = ResolvedText::get().txt.update();
                    args.propagation().stop();
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(true) = TXT_PARSE_LIVE_VAR.get_new() {
                if matches!(state.load(Ordering::Relaxed), State::Pending) {
                    // enabled live parse and parse is pending

                    let _ = ResolvedText::get().txt.update();
                }
            }

            if let Some(error) = error.get_new() {
                // remove or replace the error

                _error_note = if error.is_empty() {
                    DataNoteHandle::dummy()
                } else {
                    DATA.invalidate(error)
                };
            }
        }
        _ => {}
    })
}

pub(super) fn on_change_stop(child: impl UiNode, mut handler: impl WidgetHandler<ChangeStopArgs>) -> impl UiNode {
    let mut pending = None;
    match_node(child, move |c, op| match op {
        UiNodeOp::Event { update } => {
            if pending.is_none() {
                return;
            }

            if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                if let (KeyState::Pressed, Key::Enter) = (args.state, &args.key) {
                    if !ACCEPTS_ENTER_VAR.get() {
                        pending = None;
                        handler.event(&ChangeStopArgs {
                            cause: ChangeStopCause::Enter,
                        });
                    }
                }
            } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                let target = WIDGET.id();
                if args.is_blur(target) {
                    pending = None;
                    handler.event(&ChangeStopArgs {
                        cause: ChangeStopCause::Blur,
                    });
                }
            }
        }
        UiNodeOp::Update { updates } => {
            if ResolvedText::get().txt.is_new() {
                let deadline = TIMERS.deadline(CHANGE_STOP_DELAY_VAR.get());
                deadline.subscribe(UpdateOp::Update, WIDGET.id()).perm();
                pending = Some(deadline);
            } else if let Some(p) = &pending {
                if p.get().has_elapsed() {
                    pending = None;

                    handler.event(&ChangeStopArgs {
                        cause: ChangeStopCause::DelayElapsed,
                    });
                }
            }

            c.update(updates);
            handler.update();
        }
        _ => {}
    })
}

/// Implements the selection toolbar.
pub fn selection_toolbar_node(child: impl UiNode) -> impl UiNode {
    use super::node::*;

    let mut selection_range = None;
    let mut popup_state = None::<ReadOnlyArcVar<PopupState>>;
    match_node(child, move |c, op| {
        let mut open = false;
        let mut close = false;
        match op {
            UiNodeOp::Init => {
                WIDGET.sub_var(&SELECTION_TOOLBAR_FN_VAR);
            }
            UiNodeOp::Deinit => {
                close = true;
            }
            UiNodeOp::Event { update } => {
                c.event(update);

                let open_id = || {
                    if let Some(popup_state) = &popup_state {
                        if let PopupState::Open(id) = popup_state.get() {
                            return Some(id);
                        }
                    }
                    None
                };

                if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                    if open_id().map(|id| !args.target.contains(id)).unwrap_or(false) {
                        close = true;
                    }
                    if args.state == ButtonState::Released {
                        open = true;
                    }
                } else if TOUCH_LONG_PRESS_EVENT.has(update) {
                    open = true;
                } else if KEY_INPUT_EVENT.has(update) {
                    close = true;
                } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                    if args.is_blur(WIDGET.id())
                        && open_id()
                            .and_then(|id| args.new_focus.as_ref().map(|p| !p.contains(id)))
                            .unwrap_or(false)
                    {
                        close = true;
                    }
                } else if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
                    if matches!(args.phase, TouchPhase::Start | TouchPhase::Move)
                        && open_id().map(|id| !args.target.contains(id)).unwrap_or(false)
                    {
                        close = true;
                    }
                }

                if popup_state.is_some() {
                    let r_txt = ResolvedText::get();
                    if selection_range != r_txt.caret.lock().selection_range() {
                        close = true;
                    }
                }
            }
            UiNodeOp::Update { .. } => {
                if SELECTION_TOOLBAR_FN_VAR.is_new() {
                    close = true;
                }
            }
            _ => {}
        }
        if close {
            if let Some(state) = &popup_state.take() {
                selection_range = None;
                POPUP.close(state);
            }
        }
        if open {
            if let Some(range) = ResolvedText::get().caret.lock().selection_range() {
                selection_range = Some(range);

                let toolbar_fn = SELECTION_TOOLBAR_FN_VAR.get();
                if !toolbar_fn.is_nil() {
                    let (node, _) = toolbar_fn(SelectionToolbarArgs { anchor_id: WIDGET.id() }).init_widget();

                    let mut translate = PxVector::zero();
                    let transform_key = FrameValueKey::new_unique();
                    let node = match_widget(node, move |c, op| match op {
                        UiNodeOp::Init => {
                            c.init();
                            // c.with_context(|| );// SELECTION_TOOLBAR_ANCHOR_VAR subscribe TODO
                        }
                        UiNodeOp::Layout { wl, final_size } => {
                            let r_txt = ResolvedText::get();
                            if let Some(range) = r_txt.caret.lock().selection_range() {
                                let l_txt = LayoutText::get();
                                let r_txt = r_txt.segmented_text.text();

                                let mut bounds = PxBox::new(PxPoint::splat(Px::MAX), PxPoint::splat(Px::MIN));
                                for line_rect in l_txt.shaped_text.highlight_rects(range, r_txt) {
                                    if !line_rect.size.is_empty() {
                                        let line_box = line_rect.to_box2d();
                                        bounds.min = bounds.min.min(line_box.min);
                                        bounds.max = bounds.max.max(line_box.max);
                                    }
                                }
                                let selection_bounds = bounds.to_rect();

                                *final_size = c.layout(wl);

                                let offset = SELECTION_TOOLBAR_ANCHOR_VAR.get();

                                fn layout_offset(size: PxSize, point: Point) -> PxVector {
                                    LAYOUT
                                        .with_constraints(PxConstraints2d::new_exact_size(size), || point.layout())
                                        .to_vector()
                                }
                                let place = layout_offset(selection_bounds.size, offset.place);
                                let origin = layout_offset(*final_size, offset.origin);

                                translate = selection_bounds.origin.to_vector() + place - origin;
                            } else {
                                // no selection, must be closing
                                wl.collapse();
                                *final_size = PxSize::zero();
                            };
                        }
                        UiNodeOp::Render { frame } => {
                            let l_txt = LayoutText::get();
                            let transform = l_txt.render_info.lock().transform.then_translate(translate.cast());
                            frame.push_reference_frame(transform_key.into(), FrameValue::Value(transform), true, false, |frame| {
                                c.render(frame)
                            });
                        }
                        UiNodeOp::RenderUpdate { update } => {
                            let l_txt = LayoutText::get();
                            let transform = l_txt.render_info.lock().transform.then_translate(translate.cast());
                            update.with_transform(transform_key.update(transform, true), false, |update| c.render_update(update));
                        }
                        _ => {}
                    });

                    // capture all context including LayoutText, exclude text style properties.
                    let capture = ContextCapture::CaptureBlend {
                        filter: CaptureFilter::Exclude({
                            let mut exclude = ContextValueSet::new();
                            super::Text::context_vars_set(&mut exclude);

                            let mut allow = ContextValueSet::new();
                            super::LangMix::<()>::context_vars_set(&mut allow);
                            exclude.remove_all(&allow);

                            exclude
                        }),
                        over: false,
                    };

                    let mut base_mode = AnchorMode::tooltip();
                    base_mode.transform = AnchorTransform::None;
                    popup_state = Some(POPUP.open_config(node, base_mode, capture));
                }
            };
        }
    })
}
