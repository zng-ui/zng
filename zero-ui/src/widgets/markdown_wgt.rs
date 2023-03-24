use std::mem;

pub use pulldown_cmark::HeadingLevel;

use crate::prelude::new_widget::*;

mod resolvers;
mod view_gen;

/// Render markdown styled text.
#[widget($crate::widgets::markdown {
    ($md:literal) => {
        md = $crate::core::text::formatx!($md);
    };
    ($md:expr) => {
        md = $md;
    };
    ($md:tt, $($format:tt)*) => {
        md = $crate::core::text::formatx!($md, $($format)*);
    };
})]
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

        on_link = hn!(|args: &LinkArgs| {
            try_default_link_action(args);
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
        fn init(&mut self) {
            self.auto_subs();
            self.generate_child();
            self.child.init();
        }

        #[UiNode]
        fn deinit(&mut self) {
            self.child.deinit();
            self.child = NilUiNode.boxed();
        }

        #[UiNode]
        fn info(&self, info: &mut WidgetInfoBuilder) {
            info.meta().set(&markdown::MARKDOWN_INFO_ID, ());
            self.child.info(info);
        }

        #[UiNode]
        fn update(&mut self, updates: &WidgetUpdates) {
            use resolvers::*;
            use view_gen::*;

            if self.md.is_new()
                || TEXT_GEN_VAR.is_new()
                || LINK_GEN_VAR.is_new()
                || CODE_INLINE_GEN_VAR.is_new()
                || CODE_BLOCK_GEN_VAR.is_new()
                || PARAGRAPH_GEN_VAR.is_new()
                || HEADING_GEN_VAR.is_new()
                || LIST_GEN_VAR.is_new()
                || LIST_ITEM_BULLET_GEN_VAR.is_new()
                || LIST_ITEM_GEN_VAR.is_new()
                || IMAGE_GEN_VAR.is_new()
                || RULE_GEN_VAR.is_new()
                || BLOCK_QUOTE_GEN_VAR.is_new()
                || TABLE_GEN_VAR.is_new()
                || TABLE_CELL_GEN_VAR.is_new()
                || PANEL_GEN_VAR.is_new()
                || IMAGE_RESOLVER_VAR.is_new()
                || LINK_RESOLVER_VAR.is_new()
            {
                self.child.deinit();
                self.generate_child();
                self.child.init();
                WIDGET.update_info().layout().render();
            } else {
                self.child.update(updates);
            }
        }

        fn generate_child(&mut self) {
            self.child = self.md.with(|md| markdown_view_gen(md.as_str())).boxed();
        }
    }
    MarkdownNode {
        child: NilUiNode.boxed(),
        md: md.into_var(),
    }
}

