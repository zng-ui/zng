use std::{borrow::Cow, num::Wrapping, sync::Arc};

use parking_lot::RwLock;
use zng_app::{
    access::{ACCESS_SELECTION_EVENT, ACCESS_TEXT_EVENT},
    event::{CommandHandle, EventHandle},
    render::FontSynthesis,
    update::{EventUpdate, UpdateOp},
    widget::{
        WIDGET,
        info::INTERACTIVITY_CHANGED_EVENT,
        node::{UiNode, UiNodeOp, match_node},
    },
    window::WINDOW,
};
use zng_ext_clipboard::{CLIPBOARD, COPY_CMD, CUT_CMD, PASTE_CMD};
use zng_ext_font::{CaretIndex, FONT_CHANGED_EVENT, FONTS, FontFaceList, SegmentedText};
use zng_ext_input::{
    focus::{FOCUS, FOCUS_CHANGED_EVENT, FocusInfoBuilder, WidgetInfoFocusExt as _},
    keyboard::{KEY_INPUT_EVENT, KEYBOARD},
};
use zng_ext_l10n::LANG_VAR;
use zng_ext_undo::UNDO;
use zng_ext_window::{IME_EVENT, WINDOW_Ext as _, WindowLoadingHandle, cmd::CANCEL_IME_CMD};
use zng_layout::context::{DIRECTION_VAR, LayoutDirection};
use zng_view_api::keyboard::{Key, KeyState};
use zng_wgt::prelude::*;

use crate::{
    ACCEPTS_ENTER_VAR, ACCEPTS_TAB_VAR, AUTO_SELECTION_VAR, AutoSelection, FONT_FAMILY_VAR, FONT_STRETCH_VAR, FONT_STYLE_VAR,
    FONT_SYNTHESIS_VAR, FONT_WEIGHT_VAR, MAX_CHARS_COUNT_VAR, OBSCURE_TXT_VAR, TEXT_EDITABLE_VAR, TEXT_SELECTABLE_VAR, TEXT_TRANSFORM_VAR,
    WHITE_SPACE_VAR,
    cmd::{EDIT_CMD, SELECT_ALL_CMD, SELECT_CMD, TextEditOp, TextSelectOp, UndoTextEditOp},
};

use super::{CaretInfo, ImePreview, PendingLayout, RESOLVED_TEXT, ResolvedText, RichTextCopyParam, SelectionBy, TEXT};

