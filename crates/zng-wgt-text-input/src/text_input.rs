use colors::{ACCENT_COLOR_VAR, BASE_COLOR_VAR};
use zng_ext_clipboard::{COPY_CMD, CUT_CMD, PASTE_CMD};
use zng_wgt::base_color;
use zng_wgt::{align, is_disabled, margin, prelude::*};
use zng_wgt_access::{AccessRole, access_role};
use zng_wgt_button::Button;
use zng_wgt_data::{DATA, DataNoteLevel, DataNotes};
use zng_wgt_fill::foreground_highlight;
use zng_wgt_filter::{child_opacity, saturate};
use zng_wgt_input::{
    focus::{focusable, is_return_focus},
    pointer_capture::capture_pointer,
};
use zng_wgt_layer::popup;
use zng_wgt_menu::{
    self as menu,
    context::{ContextMenu, context_menu_fn},
};
use zng_wgt_rule_line::hr::Hr;
use zng_wgt_size_offset::{offset, y};
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};
use zng_wgt_text::{self as text, *};
use zng_wgt_undo::{UndoMix, undo_scope};

/// Simple text editor widget.
///
/// If `txt` is set to a variable that can be modified the widget becomes interactive, it implements
/// the usual *text box* capabilities: keyboard editing of text in a single style, pointer
/// caret positioning and text selection.
///
/// You can also use [`text::cmd`] to edit the text.
///
/// [`text::cmd`]: zng_wgt_text::cmd
///
/// # Undo/Redo
///
/// Undo/redo is enabled by default, the widget is an undo scope and handles undo commands. Note that external
/// changes to the `txt` variable clears the undo stack, only changes done by the widget can be undone.
///
/// # Shorthand
///
/// The `TextInput!` macro provides shorthand syntax that sets the `txt` property.
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::prelude::*;
/// # use zng_wgt_text_input::*;
/// #
/// # fn main() {
/// let editable_text = TextInput!(var_from(""));
/// # }
/// ```
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
        use zng_wgt::border;
        use zng_wgt_container::*;
        use zng_wgt_fill::*;
        use zng_wgt_input::{focus::is_focused, *};
        use zng_wgt_layer::*;

        widget_set! {
            self;
            replace = true;
            padding = (7, 10);
            cursor = CursorIcon::Text;
            base_color = light_dark(rgb(0.88, 0.88, 0.88), rgb(0.12, 0.12, 0.12));
            background_color = BASE_COLOR_VAR.rgba();
            border = {
                widths: 1,
                sides: BASE_COLOR_VAR.shade_fct_into(0.20),
            };

            popup::context_capture = default_popup_context_capture();
            context_menu_fn = WidgetFn::new(default_context_menu);
            selection_toolbar_fn = WidgetFn::new(default_selection_toolbar);
            selection_color = ACCENT_COLOR_VAR.rgba_map(|c| c.with_alpha(30.pct()));

            when *#is_cap_hovered || *#is_return_focus {
                border = {
                    widths: 1,
                    sides: BASE_COLOR_VAR.shade_fct_into(0.30),
                };
            }

            when *#is_focused {
                border = {
                    widths: 1,
                    sides: ACCENT_COLOR_VAR.rgba_into(),
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

/// Text input style for a search field.
#[widget($crate::SearchStyle)]
pub struct SearchStyle(DefaultStyle);
impl SearchStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            zng_wgt_container::padding = (7, 10, 7, 0);
            zng_wgt_access::access_role = zng_wgt_access::AccessRole::SearchBox;
            auto_selection = true;

            zng_wgt_container::child_out_start = zng_wgt_container::Container! {
                child = zng_wgt::ICONS.req("search");
                zng_wgt::align = Align::CENTER;
                zng_wgt::hit_test_mode = false;
                zng_wgt_size_offset::size = 18;
                zng_wgt::margin = DIRECTION_VAR.map(|d| match d {
                    LayoutDirection::LTR => (0, 0, 0, 6),
                    LayoutDirection::RTL => (0, 6, 0, 0),
                }.into());
            }, 0;

            zng_wgt_container::child_out_end = zng_wgt_button::Button! {
                zng_wgt::corner_radius = 0;
                style_fn = zng_wgt_button::LightStyle!();
                child = zng_wgt::ICONS.req("backspace");
                focusable = false;
                zng_wgt::visibility = zng_var::contextual_var(|| {
                    zng_wgt_text::node::TEXT.resolved().txt.clone().map(|t| match t.is_empty() {
                        true => Visibility::Collapsed,
                        false => Visibility::Visible,
                    })
                });
                on_click = hn!(|args: &zng_ext_input::gesture::ClickArgs| {
                    args.propagation().stop();
                    zng_wgt_text::cmd::EDIT_CMD
                        .scoped(WIDGET.info().parent().unwrap().id())
                        .notify_param(zng_wgt_text::cmd::TextEditOp::clear());
                });
            }, 0;
        }
    }
}

