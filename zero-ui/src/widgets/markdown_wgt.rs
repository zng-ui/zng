use std::mem;

pub use pulldown_cmark::HeadingLevel;

use crate::prelude::new_widget::*;

mod resolvers;
mod view_gen;

/// Render markdown styled text.
#[widget($crate::widgets::markdown)]
pub mod markdown {
    use super::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use super::markdown_node;

    pub use super::resolvers::*;
    #[doc(inline)]
    pub use super::view_gen::*;

    #[doc(no_inline)]
    pub use crate::widgets::text::{line_spacing, paragraph_spacing};

    properties! {
        /// Markdown text.
        pub md(impl IntoVar<Text>);

        on_link = hn!(|ctx, args: &LinkArgs| {
            try_default_link_action(ctx, args);
        })
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
        fn info(&self, ctx: &mut InfoContext, info: &mut WidgetInfoBuilder) {
            info.meta().set(&markdown::MARKDOWN_INFO_ID, ());
            self.child.info(ctx, info);
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            use resolvers::*;
            use view_gen::*;

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
                || LINK_RESOLVER_VAR.is_new(ctx)
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
    use pulldown_cmark::*;
    use resolvers::*;
    use view_gen::*;

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
    let link_resolver = LINK_RESOLVER_VAR.get();

    let mut blocks = vec![];
    let mut inlines = vec![];
    let mut link_start = None;
    let mut list_item_num = None;
    let mut list_item_checked = None;
    let mut list_items = vec![];
    let mut block_quote_start = vec![];
    let mut code_block_text = None;
    let mut heading_text = None;

    for item in Parser::new_with_broken_link_callback(md, Options::all(), Some(&mut |b| Some((b.reference, "".into())))) {
        match item {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading(_, _, _) => {
                    heading_text = Some(String::new());
                }
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
                Tag::Link(_, _, _) => {
                    link_start = Some(inlines.len());
                }
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
                                anchor: heading_anchor(heading_text.take().unwrap_or_default().as_str()),
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
                            depth: 0,
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
                            depth: 0,
                            num,
                            checked: list_item_checked.take(),
                            items: mem::take(&mut inlines).into(),
                            nested_list: None,
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
                    let url = link_resolver.resolve(url.as_ref());
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
                        if let Some(s) = link_start.take() {
                            let items = inlines.drain(s..).collect();
                            inlines.push(link_view.generate(
                                ctx,
                                LinkViewArgs {
                                    url,
                                    title: title.to_text(),
                                    items,
                                },
                            ));
                        }
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
                    if let Some(t) = &mut heading_text {
                        t.push_str(&txt);
                    }
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

/// Simple markdown run.
///
/// See [`markdown!`] for the full widget.
///
/// [`markdown!`]: mod@markdown
pub fn markdown(md: impl IntoVar<Text>) -> impl UiNode {
    markdown!(md)
}
