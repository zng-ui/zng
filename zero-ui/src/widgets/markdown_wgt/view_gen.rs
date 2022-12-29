pub use pulldown_cmark::HeadingLevel;
use zero_ui_core::{gesture::ClickArgs, image::ImageSource};

use crate::widgets::text::{PARAGRAPH_SPACING_VAR, TEXT_COLOR_VAR};

use super::*;

/// Markdown text run style.
#[derive(Default, Clone, Debug)]
pub struct MarkdownStyle {
    /// Bold.
    pub strong: bool,
    /// Italic.
    pub emphasis: bool,
    /// Strikethrough.
    pub strikethrough: bool,
}

/// Arguments for a markdown text view.
///
/// The text can be inside a paragraph, heading, list item or any other markdown block item.
///
/// See [`TEXT_GEN_VAR`] for more details.
pub struct TextGenArgs {
    /// The text run.
    pub txt: Text,
    /// The style.
    pub style: MarkdownStyle,
}

/// Arguments for a markdown inlined link view.
///
/// See [`LINK_GEN_VAR`] for more details.
pub struct LinkGenArgs {
    /// The link.
    pub url: Text,

    /// Link title, usually displayed as a tool-tip.
    pub title: Text,

    /// Inline items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown inlined code text view.
///
/// The text can be inside a paragraph, heading, list item or any other markdown block item.
///
/// See [`CODE_INLINE_GEN_VAR`] for more details.
pub struct CodeInlineGenArgs {
    /// The code text run.
    pub txt: Text,
    /// The style.
    pub style: MarkdownStyle,
}

/// Arguments for a markdown code block view.
///
/// See [`CODE_BLOCK_GEN_VAR`] for more details.
pub struct CodeBlockGenArgs {
    /// Code language, can be empty.
    pub lang: Text,
    /// Raw text.
    pub txt: Text,
}

/// Arguments for a markdown paragraph view.
///
/// See [`PARAGRAPH_GEN_VAR`] for more details.
pub struct ParagraphGenArgs {
    /// Zero-sized index of the paragraph.
    pub index: u32,
    /// Inline items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown heading view.
pub struct HeadingGenArgs {
    /// Level.
    pub level: HeadingLevel,

    /// Anchor label that identifies the header in the markdown context.
    pub anchor: Text,

    /// Inline items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown list view.
pub struct ListGenArgs {
    /// Nested list depth, starting from zero for the outer-list.
    pub depth: u32,

    /// If the list is *ordered*, the first item number.
    pub first_num: Option<u64>,

    /// List items.
    ///
    /// Each two items are the bullet or number followed by the item.
    pub items: UiNodeVec,
}

/// Arguments for a markdown list item bullet, checkmark or number.
#[derive(Clone, Copy)]
pub struct ListItemBulletGenArgs {
    /// Nested list depth, starting from zero for items in the outer-list.
    pub depth: u32,

    /// If the list is *ordered*, the item number.
    pub num: Option<u64>,

    /// If the list is checked. `Some(true)` is `[x]` and `Some(false)` is `[ ]`.
    pub checked: Option<bool>,
}

/// Arguments for a markdown list item view.
pub struct ListItemGenArgs {
    /// Copy of the bullet args.
    pub bullet: ListItemBulletGenArgs,

    /// Inline items of the list item.
    pub items: UiNodeVec,

    /// Inner list defined inside this item.
    pub nested_list: Option<BoxedUiNode>,
}

/// Arguments for a markdown image view.
pub struct ImageGenArgs {
    /// Image, resolved by the [`image_resolver`].
    ///
    /// [`image_resolver`]: fn@crate::widgets::markdown::image_resolver
    pub source: ImageSource,
    /// Image title, usually displayed as a tool-tip.
    pub title: Text,
    /// Items to display when the image does not load and for screen readers.
    pub alt_items: UiNodeVec,
}

/// Arguments for a markdown rule view.
///
/// Currently no args.
pub struct RuleGenArgs {}

/// Arguments for a markdown block quote view.
pub struct BlockQuoteGenArgs {
    /// Number of *parent* quotes in case of nesting.
    ///
    /// > 0
    /// >> 1
    /// >>> 2
    pub level: u32,