/// Text shown when the `txt` is empty.
///
/// The placeholder has the same text style as the parent widget, with 50% opacity.
/// You can use the [`placeholder`](fn@placeholder) to use a custom widget placeholder.
#[property(CHILD, default(""), widget_impl(TextInput))]
pub fn placeholder_txt(child: impl UiNode, txt: impl IntoVar<Txt>) -> impl UiNode {
    placeholder(
        child,
        Text! {
            txt;
            zng_wgt_filter::opacity = 50.pct();
            zng_wgt::hit_test_mode = false;
        },
    )
}

/// Widget shown when the `txt` is empty.
///
/// The `placeholder` can be any widget, the `Text!` widget is recommended.
#[property(CHILD, widget_impl(TextInput))]
pub fn placeholder(child: impl UiNode, placeholder: impl UiNode) -> impl UiNode {
    let mut txt_is_empty = None;
    zng_wgt_container::child_under(
        child,
        match_widget(placeholder, move |c, op| match op {
            UiNodeOp::Init => {
                let is_empty = zng_wgt_text::node::TEXT.resolved().txt.map(|t| t.is_empty());
                WIDGET.sub_var_render(&is_empty);
                txt_is_empty = Some(is_empty);
            }
            UiNodeOp::Deinit => {
                txt_is_empty = None;
            }
            UiNodeOp::Render { frame } => {
                c.delegated();
                if txt_is_empty.as_ref().unwrap().get() {
                    c.render(frame);
                } else {
                    frame.hide(|frame| c.render(frame));
                }
            }
            UiNodeOp::RenderUpdate { update } => {
                c.delegated();
                if txt_is_empty.as_ref().unwrap().get() {
                    c.render_update(update);
                } else {
                    update.hidden(|update| c.render_update(update));
                }
            }
            _ => {}
        }),
    )
}

/// Text input style that shows data notes, info, warn and error.
///
/// You can also set the [`field_help`] property in text inputs with this style to set a text that
/// shows in place of the data notes when there are none.
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
        let top_txt = top_level_and_txt.map(|(_, t)| t.clone());
        let top_color = DATA.note_color(top_level_and_txt.map(|(l, _)| *l));

        let highlight = top_level_and_txt.map(|(l, _)| *l >= DataNoteLevel::WARN);
        let adorn = merge_var!(top_txt.clone(), FIELD_HELP_VAR, |t, h| (t.is_empty(), h.is_empty()));

        let chars_count = var(0usize);
        let has_max_count = MAX_CHARS_COUNT_VAR.map(|&c| c > 0);

        widget_set! {
            self;
            zng_wgt_data::get_data_notes_top = top_notes.clone();
            get_chars_count = chars_count.clone();
            auto_selection = true;

            foreground_highlight = {
                offsets: -2, // -1 border plus -1 to be outside
                widths: 1,
                sides: merge_var!(highlight, top_color.clone(), |&h, &c| if h {
                    c.into()
                } else {
                    BorderSides::hidden()
                }),
            };
            data_notes_adorner_fn = adorn.map(move |&(top_txt_empty, help_empty)| {
                if !top_txt_empty {
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
                }
            });

            max_chars_count_adorner_fn = has_max_count.map(move |&has| {
                if has {
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
                }
            });

            margin = (0, 0, 1.2.em(), 0);
        }
    }
}

/// Adorner property used by [`FieldStyle`] to show data info, warn and error.
///
/// [`FieldStyle`]: struct@FieldStyle
#[property(FILL, default(WidgetFn::nil()))]
pub fn data_notes_adorner_fn(child: impl UiNode, adorner_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    zng_wgt_layer::adorner_fn(child, adorner_fn)
}

/// Adorner property used by [`FieldStyle`] to show the count/max indicator.
///
/// [`FieldStyle`]: struct@FieldStyle
#[property(FILL, default(WidgetFn::nil()))]
pub fn max_chars_count_adorner_fn(child: impl UiNode, adorner_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    zng_wgt_layer::adorner_fn(child, adorner_fn)
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
