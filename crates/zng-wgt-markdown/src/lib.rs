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
            on_link = hn!(|args| {
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
            wgt.set_child(child);
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
    match_node(UiNode::nil(), move |c, op| match op {
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

            *c.node() = md.with(|md| markdown_view_fn(md.as_str()));
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
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
                c.node().deinit();
                *c.node() = md.with(|md| markdown_view_fn(md.as_str()));
                c.node().init();
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

/// Parse markdown, with pre-processing, merge texts, collapse white spaces across inline items
fn markdown_parser<'a>(md: &'a str, mut next_event: impl FnMut(pulldown_cmark::Event<'a>)) {
    use pulldown_cmark::*;

    let parse_options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_SMART_PUNCTUATION
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_SUBSCRIPT
        | Options::ENABLE_SUPERSCRIPT;

    let mut broken_link_handler = |b: BrokenLink<'a>| Some((b.reference, "".into()));
    let parser = Parser::new_with_broken_link_callback(md, parse_options, Some(&mut broken_link_handler));

    enum Str<'a> {
        Md(CowStr<'a>),
        Buf(String),
    }
    impl<'a> Str<'a> {
        fn buf(&mut self) -> &mut String {
            if let Str::Md(s) = self {
                *self = Str::Buf(mem::replace(s, CowStr::Borrowed("")).into_string());
            }
            match self {
                Str::Buf(b) => b,
                _ => unreachable!(),
            }
        }

        fn md(self) -> CowStr<'a> {
            match self {
                Str::Md(cow_str) => cow_str,
                Str::Buf(b) => b.into(),
            }
        }
    }
    let mut pending_txt: Option<Str<'a>> = None;
    let mut trim_start = false;

    for event in parser {
        // resolve breaks
        let event = match event {
            Event::SoftBreak => Event::Text(CowStr::Borrowed(" ")),
            Event::HardBreak => Event::Text(CowStr::Borrowed("\n")),
            ev => ev,
        };
        match event {
            // merge texts
            Event::Text(txt) => {
                if let Some(p) = &mut pending_txt {
                    p.buf().push_str(&txt);
                } else if mem::take(&mut trim_start) && txt.starts_with(' ') {
                    // merge spaces across inline items
                    pending_txt = Some(match txt {
                        CowStr::Borrowed(s) => Str::Md(CowStr::Borrowed(s.trim_start())),
                        CowStr::Boxed(s) => Str::Buf(s.trim_start().to_owned()),
                        CowStr::Inlined(s) => Str::Buf(s.trim_start().to_owned()),
                    });
                } else {
                    pending_txt = Some(Str::Md(txt));
                }
            }
            // items that don't merge spaces with siblings
            e @ Event::End(_)
            | e @ Event::Start(
                Tag::Paragraph
                | Tag::Heading { .. }
                | Tag::Image { .. }
                | Tag::Item
                | Tag::List(_)
                | Tag::CodeBlock(_)
                | Tag::Table(_)
                | Tag::TableHead
                | Tag::TableRow
                | Tag::TableCell
                | Tag::BlockQuote(_)
                | Tag::FootnoteDefinition(_)
                | Tag::DefinitionList
                | Tag::DefinitionListTitle
                | Tag::DefinitionListDefinition
                | Tag::HtmlBlock
                | Tag::MetadataBlock(_),
            )
            | e @ Event::Code(_)
            | e @ Event::Rule
            | e @ Event::TaskListMarker(_)
            | e @ Event::InlineMath(_)
            | e @ Event::DisplayMath(_)
            | e @ Event::Html(_)
            | e @ Event::InlineHtml(_) => {
                if let Some(txt) = pending_txt.take() {
                    next_event(Event::Text(txt.md()));
                }
                next_event(e)
            }
            // inline items that merge spaces with siblings
            Event::FootnoteReference(s) => {
                if let Some(txt) = pending_txt.take() {
                    let txt = txt.md();
                    trim_start = txt.ends_with(' ');
                    next_event(Event::Text(txt));
                }
                if mem::take(&mut trim_start) && s.starts_with(' ') {
                    let s = match s {
                        CowStr::Borrowed(s) => CowStr::Borrowed(s.trim_start()),
                        CowStr::Boxed(s) => CowStr::Boxed(s.trim_start().to_owned().into()),
                        CowStr::Inlined(s) => CowStr::Boxed(s.trim_start().to_owned().into()),
                    };
                    next_event(Event::FootnoteReference(s))
                } else {
                    next_event(Event::FootnoteReference(s))
                }
            }
            Event::Start(tag) => match tag {
                t @ Tag::Emphasis
                | t @ Tag::Strong
                | t @ Tag::Strikethrough
                | t @ Tag::Superscript
                | t @ Tag::Subscript
                | t @ Tag::Link { .. } => {
                    if let Some(txt) = pending_txt.take() {
                        let txt = txt.md();
                        trim_start = txt.ends_with(' ');
                        next_event(Event::Text(txt));
                    }
                    next_event(Event::Start(t))
                }
                t => tracing::error!("unexpected start tag {t:?}"),
            },
            // handled early
            Event::HardBreak | Event::SoftBreak => unreachable!(),
        }
        if let Some(txt) = pending_txt.take() {
            next_event(Event::Text(txt.md()));
        }
    }
}

fn markdown_view_fn(md: &str) -> UiNode {
    use pulldown_cmark::*;
    use resolvers::*;
    use view_fn::*;

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

    #[derive(Default)]
    struct StyleBuilder {
        strong: usize,
        emphasis: usize,
        strikethrough: usize,
        superscript: usize,
        subscript: usize,
    }
    impl StyleBuilder {
        fn build(&self) -> MarkdownStyle {
            MarkdownStyle {
                strong: self.strong > 0,
                emphasis: self.emphasis > 0,
                strikethrough: self.strikethrough > 0,
                subscript: self.subscript > self.superscript,
                superscript: self.superscript > self.subscript,
            }
        }
    }
    struct ListInfo {
        block_start: usize,
        inline_start: usize,
        first_num: Option<u64>,
        item_num: Option<u64>,
        item_checked: Option<bool>,
    }
    let mut blocks = vec![];
    let mut inlines = vec![];
    let mut txt_style = StyleBuilder::default();
    let mut link = None;
    let mut list_info = vec![];
    let mut list_items = vec![];
    let mut block_quote_start = vec![];
    let mut code_block = None;
    let mut html_block = None;
    let mut image = None;
    let mut heading_anchor_txt = None;
    let mut footnote_def = None;
    let mut table_cells = vec![];
    let mut table_cols = vec![];
    let mut table_col = 0;
    let mut table_head = false;

    markdown_parser(md, |event| match event {
        Event::Start(tag) => match tag {
            Tag::Paragraph => txt_style = StyleBuilder::default(),
            Tag::Heading { .. } => {
                txt_style = StyleBuilder::default();
                heading_anchor_txt = Some(String::new());
            }
            Tag::BlockQuote(_) => {
                txt_style = StyleBuilder::default();
                block_quote_start.push(blocks.len());
            }
            Tag::CodeBlock(kind) => {
                txt_style = StyleBuilder::default();
                code_block = Some((String::new(), kind));
            }
            Tag::HtmlBlock => {
                txt_style = StyleBuilder::default();
                html_block = Some(String::new());
            }
            Tag::List(n) => {
                txt_style = StyleBuilder::default();
                list_info.push(ListInfo {
                    block_start: blocks.len(),
                    inline_start: inlines.len(),
                    first_num: n,
                    item_num: n,
                    item_checked: None,
                });
            }
            Tag::DefinitionList => {
                txt_style = StyleBuilder::default();
                list_info.push(ListInfo {
                    block_start: blocks.len(),
                    inline_start: inlines.len(),
                    first_num: None,
                    item_num: None,
                    item_checked: None,
                });
            }
            Tag::Item | Tag::DefinitionListTitle | Tag::DefinitionListDefinition => {
                txt_style = StyleBuilder::default();
                if let Some(list) = list_info.last_mut() {
                    list.block_start = blocks.len();
                }
            }
            Tag::FootnoteDefinition(label) => {
                txt_style = StyleBuilder::default();
                footnote_def = Some((blocks.len(), label));
            }
            Tag::Table(columns) => {
                txt_style = StyleBuilder::default();
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
                txt_style = StyleBuilder::default();
                table_head = true;
                table_col = 0;
            }
            Tag::TableRow => {
                txt_style = StyleBuilder::default();
                table_col = 0;
            }
            Tag::TableCell => {
                txt_style = StyleBuilder::default();
            }
            Tag::Emphasis => {
                txt_style.emphasis += 1;
            }
            Tag::Strong => {
                txt_style.strong += 1;
            }
            Tag::Strikethrough => {
                txt_style.strong += 1;
            }
            Tag::Superscript => {
                txt_style.superscript += 1;
            }
            Tag::Subscript => {
                txt_style.subscript += 1;
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
                image = Some((String::new(), dest_url, title));
            }
            Tag::MetadataBlock(_) => unreachable!(), // not enabled
        },
        Event::End(tag_end) => match tag_end {
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
                        anchor: heading_anchor(heading_anchor_txt.take().unwrap_or_default().as_str()),
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
            TagEnd::HtmlBlock => {
                // TODO
                let _html = html_block.take().unwrap();
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
                txt_style.emphasis -= 1;
            }
            TagEnd::Strong => {
                txt_style.strong -= 1;
            }
            TagEnd::Strikethrough => {
                txt_style.strikethrough -= 1;
            }
            TagEnd::Superscript => {
                txt_style.superscript -= 1;
            }
            TagEnd::Subscript => txt_style.subscript -= 1,
            TagEnd::Link => {
                let (inlines_start, kind, url, title, _id) = link.take().unwrap();
                let title = html_escape::decode_html_entities(title.as_ref());
                let url = link_resolver.resolve(url.as_ref());
                match kind {
                    LinkType::Autolink | LinkType::Email => {
                        let url = html_escape::decode_html_entities(&url);
                        if let Some(txt) = text_view.call_checked(TextFnArgs {
                            txt: url.to_txt(),
                            style: txt_style.build(),
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
            TagEnd::MetadataBlock(_) => unreachable!(),
        },
        Event::Text(txt) => {
            if let Some(html) = &mut html_block {
                html.push_str(&txt);
            } else {
                let txt = html_escape::decode_html_entities(txt.as_ref());
                if let Some((code, _)) = &mut code_block {
                    code.push_str(&txt);
                } else if !txt.is_empty() {
                    if let Some(anchor_txt) = &mut heading_anchor_txt {
                        anchor_txt.push_str(&txt);
                    }
                    if let Some((alt_txt, _, _)) = &mut image {
                        alt_txt.push_str(&txt);
                    }
                    if let Some(txt) = text_view.call_checked(TextFnArgs {
                        txt: Txt::from_str(&txt),
                        style: txt_style.build(),
                    }) {
                        inlines.push(txt);
                    }
                }
            }
        }
        Event::Code(txt) => {
            let txt = html_escape::decode_html_entities(txt.as_ref());
            if let Some(txt) = code_inline_view.call_checked(CodeInlineFnArgs {
                txt: txt.to_txt(),
                style: txt_style.build(),
            }) {
                inlines.push(txt);
            }
        }
        Event::Html(h) => {
            if let Some(html) = &mut html_block {
                html.push_str(&h);
            }
        }
        Event::InlineHtml(tag) => match tag.as_ref() {
            "<b>" => txt_style.strong += 1,
            "</b>" => txt_style.strong -= 1,
            "<em>" => txt_style.emphasis += 1,
            "</em>" => txt_style.emphasis -= 1,
            "<s>" => txt_style.strikethrough += 1,
            "</s>" => txt_style.strikethrough -= 1,
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

        Event::InlineMath(_) => {} // TODO
        Event::DisplayMath(_) => {}
        Event::SoftBreak | Event::HardBreak => unreachable!(),
    });

    PANEL_FN_VAR.get()(PanelFnArgs { items: blocks.into() })
}