    /// Block items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown footnote reference view.
pub struct FootnoteRefGenArgs {
    /// Footnote referenced.
    pub label: Text,
}

/// Arguments for a markdown footnote definition view.
///
/// See [`PARAGRAPH_GEN_VAR`] for more details.
pub struct FootnoteDefGenArgs {
    /// Identifier label.
    pub label: Text,
    /// Block items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown table view.
///
/// See [`TABLE_GEN_VAR`] for more details.
pub struct TableGenArgs {
    /// Column definitions with align.
    pub columns: Vec<Align>,
    /// Cell items.
    pub cells: UiNodeVec,
}

/// Arguments for a markdown table cell view.
///
/// See [`TABLE_CELL_GEN_VAR`] for more details.
pub struct TableCellGenArgs {
    /// If the cell is inside the header row.
    pub is_heading: bool,

    /// Column align.
    pub col_align: Align,

    /// Inline items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown panel.
///
/// See [`PANEL_GEN_VAR`] for more details.
pub struct PanelGenArgs {
    /// Block items.
    pub items: UiNodeVec,
}

context_var! {
    /// Widget generator for a markdown text segment.
    pub static TEXT_GEN_VAR: WidgetGenerator<TextGenArgs> = WidgetGenerator::new(|_, args| default_text_gen(args));

    /// Widget generator for a markdown link segment.
    pub static LINK_GEN_VAR: WidgetGenerator<LinkGenArgs> = WidgetGenerator::new(|_, args| default_link_gen(args));

    /// Widget generator for a markdown inline code segment.
    pub static CODE_INLINE_GEN_VAR: WidgetGenerator<CodeInlineGenArgs> = WidgetGenerator::new(|_, args| default_code_inline_gen(args));

    /// Widget generator for a markdown code block segment.
    pub static CODE_BLOCK_GEN_VAR: WidgetGenerator<CodeBlockGenArgs> = WidgetGenerator::new(|_, args| default_code_block_gen(args));

    /// Widget generator for a markdown paragraph.
    pub static PARAGRAPH_GEN_VAR: WidgetGenerator<ParagraphGenArgs> = WidgetGenerator::new(|_, args| default_paragraph_gen(args));

    /// Widget generator for a markdown heading.
    pub static HEADING_GEN_VAR: WidgetGenerator<HeadingGenArgs> = WidgetGenerator::new(|_, args| default_heading_gen(args));

    /// Widget generator for a markdown list.
    pub static LIST_GEN_VAR: WidgetGenerator<ListGenArgs> = WidgetGenerator::new(|_, args| default_list_gen(args));

    /// Widget generator for a markdown list item bullet, checkmark or number.
    pub static LIST_ITEM_BULLET_GEN_VAR: WidgetGenerator<ListItemBulletGenArgs> = WidgetGenerator::new(|_, args| default_list_item_bullet_gen(args));

    /// Widget generator for a markdown list item content.
    pub static LIST_ITEM_GEN_VAR: WidgetGenerator<ListItemGenArgs> = WidgetGenerator::new(|_, args| default_list_item_gen(args));

    /// Widget generator for a markdown image.
    pub static IMAGE_GEN_VAR: WidgetGenerator<ImageGenArgs> = WidgetGenerator::new(|_, args| default_image_gen(args));

    /// Widget generator for a markdown rule line.
    pub static RULE_GEN_VAR: WidgetGenerator<RuleGenArgs> = WidgetGenerator::new(|_, args| default_rule_gen(args));

    /// Widget generator for a markdown block quote.
    pub static BLOCK_QUOTE_GEN_VAR: WidgetGenerator<BlockQuoteGenArgs> = WidgetGenerator::new(|_, args| default_block_quote_gen(args));

    /// Widget generator for an inline reference to a footnote.
    pub static FOOTNOTE_REF_GEN_VAR: WidgetGenerator<FootnoteRefGenArgs> = WidgetGenerator::new(|_, args| default_footnote_ref_gen(args));

    /// Widget generator for a footnote definition block.
    pub static FOOTNOTE_DEF_GEN_VAR: WidgetGenerator<FootnoteDefGenArgs> = WidgetGenerator::new(|_, args| default_footnote_def_gen(args));

    /// Widget generator for a markdown table.
    pub static TABLE_GEN_VAR: WidgetGenerator<TableGenArgs> = WidgetGenerator::new(|_, args| default_table_gen(args));

    /// Widget generator for a markdown table body cell.
    pub static TABLE_CELL_GEN_VAR: WidgetGenerator<TableCellGenArgs> = WidgetGenerator::new(|_, args| default_table_cell_gen(args));

    /// Widget generator for a markdown panel.
    pub static PANEL_GEN_VAR: WidgetGenerator<PanelGenArgs> = WidgetGenerator::new(|_, args| default_panel_gen(args));
}

/// Widget generator that converts [`TextGenArgs`] to widgets.
///
/// Sets the [`TEXT_GEN_VAR`].
#[property(CONTEXT, default(TEXT_GEN_VAR))]
pub fn text_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<TextGenArgs>>) -> impl UiNode {
    with_context_var(child, TEXT_GEN_VAR, gen)
}

/// Widget generator that converts [`LinkGenArgs`] to widgets.
///
/// Sets the [`LINK_GEN_VAR`].
#[property(CONTEXT, default(LINK_GEN_VAR))]
pub fn link_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<LinkGenArgs>>) -> impl UiNode {
    with_context_var(child, LINK_GEN_VAR, gen)
}

/// Widget generator that converts [`CodeInlineGenArgs`] to widgets.
///
/// Sets the [`CODE_INLINE_GEN_VAR`].
#[property(CONTEXT, default(CODE_INLINE_GEN_VAR))]
pub fn code_inline_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<CodeInlineGenArgs>>) -> impl UiNode {
    with_context_var(child, CODE_INLINE_GEN_VAR, gen)
}

/// Widget generator that converts [`CodeBlockGenArgs`] to widgets.
///
/// Sets the [`CODE_BLOCK_GEN_VAR`].
#[property(CONTEXT, default(CODE_BLOCK_GEN_VAR))]
pub fn code_block_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<CodeBlockGenArgs>>) -> impl UiNode {
    with_context_var(child, CODE_BLOCK_GEN_VAR, gen)
}

/// Widget generator that converts [`ParagraphGenArgs`] to widgets.
///
/// Sets the [`PARAGRAPH_GEN_VAR`].
#[property(CONTEXT, default(PARAGRAPH_GEN_VAR))]
pub fn paragraph_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<ParagraphGenArgs>>) -> impl UiNode {
    with_context_var(child, PARAGRAPH_GEN_VAR, gen)
}

/// Widget generator that converts [`HeadingGenArgs`] to widgets.
///
/// Sets the [`HEADING_GEN_VAR`].
#[property(CONTEXT, default(HEADING_GEN_VAR))]
pub fn heading_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<HeadingGenArgs>>) -> impl UiNode {
    with_context_var(child, HEADING_GEN_VAR, gen)
}

