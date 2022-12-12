use std::mem;

use crate::prelude::new_widget::*;

/// Render markdown styled text.
#[widget($crate::widgets::markdown)]
pub mod markdown {
    use super::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use super::markdown_node;

    #[doc(inline)]
    pub use super::markdown_view::*;

    #[doc(inline)]
    pub use super::markdown_view::*;

    #[doc(no_inline)]
    pub use crate::widgets::text::{line_spacing, paragraph_spacing};

    properties! {
        /// Markdown text.
        pub md(impl IntoVar<Text>);
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let md = wgt.capture_var_or_default(property_id!(self::md));
            let child = markdown_node(md);
            wgt.set_child(child.boxed());
        });
    }
}

/// Implements the markdown parsing and view generation, configured by contextual properties.
pub fn markdown_node(md: impl IntoVar<Text>) -> impl UiNode {
    #[ui_node(struct MarkdownNode {
        child: BoxedUiNode,
        #[var] md: impl Var<Text>,
    })]
    impl MarkdownNode {
        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.init_handles(ctx);
            self.generate_child(ctx);
            self.child.init(ctx);
        }

        #[UiNode]
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.child = NilUiNode.boxed();
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            use markdown_view::*;

            if self.md.is_new(ctx)
                || TEXT_VIEW_VAR.is_new(ctx)
                || LINK_VIEW_VAR.is_new(ctx)
                || CODE_INLINE_VIEW_VAR.is_new(ctx)
                || CODE_BLOCK_VIEW_VAR.is_new(ctx)
                || PARAGRAPH_VIEW_VAR.is_new(ctx)
                || HEADING_VIEW_VAR.is_new(ctx)
                || LIST_VIEW_VAR.is_new(ctx)
                || LIST_ITEM_VIEW_VAR.is_new(ctx)
                || IMAGE_VIEW_VAR.is_new(ctx)
                || RULE_VIEW_VAR.is_new(ctx)
                || BLOCK_QUOTE_VIEW_VAR.is_new(ctx)
                || TABLE_VIEW_VAR.is_new(ctx)
                || PANEL_VIEW_VAR.is_new(ctx)
                || IMAGE_RESOLVER_VAR.is_new(ctx)
            {
                self.child.deinit(ctx);
                self.generate_child(ctx);
                self.child.init(ctx);
                ctx.updates.info_layout_render();
            } else {
                self.child.update(ctx, updates);
            }
        }

        fn generate_child(&mut self, ctx: &mut WidgetContext) {
            self.child = self.md.with(|md| markdown_view_gen(ctx, md.as_str())).boxed();
        }
    }
    MarkdownNode {
        child: NilUiNode.boxed(),
        md: md.into_var(),
    }
}

