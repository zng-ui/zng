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
/// See [`TEXT_VIEW_VAR`] for more details.
pub struct TextViewArgs {
    /// The text run.
    pub txt: Text,
    /// The style.
    pub style: MarkdownStyle,
}

/// Arguments for a markdown inlined link view.
///
/// See [`LINK_VIEW_VAR`] for more details.
pub struct LinkViewArgs {
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
/// See [`CODE_INLINE_VIEW_VAR`] for more details.
pub struct CodeInlineViewArgs {
    /// The code text run.
    pub txt: Text,
    /// The style.
    pub style: MarkdownStyle,
}

/// Arguments for a markdown code block view.
///
/// See [`CODE_BLOCK_VIEW_VAR`] for more details.
pub struct CodeBlockViewArgs {
    /// Code language, can be empty.
    pub lang: Text,
    /// Raw text.
    pub txt: Text,
}

/// Arguments for a markdown paragraph view.
///
/// See [`PARAGRAPH_VIEW_VAR`] for more details.
pub struct ParagraphViewArgs {
    /// Zero-sized index of the paragraph.
    pub index: u32,
    /// Inline items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown heading view.
pub struct HeadingViewArgs {
    /// Level.
    pub level: HeadingLevel,

    /// Anchor label that identifies the header in the markdown context.
    pub anchor: Text,

    /// Inline items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown list view.
pub struct ListViewArgs {
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
pub struct ListItemBulletViewArgs {
    /// Nested list depth, starting from zero for items in the outer-list.
    pub depth: u32,

    /// If the list is *ordered*, the item number.
    pub num: Option<u64>,

    /// If the list is checked. `Some(true)` is `[x]` and `Some(false)` is `[ ]`.
    pub checked: Option<bool>,
}

/// Arguments for a markdown list item view.
pub struct ListItemViewArgs {
    /// Copy of the bullet args.
    pub bullet: ListItemBulletViewArgs,

    /// Inline items of the list item.
    pub items: UiNodeVec,

    /// Inner list defined inside this item.
    pub nested_list: Option<BoxedUiNode>,
}

/// Arguments for a markdown image view.
pub struct ImageViewArgs {
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
pub struct RuleViewArgs {}

/// Arguments for a markdown block quote view.
pub struct BlockQuoteViewArgs {
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
pub struct FootnoteRefViewArgs {
    /// Footnote referenced.
    pub label: Text,
}

/// Arguments for a markdown footnote definition view.
///
/// See [`PARAGRAPH_VIEW_VAR`] for more details.
pub struct FootnoteDefViewArgs {
    /// Identifier label.
    pub label: Text,
    /// Block items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown table view.
///
/// See [`TABLE_VIEW_VAR`] for more details.
pub struct TableViewArgs {
    /// Column definitions with align.
    pub columns: Vec<Align>,
    /// Cell items.
    pub cells: UiNodeVec,
}

/// Arguments for a markdown table cell view.
///
/// See [`TABLE_CELL_VIEW_VAR`] for more details.
pub struct TableCellViewArgs {
    /// If the cell is inside the header row.
    pub is_heading: bool,

    /// Column align.
    pub col_align: Align,