/// Widget generator that converts [`ListGenArgs`] to widgets.
///
/// Sets the [`LIST_GEN_VAR`].
#[property(CONTEXT, default(LIST_GEN_VAR))]
pub fn list_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<ListGenArgs>>) -> impl UiNode {
    with_context_var(child, LIST_GEN_VAR, gen)
}

/// Widget generator that converts [`ListItemBulletGenArgs`] to widgets.
///
/// Sets the [`LIST_ITEM_BULLET_GEN_VAR`].
#[property(CONTEXT, default(LIST_ITEM_BULLET_GEN_VAR))]
pub fn list_item_bullet_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<ListItemBulletGenArgs>>) -> impl UiNode {
    with_context_var(child, LIST_ITEM_BULLET_GEN_VAR, gen)
}

/// Widget generator that converts [`ListItemGenArgs`] to widgets.
///
/// Sets the [`LIST_ITEM_GEN_VAR`].
#[property(CONTEXT, default(LIST_ITEM_GEN_VAR))]
pub fn list_item_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<ListItemGenArgs>>) -> impl UiNode {
    with_context_var(child, LIST_ITEM_GEN_VAR, gen)
}

/// Widget generator that converts [`ImageGenArgs`] to widgets.
///
/// Sets the [`IMAGE_GEN_VAR`].
#[property(CONTEXT, default(IMAGE_GEN_VAR))]
pub fn image_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<ImageGenArgs>>) -> impl UiNode {
    with_context_var(child, IMAGE_GEN_VAR, gen)
}

