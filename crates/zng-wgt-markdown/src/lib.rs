#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Markdown widget, properties and nodes.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use std::mem;

pub use pulldown_cmark::HeadingLevel;

use zng_ext_font::WhiteSpace;
use zng_wgt::prelude::*;
use zng_wgt_input::{CursorIcon, cursor};

#[doc(hidden)]
pub use zng_wgt_text::__formatx;

use zng_wgt_text as text;

mod resolvers;
mod view_fn;

pub use resolvers::*;
pub use view_fn::*;

/// Render markdown styled text.
#[widget($crate::Markdown {
    ($txt:literal) => {
        txt = $crate::__formatx!($txt);
    };
    ($txt:expr) => {
        txt = $txt;
    };
    ($txt:tt, $($format:tt)*) => {
        txt = $crate::__formatx!($txt, $($format)*);
    };
})]
#[rustfmt::skip]
pub struct Markdown(
    text::FontMix<
    text::TextSpacingMix<
    text::ParagraphMix<
    text::LangMix<
    WidgetBase
    >>>>
);
impl Markdown {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            on_link = hn!(|args: &LinkArgs| {
                try_default_link_action(args);
            });
            zng_wgt_text::rich_text = true;

            when #txt_selectable {
                cursor = CursorIcon::Text;
            }
        };

        self.widget_builder().push_build_action(|wgt| {
            let md = wgt.capture_var_or_default(property_id!(text::txt));
            let child = markdown_node(md);
            wgt.set_child(child.boxed());
        });
    }

    widget_impl! {
        /// Markdown text.
        pub text::txt(txt: impl IntoVar<Txt>);

        /// Enable text selection, copy.
        ///
        /// Note that the copy is only in plain text, without any style.
        pub zng_wgt_text::txt_selectable(enabled: impl IntoVar<bool>);
    }
}

