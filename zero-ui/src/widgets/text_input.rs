//! Text input widget, properties and nodes..

use crate::prelude::new_widget::*;

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
#[widget($crate::widgets::TextInput)]
pub struct TextInput(StyleMix<UndoMix<EnabledMix<text::Text>>>);
impl TextInput {
    fn widget_intrinsic(&mut self) {
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
            style_fn = STYLE_VAR;
        }
    }
}

context_var! {
    /// Text input style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// Idle background dark and light color.
    pub static BASE_COLORS_VAR: ColorPair = (rgb(0.12, 0.12, 0.12), rgb(0.88, 0.88, 0.88));
}

/// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the text input style.
#[property(CONTEXT, default(BASE_COLORS_VAR), widget_impl(DefaultStyle))]
pub fn base_colors(child: impl UiNode, color: impl IntoVar<ColorPair>) -> impl UiNode {
    with_context_var(child, BASE_COLORS_VAR, color)
}

/// Sets the text input style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the text input style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
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

/// Text input default style.
#[widget($crate::widgets::text_input::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            padding = (7, 15);
            crate::properties::cursor = CursorIcon::Text;
            crate::properties::background_color = color_scheme_pair(BASE_COLORS_VAR);
            crate::properties::border = {
                widths: 1,
                sides: border_color().map_into(),
            };

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
#[widget($crate::widgets::text_input::FieldStyle)]
pub struct FieldStyle(DefaultStyle);
impl FieldStyle {
    fn widget_intrinsic(&mut self) {
        let top_notes = var(data_context::DataNotes::default());

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

        widget_set! {
            self;
            data_context::get_data_notes_top = top_notes.clone();

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
                wgt_fn!(top_txt, top_color, |_| crate::widgets::Text! {
                    txt = top_txt.clone();
                    font_color = top_color.clone();
                    font_size = 0.8.em();
                    align = Align::BOTTOM_START;
                    offset = (4, 2.dip() + 100.pct());
                })
            } else if !help_empty {
                wgt_fn!(|_| crate::widgets::Text! {
                    txt = FIELD_HELP_VAR;
                    font_size = 0.8.em();
                    font_color = crate::widgets::text::FONT_COLOR_VAR.map(|c| colors::GRAY.with_alpha(10.pct()).mix_normal(*c));
                    align = Align::BOTTOM_START;
                    offset = (4, 2.dip() + 100.pct());
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
    self::adorner_fn(child, adorner_fn)
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