/// Widget generator that converts [`RuleGenArgs`] to widgets.
///
/// Sets the [`RULE_GEN_VAR`].
#[property(CONTEXT, default(RULE_GEN_VAR))]
pub fn rule_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<RuleGenArgs>>) -> impl UiNode {
    with_context_var(child, RULE_GEN_VAR, gen)
}

/// Widget generator that converts [`BlockQuoteGenArgs`] to widgets.
///
/// Sets the [`BLOCK_QUOTE_GEN_VAR`].
#[property(CONTEXT, default(BLOCK_QUOTE_GEN_VAR))]
pub fn block_quote_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<BlockQuoteGenArgs>>) -> impl UiNode {
    with_context_var(child, BLOCK_QUOTE_GEN_VAR, gen)
}

/// Widget generator that converts [`FootnoteRefGenArgs`] to widgets.
///
/// Sets the [`FOOTNOTE_REF_GEN_VAR`].
#[property(CONTEXT, default(FOOTNOTE_REF_GEN_VAR))]
pub fn footnote_ref_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<FootnoteRefGenArgs>>) -> impl UiNode {
    with_context_var(child, FOOTNOTE_REF_GEN_VAR, gen)
}

/// Widget generator that converts [`FootnoteDefGenArgs`] to widgets.
///
/// Sets the [`FOOTNOTE_DEF_GEN_VAR`].
#[property(CONTEXT, default(FOOTNOTE_DEF_GEN_VAR))]
pub fn footnote_def_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<FootnoteDefGenArgs>>) -> impl UiNode {
    with_context_var(child, FOOTNOTE_DEF_GEN_VAR, gen)
}

/// Widget generator that converts [`TableGenArgs`] to widgets.
///
/// Sets the [`TABLE_GEN_VAR`].
#[property(CONTEXT, default(TABLE_GEN_VAR))]
pub fn table_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<TableGenArgs>>) -> impl UiNode {
    with_context_var(child, TABLE_GEN_VAR, gen)
}

/// Widget generator that converts [`PanelGenArgs`] to a widget.
///
/// This generates the panel that contains all markdown blocks, it is the child of the [`markdown!`] widget.
///
/// Sets the [`PANEL_GEN_VAR`].
///
/// [`markdown!`]: mod@crate::widgets::markdown
#[property(CONTEXT, default(PANEL_GEN_VAR))]
pub fn panel_gen(child: impl UiNode, gen: impl IntoVar<WidgetGenerator<PanelGenArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_GEN_VAR, gen)
}

fn text_view_builder(txt: Text, style: MarkdownStyle) -> WidgetBuilder {
    use crate::widgets::text as t;

    let mut builder = WidgetBuilder::new(widget_mod!(t));
    t::include(&mut builder);

    builder.push_property(
        Importance::INSTANCE,
        property_args! {
            t::txt = txt;
        },
    );

    if style.strong {
        builder.push_property(
            Importance::INSTANCE,
            property_args! {
                t::font_weight = FontWeight::BOLD;
            },
        );
    }
    if style.emphasis {
        builder.push_property(
            Importance::INSTANCE,
            property_args! {
                t::font_style = FontStyle::Italic;
            },
        );
    }
    if style.strikethrough {
        builder.push_property(
            Importance::INSTANCE,
            property_args! {
                t::strikethrough = 1, LineStyle::Solid;
            },
        );
    }

    builder
}

/// Default text view.
///
/// See [`TEXT_GEN_VAR`] for more details.
pub fn default_text_gen(args: TextGenArgs) -> impl UiNode {
    let builder = text_view_builder(args.txt, args.style);
    crate::widgets::text::build(builder)
}

/// Default inlined code text view.
///
/// See [`CODE_INLINE_GEN_VAR`] for more details.
pub fn default_code_inline_gen(args: CodeInlineGenArgs) -> impl UiNode {
    use crate::widgets::text as t;

    let mut builder = text_view_builder(args.txt, args.style);

    builder.push_property(
        Importance::INSTANCE,
        property_args! {
            t::font_family = ["JetBrains Mono", "Consolas", "monospace"];
        },
    );
    builder.push_property(
        Importance::INSTANCE,
        property_args! {
            background_color = color_scheme_map(rgb(0.05, 0.05, 0.05), rgb(0.95, 0.95, 0.95));
        },
    );

    crate::widgets::text::build(builder)
}

