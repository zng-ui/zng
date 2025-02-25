#![cfg(feature = "markdown")]

//! Markdown widget, properties and other types.
//!
//! This widget displays [CommonMark] text, without support for HTML code yet .
//!
//! [CommonMark]: https://commonmark.org/
//!
//! ```
//! # let _scope = zng::APP.defaults(); let _ =
//! zng::markdown::Markdown! {
//!     txt = "# Title\n\n- List item 1.\n- List item 2.";
//! }
//! # ;
//! ```
//!
//! # Full API
//!
//! See [`zng_wgt_markdown`] for the full widget API.

pub use zng_wgt_markdown::{
    BlockQuoteFnArgs, CodeBlockFnArgs, CodeInlineFnArgs, FootnoteDefFnArgs, FootnoteRefFnArgs, HeadingFnArgs, HeadingLevel, ImageFnArgs,
    ImageResolver, LINK_EVENT, LinkArgs, LinkFnArgs, LinkResolver, ListFnArgs, ListItemBulletFnArgs, ListItemFnArgs, Markdown,
    MarkdownStyle, PanelFnArgs, ParagraphFnArgs, RuleFnArgs, TableCellFnArgs, TableFnArgs, TextFnArgs, WidgetInfoExt, anchor,
    block_quote_fn, code_block_fn, code_inline_fn, footnote_def_fn, footnote_ref_fn, heading_anchor, heading_fn, image_fn, image_resolver,
    link_fn, link_resolver, link_scroll_mode, list_fn, list_item_bullet_fn, list_item_fn, on_link, on_pre_link, panel_fn, paragraph_fn,
    rule_fn, table_fn, text_fn,
};