fn markdown_view_gen(ctx: &mut WidgetContext, md: &str) -> impl UiNode {
    use markdown_view::*;
    use pulldown_cmark::*;

    let mut strong = 0;
    let mut emphasis = 0;
    let mut strikethrough = 0;

    let text_view = TEXT_VIEW_VAR.get();
    let link_view = LINK_VIEW_VAR.get();
    let code_inline_view = CODE_INLINE_VIEW_VAR.get();
    let code_block_view = CODE_BLOCK_VIEW_VAR.get();
    let heading_view = HEADING_VIEW_VAR.get();
    let paragraph_view = PARAGRAPH_VIEW_VAR.get();
    let list_view = LIST_VIEW_VAR.get();
    let list_item_view = LIST_ITEM_VIEW_VAR.get();
    let image_view = IMAGE_VIEW_VAR.get();
    let rule_view = RULE_VIEW_VAR.get();
    let block_quote_view = BLOCK_QUOTE_VIEW_VAR.get();
    let footnote_ref_view = FOOTNOTE_REF_VIEW_VAR.get();
    let footnote_def_view = FOOTNOTE_DEF_VIEW_VAR.get();
    let table_view = TABLE_VIEW_VAR.get();

    let image_resolver = IMAGE_RESOLVER_VAR.get();

    let mut blocks = vec![];
    let mut inlines = vec![];
    let mut list_item_num = None;
    let mut list_item_checked = None;
    let mut list_items = vec![];
    let mut block_quote_start = vec![];
    let mut code_block_text = None;

    for item in Parser::new_with_broken_link_callback(md, Options::all(), Some(&mut |b| Some((b.reference, "".into())))) {
        match item {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading(_, _, _) => {}
                Tag::BlockQuote => {
                    block_quote_start.push(blocks.len());
                }
                Tag::CodeBlock(_) => {
                    code_block_text = Some(String::new());
                }
                Tag::List(n) => {
                    list_item_num = n;
                }
                Tag::Item => {}
                Tag::FootnoteDefinition(_) => {}
                Tag::Table(_) => {}
                Tag::TableHead => {}
                Tag::TableRow => {}
                Tag::TableCell => {}
                Tag::Emphasis => {
                    emphasis += 1;
                }
                Tag::Strong => {
                    strong += 1;
                }
                Tag::Strikethrough => {
                    strong += 1;
                }
                Tag::Link(_, _, _) => {}
                Tag::Image(_, _, _) => {}
            },
            Event::End(tag) => match tag {
                Tag::Paragraph => {
                    if !inlines.is_empty() {
                        blocks.push(paragraph_view.generate(
                            ctx,
                            ParagraphViewArgs {
                                index: blocks.len() as u32,
                                items: mem::take(&mut inlines).into(),
                            },
                        ));
                    }
                }
                Tag::Heading(level, _, _) => {
                    if !inlines.is_empty() {
                        blocks.push(heading_view.generate(
                            ctx,
                            HeadingViewArgs {
                                level,
                                items: mem::take(&mut inlines).into(),
                            },
                        ));
                    }
                }
                Tag::BlockQuote => {
                    if let Some(start) = block_quote_start.pop() {
                        let items: UiNodeVec = blocks.drain(start..).collect();
                        if !items.is_empty() {
                            blocks.push(block_quote_view.generate(
                                ctx,
                                BlockQuoteViewArgs {
                                    level: block_quote_start.len() as u32,
                                    items,
                                },
                            ));
                        }
                    }
                }
                Tag::CodeBlock(kind) => {
                    if let Some(mut txt) = code_block_text.take() {
                        let _last_line_break = txt.pop();
                        debug_assert_eq!(Some('\n'), _last_line_break);
                        blocks.push(code_block_view.generate(
                            ctx,
                            CodeBlockViewArgs {
                                lang: match kind {
                                    CodeBlockKind::Indented => Text::empty(),
                                    CodeBlockKind::Fenced(l) => l.to_text(),
                                },
                                txt: txt.into(),
                            },
                        ))
                    }
                }
                Tag::List(n) => {
                    blocks.push(list_view.generate(
                        ctx,
                        ListViewArgs {
                            first_num: n,
                            items: mem::take(&mut list_items).into(),
                        },
                    ));
                }
                Tag::Item => {
                    let num = match &mut list_item_num {
                        Some(n) => {
                            let r = *n;
                            *n += 1;
                            Some(r)
                        }
                        None => None,
                    };

                    list_items.push(list_item_view.generate(
                        ctx,
                        ListItemViewArgs {
                            num,
                            checked: list_item_checked.take(),
                            items: mem::take(&mut inlines).into(),
                        },
                    ));
                }
                Tag::FootnoteDefinition(label) => {
                    let label = html_escape::decode_html_entities(label.as_ref());
                    blocks.push(footnote_def_view.generate(
                        ctx,
                        FootnoteDefViewArgs {
                            label: label.to_text(),
                            items: mem::take(&mut inlines).into(),
                        },
                    ));
                }
                Tag::Table(_) => {
                    // !!: TODO
                    inlines.clear();
                }
                Tag::TableHead => {}
                Tag::TableRow => {}
                Tag::TableCell => {}
                Tag::Emphasis => {
                    emphasis -= 1;
                }
                Tag::Strong => {
                    strong -= 1;
                }
                Tag::Strikethrough => {
                    strikethrough -= 1;
                }
                Tag::Link(kind, url, title) => {
                    let title = html_escape::decode_html_entities(title.as_ref());
                    match kind {
                        LinkType::Inline => {}
                        LinkType::Reference => {}
                        LinkType::ReferenceUnknown => {}
                        LinkType::Collapsed => {}
                        LinkType::CollapsedUnknown => {}
                        LinkType::Shortcut => {}
                        LinkType::ShortcutUnknown => {}
                        LinkType::Autolink | LinkType::Email => {
                            let url = html_escape::decode_html_entities(url.as_ref());
                            inlines.push(text_view.generate(
                                ctx,
                                TextViewArgs {
                                    txt: url.to_text(),
                                    style: MarkdownStyle {
                                        strong: strong > 0,
                                        emphasis: emphasis > 0,
                                        strikethrough: strikethrough > 0,
                                    },
                                },
                            ));
                        }
                    }
                    if !inlines.is_empty() {
                        let items = mem::take(&mut inlines);
                        inlines.push(link_view.generate(
                            ctx,
                            LinkViewArgs {
                                title: title.to_text(),
                                items: items.into(),
                            },
                        ));
                    }
                }
                Tag::Image(_, url, title) => {
                    let title = html_escape::decode_html_entities(title.as_ref());
                    blocks.push(image_view.generate(
                        ctx,
                        ImageViewArgs {
                            source: image_resolver.resolve(&url),
                            title: title.to_text(),
                            alt_items: mem::take(&mut inlines).into(),
                        },
                    ));
                }
            },
            Event::Text(txt) => {
                let txt = html_escape::decode_html_entities(txt.as_ref());
                if let Some(t) = &mut code_block_text {
                    t.push_str(&txt);
                } else {
                    inlines.push(
                        text_view
                            .generate(
                                ctx,
                                TextViewArgs {
                                    txt: txt.to_text(),
                                    style: MarkdownStyle {
                                        strong: strong > 0,
                                        emphasis: emphasis > 0,
                                        strikethrough: strikethrough > 0,
                                    },
                                },
                            )
                            .boxed(),
                    );
                }
            }
            Event::Code(txt) => {
                let txt = html_escape::decode_html_entities(txt.as_ref());
                inlines.push(
                    code_inline_view
                        .generate(
                            ctx,
                            CodeInlineViewArgs {
                                txt: txt.to_text(),
                                style: MarkdownStyle {
                                    strong: strong > 0,
                                    emphasis: emphasis > 0,
                                    strikethrough: strikethrough > 0,
                                },
                            },
                        )
                        .boxed(),
                );
            }
            Event::Html(tag) => match tag.as_ref() {
                "<b>" => strong += 1,
                "</b>" => strong -= 1,
                "<em>" => emphasis += 1,
                "</em>" => emphasis -= 1,
                "<s>" => strikethrough += 1,
                "</s>" => strikethrough -= 1,
                _ => {}
            },
            Event::FootnoteReference(label) => {
                let label = html_escape::decode_html_entities(label.as_ref());
                inlines.push(footnote_ref_view.generate(ctx, FootnoteRefViewArgs { label: label.to_text() }));
            }
            Event::SoftBreak => {}
            Event::HardBreak => {}
            Event::Rule => {
                blocks.push(rule_view.generate(ctx, RuleViewArgs {}));
            }
            Event::TaskListMarker(c) => {
                list_item_checked = Some(c);
            }
        }
    }

    PANEL_VIEW_VAR.get().generate(ctx, PanelViewArgs { items: blocks.into() })
}