/// Default inlined link view.
///
/// See [`LINK_GEN_VAR`] for more details.
pub fn default_link_gen(args: LinkGenArgs) -> impl UiNode {
    if args.items.is_empty() {
        NilUiNode.boxed()
    } else {
        let url = args.url;

        let mut items = args.items;
        let items = if items.len() == 1 {
            items.remove(0)
        } else {
            crate::widgets::layouts::wrap! {
                children = items;
            }
            .boxed()
        };

        crate::widgets::link! {
            child = items;

            on_click = hn!(|ctx, args: &ClickArgs| {
                args.propagation().stop();

                let link = ctx.info_tree.get(ctx.path.widget_id()).unwrap().interaction_path();
                markdown::LINK_EVENT.notify(ctx.events, markdown::LinkArgs::now(url.clone(), link));
            });
        }
        .boxed()
    }
}

/// Default code block view.
///
/// Is [`ansi_text!`] for the `ansi` language, and only raw text for the rest.
///
/// See [`CODE_BLOCK_GEN_VAR`] for more details.
///
/// [`ansi_text!`]: mod@crate::widgets::ansi_text
pub fn default_code_block_gen(args: CodeBlockGenArgs) -> impl UiNode {
    if args.lang == "ansi" {
        crate::widgets::ansi_text! {
            txt = args.txt;
            padding = 6;
            corner_radius = 4;
            background_color = color_scheme_map(rgb(0.05, 0.05, 0.05), rgb(0.95, 0.95, 0.95));
        }
        .boxed()
    } else {
        crate::widgets::text! {
            txt = args.txt;
            padding = 6;
            corner_radius = 4;
            font_family = ["JetBrains Mono", "Consolas", "monospace"];
            background_color = color_scheme_map(rgb(0.05, 0.05, 0.05), rgb(0.95, 0.95, 0.95));
        }
        .boxed()
    }
}

/// Default paragraph view.
///
/// See [`PARAGRAPH_GEN_VAR`] for more details.
pub fn default_paragraph_gen(mut args: ParagraphGenArgs) -> impl UiNode {
    if args.items.is_empty() {
        NilUiNode.boxed()
    } else if args.items.len() == 1 {
        args.items.remove(0)
    } else {
        crate::widgets::layouts::wrap! {
            children = args.items;
        }
        .boxed()
    }
}

/// Default heading view.
///
/// See [`HEADING_GEN_VAR`] for more details.
pub fn default_heading_gen(args: HeadingGenArgs) -> impl UiNode {
    if args.items.is_empty() {
        NilUiNode.boxed()
    } else {
        crate::widgets::layouts::wrap! {
            font_size = match args.level {
                HeadingLevel::H1 => 2.em(),
                HeadingLevel::H2 => 1.5.em(),
                HeadingLevel::H3 => 1.4.em(),
                HeadingLevel::H4 => 1.3.em(),
                HeadingLevel::H5 => 1.2.em(),
                HeadingLevel::H6 => 1.1.em()
            };
            children = args.items;
            super::markdown::anchor = args.anchor;
        }
        .boxed()
    }
}

/// Default list view.
///
/// Uses a [`grid!`] with two columns, one default for the bullet or number, the other fills the leftover space.
///
/// See [`LIST_GEN_VAR`] for more details.
///
/// [`grid!`]: mod@crate::widgets::layouts::grid
pub fn default_list_gen(args: ListGenArgs) -> impl UiNode {
    if args.items.is_empty() {
        NilUiNode.boxed()
    } else {
        use crate::widgets::layouts::grid;
        grid! {
            margin = (0, 0, 0, 1.em());
            cells = args.items;
            columns = ui_vec![
                grid::column!(),
                grid::column! { width = 1.lft() },
            ];
        }
        .boxed()
    }
}

