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
    pub use super::markdown_style::*;

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

mod markdown_view {}

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

            if self.md.is_new(ctx) {
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
    use markdown_style::*;
    use pulldown_cmark::*;

    let mut strong = 0;
    let mut emphasis = 0;
    let mut strikethrough = 0;

    let text_view = TEXT_VIEW_VAR.get();

    let mut paragraphs = vec![];
    let mut inline = vec![];

    for item in Parser::new_ext(md, Options::all()) {
        match item {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {}
                Tag::Heading(_, _, _) => {}
                Tag::BlockQuote => {}
                Tag::CodeBlock(_) => {}
                Tag::List(_) => {}
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
                    if !inline.is_empty() {
                        // !!: TODO PARAGRAPH_VIEW_VAR
                        paragraphs.push(
                            crate::widgets::layouts::wrap! {
                                children = mem::take(&mut inline)
                            }
                            .boxed(),
                        );
                    }
                }
                Tag::Heading(_, _, _) => {}
                Tag::BlockQuote => {}
                Tag::CodeBlock(_) => {}
                Tag::List(_) => {}
                Tag::Item => {}
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
                inline.push(
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

    // !!: TODO PANEL_VIEW_VAR
    crate::widgets::layouts::v_stack! {
        children = paragraphs;
    }
}

mod markdown_style {
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
    /// See [`TEXT_VIEW_VAR`] for more details.
    pub struct TextViewArgs {
        /// The text run.
        pub txt: Text,
        /// The style.
        pub style: MarkdownStyle,
    }

    context_var! {
        /// View generator for a markdown text segment.
        pub static TEXT_VIEW_VAR: ViewGenerator<TextViewArgs> = ViewGenerator::new(|_, args| default_text_view(args));
    }

    /// View generator that converts [`TextViewArgs`] to widgets.
    ///
    /// Sets the [`TEXT_VIEW_VAR`].
    #[property(CONTEXT, default(TEXT_VIEW_VAR))]
    pub fn text_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<TextViewArgs>>) -> impl UiNode {
        with_context_var(child, TEXT_VIEW_VAR, view)
    }

    /// Default text view.
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
}
