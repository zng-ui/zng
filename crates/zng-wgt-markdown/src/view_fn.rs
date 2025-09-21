use std::num::NonZeroU32;

pub use pulldown_cmark::HeadingLevel;
use zng_ext_font::*;
use zng_ext_image::ImageSource;
use zng_ext_input::gesture::ClickArgs;
use zng_wgt::*;
use zng_wgt_access::{self as access, AccessRole, access_role};
use zng_wgt_button::{Button, LinkStyle};
use zng_wgt_container::{Container, child_align, padding};
use zng_wgt_fill::background_color;
use zng_wgt_filter::opacity;
use zng_wgt_grid::{self as grid, Grid};
use zng_wgt_size_offset::{offset, size};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::{FONT_COLOR_VAR, PARAGRAPH_SPACING_VAR, Text, font_size, font_weight};
use zng_wgt_tooltip::*;
use zng_wgt_transform::scale;
use zng_wgt_wrap::Wrap;

use super::*;

/// Markdown text run style.
#[derive(Default, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub struct MarkdownStyle {
    /// Bold.
    pub strong: bool,
    /// Italic.
    pub emphasis: bool,
    /// Strikethrough.
    pub strikethrough: bool,

    /// !!: TODO
    pub subscript: bool,
    /// !!: TODO
    pub superscript: bool,
}

/// Arguments for a markdown text view.
///
/// The text can be inside a paragraph, heading, list item or any other markdown block item.
///
/// See [`TEXT_FN_VAR`] for more details.
#[non_exhaustive]
pub struct TextFnArgs {
    /// The text run.
    pub txt: Txt,
    /// The style.
    pub style: MarkdownStyle,
}
impl TextFnArgs {
    /// New args.
    pub fn new(txt: impl Into<Txt>, style: MarkdownStyle) -> Self {
        Self { txt: txt.into(), style }
    }
}

/// Arguments for a markdown inlined link view.
///
/// See [`LINK_FN_VAR`] for more details.
#[non_exhaustive]
pub struct LinkFnArgs {
    /// The link.
    pub url: Txt,

    /// Link title, usually displayed as a tooltip.
    pub title: Txt,

    /// Inline items.
    pub items: UiVec,
}
impl LinkFnArgs {
    /// New args.
    pub fn new(url: impl Into<Txt>, title: impl Into<Txt>, items: UiVec) -> Self {
        Self {
            url: url.into(),
            title: title.into(),
            items,
        }
    }
}

/// Arguments for a markdown inlined code text view.
///
/// The text can be inside a paragraph, heading, list item or any other markdown block item.
///
/// See [`CODE_INLINE_FN_VAR`] for more details.
#[non_exhaustive]
pub struct CodeInlineFnArgs {
    /// The code text run.
    pub txt: Txt,
    /// The style.
    pub style: MarkdownStyle,
}
impl CodeInlineFnArgs {
    /// New args.
    pub fn new(txt: impl Into<Txt>, style: MarkdownStyle) -> Self {
        Self { txt: txt.into(), style }
    }
}

/// Arguments for a markdown code block view.
///
/// See [`CODE_BLOCK_FN_VAR`] for more details.
#[non_exhaustive]
pub struct CodeBlockFnArgs {
    /// Code language, can be empty.
    pub lang: Txt,
    /// Raw text.
    pub txt: Txt,
}
impl CodeBlockFnArgs {
    /// New args.
    pub fn new(lang: impl Into<Txt>, txt: impl Into<Txt>) -> Self {
        Self {
            lang: lang.into(),
            txt: txt.into(),
        }
    }
}

/// Arguments for a markdown paragraph view.
///
/// See [`PARAGRAPH_FN_VAR`] for more details.
#[non_exhaustive]
pub struct ParagraphFnArgs {
    /// Zero-sized index of the paragraph.
    pub index: u32,
    /// Inline items.
    pub items: UiVec,
}
impl ParagraphFnArgs {
    /// New args.
    pub fn new(index: u32, items: UiVec) -> Self {
        Self { index, items }
    }
}

/// Arguments for a markdown heading view.
#[non_exhaustive]
pub struct HeadingFnArgs {
    /// Level.
    pub level: HeadingLevel,

    /// Anchor label that identifies the header in the markdown context.
    pub anchor: Txt,

    /// Inline items.
    pub items: UiVec,
}
impl HeadingFnArgs {
    /// New args.
    pub fn new(level: HeadingLevel, anchor: impl Into<Txt>, items: UiVec) -> Self {
        Self {
            level,
            anchor: anchor.into(),
            items,
        }
    }
}

/// Arguments for a markdown list view.
#[non_exhaustive]
pub struct ListFnArgs {
    /// Nested list depth, starting from zero for the outer-list.
    pub depth: u32,

