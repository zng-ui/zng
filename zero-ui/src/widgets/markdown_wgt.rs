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
                || PARAGRAPH_VIEW_VAR.is_new(ctx)
                || HEADING_VIEW_VAR.is_new(ctx)
                || LIST_VIEW_VAR.is_new(ctx)
                || LIST_ITEM_VIEW_VAR.is_new(ctx)
                || PANEL_VIEW_VAR.is_new(ctx)
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
    let heading_view = HEADING_VIEW_VAR.get();
    let paragraph_view = PARAGRAPH_VIEW_VAR.get();
    let list_view = LIST_VIEW_VAR.get();
    let list_item_view = LIST_ITEM_VIEW_VAR.get();

    let mut blocks = vec![];
    let mut inlines = vec![];
    let mut list_item_num = None;
    let mut list_items = vec![];

    for item in Parser::new_ext(md, Options::all()) {
        match item {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading(_, _, _) => {}
                Tag::BlockQuote => {}
                Tag::CodeBlock(_) => {}
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
                Tag::BlockQuote => {}
                Tag::CodeBlock(_) => {}
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
                            items: mem::take(&mut inlines).into(),
                        },
                    ));
                }
                Tag::FootnoteDefinition(_) => {}
                Tag::Table(_) => {}
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
                Tag::Link(_, _, _) => {}
                Tag::Image(_, _, _) => {}
            },
            Event::Text(txt) => {
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
            Event::Code(_) => {}
            Event::Html(_) => {}
            Event::FootnoteReference(_) => {}
            Event::SoftBreak => {}
            Event::HardBreak => {}
            Event::Rule => {}
            Event::TaskListMarker(_) => {}
        }
    }

    PANEL_VIEW_VAR.get().generate(ctx, PanelViewArgs { items: blocks.into() })
}

mod markdown_view {
    pub use pulldown_cmark::HeadingLevel;

    use crate::widgets::text::PARAGRAPH_SPACING_VAR;

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
    /// The text can be inside a paragraph or heading.
    ///
    /// See [`TEXT_VIEW_VAR`] for more details.
    pub struct TextViewArgs {
        /// The text run.
        pub txt: Text,
        /// The style.
        pub style: MarkdownStyle,
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
        /// Inline items of the list item.
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

        /// View generator for a markdown paragraph.
        pub static PARAGRAPH_VIEW_VAR: ViewGenerator<ParagraphViewArgs> = ViewGenerator::new(|_, args| default_paragraph_view(args));

        /// View generator for a markdown heading.
        pub static HEADING_VIEW_VAR: ViewGenerator<HeadingViewArgs> = ViewGenerator::new(|_, args| default_heading_view(args));

        /// View generator for a markdown list.
        pub static LIST_VIEW_VAR: ViewGenerator<ListViewArgs> = ViewGenerator::new(|_, args| default_list_view(args));

        /// View generator for a markdown list item.
        pub static LIST_ITEM_VIEW_VAR: ViewGenerator<ListItemViewArgs> = ViewGenerator::new(|_, args| default_list_item_view(args));

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

    /// Default text view.
    ///
    /// See [`TEXT_VIEW_VAR`] for more details.
    pub fn default_text_view(args: TextViewArgs) -> impl UiNode {
        use crate::widgets::text as t;

        let mut builder = WidgetBuilder::new(widget_mod!(t));
        t::include(&mut builder);

        builder.push_property(
            Importance::INSTANCE,
            property_args! {
                t::txt = args.txt;
            },
        );

        if args.style.strong {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::font_weight = FontWeight::BOLD;
                },
            );
        }
        if args.style.emphasis {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::font_style = FontStyle::Italic;
                },
            );
        }
        if args.style.strikethrough {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::strikethrough = 1, LineStyle::Solid;
                },
            );
        }

        t::build(builder)
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
    /// See [`LIST_ITEM_VAR`] for more details.
    pub fn default_list_item_view(args: ListItemViewArgs) -> impl UiNode {
        let mut items = args.items;
        if let Some(n) = args.num {
            items.0.insert(
                0,
                crate::widgets::text! {
                    font_weight = FontWeight::BOLD;
                    txt = formatx!("{n}. ");
                    margin = (0, 0.3.em(), 0, 0);
                }
                .boxed(),
            );
        } else {
            items.0.insert(
                0,
                crate::widgets::text! {
                    txt = "•";
                    font_size = 16;
                    margin = (0, 0.3.em(), 0, 0);
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
}