/// Default list item bullet, checkmark or number view.
///
/// See [`LIST_ITEM_BULLET_GEN_VAR`] for more details.
pub fn default_list_item_bullet_gen(args: ListItemBulletGenArgs) -> impl UiNode {
    use crate::prelude::*;

    if let Some(checked) = args.checked {
        text! {
            txt = " ✓ ";
            txt_color = TEXT_COLOR_VAR.map(move |c| if checked { *c } else { c.transparent() });
            background_color = TEXT_COLOR_VAR.map(|c| c.with_alpha(10.pct()));
            corner_radius = 4;
            scale = 0.8.fct();
            offset = (-(0.1.fct()), 0);
        }
        .boxed()
    } else if let Some(n) = args.num {
        text! {
            txt = formatx!("{n}. ");
            align = Align::RIGHT;
        }
        .boxed()
    } else {
        match args.depth {
            0 => wgt! {
                size = (5, 5);
                corner_radius = 5;
                margin = (0.6.em(), 0.5.em(), 0, 0);
                background_color = TEXT_COLOR_VAR;
            },
            1 => wgt! {
                size = (5, 5);
                corner_radius = 5;
                margin = (0.6.em(), 0.5.em(), 0, 0);
                border = 1.px(), TEXT_COLOR_VAR.map_into();
            },
            _ => wgt! {
                size = (5, 5);
                margin = (0.6.em(), 0.5.em(), 0, 0);
                background_color = TEXT_COLOR_VAR;
            },
        }
        .boxed()
    }
}

/// Default list item view.
///
/// See [`LIST_ITEM_GEN_VAR`] for more details.
pub fn default_list_item_gen(args: ListItemGenArgs) -> impl UiNode {
    use crate::prelude::*;

    let mut items = args.items;

    if items.is_empty() {
        return if let Some(inner) = args.nested_list {
            inner
        } else {
            NilUiNode.boxed()
        };
    }

    let mut r = if items.len() == 1 {
        items.remove(0)
    } else {
        wrap! {
            children = items;
        }
        .boxed()
    };

    if let Some(inner) = args.nested_list {
        r = stack! {
            direction = StackDirection::top_to_bottom();
            children = ui_vec![
                r,
                inner
            ]
        }
        .boxed();
    }

    r
}

/// Default image view.
///
/// See [`IMAGE_GEN_VAR`] for more details.
pub fn default_image_gen(args: ImageGenArgs) -> impl UiNode {
    use crate::prelude::*;

    let mut alt_items = args.alt_items;
    if alt_items.is_empty() {
        image! {
            align = Align::TOP_LEFT;
            source = args.source;
        }
    } else {
        let alt_items = if alt_items.len() == 1 {
            alt_items.remove(0)
        } else {
            wrap! {
                children = alt_items;
            }
            .boxed()
        };
        let alt_items = ArcNode::new(alt_items);
        image! {
            align = Align::TOP_LEFT;
            source = args.source;
            img_error_gen = wgt_gen!(|_, _| {
                alt_items.take_on_init()
            });
        }
    }
}

/// Default rule view.
///
/// See [`RULE_GEN_VAR`] for more details.
pub fn default_rule_gen(_: RuleGenArgs) -> impl UiNode {
    crate::widgets::hr! {
        opacity = 50.pct();
    }
}

/// Default block quote view.
///
/// See [`BLOCK_QUOTE_GEN_VAR`] for more details.
pub fn default_block_quote_gen(args: BlockQuoteGenArgs) -> impl UiNode {
    use crate::prelude::*;

    if args.items.is_empty() {
        NilUiNode.boxed()
    } else {
        stack! {
            direction = StackDirection::top_to_bottom();
            spacing = PARAGRAPH_SPACING_VAR;
            children = args.items;
            corner_radius = 2;
            background_color = if args.level < 3 {
                TEXT_COLOR_VAR.map(|c| c.with_alpha(5.pct())).boxed()
            } else {
                colors::BLACK.transparent().into_boxed_var()
            };
            border = {
                widths: (0, 0, 0, 4u32.saturating_sub(args.level).max(1) as i32),
                sides: TEXT_COLOR_VAR.map(|c| BorderSides::solid(c.with_alpha(60.pct()))),
            };
            padding = 4;
        }
        .boxed()
    }
}