    /// If the list is *ordered*, the first item number.
    pub first_num: Option<u64>,

    /// List items.
    ///
    /// Each two items are the bullet or number followed by the item.
    pub items: UiVec,
}
impl ListFnArgs {
    /// New args.
    pub fn new(depth: u32, first_num: Option<u64>, items: UiVec) -> Self {
        Self { depth, first_num, items }
    }
}

/// Arguments for a markdown list item bullet, check mark or number.
#[derive(Clone, Copy)]
#[non_exhaustive]
pub struct ListItemBulletFnArgs {
    /// Nested list depth, starting from zero for items in the outer-list.
    pub depth: u32,

    /// If the list is *ordered*, the item number.
    pub num: Option<u64>,

    /// If the list is checked. `Some(true)` is `[x]` and `Some(false)` is `[ ]`.
    pub checked: Option<bool>,
}
impl ListItemBulletFnArgs {
    /// New args.
    pub fn new(depth: u32, num: Option<u64>, checked: Option<bool>) -> Self {
        Self { depth, num, checked }
    }
}

/// Arguments for a markdown list item view.
#[non_exhaustive]
pub struct ListItemFnArgs {
    /// Copy of the bullet args.
    pub bullet: ListItemBulletFnArgs,

    /// Inline items of the list item.
    pub items: UiVec,

    /// Inner block items, paragraphs and nested lists.
    pub blocks: UiVec,
}
impl ListItemFnArgs {
    /// New args.
    pub fn new(bullet: ListItemBulletFnArgs, items: UiVec, blocks: UiVec) -> Self {
        Self { bullet, items, blocks }
    }
}

/// Arguments for a markdown definition list.
#[non_exhaustive]
pub struct DefListArgs {
    /// List items.
    ///
    /// Each two items are the title and definition.
    pub items: UiVec,
}
impl DefListArgs {
    /// New args.
    pub fn new(items: UiVec) -> Self {
        Self { items }
    }
}

/// Arguments for a markdown definition list item title.
#[non_exhaustive]
pub struct DefListItemTitleArgs {
    /// Inline items of the title.
    pub items: UiVec,
}
impl DefListItemTitleArgs {
    /// New args.
    pub fn new(items: UiVec) -> Self {
        Self { items }
    }
}

/// Arguments for a markdown definition list item description.
#[non_exhaustive]
pub struct DefListItemDefinitionArgs {
    /// Inline items of the description.
    pub items: UiVec,
}
impl DefListItemDefinitionArgs {
    /// New args.
    pub fn new(items: UiVec) -> Self {
        Self { items }
    }
}

/// Arguments for a markdown image view.
#[non_exhaustive]
pub struct ImageFnArgs {
    /// Image, resolved by the [`image_resolver`].
    ///
    /// [`image_resolver`]: fn@crate::image_resolver
    pub source: ImageSource,
    /// Image title, usually displayed as a tooltip.
    pub title: Txt,
    /// Items to display when the image does not load and for screen readers.
    pub alt_items: UiVec,
    /// Alt items in text form.
    pub alt_txt: Txt,
}
impl ImageFnArgs {
    /// New args.
    pub fn new(source: ImageSource, title: impl Into<Txt>, alt_items: UiVec, alt_txt: impl Into<Txt>) -> Self {
        Self {
            source,
            title: title.into(),
            alt_items,
            alt_txt: alt_txt.into(),
        }
    }
}

/// Arguments for a markdown rule view.
///
/// Currently no args.
#[derive(Default)]
#[non_exhaustive]
pub struct RuleFnArgs {}

/// Arguments for a markdown block quote view.
#[non_exhaustive]
pub struct BlockQuoteFnArgs {
    /// Number of *parent* quotes in case of nesting.
    ///
    /// > 0
    /// >> 1
    /// >>> 2
    pub level: u32,

    /// Block items.
    pub items: UiVec,
}
impl BlockQuoteFnArgs {
    /// New args.
    pub fn new(level: u32, items: UiVec) -> Self {
        Self { level, items }
    }
}

/// Arguments for a markdown footnote reference view.
#[non_exhaustive]
pub struct FootnoteRefFnArgs {
    /// Footnote referenced.
    pub label: Txt,
}
impl FootnoteRefFnArgs {
    /// New args.
    pub fn new(label: impl Into<Txt>) -> Self {
        Self { label: label.into() }
    }
}