    /// Inline items.
    pub items: UiNodeVec,
}

/// Arguments for a markdown panel.
///
/// See [`PANEL_VIEW_VAR`] for more details.
pub struct PanelViewArgs {
    /// Block items.
    pub items: UiNodeVec,
}

context_var! {
    /// View generator for a markdown text segment.
    pub static TEXT_VIEW_VAR: ViewGenerator<TextViewArgs> = ViewGenerator::new(|_, args| default_text_view(args));

    /// View generator for a markdown link segment.
    pub static LINK_VIEW_VAR: ViewGenerator<LinkViewArgs> = ViewGenerator::new(|_, args| default_link_view(args));

    /// View generator for a markdown inline code segment.
    pub static CODE_INLINE_VIEW_VAR: ViewGenerator<CodeInlineViewArgs> = ViewGenerator::new(|_, args| default_code_inline_view(args));

    /// View generator for a markdown code block segment.
    pub static CODE_BLOCK_VIEW_VAR: ViewGenerator<CodeBlockViewArgs> = ViewGenerator::new(|_, args| default_code_block_view(args));

    /// View generator for a markdown paragraph.
    pub static PARAGRAPH_VIEW_VAR: ViewGenerator<ParagraphViewArgs> = ViewGenerator::new(|_, args| default_paragraph_view(args));

    /// View generator for a markdown heading.
    pub static HEADING_VIEW_VAR: ViewGenerator<HeadingViewArgs> = ViewGenerator::new(|_, args| default_heading_view(args));

    /// View generator for a markdown list.
    pub static LIST_VIEW_VAR: ViewGenerator<ListViewArgs> = ViewGenerator::new(|_, args| default_list_view(args));

    /// View generator for a markdown list item bullet, checkmark or number.
    pub static LIST_ITEM_BULLET_VIEW_VAR: ViewGenerator<ListItemBulletViewArgs> = ViewGenerator::new(|_, args| default_list_item_bullet_view(args));

    /// View generator for a markdown list item content.
    pub static LIST_ITEM_VIEW_VAR: ViewGenerator<ListItemViewArgs> = ViewGenerator::new(|_, args| default_list_item_view(args));

    /// View generator for a markdown image.
    pub static IMAGE_VIEW_VAR: ViewGenerator<ImageViewArgs> = ViewGenerator::new(|_, args| default_image_view(args));

    /// View generator for a markdown rule line.
    pub static RULE_VIEW_VAR: ViewGenerator<RuleViewArgs> = ViewGenerator::new(|_, args| default_rule_view(args));

    /// View generator for a markdown block quote.
    pub static BLOCK_QUOTE_VIEW_VAR: ViewGenerator<BlockQuoteViewArgs> = ViewGenerator::new(|_, args| default_block_quote_view(args));

    /// View generator for an inline reference to a footnote.
    pub static FOOTNOTE_REF_VIEW_VAR: ViewGenerator<FootnoteRefViewArgs> = ViewGenerator::new(|_, args| default_footnote_ref_view(args));

    /// View generator for a footnote definition block.
    pub static FOOTNOTE_DEF_VIEW_VAR: ViewGenerator<FootnoteDefViewArgs> = ViewGenerator::new(|_, args| default_footnote_def_view(args));

    /// View generator for a markdown table.
    pub static TABLE_VIEW_VAR: ViewGenerator<TableViewArgs> = ViewGenerator::new(|_, args| default_table_view(args));

    /// View generator for a markdown table body cell.
    pub static TABLE_CELL_VIEW_VAR: ViewGenerator<TableCellViewArgs> = ViewGenerator::new(|_, args| default_table_cell_view(args));

    /// View generator for a markdown panel.
    pub static PANEL_VIEW_VAR: ViewGenerator<PanelViewArgs> = ViewGenerator::new(|_, args| default_panel_view(args));
}

/// View generator that converts [`TextViewArgs`] to widgets.
///
/// Sets the [`TEXT_VIEW_VAR`].
#[property(CONTEXT, default(TEXT_VIEW_VAR))]
pub fn text_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<TextViewArgs>>) -> impl UiNode {
    with_context_var(child, TEXT_VIEW_VAR, view)
}

/// View generator that converts [`LinkViewArgs`] to widgets.
///
/// Sets the [`LINK_VIEW_VAR`].
#[property(CONTEXT, default(LINK_VIEW_VAR))]
pub fn link_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<LinkViewArgs>>) -> impl UiNode {
    with_context_var(child, LINK_VIEW_VAR, view)
}

/// View generator that converts [`CodeInlineViewArgs`] to widgets.
///
/// Sets the [`CODE_INLINE_VIEW_VAR`].
#[property(CONTEXT, default(CODE_INLINE_VIEW_VAR))]
pub fn code_inline_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<CodeInlineViewArgs>>) -> impl UiNode {
    with_context_var(child, CODE_INLINE_VIEW_VAR, view)
}

/// View generator that converts [`CodeBlockViewArgs`] to widgets.
///
/// Sets the [`CODE_BLOCK_VIEW_VAR`].
#[property(CONTEXT, default(CODE_BLOCK_VIEW_VAR))]
pub fn code_block_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<CodeBlockViewArgs>>) -> impl UiNode {
    with_context_var(child, CODE_BLOCK_VIEW_VAR, view)
}

/// View generator that converts [`ParagraphViewArgs`] to widgets.
///
/// Sets the [`PARAGRAPH_VIEW_VAR`].
#[property(CONTEXT, default(PARAGRAPH_VIEW_VAR))]
pub fn paragraph_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<ParagraphViewArgs>>) -> impl UiNode {
    with_context_var(child, PARAGRAPH_VIEW_VAR, view)
}

/// View generator that converts [`HeadingViewArgs`] to widgets.
///
/// Sets the [`HEADING_VIEW_VAR`].
#[property(CONTEXT, default(HEADING_VIEW_VAR))]
pub fn heading_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<HeadingViewArgs>>) -> impl UiNode {
    with_context_var(child, HEADING_VIEW_VAR, view)
}

/// View generator that converts [`ListViewArgs`] to widgets.
///
/// Sets the [`LIST_VIEW_VAR`].
#[property(CONTEXT, default(LIST_VIEW_VAR))]
pub fn list_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<ListViewArgs>>) -> impl UiNode {
    with_context_var(child, LIST_VIEW_VAR, view)
}