/// An UI node that resolves the text context vars, applies the text transform and white space correction and segments the `text`.
///
/// This node setups the [`ResolvedText`] for all inner nodes, the `Text!` widget includes this node in the [`NestGroup::EVENT`] group,
/// so all properties except [`NestGroup::CONTEXT`] have access using the [`TEXT::resolved`] method.
///
/// This node also sets the accessibility label to the resolved text.
///
/// [`NestGroup::EVENT`]: zng_wgt::prelude::NestGroup::EVENT
/// [`NestGroup::CONTEXT`]: zng_wgt::prelude::NestGroup::CONTEXT
pub fn resolve_text(child: impl UiNode, text: impl IntoVar<Txt>) -> impl UiNode {
    let child = resolve_text_font(child);
    let child = resolve_text_access(child);
    let child = resolve_text_edit(child);
    let child = resolve_text_segments(child);
    resolve_text_context(child, text.into_var())
}
fn resolve_text_context(child: impl UiNode, text: Var<Txt>) -> impl UiNode {
    let mut resolved = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            resolved = Some(Arc::new(RwLock::new(ResolvedText {
                txt: text.clone(),
                ime_preview: None,
                synthesis: FontSynthesis::empty(),
                faces: FontFaceList::empty(),
                segmented_text: SegmentedText::new(Txt::from_static(""), LayoutDirection::LTR),
                pending_layout: PendingLayout::empty(),
                pending_edit: false,
                caret: CaretInfo {
                    opacity: var(0.fct()).read_only(),
                    index: None,
                    selection_index: None,
                    initial_selection: None,
                    index_version: Wrapping(0),
                    used_retained_x: false,
                    skip_next_scroll: false,
                },
                selection_by: SelectionBy::Command,
                selection_toolbar_is_open: false,
            })));

            RESOLVED_TEXT.with_context(&mut resolved, || child.init());
        }
        UiNodeOp::Deinit => {
            RESOLVED_TEXT.with_context(&mut resolved, || child.deinit());

            resolved = None;
        }
        UiNodeOp::Layout { wl, final_size } => RESOLVED_TEXT.with_context(&mut resolved, || {
            *final_size = child.layout(wl);
            TEXT.resolve().pending_layout = PendingLayout::empty();
        }),
        op => RESOLVED_TEXT.with_context(&mut resolved, || child.op(op)),
    })
}
fn resolve_text_font(child: impl UiNode) -> impl UiNode {
    enum State {
        Reload,
        Loading {
            response: ResponseVar<FontFaceList>,
            _update_handle: VarHandle,
            _window_load_handle: Option<WindowLoadingHandle>,
        },
        Loaded,
    }
    let mut state = State::Reload;

    match_node(child, move |_, op| {
        match op {
            UiNodeOp::Init => {
                WIDGET
                    .sub_var(&FONT_FAMILY_VAR)
                    .sub_var(&FONT_STYLE_VAR)
                    .sub_var(&FONT_WEIGHT_VAR)
                    .sub_var(&FONT_STRETCH_VAR)
                    .sub_event(&FONT_CHANGED_EVENT)
                    .sub_var(&FONT_SYNTHESIS_VAR);
            }
            UiNodeOp::Event { update } => {
                if FONT_CHANGED_EVENT.has(update) {
                    state = State::Reload;
                }
            }
            UiNodeOp::Update { .. } => {
                if FONT_FAMILY_VAR.is_new() || FONT_STYLE_VAR.is_new() || FONT_WEIGHT_VAR.is_new() || FONT_STRETCH_VAR.is_new() {
                    state = State::Reload;
                } else if let State::Loading { response, .. } = &state {
                    if let Some(f) = response.rsp() {
                        let mut txt = TEXT.resolve();
                        txt.synthesis = FONT_SYNTHESIS_VAR.get() & f.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());
                        txt.faces = f;
                        state = State::Loaded;

                        WIDGET.layout();
                    }
                } else if let State::Loaded = &state {
                    if FONT_SYNTHESIS_VAR.is_new() {
                        let mut txt = TEXT.resolve();
                        txt.synthesis =
                            FONT_SYNTHESIS_VAR.get() & txt.faces.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());

                        WIDGET.render();
                    }
                }
            }
            UiNodeOp::Deinit => {
                state = State::Reload;
                return;
            }
            _ => {}
        }

        if let State::Reload = &state {
            let font_list = FONT_FAMILY_VAR.with(|family| {
                LANG_VAR.with(|lang| {
                    FONTS.list(
                        family,
                        FONT_STYLE_VAR.get(),
                        FONT_WEIGHT_VAR.get(),
                        FONT_STRETCH_VAR.get(),
                        lang.best(),
                    )
                })
            });

            if let Some(f) = font_list.rsp() {
                let mut txt = TEXT.resolve();
                txt.synthesis = FONT_SYNTHESIS_VAR.get() & f.best().synthesis_for(FONT_STYLE_VAR.get(), FONT_WEIGHT_VAR.get());
                txt.faces = f;
                state = State::Loaded;

                WIDGET.layout();
            } else {
                state = State::Loading {
                    _update_handle: font_list.subscribe(UpdateOp::Update, WIDGET.id()),
                    response: font_list,
                    _window_load_handle: WINDOW.loading_handle(1.secs()),
                };
            }
        }
    })
}
fn resolve_text_access(child: impl UiNode) -> impl UiNode {
    match_node(child, |child, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var_info(&TEXT.resolved().txt)
                .sub_var_info(&TEXT_EDITABLE_VAR)
                .sub_var_info(&TEXT_SELECTABLE_VAR)
                .sub_var_info(&OBSCURE_TXT_VAR);
        }
        UiNodeOp::Info { info } => {
            let editable = TEXT_EDITABLE_VAR.get();
            if editable || TEXT_SELECTABLE_VAR.get() {
                FocusInfoBuilder::new(info).focusable_passive(true);
            }

            child.info(info);

            if !editable && !OBSCURE_TXT_VAR.get() {
                if let Some(mut a) = info.access() {
                    a.set_label(TEXT.resolved().segmented_text.text().clone());
                }
            }
        }
        _ => {}
    })
}
fn resolve_text_segments(child: impl UiNode) -> impl UiNode {
    match_node(child, |_, op| {
        let mut segment = false;
        match op {
            UiNodeOp::Init => {
                WIDGET
                    .sub_var(&TEXT.resolved().txt)
                    .sub_var(&TEXT_TRANSFORM_VAR)
                    .sub_var(&WHITE_SPACE_VAR)
                    .sub_var(&DIRECTION_VAR)
                    .sub_var(&TEXT_EDITABLE_VAR);

                segment = true;
            }
            UiNodeOp::Update { .. } => {
                segment = TEXT.resolved().txt.is_new()
                    || TEXT_TRANSFORM_VAR.is_new()
                    || WHITE_SPACE_VAR.is_new()
                    || DIRECTION_VAR.is_new()
                    || TEXT_EDITABLE_VAR.is_new();
            }
            _ => {}
        }
        if segment {
            let mut ctx = TEXT.resolve();

            let mut txt = ctx.txt.get();

            if !TEXT_EDITABLE_VAR.get() {
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

            let direction = DIRECTION_VAR.get();
            if ctx.segmented_text.text() != &txt || ctx.segmented_text.base_direction() != direction {
                ctx.segmented_text = SegmentedText::new(txt, direction);

                ctx.pending_layout = PendingLayout::RESHAPE;
                WIDGET.layout();
            }
        }
    })
}
fn resolve_text_edit(child: impl UiNode) -> impl UiNode {
    // Use `ResolveTextEdit::get` to access.
    let mut edit = None::<Box<ResolveTextEdit>>;

    match_node(child, move |child, op| {
        let mut enable = false;
        match op {
            UiNodeOp::Init => {
                WIDGET
                    .sub_var(&TEXT_EDITABLE_VAR)
                    .sub_var(&TEXT_SELECTABLE_VAR)
                    .sub_var(&MAX_CHARS_COUNT_VAR);
                enable = TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get();
            }
            UiNodeOp::Deinit => {
                edit = None;
            }
            UiNodeOp::Update { .. } => {
                if TEXT_EDITABLE_VAR.is_new() || TEXT_SELECTABLE_VAR.is_new() {
                    enable = TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get();
                    if !enable && edit.is_some() {
                        edit = None;
                        TEXT.resolve().caret.opacity = var(0.fct()).read_only();
                    }
                }

                if let Some(edit) = &mut edit {
                    resolve_text_edit_update(edit);
                }
            }
            UiNodeOp::Event { update } => {
                child.event(update);

                if let Some(edit) = &mut edit {
                    if TEXT_EDITABLE_VAR.get() && TEXT.resolved().txt.capabilities().can_modify() {
                        resolve_text_edit_events(update, edit);
                    }
                    if TEXT_EDITABLE_VAR.get() || TEXT_SELECTABLE_VAR.get() {
                        resolve_text_edit_or_select_events(update, edit);
                    }

                    let enable = !OBSCURE_TXT_VAR.get() && TEXT.resolved().caret.selection_range().is_some();
                    edit.cut.set_enabled(enable);
                    edit.copy.set_enabled(enable);
                }
            }
            _ => {}
        }
        if enable {
            let edit = ResolveTextEdit::get(&mut edit);

            let editable = TEXT_EDITABLE_VAR.get();
            if editable {
                let id = WIDGET.id();

                edit.events[0] = FOCUS_CHANGED_EVENT.subscribe(id);
                edit.events[1] = INTERACTIVITY_CHANGED_EVENT.subscribe(id);
                edit.events[2] = KEY_INPUT_EVENT.subscribe(id);
                edit.events[3] = ACCESS_TEXT_EVENT.subscribe(id);
                edit.events[5] = IME_EVENT.subscribe(id);

                edit.paste = PASTE_CMD.scoped(id).subscribe(true);
                edit.edit = EDIT_CMD.scoped(id).subscribe(true);

                edit.max_count = MAX_CHARS_COUNT_VAR.subscribe(UpdateOp::Update, id);

                let mut ctx = TEXT.resolve();

                enforce_max_count(&ctx.txt);

                if FOCUS.is_focused(WIDGET.id()).get() {
                    ctx.caret.opacity = KEYBOARD.caret_animation();
                    edit.caret_animation = ctx.caret.opacity.subscribe(UpdateOp::Update, WIDGET.id());
                }
            }

            if TEXT_SELECTABLE_VAR.get() {
                let id = WIDGET.id();

                edit.events[4] = ACCESS_SELECTION_EVENT.subscribe(id);

                let enabled = !OBSCURE_TXT_VAR.get() && TEXT.resolved().caret.selection_range().is_some();
                edit.copy = COPY_CMD.scoped(id).subscribe(enabled);
                if editable {
                    edit.cut = CUT_CMD.scoped(id).subscribe(enabled);
                } else {
                    // used in `render_selection`
                    edit.events[0] = FOCUS_CHANGED_EVENT.subscribe(id);

                    edit.events[2] = KEY_INPUT_EVENT.subscribe(id);
                }
            }
        }
    })
}
/// Data allocated only when `editable`.
#[derive(Default)]
struct ResolveTextEdit {
    events: [EventHandle; 6],
    caret_animation: VarHandle,
    max_count: VarHandle,
    cut: CommandHandle,
    copy: CommandHandle,
    paste: CommandHandle,
    edit: CommandHandle,
}
impl ResolveTextEdit {
    fn get(edit_data: &mut Option<Box<Self>>) -> &mut Self {
        &mut *edit_data.get_or_insert_with(Default::default)
    }
}
fn enforce_max_count(text: &Var<Txt>) {
    let max_count = MAX_CHARS_COUNT_VAR.get();
    if max_count > 0 {
        let count = text.with(|t| t.chars().count());
        if count > max_count {
            tracing::debug!("txt var set to text longer than can be typed");
            text.modify(move |t| {
                if let Some((i, _)) = t.as_str().char_indices().nth(max_count) {
                    t.to_mut().truncate(i);
                }
            });
        }
    }
}
fn resolve_text_edit_events(update: &EventUpdate, edit: &mut ResolveTextEdit) {
    if let Some(args) = INTERACTIVITY_CHANGED_EVENT.on(update) {
        if args.is_disable(WIDGET.id()) {
            edit.caret_animation = VarHandle::dummy();
            TEXT.resolve().caret.opacity = var(0.fct()).read_only();
        }
    }

    if TEXT.resolved().pending_edit {
        return;
    }
    let widget = WIDGET.info();
    if !widget.interactivity().is_enabled() {
        return;
    }

    let prev_caret = {
        let r = TEXT.resolved();
        (r.caret.index, r.caret.index_version, r.caret.selection_index)
    };

    if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
        let mut ctx = TEXT.resolve();
        if args.state == KeyState::Pressed && args.target.widget_id() == widget.id() {
            match &args.key {
                Key::Backspace => {
                    let caret = &mut ctx.caret;
                    if caret.selection_index.is_some() || caret.index.unwrap_or(CaretIndex::ZERO).index > 0 {
                        if args.modifiers.is_only_ctrl() {
                            args.propagation().stop();
                            ctx.selection_by = SelectionBy::Keyboard;
                            drop(ctx);
                            TextEditOp::backspace_word().call_edit_op();
                        } else if args.modifiers.is_empty() {
                            args.propagation().stop();
                            ctx.selection_by = SelectionBy::Keyboard;
                            drop(ctx);
                            TextEditOp::backspace().call_edit_op();
                        }
                    }
                }
                Key::Delete => {
                    let caret = &mut ctx.caret;
                    let caret_idx = caret.index.unwrap_or(CaretIndex::ZERO);
                    if caret.selection_index.is_some() || caret_idx.index < ctx.segmented_text.text().len() {
                        if args.modifiers.is_only_ctrl() {
                            args.propagation().stop();
                            ctx.selection_by = SelectionBy::Keyboard;
                            drop(ctx);
                            TextEditOp::delete_word().call_edit_op();
                        } else if args.modifiers.is_empty() {
                            args.propagation().stop();
                            ctx.selection_by = SelectionBy::Keyboard;
                            drop(ctx);
                            TextEditOp::delete().call_edit_op();
                        }
                    }
                }
                _ => {
                    let insert = args.insert_str();
                    if !insert.is_empty() {
                        let skip = (args.is_tab() && !ACCEPTS_TAB_VAR.get()) || (args.is_line_break() && !ACCEPTS_ENTER_VAR.get());
                        if !skip {
                            args.propagation().stop();
                            ctx.selection_by = SelectionBy::Keyboard;
                            drop(ctx);
                            TextEditOp::insert(Txt::from_str(insert)).call_edit_op();
                        }
                    }
                }
            }
        }
    } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
        let mut ctx = TEXT.resolve();
        let caret = &mut ctx.caret;
        let caret_index = &mut caret.index;

        if args.is_focused(widget.id()) {
            if caret_index.is_none() {
                *caret_index = Some(CaretIndex::ZERO);
            } else {
                // restore animation when the caret_index did not change
                caret.opacity = KEYBOARD.caret_animation();
                edit.caret_animation = caret.opacity.subscribe(UpdateOp::RenderUpdate, widget.id());
            }
        } else {
            edit.caret_animation = VarHandle::dummy();
            caret.opacity = var(0.fct()).read_only();
        }

        let auto_select = AUTO_SELECTION_VAR.get();
        if auto_select != AutoSelection::DISABLED && caret.selection_index.is_some() && TEXT_SELECTABLE_VAR.get() {
            if auto_select.contains(AutoSelection::CLEAR_ON_BLUR) {
                if let Some(rich) = TEXT.try_rich() {
                    if args.is_focus_leave(rich.root_id) {
                        // deselect if the ALT return and parent scope return are not inside the rich text context

                        if let Some(rich_root) = rich.root_info() {
                            let alt_return = FOCUS.alt_return().with(|p| p.as_ref().map(|p| p.widget_id()));
                            if alt_return.is_none() || rich_root.descendants().all(|d| d.id() != alt_return.unwrap()) {
                                // not ALT return
                                if let Some(info) = WIDGET.info().into_focusable(true, true) {
                                    if let Some(scope) = info.scope() {
                                        let parent_return =
                                            FOCUS.return_focused(scope.info().id()).with(|p| p.as_ref().map(|p| p.widget_id()));
                                        if parent_return.is_none() || rich_root.descendants().all(|d| d.id() != alt_return.unwrap()) {
                                            // not parent scope return
                                            SELECT_CMD.scoped(widget.id()).notify_param(TextSelectOp::next());
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else if args.is_blur(widget.id()) {
                    // deselect if the widget is not the ALT return focus and is not the parent scope return focus.

                    let us = Some(widget.id());
                    let alt_return = FOCUS.alt_return().with(|p| p.as_ref().map(|p| p.widget_id()));
                    if alt_return != us {
                        // not ALT return
                        if let Some(info) = WIDGET.info().into_focusable(true, true) {
                            if let Some(scope) = info.scope() {
                                let parent_return = FOCUS.return_focused(scope.info().id()).with(|p| p.as_ref().map(|p| p.widget_id()));
                                if parent_return != us {
                                    // not parent scope return
                                    SELECT_CMD.scoped(widget.id()).notify_param(TextSelectOp::next());
                                }
                            }
                        }
                    }
                }
            }

            if auto_select.contains(AutoSelection::ALL_ON_FOCUS_KEYBOARD) && args.highlight && args.is_focus(widget.id()) {
                // select all on keyboard caused focus
                SELECT_ALL_CMD.scoped(widget.id()).notify();
            }

            // ALL_ON_FOCUS_POINTER handled by `layout_text_edit_events`
        }
    } else if let Some(args) = CUT_CMD.scoped(widget.id()).on_unhandled(update) {
        let mut ctx = TEXT.resolve();
        if let Some(range) = ctx.caret.selection_char_range() {
            args.propagation().stop();
            ctx.selection_by = SelectionBy::Command;
            CLIPBOARD.set_text(Txt::from_str(&ctx.segmented_text.text()[range]));
            drop(ctx);
            TextEditOp::delete().call_edit_op();
        }
    } else if let Some(args) = PASTE_CMD.scoped(widget.id()).on_unhandled(update) {
        if let Some(paste) = CLIPBOARD.text().ok().flatten() {
            if !paste.is_empty() {
                args.propagation().stop();
                TEXT.resolve().selection_by = SelectionBy::Command;
                TextEditOp::insert(paste).call_edit_op();
            }
        }
    } else if let Some(args) = EDIT_CMD.scoped(widget.id()).on_unhandled(update) {
        if let Some(op) = args.param::<UndoTextEditOp>() {
            args.propagation().stop();

            op.call();
            if !TEXT.resolved().pending_edit {
                TEXT.resolve().pending_edit = true;
                WIDGET.update();
            }
        } else if let Some(op) = args.param::<TextEditOp>() {
            args.propagation().stop();

            op.clone().call_edit_op();
        }
    } else if let Some(args) = ACCESS_TEXT_EVENT.on_unhandled(update) {
        if args.widget_id == widget.id() {
            args.propagation().stop();

            if args.selection_only {
                TextEditOp::insert(args.txt.clone())
            } else {
                let current_len = TEXT.resolved().txt.with(|t| t.len());
                let new_len = args.txt.len();
                TextEditOp::replace(0..current_len, args.txt.clone(), new_len..new_len)
            }
            .call_edit_op();
        }
    } else if let Some(args) = IME_EVENT.on_unhandled(update) {
        let mut resegment = false;

        if let Some((start, end)) = args.preview_caret {
            // update preview txt

            let mut ctx = TEXT.resolve();
            let ctx = &mut *ctx;

            if args.txt.is_empty() {
                if let Some(preview) = ctx.ime_preview.take() {
                    resegment = true;
                    let caret = &mut ctx.caret;
                    caret.set_index(preview.prev_caret);
                    caret.selection_index = preview.prev_selection;
                }
            } else if let Some(preview) = &mut ctx.ime_preview {
                resegment = preview.txt != args.txt;
                if resegment {
                    preview.txt = args.txt.clone();
                }
            } else {
                resegment = true;
                let caret = &mut ctx.caret;
                ctx.ime_preview = Some(ImePreview {
                    txt: args.txt.clone(),
                    prev_caret: caret.index.unwrap_or(CaretIndex::ZERO),
                    prev_selection: caret.selection_index,
                });
            }

            // update preview caret/selection indexes.
            if let Some(preview) = &ctx.ime_preview {
                let caret = &mut ctx.caret;
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
                let mut ctx = TEXT.resolve();
                if let Some(preview) = ctx.ime_preview.take() {
                    // restore caret
                    let caret = &mut ctx.caret;
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
                let mut ctx = TEXT.resolve();
                ctx.selection_by = SelectionBy::Keyboard;

                // if the committed text is equal the last preview reshape is skipped
                // leaving behind the IME underline highlight.
                ctx.pending_layout |= PendingLayout::UNDERLINE;
                WIDGET.layout();

                drop(ctx);
                TextEditOp::insert(args.txt.clone()).call_edit_op();
            }
        }

        if resegment {
            let mut ctx = TEXT.resolve();

            // re-segment text to insert or remove the preview
            let mut text = ctx.txt.get();
            if let Some(preview) = &ctx.ime_preview {
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
            ctx.segmented_text = SegmentedText::new(text, DIRECTION_VAR.get());

            ctx.pending_layout |= PendingLayout::RESHAPE;
            WIDGET.layout();
        }
    }

    let mut ctx = TEXT.resolve();
    let caret = &mut ctx.caret;

    if (caret.index, caret.index_version, caret.selection_index) != prev_caret {
        caret.used_retained_x = false;
        if caret.index.is_none() || !FOCUS.is_focused(widget.id()).get() {
            edit.caret_animation = VarHandle::dummy();
            caret.opacity = var(0.fct()).read_only();
        } else {
            caret.opacity = KEYBOARD.caret_animation();
            edit.caret_animation = caret.opacity.subscribe(UpdateOp::RenderUpdate, widget.id());
        }
        ctx.pending_layout |= PendingLayout::CARET;
        WIDGET.layout(); // update caret_origin
    }
}
fn resolve_text_edit_or_select_events(update: &EventUpdate, _: &mut ResolveTextEdit) {
    let widget_id = WIDGET.id();

    if let Some(args) = COPY_CMD.scoped(widget_id).on_unhandled(update) {
        let ctx = TEXT.resolved();
        if let Some(range) = ctx.caret.selection_char_range() {
            args.propagation().stop();
            let txt = Txt::from_str(&ctx.segmented_text.text()[range]);
            if let Some(rt) = args.param::<RichTextCopyParam>() {
                rt.set_text(txt);
            } else {
                let _ = CLIPBOARD.set_text(txt);
            }
        }
    } else if let Some(args) = ACCESS_SELECTION_EVENT.on_unhandled(update) {
        if args.start.0 == widget_id && args.caret.0 == widget_id {
            args.propagation().stop();

            let mut ctx = TEXT.resolve();

            ctx.caret.set_char_selection(args.start.1, args.caret.1);

            ctx.pending_layout |= PendingLayout::CARET;
            WIDGET.layout();
        }
    }
}
fn resolve_text_edit_update(_: &mut ResolveTextEdit) {
    let mut ctx = TEXT.resolve();
    let ctx = &mut *ctx;
    if ctx.txt.is_new() {
        if !ctx.pending_edit && UNDO.scope() == Some(WIDGET.id()) {
            UNDO.clear();
        }

        if let Some(p) = ctx.ime_preview.take() {
            ctx.caret.index = Some(p.prev_caret);
            ctx.caret.selection_index = p.prev_selection;

            CANCEL_IME_CMD.scoped(WINDOW.id()).notify();
        }

        enforce_max_count(&ctx.txt);

        // prevent invalid indexes
        let caret = &mut ctx.caret;
        if let Some(i) = &mut caret.index {
            i.index = ctx.segmented_text.snap_grapheme_boundary(i.index);
        }
        if let Some(i) = &mut caret.selection_index {
            i.index = ctx.segmented_text.snap_grapheme_boundary(i.index);
        }
        if let Some((cr, _)) = &mut caret.initial_selection {
            cr.start.index = ctx.segmented_text.snap_grapheme_boundary(cr.start.index);
            cr.end.index = ctx.segmented_text.snap_grapheme_boundary(cr.end.index);
        }
    }

    if TEXT_EDITABLE_VAR.get() && MAX_CHARS_COUNT_VAR.is_new() {
        enforce_max_count(&TEXT.resolved().txt);
    }

    // either txt was new or the edit did not change the text.
    ctx.pending_edit = false;
}