/// Arguments for a markdown footnote definition view.
///
/// See [`PARAGRAPH_FN_VAR`] for more details.
#[non_exhaustive]
pub struct FootnoteDefFnArgs {
    /// Identifier label.
    pub label: Txt,
    /// Block items.
    pub items: UiVec,
}
impl FootnoteDefFnArgs {
    /// New args.
    pub fn new(label: impl Into<Txt>, items: UiVec) -> Self {
        Self {
            label: label.into(),
            items,
        }
    }
}

/// Arguments for a markdown table view.
///
/// See [`TABLE_FN_VAR`] for more details.
#[non_exhaustive]
pub struct TableFnArgs {
    /// Column definitions with align.
    pub columns: Vec<Align>,
    /// Cell items.
    pub cells: UiVec,
}
impl TableFnArgs {
    /// New args.
    pub fn new(columns: Vec<Align>, cells: UiVec) -> Self {
        Self { columns, cells }
    }
}

/// Arguments for a markdown table cell view.
///
/// See [`TABLE_CELL_FN_VAR`] for more details.
#[non_exhaustive]
pub struct TableCellFnArgs {
    /// If the cell is inside the header row.
    pub is_heading: bool,

    /// Column align.
    pub col_align: Align,

    /// Inline items.
    pub items: UiVec,
}
impl TableCellFnArgs {
    /// New args.
    pub fn new(is_heading: bool, col_align: Align, items: UiVec) -> Self {
        Self {
            is_heading,
            col_align,
            items,
        }
    }
}

/// Arguments for a markdown panel.
///
/// See [`PANEL_FN_VAR`] for more details.
#[non_exhaustive]
pub struct PanelFnArgs {
    /// Block items.
    pub items: UiVec,
}
impl PanelFnArgs {
    /// New args.
    pub fn new(items: UiVec) -> Self {
        Self { items }
    }
}

context_var! {
    /// Widget function for a markdown text segment.
    pub static TEXT_FN_VAR: WidgetFn<TextFnArgs> = WidgetFn::new(default_text_fn);

    /// Widget function for a markdown link segment.
    pub static LINK_FN_VAR: WidgetFn<LinkFnArgs> = WidgetFn::new(default_link_fn);

    /// Widget function for a markdown inline code segment.
    pub static CODE_INLINE_FN_VAR: WidgetFn<CodeInlineFnArgs> = WidgetFn::new(default_code_inline_fn);

    /// Widget function for a markdown code block segment.
    pub static CODE_BLOCK_FN_VAR: WidgetFn<CodeBlockFnArgs> = WidgetFn::new(default_code_block_fn);

    /// Widget function for a markdown paragraph.
    pub static PARAGRAPH_FN_VAR: WidgetFn<ParagraphFnArgs> = WidgetFn::new(default_paragraph_fn);

    /// Widget function for a markdown heading.
    pub static HEADING_FN_VAR: WidgetFn<HeadingFnArgs> = WidgetFn::new(default_heading_fn);

    /// Widget function for a markdown list.
    pub static LIST_FN_VAR: WidgetFn<ListFnArgs> = WidgetFn::new(default_list_fn);

    /// Widget function for a markdown list item bullet, check mark or number.
    pub static LIST_ITEM_BULLET_FN_VAR: WidgetFn<ListItemBulletFnArgs> = WidgetFn::new(default_list_item_bullet_fn);

    /// Widget function for a markdown list item content.
    pub static LIST_ITEM_FN_VAR: WidgetFn<ListItemFnArgs> = WidgetFn::new(default_list_item_fn);

    /// Widget function for a markdown definition list.
    pub static DEF_LIST_FN_VAR: WidgetFn<DefListArgs> = WidgetFn::new(default_def_list_fn);

    /// Widget function for a markdown definition list item title.
    pub static DEF_LIST_ITEM_TITLE_FN_VAR: WidgetFn<DefListItemTitleArgs> = WidgetFn::new(default_def_list_item_title_fn);

    /// Widget function for a markdown definition list item description.
    pub static DEF_LIST_ITEM_DEFINITION_FN_VAR: WidgetFn<DefListItemDefinitionArgs> =
        WidgetFn::new(default_def_list_item_definition_fn);

    /// Widget function for a markdown image.
    pub static IMAGE_FN_VAR: WidgetFn<ImageFnArgs> = WidgetFn::new(default_image_fn);

    /// Widget function for a markdown rule line.
    pub static RULE_FN_VAR: WidgetFn<RuleFnArgs> = WidgetFn::new(default_rule_fn);

    /// Widget function for a markdown block quote.
    pub static BLOCK_QUOTE_FN_VAR: WidgetFn<BlockQuoteFnArgs> = WidgetFn::new(default_block_quote_fn);

    /// Widget function for an inline reference to a footnote.
    pub static FOOTNOTE_REF_FN_VAR: WidgetFn<FootnoteRefFnArgs> = WidgetFn::new(default_footnote_ref_fn);

    /// Widget function for a footnote definition block.
    pub static FOOTNOTE_DEF_FN_VAR: WidgetFn<FootnoteDefFnArgs> = WidgetFn::new(default_footnote_def_fn);

    /// Widget function for a markdown table.
    pub static TABLE_FN_VAR: WidgetFn<TableFnArgs> = WidgetFn::new(default_table_fn);

    /// Widget function for a markdown table body cell.
    pub static TABLE_CELL_FN_VAR: WidgetFn<TableCellFnArgs> = WidgetFn::new(default_table_cell_fn);

    /// Widget function for a markdown panel.
    pub static PANEL_FN_VAR: WidgetFn<PanelFnArgs> = WidgetFn::new(default_panel_fn);
}