/// Implements the markdown parsing and view generation, configured by contextual properties.
pub fn markdown_node(md: impl IntoVar<Txt>) -> UiNode {
    let md = md.into_var();
    match_node(NilUiNode.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&md)
                .sub_var(&TEXT_FN_VAR)
                .sub_var(&LINK_FN_VAR)
                .sub_var(&CODE_INLINE_FN_VAR)
                .sub_var(&CODE_BLOCK_FN_VAR)
                .sub_var(&PARAGRAPH_FN_VAR)
                .sub_var(&HEADING_FN_VAR)
                .sub_var(&LIST_FN_VAR)
                .sub_var(&LIST_ITEM_BULLET_FN_VAR)
                .sub_var(&LIST_ITEM_FN_VAR)
                .sub_var(&IMAGE_FN_VAR)
                .sub_var(&RULE_FN_VAR)
                .sub_var(&BLOCK_QUOTE_FN_VAR)
                .sub_var(&TABLE_FN_VAR)
                .sub_var(&TABLE_CELL_FN_VAR)
                .sub_var(&PANEL_FN_VAR)
                .sub_var(&IMAGE_RESOLVER_VAR)
                .sub_var(&LINK_RESOLVER_VAR);

            *c.child() = md.with(|md| markdown_view_fn(md.as_str())).boxed();
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = NilUiNode.boxed();
        }
        UiNodeOp::Info { info } => {
            info.flag_meta(*MARKDOWN_INFO_ID);
        }
        UiNodeOp::Update { .. } => {
            use resolvers::*;
            use view_fn::*;

            if md.is_new()
                || TEXT_FN_VAR.is_new()
                || LINK_FN_VAR.is_new()
                || CODE_INLINE_FN_VAR.is_new()
                || CODE_BLOCK_FN_VAR.is_new()
                || PARAGRAPH_FN_VAR.is_new()
                || HEADING_FN_VAR.is_new()
                || LIST_FN_VAR.is_new()
                || LIST_ITEM_BULLET_FN_VAR.is_new()
                || LIST_ITEM_FN_VAR.is_new()
                || IMAGE_FN_VAR.is_new()
                || RULE_FN_VAR.is_new()
                || BLOCK_QUOTE_FN_VAR.is_new()
                || TABLE_FN_VAR.is_new()
                || TABLE_CELL_FN_VAR.is_new()
                || PANEL_FN_VAR.is_new()
                || IMAGE_RESOLVER_VAR.is_new()
                || LINK_RESOLVER_VAR.is_new()
            {
                c.delegated();
                c.child().deinit();
                *c.child() = md.with(|md| markdown_view_fn(md.as_str())).boxed();
                c.child().init();
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

fn markdown_view_fn<'a>(md: &'a str) -> impl UiNode + use<> {
    use pulldown_cmark::*;
    use resolvers::*;
    use view_fn::*;

    let parse_options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_DEFINITION_LIST;

    let mut strong = 0;
    let mut emphasis = 0;
    let mut strikethrough = 0;

    let text_view = TEXT_FN_VAR.get();
    let link_view = LINK_FN_VAR.get();
    let code_inline_view = CODE_INLINE_FN_VAR.get();
    let code_block_view = CODE_BLOCK_FN_VAR.get();
    let heading_view = HEADING_FN_VAR.get();
    let paragraph_view = PARAGRAPH_FN_VAR.get();
    let list_view = LIST_FN_VAR.get();
    let definition_list_view = DEF_LIST_FN_VAR.get();
    let list_item_bullet_view = LIST_ITEM_BULLET_FN_VAR.get();
    let list_item_view = LIST_ITEM_FN_VAR.get();
    let image_view = IMAGE_FN_VAR.get();
    let rule_view = RULE_FN_VAR.get();
    let block_quote_view = BLOCK_QUOTE_FN_VAR.get();
    let footnote_ref_view = FOOTNOTE_REF_FN_VAR.get();
    let footnote_def_view = FOOTNOTE_DEF_FN_VAR.get();
    let def_list_item_title_view = DEF_LIST_ITEM_TITLE_FN_VAR.get();
    let def_list_item_definition_view = DEF_LIST_ITEM_DEFINITION_FN_VAR.get();
    let table_view = TABLE_FN_VAR.get();
    let table_cell_view = TABLE_CELL_FN_VAR.get();

    let image_resolver = IMAGE_RESOLVER_VAR.get();
    let link_resolver = LINK_RESOLVER_VAR.get();

    struct ListInfo {
        block_start: usize,
        inline_start: usize,
        first_num: Option<u64>,
        item_num: Option<u64>,
        item_checked: Option<bool>,
    }
    let mut blocks = vec![];
    let mut inlines = vec![];

    let mut link = None;
    let mut list_info = vec![];
    let mut list_items = vec![];
    let mut block_quote_start = vec![];
    let mut code_block = None;
    let mut image = None;
    let mut heading_text = None;
    let mut footnote_def = None;
    let mut table_cells = vec![];
    let mut table_cols = vec![];
    let mut table_col = 0;
    let mut table_head = false;

    let mut last_txt_end = '\0';

    for item in Parser::new_with_broken_link_callback(md, parse_options, Some(&mut |b: BrokenLink<'a>| Some((b.reference, "".into())))) {
        let item = match item {
            Event::SoftBreak => Event::Text(pulldown_cmark::CowStr::Borrowed(" ")),
            Event::HardBreak => Event::Text(pulldown_cmark::CowStr::Borrowed("\n")),
            item => item,
        };
        match item {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    // close unbalanced HTML tags
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                }
                Tag::Heading { .. } => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    heading_text = Some(String::new());
                }
                Tag::BlockQuote(_) => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    block_quote_start.push(blocks.len());
                }
                Tag::CodeBlock(kind) => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    code_block = Some((String::new(), kind));
                }
                Tag::List(n) => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    list_info.push(ListInfo {
                        block_start: blocks.len(),
                        inline_start: inlines.len(),
                        first_num: n,
                        item_num: n,
                        item_checked: None,
                    });
                }
                Tag::DefinitionList => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    list_info.push(ListInfo {
                        block_start: blocks.len(),
                        inline_start: inlines.len(),
                        first_num: None,
                        item_num: None,
                        item_checked: None,
                    });
                }
                Tag::Item | Tag::DefinitionListTitle | Tag::DefinitionListDefinition => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    if let Some(list) = list_info.last_mut() {
                        list.block_start = blocks.len();
                    }
                }
                Tag::FootnoteDefinition(label) => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    footnote_def = Some((blocks.len(), label));
                }
                Tag::Table(columns) => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
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
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    table_head = true;
                    table_col = 0;
                }
                Tag::TableRow => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                    table_col = 0;
                }
                Tag::TableCell => {
                    (strong, emphasis, strikethrough) = (0, 0, 0);
                    last_txt_end = '\0';
                }
                Tag::Emphasis => {
                    emphasis += 1;
                }
                Tag::Strong => {
                    strong += 1;
                }
                Tag::Strikethrough => {
                    strong += 1;
                }
                Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                } => {
                    link = Some((inlines.len(), link_type, dest_url, title, id));
                }
                Tag::Image { dest_url, title, .. } => {
                    last_txt_end = '\0';
                    image = Some((String::new(), dest_url, title));
                }
                Tag::Superscript => {}
                Tag::Subscript => {}
                Tag::HtmlBlock => {}
                Tag::MetadataBlock(_) => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => {
                    if !inlines.is_empty() {
                        blocks.push(paragraph_view(ParagraphFnArgs {
                            index: blocks.len() as u32,
                            items: mem::take(&mut inlines).into(),
                        }));
                    }
                }
                TagEnd::Heading(level) => {
                    if !inlines.is_empty() {
                        blocks.push(heading_view(HeadingFnArgs {
                            level,
                            anchor: heading_anchor(heading_text.take().unwrap_or_default().as_str()),
                            items: mem::take(&mut inlines).into(),
                        }));
                    }
                }
                TagEnd::BlockQuote(_) => {
                    if let Some(start) = block_quote_start.pop() {
                        let items: UiVec = blocks.drain(start..).collect();
                        if !items.is_empty() {
                            blocks.push(block_quote_view(BlockQuoteFnArgs {
                                level: block_quote_start.len() as u32,
                                items,
                            }));
                        }
                    }
                }
                TagEnd::CodeBlock => {
                    let (mut txt, kind) = code_block.take().unwrap();
                    if txt.ends_with('\n') {
                        txt.pop();
                    }
                    blocks.push(code_block_view(CodeBlockFnArgs {
                        lang: match kind {
                            CodeBlockKind::Indented => Txt::from_str(""),
                            CodeBlockKind::Fenced(l) => l.to_txt(),
                        },
                        txt: txt.into(),
                    }))
                }
                TagEnd::List(_) => {
                    if let Some(list) = list_info.pop() {
                        blocks.push(list_view(ListFnArgs {
                            depth: list_info.len() as u32,
                            first_num: list.first_num,
                            items: mem::take(&mut list_items).into(),
                        }));
                    }
                }
                TagEnd::DefinitionList => {
                    if list_info.pop().is_some() {
                        blocks.push(definition_list_view(DefListArgs {
                            items: mem::take(&mut list_items).into(),
                        }));
                    }
                }
                TagEnd::Item => {
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

                        let bullet_args = ListItemBulletFnArgs {
                            depth: depth as u32,
                            num,
                            checked: list.item_checked.take(),
                        };
                        list_items.push(list_item_bullet_view(bullet_args));
                        list_items.push(list_item_view(ListItemFnArgs {
                            bullet: bullet_args,
                            items: inlines.drain(list.inline_start..).collect(),
                            blocks: blocks.drain(list.block_start..).collect(),
                        }));
                    }
                }
                TagEnd::DefinitionListTitle => {
                    if let Some(list) = list_info.last_mut() {
                        list_items.push(def_list_item_title_view(DefListItemTitleArgs {
                            items: inlines.drain(list.inline_start..).collect(),
                        }));
                    }
                }
                TagEnd::DefinitionListDefinition => {
                    if let Some(list) = list_info.last_mut() {
                        list_items.push(def_list_item_definition_view(DefListItemDefinitionArgs {
                            items: inlines.drain(list.inline_start..).collect(),
                        }));
                    }
                }
                TagEnd::FootnoteDefinition => {
                    if let Some((i, label)) = footnote_def.take() {
                        let label = html_escape::decode_html_entities(label.as_ref());
                        let items = blocks.drain(i..).collect();
                        blocks.push(footnote_def_view(FootnoteDefFnArgs {
                            label: label.to_txt(),
                            items,
                        }));
                    }
                }
                TagEnd::Table => {
                    if !table_cells.is_empty() {
                        blocks.push(table_view(TableFnArgs {
                            columns: mem::take(&mut table_cols),
                            cells: mem::take(&mut table_cells).into(),
                        }));
                    }
                }
                TagEnd::TableHead => {
                    table_head = false;
                }
                TagEnd::TableRow => {}
                TagEnd::TableCell => {
                    table_cells.push(table_cell_view(TableCellFnArgs {
                        is_heading: table_head,
                        col_align: table_cols[table_col],
                        items: mem::take(&mut inlines).into(),
                    }));
                    table_col += 1;
                }
                TagEnd::Emphasis => {
                    emphasis -= 1;
                }
                TagEnd::Strong => {
                    strong -= 1;
                }
                TagEnd::Strikethrough => {
                    strikethrough -= 1;
                }
                TagEnd::Link => {
                    let (inlines_start, kind, url, title, _id) = link.take().unwrap();
                    let title = html_escape::decode_html_entities(title.as_ref());
                    let url = link_resolver.resolve(url.as_ref());
                    match kind {
                        LinkType::Autolink | LinkType::Email => {
                            let url = html_escape::decode_html_entities(&url);
                            if let Some(txt) = text_view.call_checked(TextFnArgs {
                                txt: url.to_txt(),
                                style: MarkdownStyle {
                                    strong: strong > 0,
                                    emphasis: emphasis > 0,
                                    strikethrough: strikethrough > 0,
                                },
                            }) {
                                inlines.push(txt);
                            }
                        }
                        LinkType::Inline => {}
                        LinkType::Reference => {}
                        LinkType::ReferenceUnknown => {}
                        LinkType::Collapsed => {}
                        LinkType::CollapsedUnknown => {}
                        LinkType::Shortcut => {}
                        LinkType::ShortcutUnknown => {}
                        LinkType::WikiLink { .. } => {}
                    }
                    if !inlines.is_empty() {
                        let items = inlines.drain(inlines_start..).collect();
                        if let Some(lnk) = link_view.call_checked(LinkFnArgs {
                            url,
                            title: title.to_txt(),
                            items,
                        }) {
                            inlines.push(lnk);
                        }
                    }
                }
                TagEnd::Image => {
                    let (alt_txt, url, title) = image.take().unwrap();
                    let title = html_escape::decode_html_entities(title.as_ref());
                    blocks.push(image_view(ImageFnArgs {
                        source: image_resolver.resolve(&url),
                        title: title.to_txt(),
                        alt_items: mem::take(&mut inlines).into(),
                        alt_txt: alt_txt.into(),
                    }));
                }
                TagEnd::Superscript => {}
                TagEnd::Subscript => {}
                TagEnd::HtmlBlock => {}
                TagEnd::MetadataBlock(_) => {}
            },
            Event::Text(txt) => {
                let txt = html_escape::decode_html_entities(txt.as_ref());
                if let Some((t, _)) = &mut code_block {
                    t.push_str(&txt);
                } else if !txt.is_empty() {
                    let mut txt = Txt::from_string(txt.into_owned());

                    // apply `WhiteSpace::MergeAll` across texts.
                    let txt_end = txt.chars().next_back().unwrap();

                    if txt != " " && txt != "\n" {
                        // not Soft/HardBreak
                        let starts_with_space = txt.chars().next().unwrap().is_whitespace();
                        match WhiteSpace::MergeAll.transform(&txt) {
                            std::borrow::Cow::Borrowed(_) => {
                                if starts_with_space && last_txt_end != '\0' || !txt.is_empty() && last_txt_end.is_whitespace() {
                                    txt.to_mut().insert(0, ' ');
                                }
                                txt.end_mut();
                                last_txt_end = txt_end;
                            }
                            std::borrow::Cow::Owned(t) => {
                                txt = t;
                                if !txt.is_empty() {
                                    if starts_with_space && last_txt_end != '\0' || !txt.is_empty() && last_txt_end.is_whitespace() {
                                        txt.to_mut().insert(0, ' ');
                                        txt.end_mut();
                                    }
                                    last_txt_end = txt_end;
                                }
                            }
                        }
                    }

                    if let Some(t) = &mut heading_text {
                        t.push_str(&txt);
                    }
                    if let Some((t, _, _)) = &mut image {
                        t.push_str(&txt);
                    }
                    if let Some(txt) = text_view.call_checked(TextFnArgs {
                        txt,
                        style: MarkdownStyle {
                            strong: strong > 0,
                            emphasis: emphasis > 0,
                            strikethrough: strikethrough > 0,
                        },
                    }) {
                        inlines.push(txt);
                    }
                }
            }
            Event::Code(txt) => {
                let txt = html_escape::decode_html_entities(txt.as_ref());

                let style = MarkdownStyle {
                    strong: strong > 0,
                    emphasis: emphasis > 0,
                    strikethrough: strikethrough > 0,
                };

                if last_txt_end.is_whitespace() {
                    if let Some(txt) = text_view.call_checked(TextFnArgs {
                        txt: ' '.into(),
                        style: style.clone(),
                    }) {
                        inlines.push(txt);
                    }
                }

                if let Some(txt) = code_inline_view.call_checked(CodeInlineFnArgs { txt: txt.to_txt(), style }) {
                    inlines.push(txt);
                }
            }
            Event::Html(tag) | Event::InlineHtml(tag) => match tag.as_ref() {
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
                if let Some(txt) = footnote_ref_view.call_checked(FootnoteRefFnArgs { label: label.to_txt() }) {
                    inlines.push(txt);
                }
            }
            Event::Rule => {
                blocks.push(rule_view(RuleFnArgs {}));
            }
            Event::TaskListMarker(c) => {
                if let Some(l) = &mut list_info.last_mut() {
                    l.item_checked = Some(c);
                }
            }
            Event::InlineMath(_) => {}
            Event::DisplayMath(_) => {}
            // handled early
            Event::SoftBreak | Event::HardBreak => unreachable!(),
        }
    }

    PANEL_FN_VAR.get()(PanelFnArgs { items: blocks.into() })
}