/// View generator that converts [`ListItemBulletViewArgs`] to widgets.
///
/// Sets the [`LIST_ITEM_BULLET_VIEW_VAR`].
#[property(CONTEXT, default(LIST_ITEM_BULLET_VIEW_VAR))]
pub fn list_item_bullet_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<ListItemBulletViewArgs>>) -> impl UiNode {
    with_context_var(child, LIST_ITEM_BULLET_VIEW_VAR, view)
}

/// View generator that converts [`ListItemViewArgs`] to widgets.
///
/// Sets the [`LIST_ITEM_VIEW_VAR`].
#[property(CONTEXT, default(LIST_ITEM_VIEW_VAR))]
pub fn list_item_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<ListItemViewArgs>>) -> impl UiNode {
    with_context_var(child, LIST_ITEM_VIEW_VAR, view)
}

/// View generator that converts [`ImageViewArgs`] to widgets.
///
/// Sets the [`IMAGE_VIEW_VAR`].
#[property(CONTEXT, default(IMAGE_VIEW_VAR))]
pub fn image_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<ImageViewArgs>>) -> impl UiNode {
    with_context_var(child, IMAGE_VIEW_VAR, view)
}

/// View generator that converts [`RuleViewArgs`] to widgets.
///
/// Sets the [`RULE_VIEW_VAR`].
#[property(CONTEXT, default(RULE_VIEW_VAR))]
pub fn rule_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<RuleViewArgs>>) -> impl UiNode {
    with_context_var(child, RULE_VIEW_VAR, view)
}

/// View generator that converts [`BlockQuoteViewArgs`] to widgets.
///
/// Sets the [`BLOCK_QUOTE_VIEW_VAR`].
#[property(CONTEXT, default(BLOCK_QUOTE_VIEW_VAR))]
pub fn block_quote_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<BlockQuoteViewArgs>>) -> impl UiNode {
    with_context_var(child, BLOCK_QUOTE_VIEW_VAR, view)
}

/// View generator that converts [`FootnoteRefViewArgs`] to widgets.
///
/// Sets the [`FOOTNOTE_REF_VIEW_VAR`].
#[property(CONTEXT, default(FOOTNOTE_REF_VIEW_VAR))]
pub fn footnote_ref_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<FootnoteRefViewArgs>>) -> impl UiNode {
    with_context_var(child, FOOTNOTE_REF_VIEW_VAR, view)
}

/// View generator that converts [`FootnoteDefViewArgs`] to widgets.
///
/// Sets the [`FOOTNOTE_DEF_VIEW_VAR`].
#[property(CONTEXT, default(FOOTNOTE_DEF_VIEW_VAR))]
pub fn footnote_def_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<FootnoteDefViewArgs>>) -> impl UiNode {
    with_context_var(child, FOOTNOTE_DEF_VIEW_VAR, view)
}

/// View generator that converts [`TableViewArgs`] to widgets.
///
/// Sets the [`TABLE_VIEW_VAR`].
#[property(CONTEXT, default(TABLE_VIEW_VAR))]
pub fn table_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<TableViewArgs>>) -> impl UiNode {
    with_context_var(child, TABLE_VIEW_VAR, view)
}