/// Widget function that converts [`TextFnArgs`] to widgets.
///
/// Sets the [`TEXT_FN_VAR`].
#[property(CONTEXT, default(TEXT_FN_VAR), widget_impl(Markdown))]
pub fn text_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<TextFnArgs>>) -> UiNode {
    with_context_var(child, TEXT_FN_VAR, wgt_fn)
}

/// Widget function that converts [`LinkFnArgs`] to widgets.
///
/// Sets the [`LINK_FN_VAR`].
#[property(CONTEXT, default(LINK_FN_VAR), widget_impl(Markdown))]
pub fn link_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<LinkFnArgs>>) -> UiNode {
    with_context_var(child, LINK_FN_VAR, wgt_fn)
}

/// Widget function that converts [`CodeInlineFnArgs`] to widgets.
///
/// Sets the [`CODE_INLINE_FN_VAR`].
#[property(CONTEXT, default(CODE_INLINE_FN_VAR), widget_impl(Markdown))]
pub fn code_inline_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<CodeInlineFnArgs>>) -> UiNode {
    with_context_var(child, CODE_INLINE_FN_VAR, wgt_fn)
}

/// Widget function that converts [`CodeBlockFnArgs`] to widgets.
///
/// Sets the [`CODE_BLOCK_FN_VAR`].
#[property(CONTEXT, default(CODE_BLOCK_FN_VAR), widget_impl(Markdown))]
pub fn code_block_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<CodeBlockFnArgs>>) -> UiNode {
    with_context_var(child, CODE_BLOCK_FN_VAR, wgt_fn)
}

/// Widget function that converts [`ParagraphFnArgs`] to widgets.
///
/// Sets the [`PARAGRAPH_FN_VAR`].
#[property(CONTEXT, default(PARAGRAPH_FN_VAR), widget_impl(Markdown))]
pub fn paragraph_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<ParagraphFnArgs>>) -> UiNode {
    with_context_var(child, PARAGRAPH_FN_VAR, wgt_fn)
}

/// Widget function that converts [`HeadingFnArgs`] to widgets.
///
/// Sets the [`HEADING_FN_VAR`].
#[property(CONTEXT, default(HEADING_FN_VAR), widget_impl(Markdown))]
pub fn heading_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<HeadingFnArgs>>) -> UiNode {
    with_context_var(child, HEADING_FN_VAR, wgt_fn)
}

/// Widget function that converts [`ListFnArgs`] to widgets.
///
/// Sets the [`LIST_FN_VAR`].
#[property(CONTEXT, default(LIST_FN_VAR), widget_impl(Markdown))]
pub fn list_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<ListFnArgs>>) -> UiNode {
    with_context_var(child, LIST_FN_VAR, wgt_fn)
}

/// Widget function that converts [`DefListArgs`] to widgets.
///
/// Sets the [`DEF_LIST_FN_VAR`].
#[property(CONTEXT, default(DEF_LIST_FN_VAR), widget_impl(Markdown))]
pub fn def_list_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<DefListArgs>>) -> UiNode {
    with_context_var(child, DEF_LIST_FN_VAR, wgt_fn)
}

/// Widget function that converts [`DefListItemTitleArgs`] to widgets.
///
/// Sets the [`DEF_LIST_ITEM_TITLE_FN_VAR`].
#[property(CONTEXT, default(DEF_LIST_ITEM_TITLE_FN_VAR), widget_impl(Markdown))]
pub fn def_list_item_title_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<DefListItemTitleArgs>>) -> UiNode {
    with_context_var(child, DEF_LIST_ITEM_TITLE_FN_VAR, wgt_fn)
}

/// Widget function that converts [`DefListItemDefinitionArgs`] to widgets.
///
/// Sets the [`DEF_LIST_ITEM_DEFINITION_FN_VAR`].
#[property(CONTEXT, default(DEF_LIST_ITEM_DEFINITION_FN_VAR), widget_impl(Markdown))]
pub fn def_list_item_definition_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<DefListItemDefinitionArgs>>) -> UiNode {
    with_context_var(child, DEF_LIST_ITEM_DEFINITION_FN_VAR, wgt_fn)
}

