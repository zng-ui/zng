use zero_ui_ext_clipboard::{COPY_CMD, CUT_CMD, PASTE_CMD};
use zero_ui_wgt::{align, is_disabled, margin, prelude::*};
use zero_ui_wgt_access::{access_role, AccessRole};
use zero_ui_wgt_button::Button;
use zero_ui_wgt_data::{DataNoteLevel, DataNotes, DATA};
use zero_ui_wgt_fill::foreground_highlight;
use zero_ui_wgt_filter::{child_opacity, saturate};
use zero_ui_wgt_input::{
    focus::{focusable, is_return_focus},
    pointer_capture::capture_pointer,
};
use zero_ui_wgt_layer::popup;
use zero_ui_wgt_menu::{
    self as menu,
    context::{context_menu_fn, ContextMenu},
};
use zero_ui_wgt_rule_line::hr::Hr;
use zero_ui_wgt_size_offset::{offset, y};
use zero_ui_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};
use zero_ui_wgt_text::{self as text, *};
use zero_ui_wgt_undo::{undo_scope, UndoMix};

/// Simple text editor widget.
///
/// If `txt` is set to a variable that can be modified the widget becomes interactive, it implements
/// the usual *text box* capabilities, keyboard controlled editing of short text in a single style, mouse
/// selecting and caret positioning.
///
/// You can also use the [`text::commands`] to edit the text.
///
/// # Undo/Redo
///
/// Undo/redo is enabled by default, the widget is an undo scope and handles undo commands. Note that external
/// changes to the `txt` variable will clear the undo stack, only changes done by the widget can be undone.
///
/// # Shorthand
///
/// The `TextInput!` macro provides shorthand syntax sets the text variable, `TextInput!(var(Txt::from("")))` creates
/// an editable text input.
#[widget($crate::TextInput {
    ($txt:expr) => {
        txt = $txt;
    };
})]
pub struct TextInput(StyleMix<UndoMix<Text>>);
impl TextInput {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
        widget_set! {
            self;

            access_role = AccessRole::TextInput;
            txt_editable = true;
            txt_selectable = true;
            capture_pointer = true;
            txt_align = Align::TOP_START;
            focusable = true;
            undo_scope = true;
            undo_limit = 100;
            style_base_fn = style_fn!(|_| DefaultStyle!());
        }
    }
}
impl_style_fn!(TextInput);

context_var! {
    /// Idle background dark and light color.
    pub static BASE_COLORS_VAR: ColorPair = (rgb(0.12, 0.12, 0.12), rgb(0.88, 0.88, 0.88));
}

/// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the text input style.
#[property(CONTEXT, default(BASE_COLORS_VAR), widget_impl(DefaultStyle))]
pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
    with_context_var(child, BASE_COLORS_VAR, color)
}

/// Default border color.
pub fn border_color() -> impl Var<Rgba> {
    color_scheme_highlight(BASE_COLORS_VAR, 0.20)
}

/// Border color hovered.
pub fn border_color_hovered() -> impl Var<Rgba> {
    color_scheme_highlight(BASE_COLORS_VAR, 0.30)
}

/// Border color focused.
pub fn border_color_focused() -> impl Var<Rgba> {
    color_scheme_highlight(BASE_COLORS_VAR, 0.40)
}

/// Context menu set by the [`DefaultStyle!`].
///
/// [`DefaultStyle!`]: struct@DefaultStyle
pub fn default_context_menu(args: menu::context::ContextMenuArgs) -> impl UiNode {
    let id = args.anchor_id;
    ContextMenu!(ui_vec![
        Button!(CUT_CMD.scoped(id)),
        Button!(COPY_CMD.scoped(id)),
        Button!(PASTE_CMD.scoped(id)),
        Hr!(),
        Button!(text::cmd::SELECT_ALL_CMD.scoped(id)),
    ])
}

/// Selection toolbar set by the [`DefaultStyle!`].
///
/// [`DefaultStyle!`]: struct@DefaultStyle
pub fn default_selection_toolbar(args: text::SelectionToolbarArgs) -> impl UiNode {
    if args.is_touch {
        let id = args.anchor_id;
        ContextMenu! {
            style_fn = menu::context::TouchStyle!();
            children = ui_vec![
                Button!(CUT_CMD.scoped(id)),
                Button!(COPY_CMD.scoped(id)),
                Button!(PASTE_CMD.scoped(id)),
                Button!(text::cmd::SELECT_ALL_CMD.scoped(id)),
            ]
        }
        .boxed()
    } else {
        NilUiNode.boxed()
    }
}

/// Context captured for the context menu, set by the [`DefaultStyle!`].
///
/// Captures all context vars, except text style vars.
///
/// [`DefaultStyle!`]: struct@DefaultStyle
pub fn default_popup_context_capture() -> popup::ContextCapture {
    popup::ContextCapture::CaptureBlend {
        filter: CaptureFilter::ContextVars {
            exclude: {
                let mut exclude = ContextValueSet::new();
                Text::context_vars_set(&mut exclude);

                let mut allow = ContextValueSet::new();
                LangMix::<()>::context_vars_set(&mut allow);
                exclude.remove_all(&allow);

                exclude
            },
        },
        over: false,
    }
}