/// Default markdown table.
///
/// See [`TABLE_GEN_VAR`] for more details.
pub fn default_table_gen(args: TableGenArgs) -> impl UiNode {
    use crate::widgets::layouts::grid;

    grid! {
        background_color = TEXT_COLOR_VAR.map(|c| c.with_alpha(5.pct()));
        border = 1, TEXT_COLOR_VAR.map(|c| c.with_alpha(30.pct()).into());
        align = Align::LEFT;
        auto_grow_gen = wgt_gen!(|_, args: grid::AutoGrowGenArgs| {
            grid::row! {
                border = (0, 0, 1, 0), TEXT_COLOR_VAR.map(|c| c.with_alpha(10.pct()).into());
                background_color = {
                    let alpha = if args.index % 2 == 0 {
                        5.pct()
                    } else {
                        0.pct()
                    };
                    TEXT_COLOR_VAR.map(move |c| c.with_alpha(alpha))
                };

                when *#is_last {
                    border = 0, BorderStyle::Hidden;
                }
            }
        });
        columns = std::iter::repeat_with(|| grid::column!{}.boxed()).take(args.columns.len()).collect::<UiNodeVec>();
        cells = args.cells;
    }
}

/// Default markdown table.
///
/// See [`TABLE_CELL_GEN_VAR`] for more details.
pub fn default_table_cell_gen(args: TableCellGenArgs) -> impl UiNode {
    use crate::prelude::*;

    if args.items.is_empty() {
        NilUiNode.boxed()
    } else if args.is_heading {
        wrap! {
            crate::widgets::text::font_weight = crate::core::text::FontWeight::BOLD;
            padding = 6;
            child_align = args.col_align;
            children = args.items;
        }
        .boxed()
    } else {
        wrap! {
            padding = 6;
            child_align = args.col_align;
            children = args.items;
        }
        .boxed()
    }
}

/// Default markdown panel.
///
/// See [`PANEL_GEN_VAR`] for more details.
pub fn default_panel_gen(mut args: PanelGenArgs) -> impl UiNode {
    use crate::prelude::*;

    if args.items.is_empty() {
        NilUiNode.boxed()
    } else if args.items.len() == 1 {
        args.items.remove(0)
    } else {
        stack! {
            direction = StackDirection::top_to_bottom();
            spacing = PARAGRAPH_SPACING_VAR;
            children = args.items;
        }
        .boxed()
    }
}

/// Default markdown footnote definition.
///
/// See [`FOOTNOTE_REF_GEN_VAR`] for more details.
pub fn default_footnote_ref_gen(args: FootnoteRefGenArgs) -> impl UiNode {
    use crate::widgets::*;

    let url = formatx!("#footnote-{}", args.label);
    link! {
        font_size = 0.7.em();
        offset = (0, (-0.5).em());
        markdown::anchor = formatx!("footnote-ref-{}", args.label);
        child = text!("[{}]", args.label);
        on_click = hn!(|ctx, args: &ClickArgs| {
            args.propagation().stop();

            let link = ctx.info_tree.get(ctx.path.widget_id()).unwrap().interaction_path();
            markdown::LINK_EVENT.notify(ctx.events, markdown::LinkArgs::now(url.clone(), link));
        });
    }
}

/// Default markdown footnote definition.
///
/// See [`FOOTNOTE_DEF_GEN_VAR`] for more details.
pub fn default_footnote_def_gen(args: FootnoteDefGenArgs) -> impl UiNode {
    use crate::prelude::*;

    let mut items = args.items;
    let items = if items.is_empty() {
        NilUiNode.boxed()
    } else if items.len() == 1 {
        items.remove(0)
    } else {
        stack! {
            direction = StackDirection::top_to_bottom();
            children = items;
        }
        .boxed()
    };

    let url_back = formatx!("#footnote-ref-{}", args.label);
    stack! {
        direction = StackDirection::left_to_right();
        spacing = 0.5.em();
        markdown::anchor = formatx!("footnote-{}", args.label);
        children = ui_vec![
            link! {
                child = text!("[^{}]", args.label);
                on_click = hn!(|ctx, args: &ClickArgs| {
                    args.propagation().stop();

                    let link = ctx.info_tree.get(ctx.path.widget_id()).unwrap().interaction_path();
                    markdown::LINK_EVENT.notify(ctx.events, markdown::LinkArgs::now(url_back.clone(), link));
                });
            },
            items,
        ];
    }
}