/// Widget function that converts [`ListItemBulletFnArgs`] to widgets.
///
/// Sets the [`LIST_ITEM_BULLET_FN_VAR`].
#[property(CONTEXT, default(LIST_ITEM_BULLET_FN_VAR), widget_impl(Markdown))]
pub fn list_item_bullet_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<ListItemBulletFnArgs>>) -> UiNode {
    with_context_var(child, LIST_ITEM_BULLET_FN_VAR, wgt_fn)
}

/// Widget function that converts [`ListItemFnArgs`] to widgets.
///
/// Sets the [`LIST_ITEM_FN_VAR`].
#[property(CONTEXT, default(LIST_ITEM_FN_VAR), widget_impl(Markdown))]
pub fn list_item_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<ListItemFnArgs>>) -> UiNode {
    with_context_var(child, LIST_ITEM_FN_VAR, wgt_fn)
}

/// Widget function that converts [`ImageFnArgs`] to widgets.
///
/// Sets the [`IMAGE_FN_VAR`].
#[property(CONTEXT, default(IMAGE_FN_VAR), widget_impl(Markdown))]
pub fn image_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<ImageFnArgs>>) -> UiNode {
    with_context_var(child, IMAGE_FN_VAR, wgt_fn)
}

/// Widget function that converts [`RuleFnArgs`] to widgets.
///
/// Sets the [`RULE_FN_VAR`].
#[property(CONTEXT, default(RULE_FN_VAR), widget_impl(Markdown))]
pub fn rule_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<RuleFnArgs>>) -> UiNode {
    with_context_var(child, RULE_FN_VAR, wgt_fn)
}

/// Widget function that converts [`BlockQuoteFnArgs`] to widgets.
///
/// Sets the [`BLOCK_QUOTE_FN_VAR`].
#[property(CONTEXT, default(BLOCK_QUOTE_FN_VAR), widget_impl(Markdown))]
pub fn block_quote_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<BlockQuoteFnArgs>>) -> UiNode {
    with_context_var(child, BLOCK_QUOTE_FN_VAR, wgt_fn)
}

/// Widget function that converts [`FootnoteRefFnArgs`] to widgets.
///
/// Sets the [`FOOTNOTE_REF_FN_VAR`].
#[property(CONTEXT, default(FOOTNOTE_REF_FN_VAR), widget_impl(Markdown))]
pub fn footnote_ref_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<FootnoteRefFnArgs>>) -> UiNode {
    with_context_var(child, FOOTNOTE_REF_FN_VAR, wgt_fn)
}

/// Widget function that converts [`FootnoteDefFnArgs`] to widgets.
///
/// Sets the [`FOOTNOTE_DEF_FN_VAR`].
#[property(CONTEXT, default(FOOTNOTE_DEF_FN_VAR), widget_impl(Markdown))]
pub fn footnote_def_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<FootnoteDefFnArgs>>) -> UiNode {
    with_context_var(child, FOOTNOTE_DEF_FN_VAR, wgt_fn)
}

/// Widget function that converts [`TableFnArgs`] to widgets.
///
/// Sets the [`TABLE_FN_VAR`].
#[property(CONTEXT, default(TABLE_FN_VAR), widget_impl(Markdown))]
pub fn table_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<TableFnArgs>>) -> UiNode {
    with_context_var(child, TABLE_FN_VAR, wgt_fn)
}

/// Widget function that converts [`PanelFnArgs`] to a widget.
///
/// This generates the panel that contains all markdown blocks, it is the child of the [`Markdown!`] widget.
///
/// Sets the [`PANEL_FN_VAR`].
///
/// [`Markdown!`]: struct@crate::Markdown
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(Markdown))]
pub fn panel_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<PanelFnArgs>>) -> UiNode {
    with_context_var(child, PANEL_FN_VAR, wgt_fn)
}

fn text_view_builder(txt: Txt, style: MarkdownStyle) -> Text {
    let mut builder = Text::widget_new();

    widget_set! {
        &mut builder;
        txt;
        // white_space = WhiteSpace::Merge;
    }

    if style.strong {
        widget_set! {
            &mut builder;
            font_weight = FontWeight::BOLD;
        }
    }
    if style.emphasis {
        widget_set! {
            &mut builder;
            font_style = FontStyle::Italic;
        }
    }
    if style.strikethrough {
        widget_set! {
            &mut builder;
            strikethrough = 1, LineStyle::Solid;
        }
    }

    builder
}