/// View generator that converts [`PanelViewArgs`] to a widget.
///
/// This generates the panel that contains all markdown blocks, it is the child of the [`markdown!`] widget.
///
/// Sets the [`PANEL_VIEW_VAR`].
///
/// [`markdown!`]: mod@crate::widgets::markdown
#[property(CONTEXT, default(PANEL_VIEW_VAR))]
pub fn panel_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<PanelViewArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_VIEW_VAR, view)
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
/// See [`TEXT_VIEW_VAR`] for more details.
pub fn default_text_view(args: TextViewArgs) -> impl UiNode {
    let builder = text_view_builder(args.txt, args.style);
    crate::widgets::text::build(builder)
}

/// Default inlined code text view.
///
/// See [`CODE_INLINE_VIEW_VAR`] for more details.
pub fn default_code_inline_view(args: CodeInlineViewArgs) -> impl UiNode {
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
/// See [`LINK_VIEW_VAR`] for more details.
pub fn default_link_view(args: LinkViewArgs) -> impl UiNode {
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
/// See [`CODE_BLOCK_VIEW_VAR`] for more details.
///
/// [`ansi_text!`]: mod@crate::widgets::ansi_text
pub fn default_code_block_view(args: CodeBlockViewArgs) -> impl UiNode {
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
/// See [`PARAGRAPH_VIEW_VAR`] for more details.
pub fn default_paragraph_view(mut args: ParagraphViewArgs) -> impl UiNode {
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
/// See [`HEADING_VIEW_VAR`] for more details.
pub fn default_heading_view(args: HeadingViewArgs) -> impl UiNode {
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
/// See [`LIST_VIEW_VAR`] for more details.
///
/// [`grid!`]: mod@crate::widgets::layouts::grid
pub fn default_list_view(args: ListViewArgs) -> impl UiNode {
    if args.items.is_empty() {
        NilUiNode.boxed()
    } else {
        use crate::widgets::layouts::grid;
        grid! {
            margin = (0, 0, 0, 1.em());
            cells = args.items;
            columns = ui_list![
                grid::column!(),
                grid::column! { width = 1.lft() },
            ];
        }
        .boxed()
    }
}

/// Default list item bullet, checkmark or number view.
///
/// See [`LIST_ITEM_BULLET_VIEW_VAR`] for more details.
pub fn default_list_item_bullet_view(args: ListItemBulletViewArgs) -> impl UiNode {
    if let Some(checked) = args.checked {
        crate::widgets::text! {
            txt = " ✓ ";
            txt_color = TEXT_COLOR_VAR.map(move |c| if checked { *c } else { c.transparent() });
            background_color = TEXT_COLOR_VAR.map(|c| c.with_alpha(10.pct()));
            corner_radius = 4;
            scale = 0.8.fct();
            offset = (-(0.1.fct()), 0);
        }
        .boxed()
    } else if let Some(n) = args.num {
        crate::widgets::text! {
            txt = formatx!("{n}. ");
            align = Align::RIGHT;
        }
        .boxed()
    } else {
        match args.depth {
            0 => crate::widgets::wgt! {
                size = (5, 5);
                corner_radius = 5;
                margin = (0.6.em(), 0.5.em(), 0, 0);
                background_color = TEXT_COLOR_VAR;
            },
            1 => crate::widgets::wgt! {
                size = (5, 5);
                corner_radius = 5;
                margin = (0.6.em(), 0.5.em(), 0, 0);
                border = 1.px(), TEXT_COLOR_VAR.map_into();
            },
            _ => crate::widgets::wgt! {
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
/// See [`LIST_ITEM_VIEW_VAR`] for more details.
pub fn default_list_item_view(args: ListItemViewArgs) -> impl UiNode {
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
        crate::widgets::layouts::wrap! {
            children = items;
        }
        .boxed()
    };

    if let Some(inner) = args.nested_list {
        r = crate::widgets::layouts::v_stack! {
            children = ui_list![
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
/// See [`IMAGE_VIEW_VAR`] for more details.
pub fn default_image_view(args: ImageViewArgs) -> impl UiNode {
    let mut alt_items = args.alt_items;
    if alt_items.is_empty() {
        crate::widgets::image! {
            align = Align::TOP_LEFT;
            source = args.source;
        }
    } else {
        let alt_items = if alt_items.len() == 1 {
            alt_items.remove(0)
        } else {
            crate::widgets::layouts::wrap! {
                children = alt_items;
            }
            .boxed()
        };
        let alt_items = crate::core::widget_instance::ArcNode::new(alt_items);
        crate::widgets::image! {
            align = Align::TOP_LEFT;
            source = args.source;
            img_error_view = view_generator!(|_, _| {
                alt_items.take_on_init()
            });
        }
    }
}

/// Default rule view.
///
/// See [`RULE_VIEW_VAR`] for more details.
pub fn default_rule_view(_: RuleViewArgs) -> impl UiNode {
    crate::widgets::hr! {
        opacity = 50.pct();
    }
}

/// Default block quote view.
///
/// See [`BLOCK_QUOTE_VIEW_VAR`] for more details.
pub fn default_block_quote_view(args: BlockQuoteViewArgs) -> impl UiNode {
    if args.items.is_empty() {
        NilUiNode.boxed()
    } else {
        crate::widgets::layouts::v_stack! {
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
/// See [`TABLE_VIEW_VAR`] for more details.
pub fn default_table_view(args: TableViewArgs) -> impl UiNode {
    use crate::widgets::layouts::grid;

    grid! {
        background_color = TEXT_COLOR_VAR.map(|c| c.with_alpha(5.pct()));
        border = 1, TEXT_COLOR_VAR.map(|c| c.with_alpha(30.pct()).into());
        align = Align::LEFT;
        auto_grow_view = view_generator!(|_, args: grid::AutoGrowViewArgs| {
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
/// See [`TABLE_CELL_VIEW_VAR`] for more details.
pub fn default_table_cell_view(args: TableCellViewArgs) -> impl UiNode {
    if args.items.is_empty() {
        NilUiNode.boxed()
    } else if args.is_heading {
        crate::widgets::layouts::wrap! {
            crate::widgets::text::font_weight = crate::core::text::FontWeight::BOLD;
            padding = 6;
            child_align = args.col_align;
            children = args.items;
        }
        .boxed()
    } else {
        crate::widgets::layouts::wrap! {
            padding = 6;
            child_align = args.col_align;
            children = args.items;
        }
        .boxed()
    }
}

/// Default markdown panel.
///
/// See [`PANEL_VIEW_VAR`] for more details.
pub fn default_panel_view(mut args: PanelViewArgs) -> impl UiNode {
    if args.items.is_empty() {
        NilUiNode.boxed()
    } else if args.items.len() == 1 {
        args.items.remove(0)
    } else {
        crate::widgets::layouts::v_stack! {
            spacing = PARAGRAPH_SPACING_VAR;
            children = args.items;
        }
        .boxed()
    }
}

/// Default markdown footnote definition.
///
/// See [`FOOTNOTE_REF_VIEW_VAR`] for more details.
pub fn default_footnote_ref_view(args: FootnoteRefViewArgs) -> impl UiNode {
    use crate::widgets::*;

    let url = formatx!("#footnote-{}", args.label);
    link! {
        font_size = 0.7.em();
        offset = (0, (-0.5).em());
        markdown::anchor = formatx!("footnote-ref-{}", args.label);
        child = text(formatx!("[{}]", args.label));
        on_click = hn!(|ctx, args: &ClickArgs| {
            args.propagation().stop();

            let link = ctx.info_tree.get(ctx.path.widget_id()).unwrap().interaction_path();
            markdown::LINK_EVENT.notify(ctx.events, markdown::LinkArgs::now(url.clone(), link));
        });
    }
}

/// Default markdown footnote definition.
///
/// See [`FOOTNOTE_DEF_VIEW_VAR`] for more details.
pub fn default_footnote_def_view(args: FootnoteDefViewArgs) -> impl UiNode {
    let mut items = args.items;
    let items = if items.is_empty() {
        NilUiNode.boxed()
    } else if items.len() == 1 {
        items.remove(0)
    } else {
        crate::widgets::layouts::v_stack! {
            children = items;
        }
        .boxed()
    };

    let url_back = formatx!("#footnote-ref-{}", args.label);
    crate::widgets::layouts::h_stack! {
        spacing = 0.5.em();
        crate::widgets::markdown::anchor = formatx!("footnote-{}", args.label);
        children = ui_list![
            crate::widgets::link! {
                child = crate::widgets::text(formatx!("[^{}]", args.label));
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