/// Text input default style.
#[widget($crate::text_input::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        use zero_ui_wgt::border;
        use zero_ui_wgt_container::*;
        use zero_ui_wgt_fill::*;
        use zero_ui_wgt_input::{focus::is_focused, *};
        use zero_ui_wgt_layer::*;

        widget_set! {
            self;
            replace = true;
            padding = (7, 15);
            cursor = CursorIcon::Text;
            background_color = color_scheme_pair(BASE_COLORS_VAR);
            border = {
                widths: 1,
                sides: border_color().map_into(),
            };

            popup::context_capture = default_popup_context_capture();
            context_menu_fn = WidgetFn::new(default_context_menu);
            selection_toolbar_fn = WidgetFn::new(default_selection_toolbar);

            when *#is_cap_hovered || *#is_return_focus {
                border = {
                    widths: 1,
                    sides: border_color_hovered().map_into(),
                };
            }

            when *#is_focused {
                border = {
                    widths: 1,
                    sides: border_color_focused().map_into(),
                };
            }

            when *#is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}

/// Text input style that shows data notes, info, warn and error.
///
/// You can also set the [`field_help`] in text inputs with this style to set a text that
/// show in the same place as the data notes when there is no note.
///
/// [`field_help`]: fn@field_help
#[widget($crate::FieldStyle)]
pub struct FieldStyle(DefaultStyle);
impl FieldStyle {
    fn widget_intrinsic(&mut self) {
        let top_notes = var(DataNotes::default());

        let top_level_and_txt = top_notes.map(|ns| {
            if let Some(n) = ns.first() {
                return (n.level(), formatx!("{ns}"));
            }
            (DataNoteLevel::INFO, "".into())
        });
        let top_txt = top_level_and_txt.map_ref(|(_, t)| t);
        let top_color = DATA.note_color(top_level_and_txt.map_ref(|(l, _)| l));

        let highlight = top_level_and_txt.map(|(l, _)| *l >= DataNoteLevel::WARN);
        let adorn = merge_var!(top_txt.clone(), FIELD_HELP_VAR, |t, h| (t.is_empty(), h.is_empty()));

        let chars_count = var(0usize);
        let has_max_count = text::MAX_CHARS_COUNT_VAR.map(|&c| c > 0);

        widget_set! {
            self;
            zero_ui_wgt_data::get_data_notes_top = top_notes.clone();
            text::get_chars_count = chars_count.clone();

            foreground_highlight = {
                offsets: -2, // -1 border plus -1 to be outside
                widths: 1,
                sides: merge_var!(highlight, top_color.clone(), |&h, &c| if h {
                    c.into()
                } else {
                    BorderSides::hidden()
                }),
            };
            data_notes_adorner_fn = adorn.map(move |&(top_txt_empty, help_empty)| if !top_txt_empty {
                wgt_fn!(top_txt, top_color, |_| Text! {
                    focusable = false;
                    txt_editable = false;
                    txt_selectable = false;
                    txt = top_txt.clone();
                    font_color = top_color.clone();
                    font_size = 0.8.em();
                    align = Align::BOTTOM_START;
                    margin = (0, 4);
                    y = 2.dip() + 100.pct();
                })
            } else if !help_empty {
                wgt_fn!(|_| Text! {
                    focusable = false;
                    txt_editable = false;
                    txt_selectable = false;
                    txt = FIELD_HELP_VAR;
                    font_size = 0.8.em();
                    font_color = text::FONT_COLOR_VAR.map(|c| colors::GRAY.with_alpha(10.pct()).mix_normal(*c));
                    align = Align::BOTTOM_START;
                    margin = (0, 4);
                    y = 2.dip() + 100.pct();
                })
            } else {
                WidgetFn::nil()
            });

            max_chars_count_adorner_fn = has_max_count.map(move |&has| if has {
                wgt_fn!(chars_count, |_| Text! {
                    focusable = false;
                    txt_editable = false;
                    txt_selectable = false;
                    txt = merge_var!(chars_count.clone(), text::MAX_CHARS_COUNT_VAR, |c, m| formatx!("{c}/{m}"));
                    font_color = text::FONT_COLOR_VAR.map(|c| colors::GRAY.with_alpha(10.pct()).mix_normal(*c));
                    font_size = 0.8.em();
                    align = Align::BOTTOM_END;
                    offset = (-4, 2.dip() + 100.pct());
                })
            } else {
                WidgetFn::nil()
            });

            margin = (0, 0, 1.em(), 0);
        }
    }
}

/// Adorner property used by [`FieldStyle`] to show data info, warn and error.
///
/// [`FieldStyle`]: struct@FieldStyle
#[property(FILL, default(WidgetFn::nil()))]
pub fn data_notes_adorner_fn(child: impl UiNode, adorner_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    zero_ui_wgt_layer::adorner_fn(child, adorner_fn)
}

/// Adorner property used by [`FieldStyle`] to show the count/max indicator when the number of chars is limited.
///
/// [`FieldStyle`]: struct@FieldStyle
#[property(FILL, default(WidgetFn::nil()))]
pub fn max_chars_count_adorner_fn(child: impl UiNode, adorner_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    zero_ui_wgt_layer::adorner_fn(child, adorner_fn)
}

context_var! {
    /// Text shown under a [`FieldStyle`] when it has no data notes (no info, warn or error).
    ///
    /// [`FieldStyle`]: struct@FieldStyle
    pub static FIELD_HELP_VAR: Txt = "";
}

/// Text shown under a [`FieldStyle`] when it has no data notes (no info, warn or error).
///
/// [`FieldStyle`]: struct@FieldStyle
#[property(CONTEXT, default(""))]
pub fn field_help(child: impl UiNode, help: impl IntoVar<Txt>) -> impl UiNode {
    with_context_var(child, FIELD_HELP_VAR, help)
}