/// Default text view.
///
/// See [`TEXT_FN_VAR`] for more details.
pub fn default_text_fn(args: TextFnArgs) -> UiNode {
    let mut builder = text_view_builder(args.txt, args.style);
    builder.widget_build()
}

/// Default inlined code text view.
///
/// See [`CODE_INLINE_FN_VAR`] for more details.
pub fn default_code_inline_fn(args: CodeInlineFnArgs) -> UiNode {
    let mut builder = text_view_builder(args.txt, args.style);

    widget_set! {
        &mut builder;
        font_family = ["JetBrains Mono", "Consolas", "monospace"];
        background_color = light_dark(rgb(0.95, 0.95, 0.95), rgb(0.05, 0.05, 0.05));
    }

    builder.widget_build()
}

/// Default inlined link view.
///
/// See [`LINK_FN_VAR`] for more details.
pub fn default_link_fn(args: LinkFnArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else {
        let url = args.url;

        let mut items = args.items;
        let items = if items.len() == 1 {
            items.remove(0)
        } else {
            Wrap! {
                children = items;
            }
        };

        Button! {
            style_fn = LinkStyle!();
            child = items;

            on_click = hn!(|args: &ClickArgs| {
                args.propagation().stop();

                let link = WINDOW.info().get(WIDGET.id()).unwrap().interaction_path();
                LINK_EVENT.notify(LinkArgs::now(url.clone(), link));
            });
        }
    }
}

/// Default code block view.
///
/// Is [`AnsiText!`] for the `ansi` and `console` languages, and only raw text for the rest.
///
/// See [`CODE_BLOCK_FN_VAR`] for more details.
///
/// [`AnsiText!`]: struct@zng_wgt_ansi_text::AnsiText
pub fn default_code_block_fn(args: CodeBlockFnArgs) -> UiNode {
    if ["ansi", "console"].contains(&args.lang.as_str()) {
        zng_wgt_ansi_text::AnsiText! {
            txt = args.txt;
            padding = 6;
            corner_radius = 4;
            background_color = light_dark(rgb(0.95, 0.95, 0.95), rgb(0.05, 0.05, 0.05));
        }
    } else {
        Text! {
            txt = args.txt;
            padding = 6;
            corner_radius = 4;
            font_family = ["JetBrains Mono", "Consolas", "monospace"];
            background_color = light_dark(rgb(0.95, 0.95, 0.95), rgb(0.05, 0.05, 0.05));
        }
    }
}

/// Default paragraph view.
///
/// See [`PARAGRAPH_FN_VAR`] for more details.
pub fn default_paragraph_fn(mut args: ParagraphFnArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else if args.items.len() == 1 {
        args.items.remove(0)
    } else {
        Wrap! {
            children = args.items;
        }
    }
}

/// Default heading view.
///
/// See [`HEADING_FN_VAR`] for more details.
pub fn default_heading_fn(args: HeadingFnArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else {
        Wrap! {
            access_role = AccessRole::Heading;
            access::level = NonZeroU32::new(args.level as _).unwrap();
            font_size = match args.level {
                HeadingLevel::H1 => 2.em(),
                HeadingLevel::H2 => 1.5.em(),
                HeadingLevel::H3 => 1.4.em(),
                HeadingLevel::H4 => 1.3.em(),
                HeadingLevel::H5 => 1.2.em(),
                HeadingLevel::H6 => 1.1.em(),
            };
            children = args.items;
            anchor = args.anchor;
        }
    }
}

/// Default list view.
///
/// Uses a [`Grid!`] with two columns, one default for the bullet or number, the other fills the leftover space.
///
/// See [`LIST_FN_VAR`] for more details.
///
/// [`Grid!`]: struct@Grid
pub fn default_list_fn(args: ListFnArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else {
        Grid! {
            grid::cell::at = grid::cell::AT_AUTO; // in case it is nested

            access_role = AccessRole::List;
            margin = (0, 0, 0, 1.em());
            cells = args.items;
            columns = ui_vec![
                grid::Column!(),
                grid::Column! {
                    width = 1.lft();
                },
            ];
        }
    }
}

/// Default definition list view.
///
/// Is a simple vertical [`Stack!`].
///
/// [`Stack!`]: struct@Stack
pub fn default_def_list_fn(args: DefListArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else {
        Stack! {
            access_role = AccessRole::List;
            direction = StackDirection::top_to_bottom();
            spacing = PARAGRAPH_SPACING_VAR;
            children = args.items;
        }
    }
}

/// Default definition list item title view.
///
/// Is a [`Wrap!`] with bold text.
///
/// [`Wrap!`]: struct@Wrap
pub fn default_def_list_item_title_fn(args: DefListItemTitleArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else {
        Wrap! {
            access_role = AccessRole::Term;
            children = args.items;
            font_weight = FontWeight::BOLD;
        }
    }
}