fn markdown_view_gen(md: &str) -> impl UiNode {
    use pulldown_cmark::*;
    use resolvers::*;
    use view_gen::*;

    let mut strong = 0;
    let mut emphasis = 0;
    let mut strikethrough = 0;

    let text_view = TEXT_GEN_VAR.get();
    let link_view = LINK_GEN_VAR.get();
    let code_inline_view = CODE_INLINE_GEN_VAR.get();
    let code_block_view = CODE_BLOCK_GEN_VAR.get();
    let heading_view = HEADING_GEN_VAR.get();
    let paragraph_view = PARAGRAPH_GEN_VAR.get();
    let list_view = LIST_GEN_VAR.get();
    let list_item_bullet_view = LIST_ITEM_BULLET_GEN_VAR.get();
    let list_item_view = LIST_ITEM_GEN_VAR.get();
    let image_view = IMAGE_GEN_VAR.get();
    let rule_view = RULE_GEN_VAR.get();
    let block_quote_view = BLOCK_QUOTE_GEN_VAR.get();
    let footnote_ref_view = FOOTNOTE_REF_GEN_VAR.get();
    let footnote_def_view = FOOTNOTE_DEF_GEN_VAR.get();
    let table_view = TABLE_GEN_VAR.get();
    let table_cell_view = TABLE_CELL_GEN_VAR.get();

    let image_resolver = IMAGE_RESOLVER_VAR.get();
    let link_resolver = LINK_RESOLVER_VAR.get();

    struct ListInfo {
        block_start: usize,
        inline_start: usize,
        item_num: Option<u64>,
        item_checked: Option<bool>,
    }
    let mut blocks = vec![];
    let mut inlines = vec![];
    let mut link_start = None;
    let mut list_info = vec![];
    let mut list_items = vec![];
    let mut block_quote_start = vec![];
    let mut code_block_text = None;
    let mut heading_text = None;
    let mut footnote_def_start = None;
    let mut table_cells = vec![];
    let mut table_cols = vec![];
    let mut table_col = 0;
    let mut table_head = false;

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
                    list_info.push(ListInfo {
                        block_start: blocks.len(),
                        inline_start: inlines.len(),
                        item_num: n,
                        item_checked: None,
                    });
                }
                Tag::Item => {}
                Tag::FootnoteDefinition(_) => {
                    footnote_def_start = Some(blocks.len());
                }
                Tag::Table(columns) => {
                    table_cols = columns
                        .into_iter()
                        .map(|c| match c {
                            Alignment::None => Align::START,
                            Alignment::Left => Align::LEFT,
                            Alignment::Center => Align::CENTER,
                            Alignment::Right => Align::RIGHT,
                        })
                        .collect()
                }
                Tag::TableHead => {
                    table_head = true;
                    table_col = 0;
                }
                Tag::TableRow => {
                    table_col = 0;
                }
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
                        blocks.push(paragraph_view.generate(ParagraphGenArgs {
                            index: blocks.len() as u32,
                            items: mem::take(&mut inlines).into(),
                        }));
                    }
                }
                Tag::Heading(level, _, _) => {
                    if !inlines.is_empty() {
                        blocks.push(heading_view.generate(HeadingGenArgs {
                            level,
                            anchor: heading_anchor(heading_text.take().unwrap_or_default().as_str()),
                            items: mem::take(&mut inlines).into(),
                        }));
                    }
                }
                Tag::BlockQuote => {
                    if let Some(start) = block_quote_start.pop() {
                        let items: UiNodeVec = blocks.drain(start..).collect();
                        if !items.is_empty() {
                            blocks.push(block_quote_view.generate(BlockQuoteGenArgs {
                                level: block_quote_start.len() as u32,
                                items,
                            }));
                        }
                    }
                }
                Tag::CodeBlock(kind) => {
                    if let Some(mut txt) = code_block_text.take() {
                        if txt.chars().rev().next() == Some('\n') {
                            txt.pop();
                        }
                        blocks.push(code_block_view.generate(CodeBlockGenArgs {
                            lang: match kind {
                                CodeBlockKind::Indented => Text::empty(),
                                CodeBlockKind::Fenced(l) => l.to_text(),
                            },
                            txt: txt.into(),
                        }))
                    }
                }
                Tag::List(n) => {
                    if let Some(_list) = list_info.pop() {
                        blocks.push(list_view.generate(ListGenArgs {
                            depth: list_info.len() as u32,
                            first_num: n,
                            items: mem::take(&mut list_items).into(),
                        }));
                    }
                }
                Tag::Item => {
                    let depth = list_info.len().saturating_sub(1);
                    if let Some(list) = list_info.last_mut() {
                        let num = match &mut list.item_num {
                            Some(n) => {
                                let r = *n;
                                *n += 1;
                                Some(r)
                            }
                            None => None,
                        };

                        let nested_list = if list.block_start < blocks.len() {
                            debug_assert_eq!(blocks.len() - list.block_start, 1);
                            blocks.pop()
                        } else {
                            None
                        };

                        let bullet_args = ListItemBulletGenArgs {
                            depth: depth as u32,
                            num,
                            checked: list.item_checked.take(),
                        };
                        list_items.push(list_item_bullet_view.generate(bullet_args));
                        list_items.push(list_item_view.generate(ListItemGenArgs {
                            bullet: bullet_args,
                            items: inlines.drain(list.inline_start..).collect(),
                            nested_list,
                        }));
                    }
                }
                Tag::FootnoteDefinition(label) => {
                    if let Some(i) = footnote_def_start.take() {
                        let label = html_escape::decode_html_entities(label.as_ref());
                        let items = blocks.drain(i..).collect();
                        blocks.push(footnote_def_view.generate(FootnoteDefGenArgs {
                            label: label.to_text(),
                            items,
                        }));
                    }
                }
                Tag::Table(_) => {
                    if !table_cells.is_empty() {
                        blocks.push(table_view.generate(TableGenArgs {
                            columns: mem::take(&mut table_cols),
                            cells: mem::take(&mut table_cells).into(),
                        }));
                    }
                }
                Tag::TableHead => {
                    table_head = false;
                }
                Tag::TableRow => {}
                Tag::TableCell => {
                    table_cells.push(table_cell_view.generate(TableCellGenArgs {
                        is_heading: table_head,
                        col_align: table_cols[table_col],
                        items: mem::take(&mut inlines).into(),
                    }));
                    table_col += 1;
                }
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
                            inlines.push(text_view.generate(TextGenArgs {
                                txt: url.to_text(),
                                style: MarkdownStyle {
                                    strong: strong > 0,
                                    emphasis: emphasis > 0,
                                    strikethrough: strikethrough > 0,
                                },
                            }));
                        }
                    }
                    if !inlines.is_empty() {
                        if let Some(s) = link_start.take() {
                            let items = inlines.drain(s..).collect();
                            inlines.push(link_view.generate(LinkGenArgs {
                                url,
                                title: title.to_text(),
                                items,
                            }));
                        }
                    }
                }
                Tag::Image(_, url, title) => {
                    let title = html_escape::decode_html_entities(title.as_ref());
                    blocks.push(image_view.generate(ImageGenArgs {
                        source: image_resolver.resolve(&url),
                        title: title.to_text(),
                        alt_items: mem::take(&mut inlines).into(),
                    }));
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
                            .generate(TextGenArgs {
                                txt: txt.to_text(),
                                style: MarkdownStyle {
                                    strong: strong > 0,
                                    emphasis: emphasis > 0,
                                    strikethrough: strikethrough > 0,
                                },
                            })
                            .boxed(),
                    );
                }
            }
            Event::Code(txt) => {
                let txt = html_escape::decode_html_entities(txt.as_ref());
                inlines.push(
                    code_inline_view
                        .generate(CodeInlineGenArgs {
                            txt: txt.to_text(),
                            style: MarkdownStyle {
                                strong: strong > 0,
                                emphasis: emphasis > 0,
                                strikethrough: strikethrough > 0,
                            },
                        })
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
                inlines.push(footnote_ref_view.generate(FootnoteRefGenArgs { label: label.to_text() }));
            }
            Event::SoftBreak => {}
            Event::HardBreak => {}
            Event::Rule => {
                blocks.push(rule_view.generate(RuleGenArgs {}));
            }
            Event::TaskListMarker(c) => {
                if let Some(l) = &mut list_info.last_mut() {
                    l.item_checked = Some(c);
                }
            }
        }
    }

    PANEL_GEN_VAR.get().generate(PanelGenArgs { items: blocks.into() })
}