mod markdown_view {
    use std::{fmt, sync::Arc};

    pub use pulldown_cmark::HeadingLevel;
    use zero_ui_core::image::ImageSource;

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

        /// Inline items.
        pub items: UiNodeVec,
    }

    /// Arguments for a markdown list view.
    pub struct ListViewArgs {
        /// If the list is *ordered*, the first item number.
        pub first_num: Option<u64>,
        /// List items.
        pub items: UiNodeVec,
    }

    /// Arguments for a markdown list item view.
    pub struct ListItemViewArgs {
        /// If the list is *ordered*, the item number.
        pub num: Option<u64>,
        /// If the list is checked. `Some(true)` is `[x]` and `Some(false)` is `[ ]`.
        pub checked: Option<bool>,

        /// Inline items of the list item.
        pub items: UiNodeVec,
    }

    /// Arguments for a markdown image view.
    pub struct ImageViewArgs {
        /// Image, resolved by the [`image_resolver`].
        ///
        /// [`image_resolver`]: fn@image_resolver
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
        /// Inline items.
        pub items: UiNodeVec,
    }

    /// Arguments for a markdown table view.
    ///
    /// See [`TABLE_VIEW_VAR`] for more details.
    pub struct TableViewArgs {}

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

        /// View generator for a markdown list item.
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
            // !!: TODO
            crate::widgets::layouts::wrap! {
                children = args.items;
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
            }
            .boxed()
        }
    }

    /// Default list view.
    ///
    /// See [`LIST_VIEW_VAR`] for more details.
    pub fn default_list_view(args: ListViewArgs) -> impl UiNode {
        if args.items.is_empty() {
            NilUiNode.boxed()
        } else {
            crate::widgets::layouts::v_stack! {
                margin = (0, 0, 0, 1.em());
                children = args.items;
            }
            .boxed()
        }
    }

    /// Default list item view.
    ///
    /// See [`LIST_ITEM_VIEW_VAR`] for more details.
    pub fn default_list_item_view(args: ListItemViewArgs) -> impl UiNode {
        let mut items = args.items;

        if let Some(checked) = args.checked {
            items.0.insert(
                0,
                crate::widgets::text! {
                    txt = " ✓ ";
                    txt_color = TEXT_COLOR_VAR.map(move |c| if checked { *c } else { c.transparent() });
                    background_color = TEXT_COLOR_VAR.map(|c| c.with_alpha(10.pct()));
                    corner_radius = 4;
                    scale = 0.8.fct();
                    offset = (-(0.1.fct()), 0);
                }
                .boxed(),
            );
        }

        if let Some(n) = args.num {
            items.0.insert(
                0,
                crate::widgets::text! {
                    txt = formatx!("{n}. ");
                }
                .boxed(),
            );
        } else if args.checked.is_none() {
            items.0.insert(
                0,
                crate::widgets::text! {
                    txt = "• ";
                    scale = 1.3.fct();
                }
                .boxed(),
            );
        }

        if items.len() == 1 {
            items.remove(0)
        } else {
            crate::widgets::layouts::wrap! {
                children = items;
            }
            .boxed()
        }
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
        crate::widgets::hr!()
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
        // !!: TODO
        NilUiNode
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
    /// See [`FOOTNOTE_REF_VIEW`] for more details.
    pub fn default_footnote_ref_view(args: FootnoteRefViewArgs) -> impl UiNode {
        // !!: TODO, implement links first
        NilUiNode
    }

    /// Default markdown footnote definition.
    ///
    /// See [`FOOTNOTE_DEF_VIEW`] for more details.
    pub fn default_footnote_def_view(args: FootnoteDefViewArgs) -> impl UiNode {
        // !!: TODO, like a list item with the bullet is the label?
        // also need to register the ID with label to the scroll nav.
        NilUiNode
    }

    /// Markdown image resolver.
    ///
    /// See [`IMAGE_RESOLVER_VAR`] for more details.
    #[derive(Clone)]
    pub enum ImageResolver {
        /// No extra resolution, just convert into [`ImageSource`].
        None,
        /// Custom resolution.
        Resolve(Arc<dyn Fn(&str) -> ImageSource + Send + Sync>),
    }
    impl ImageResolver {
        /// Apply the text transform.
        pub fn resolve(&self, img: &str) -> ImageSource {
            match self {
                ImageResolver::None => img.into(),
                ImageResolver::Resolve(r) => r(img),
            }
        }

        /// New [`Resolve`](Self::Resolve).
        pub fn new(fn_: impl Fn(&str) -> ImageSource + Send + Sync + 'static) -> Self {
            ImageResolver::Resolve(Arc::new(fn_))
        }
    }
    impl fmt::Debug for ImageResolver {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            if f.alternate() {
                write!(f, "ImgSourceResolver::")?;
            }
            match self {
                ImageResolver::None => write!(f, "None"),
                ImageResolver::Resolve(_) => write!(f, "Resolve(_)"),
            }
        }
    }

    context_var! {
        /// Markdown image resolver.
        pub static IMAGE_RESOLVER_VAR: ImageResolver = ImageResolver::None;
    }

    /// Markdown image resolver.
    ///
    /// This can be used to override image source resolution, by default the image URL or URI is passed as parsed to the [`image_view`].
    ///
    /// Note that image downloads are blocked by default, you can enable this by using the [`image::img_limits`] property.
    ///
    /// [`image_view`]: fn@image_view
    /// [`image::img_limits`]: fn@crate::widgets::image::img_limits
    #[property(CONTEXT, default(IMAGE_RESOLVER_VAR))]
    pub fn image_resolver(child: impl UiNode, resolver: impl IntoVar<ImageResolver>) -> impl UiNode {
        with_context_var(child, IMAGE_RESOLVER_VAR, resolver)
    }
}