/// Default definition list item description view.
///
/// Is a [`Wrap!`].
///
/// [`Wrap!`]: struct@Wrap
pub fn default_def_list_item_definition_fn(args: DefListItemDefinitionArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else {
        Wrap! {
            access_role = AccessRole::Definition;
            children = args.items;
            margin = (0, 2.em());
        }
    }
}

/// Default list item bullet, check mark or number view.
///
/// See [`LIST_ITEM_BULLET_FN_VAR`] for more details.
pub fn default_list_item_bullet_fn(args: ListItemBulletFnArgs) -> UiNode {
    if let Some(checked) = args.checked {
        Text! {
            grid::cell::at = grid::cell::AT_AUTO;
            align = Align::TOP;
            txt = " ✓ ";
            font_color = FONT_COLOR_VAR.map(move |c| if checked { *c } else { c.transparent() });
            background_color = FONT_COLOR_VAR.map(|c| c.with_alpha(10.pct()));
            corner_radius = 4;
            scale = 0.8.fct();
            offset = (-(0.1.fct()), 0);
        }
    } else if let Some(n) = args.num {
        Text! {
            grid::cell::at = grid::cell::AT_AUTO;
            txt = formatx!("{n}. ");
            align = Align::RIGHT;
        }
    } else {
        match args.depth {
            0 => Wgt! {
                grid::cell::at = grid::cell::AT_AUTO;
                align = Align::TOP;
                size = (5, 5);
                corner_radius = 5;
                margin = (0.6.em(), 0.5.em(), 0, 0);
                background_color = FONT_COLOR_VAR;
            },
            1 => Wgt! {
                grid::cell::at = grid::cell::AT_AUTO;
                align = Align::TOP;
                size = (5, 5);
                corner_radius = 5;
                margin = (0.6.em(), 0.5.em(), 0, 0);
                border = 1.px(), FONT_COLOR_VAR.map_into();
            },
            _ => Wgt! {
                grid::cell::at = grid::cell::AT_AUTO;
                align = Align::TOP;
                size = (5, 5);
                margin = (0.6.em(), 0.5.em(), 0, 0);
                background_color = FONT_COLOR_VAR;
            },
        }
    }
}

/// Default list item view.
///
/// See [`LIST_ITEM_FN_VAR`] for more details.
pub fn default_list_item_fn(args: ListItemFnArgs) -> UiNode {
    let mut blocks = args.blocks;
    let mut items = args.items;

    if items.is_empty() {
        if blocks.is_empty() {
            return UiNode::nil();
        }
    } else {
        let r = if items.len() == 1 { items.remove(0) } else { Wrap!(items) };
        blocks.insert(0, r);
    }

    if blocks.len() > 1 {
        Stack! {
            access_role = AccessRole::ListItem;
            grid::cell::at = grid::cell::AT_AUTO;
            direction = StackDirection::top_to_bottom();
            children = blocks;
        }
    } else {
        Container! {
            access_role = AccessRole::ListItem;
            grid::cell::at = grid::cell::AT_AUTO;
            child = blocks.remove(0);
        }
    }
}

/// Default image view.
///
/// See [`IMAGE_FN_VAR`] for more details.
pub fn default_image_fn(args: ImageFnArgs) -> UiNode {
    let tooltip_fn = if args.title.is_empty() {
        wgt_fn!()
    } else {
        let title = args.title;
        wgt_fn!(|_| Tip!(Text!(title.clone())))
    };

    let alt_txt = args.alt_txt;
    let mut alt_items = args.alt_items;
    if alt_items.is_empty() {
        zng_wgt_image::Image! {
            align = Align::TOP_LEFT;
            tooltip_fn;
            access::label = alt_txt;
            source = args.source;
        }
    } else {
        let alt_items = if alt_items.len() == 1 {
            alt_items.remove(0)
        } else {
            Wrap! {
                children = alt_items;
            }
        };
        let alt_items = ArcNode::new(alt_items);
        zng_wgt_image::Image! {
            align = Align::TOP_LEFT;
            source = args.source;
            tooltip_fn;
            zng_wgt_access::label = alt_txt;
            img_error_fn = wgt_fn!(|_| { alt_items.take_on_init() });
        }
    }
}

/// Default rule view.
///
/// See [`RULE_FN_VAR`] for more details.
pub fn default_rule_fn(_: RuleFnArgs) -> UiNode {
    zng_wgt_rule_line::hr::Hr! {
        opacity = 50.pct();
    }
}

/// Default block quote view.
///
/// See [`BLOCK_QUOTE_FN_VAR`] for more details.
pub fn default_block_quote_fn(args: BlockQuoteFnArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else {
        Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = PARAGRAPH_SPACING_VAR;
            children = args.items;
            corner_radius = 2;
            background_color = if args.level < 3 {
                FONT_COLOR_VAR.map(|c| c.with_alpha(5.pct()))
            } else {
                colors::BLACK.transparent().into_var()
            };
            border = {
                widths: (0, 0, 0, 4u32.saturating_sub(args.level).max(1) as i32),
                sides: FONT_COLOR_VAR.map(|c| BorderSides::solid(c.with_alpha(60.pct()))),
            };
            padding = 4;
        }
    }
}

/// Default markdown table.
///
/// See [`TABLE_FN_VAR`] for more details.
pub fn default_table_fn(args: TableFnArgs) -> UiNode {
    Grid! {
        access_role = AccessRole::Table;
        background_color = FONT_COLOR_VAR.map(|c| c.with_alpha(5.pct()));
        border = 1, FONT_COLOR_VAR.map(|c| c.with_alpha(30.pct()).into());
        align = Align::LEFT;
        auto_grow_fn = wgt_fn!(|args: grid::AutoGrowFnArgs| {
            grid::Row! {
                border = (0, 0, 1, 0), FONT_COLOR_VAR.map(|c| c.with_alpha(10.pct()).into());
                background_color = {
                    let alpha = if args.index.is_multiple_of(2) { 5.pct() } else { 0.pct() };
                    FONT_COLOR_VAR.map(move |c| c.with_alpha(alpha))
                };

                when *#is_last {
                    border = 0, BorderStyle::Hidden;
                }
            }
        });
        columns = std::iter::repeat_with(|| grid::Column! {}).take(args.columns.len());
        cells = args.cells;
    }
}

/// Default markdown table.
///
/// See [`TABLE_CELL_FN_VAR`] for more details.
pub fn default_table_cell_fn(args: TableCellFnArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else if args.is_heading {
        Wrap! {
            access_role = AccessRole::Cell;
            grid::cell::at = grid::cell::AT_AUTO;
            font_weight = FontWeight::BOLD;
            padding = 6;
            child_align = args.col_align;
            children = args.items;
        }
    } else {
        Wrap! {
            access_role = AccessRole::Cell;
            grid::cell::at = grid::cell::AT_AUTO;
            padding = 6;
            child_align = args.col_align;
            children = args.items;
        }
    }
}

/// Default markdown panel.
///
/// See [`PANEL_FN_VAR`] for more details.
pub fn default_panel_fn(args: PanelFnArgs) -> UiNode {
    if args.items.is_empty() {
        UiNode::nil()
    } else {
        Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = PARAGRAPH_SPACING_VAR;
            children = args.items;
        }
    }
}

/// Default markdown footnote reference.
///
/// See [`FOOTNOTE_REF_FN_VAR`] for more details.
pub fn default_footnote_ref_fn(args: FootnoteRefFnArgs) -> UiNode {
    let url = formatx!("#footnote-{}", args.label);
    Button! {
        style_fn = LinkStyle!();
        font_size = 0.7.em();
        offset = (0, (-0.5).em());
        crate::anchor = formatx!("footnote-ref-{}", args.label);
        child = Text!("[{}]", args.label);
        on_click = hn!(|args: &ClickArgs| {
            args.propagation().stop();

            let link = WINDOW.info().get(WIDGET.id()).unwrap().interaction_path();
            crate::LINK_EVENT.notify(crate::LinkArgs::now(url.clone(), link));
        });
    }
}

/// Default markdown footnote definition.
///
/// See [`FOOTNOTE_DEF_FN_VAR`] for more details.
pub fn default_footnote_def_fn(args: FootnoteDefFnArgs) -> UiNode {
    let mut items = args.items;
    let items = if items.is_empty() {
        UiNode::nil()
    } else if items.len() == 1 {
        items.remove(0)
    } else {
        Stack! {
            direction = StackDirection::top_to_bottom();
            children = items;
        }
    };

    let url_back = formatx!("#footnote-ref-{}", args.label);
    Stack! {
        direction = StackDirection::left_to_right();
        spacing = 0.5.em();
        anchor = formatx!("footnote-{}", args.label);
        children = ui_vec![
            Button! {
                style_fn = LinkStyle!();
                child = Text!("[^{}]", args.label);
                on_click = hn!(|args: &ClickArgs| {
                    args.propagation().stop();

                    let link = WINDOW.info().get(WIDGET.id()).unwrap().interaction_path();
                    LINK_EVENT.notify(LinkArgs::now(url_back.clone(), link));
                });
            },
            items,
        ];
    }
}
