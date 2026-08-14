use crate::{BACK_CMD, CANCEL_CMD, FINISH_CMD, NEXT_CMD, PageArgs, Wizard};
use zng_ext_font::FontWeight;
use zng_ext_input::focus::TabIndex;
use zng_wgt::{align, border, is_rtl, prelude::*};
use zng_wgt_button::Button;
use zng_wgt_container::{Container, padding};
use zng_wgt_fill::{background, background_color};
use zng_wgt_input::focus::tab_index;
use zng_wgt_markdown::Markdown;
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_size_offset::width;
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_style::style_fn;
use zng_wgt_text::Text;

context_var! {
    /// Widget function that builds a wizard header container.
    pub static HEADER_FN_VAR: WidgetFn<HeaderFnArgs> = WidgetFn::new(default_header_fn);

    /// Widget function that builds a background visual for the header container.
    pub static HEADER_BACKGROUND_FN_VAR: WidgetFn<()> = WidgetFn::nil();

    /// Widget function that builds a wizard side container.
    pub static SIDE_FN_VAR: WidgetFn<SideFnArgs> = WidgetFn::new(default_side_fn);

    /// Widget function that builds a background visual for the side container.
    pub static SIDE_BACKGROUND_FN_VAR: WidgetFn<()> = WidgetFn::nil();

    /// Widget function that builds a wizard main content container.
    pub static CONTENT_FN_VAR: WidgetFn<ContentFnArgs> = WidgetFn::new(default_content_fn);

    /// Widget function that builds a wizard footer container.
    pub static FOOTER_FN_VAR: WidgetFn<FooterFnArgs> = WidgetFn::new(default_footer_fn);

    /// Widget function that brings together all the wizard parts.
    pub static PANEL_FN_VAR: WidgetFn<PanelFnArgs> = WidgetFn::new(default_panel_fn);
}

/// Arguments for a wizard header container builder.
///
/// See [`HEADER_FN_VAR`] for more details.
#[non_exhaustive]
pub struct HeaderFnArgs {
    /// Page header instance.
    ///
    /// This is the [`Page::header`] instance.
    ///
    /// [`Page::header`]: crate::Page::header
    pub header: UiNode,

    /// Background visual.
    ///
    /// This is the [`HEADER_BACKGROUND_FN_VAR`] instance.
    pub background: UiNode,
}

/// Arguments for a wizard side container builder.
///
/// See [`SIDE_FN_VAR`] for more details.
#[non_exhaustive]
pub struct SideFnArgs {
    /// Page side instance.
    ///
    /// This is the [`Page::side`] instance.
    ///
    /// This is never [`UiNode::nil`] as that is handled by [`Wizard!`].
    ///
    /// This can be [`UiNode::is_list`], in this case a layout panel must be generated for
    /// the items, usually a vertical stack aligned to [`Align::BOTTOM`].
    ///
    /// [`Page::side`]: crate::Page::side
    /// [`Wizard!`]: struct@Wizard
    pub side: UiNode,

    /// Background visual.
    ///
    /// This is the [`SIDE_BACKGROUND_FN_VAR`] instance.
    pub background: UiNode,
}

/// Arguments for a wizard main content container builder.
///
/// See [`CONTENT_FN_VAR`] for more details.
#[non_exhaustive]
pub struct ContentFnArgs {
    /// Page content instance.
    ///
    /// This is the [`Page::content`] instance.
    ///
    /// [`Page::content`]: crate::Page::content
    pub content: UiNode,
}

/// Arguments for a wizard footer container builder.
///
/// See [`FOOTER_FN_VAR`] for more details.
#[non_exhaustive]
pub struct FooterFnArgs {
    /// Page footer instance.
    ///
    /// This can be [`UiNode::is_list`], in this case a layout panel must be generated for
    /// the items, usually an horizontal stack aligned to the [`Align::END`] side.
    pub footer: UiNode,
}

/// Arguments for the wizard root panel builder.
///
/// if any part [`is_nil`] its region on the panel must be fully collapsed.
///
/// See [`PANEL_FN_VAR`] for more details.
///
/// [`is_nil`]: UiNode::is_nil
pub struct PanelFnArgs {
    /// Header container.
    ///
    /// This is the [`header_fn`] instance.
    ///
    /// [`header_fn`]: fn@header_fn
    pub header: UiNode,
    /// Side container.
    ///
    /// This is the [`side_fn`] instance.
    ///
    /// [`side_fn`]: fn@side_fn
    pub side: UiNode,
    /// Main content container.
    ///
    /// This is the [`content_fn`] instance.
    ///
    /// [`content_fn`]: fn@content_fn
    pub content: UiNode,
    /// Footer container.
    ///
    /// This is the [`footer_fn`] instance.
    ///
    /// [`footer_fn`]: fn@footer_fn
    pub footer: UiNode,
}

/// Default wizard header container.
///
/// See [`HEADER_FN_VAR`] for more details.
pub fn default_header_fn(args: HeaderFnArgs) -> UiNode {
    Container! {
        child = args.header;
        padding = (10, 10, 10, 20);
        border = {
            widths: (0, 0, 1, 0),
            sides: colors::GRAY.with_alpha(40.pct()),
        };
        background = args.background;
    }
}

/// Default wizard page header content.
///
/// See [`Page::header`] for more details.
///
/// [`Page::header`]: crate::Page::header
pub fn default_page_header(args: PageArgs) -> UiNode {
    Stack! {
        children = ui_vec![default_page_header_title(args.title), default_page_header_info(args.info),];
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        align = Align::START;
    }
}
/// Default [`Page::title`] presenter.
///
/// [`Page::title`]: crate::Page::title
pub fn default_page_header_title(title: Var<Txt>) -> UiNode {
    Text! {
        txt = title;
        font_weight = FontWeight::BOLD;
    }
}
/// Default [`Page::info`] presenter.
///
/// [`Page::info`]: crate::Page::info
pub fn default_page_header_info(info: Var<Txt>) -> UiNode {
    Markdown! {
        txt = info;
    }
}

/// Default wizard side container.
///
/// See [`SIDE_FN_VAR`] for more details.
pub fn default_side_fn(args: SideFnArgs) -> UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        children = args.side;
        children_align = Align::BOTTOM_START;
        spacing = 5;
        padding = 5;
        width = 200;
        background = args.background;
        border = {
            widths: (0, 1, 0, 0),
            sides: colors::GRAY.with_alpha(40.pct()),
        };
        when #is_rtl {
            border = {
                widths: (0, 0, 0, 1),
                sides: colors::GRAY.with_alpha(40.pct()),
            };
        }

        zng_wgt_button::style_fn = style_fn!(|_| zng_wgt_button::LinkStyle!());
    }
}

/// Default wizard page side content.
///
/// See [`Page::side`] for more details.
///
/// [`Page::side`]: crate::Page::side
pub fn default_page_side(_: PageArgs) -> UiNode {
    ui_vec![].into_node()
}

/// Default wizard main content container.
///
/// See [`SIDE_FN_VAR`] for more details.
pub fn default_content_fn(args: ContentFnArgs) -> UiNode {
    Scroll! {
        mode = ScrollMode::VERTICAL;
        child = args.content;
        child_align = Align::FILL;
        padding = 20;
        background_color = light_dark(rgb(0.85, 0.85, 0.85), rgb(0.15, 0.15, 0.15));
    }
}

/// Default wizard footer container.
///
/// See [`FOOTER_FN_VAR`] for more details.
pub fn default_footer_fn(args: FooterFnArgs) -> UiNode {
    Stack! {
        children = args.footer.into_list();
        direction = StackDirection::start_to_end();
        spacing = 5;
        padding = 5;
        children_align = Align::END;
        border = {
            widths: (1, 0, 0, 0),
            sides: colors::GRAY.with_alpha(40.pct()),
        };
    }
}

/// Default wizard page footer content.
///
/// See [`Page::footer`] for more details.
///
/// [`Page::footer`]: crate::Page::footer
pub fn default_page_footer(args: PageArgs) -> UiNode {
    let id = args.wizard_id();
    if args.is_first() {
        ui_vec![default_page_footer_next(id), default_page_footer_cancel(id)]
    } else if args.is_last() {
        ui_vec![default_page_footer_back(id), default_page_footer_finish(id)]
    } else {
        ui_vec![
            default_page_footer_back(id),
            default_page_footer_next(id),
            default_page_footer_cancel(id)
        ]
    }
    .into_node()
}
/// Default [`BACK_CMD`] button.
pub fn default_page_footer_back(wizard_id: WidgetId) -> UiNode {
    Button! {
        cmd = BACK_CMD.scoped(wizard_id);
        tab_index = TabIndex::FIRST - 1;
    }
}
/// Default [`NEXT_CMD`] button.
pub fn default_page_footer_next(wizard_id: WidgetId) -> UiNode {
    Button! {
        cmd = NEXT_CMD.scoped(wizard_id);
        tab_index = TabIndex::FIRST;
    }
}
/// Default [`FINISH_CMD`] button.
pub fn default_page_footer_finish(wizard_id: WidgetId) -> UiNode {
    Button! {
        cmd = FINISH_CMD.scoped(wizard_id);
        tab_index = TabIndex::FIRST;
        style_fn = style_fn!(|_| zng_wgt_button::PrimaryStyle!());
    }
}
/// Default [`CANCEL_CMD`] button.
pub fn default_page_footer_cancel(wizard_id: WidgetId) -> UiNode {
    Button! {
        cmd = CANCEL_CMD.scoped(wizard_id);
        tab_index = TabIndex::FIRST - 2;
    }
}

/// Default wizard root panel.
///
/// See [`PANEL_FN_VAR`] for more details.
pub fn default_panel_fn(args: PanelFnArgs) -> UiNode {
    Container! {
        child_bottom = args.footer;
        child_start = args.side;
        child_top = args.header;
        child = args.content;
    }
}

/// Widget function that converts a [`HeaderFnArgs`] into a page header container widget.
///
/// This property sets the [`HEADER_FN_VAR`].
#[property(CONTEXT, default(HEADER_FN_VAR), widget_impl(Wizard))]
pub fn header_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<HeaderFnArgs>>) -> UiNode {
    with_context_var(child, HEADER_FN_VAR, wgt_fn)
}

/// Widget function that makes a background visual for the page header container widget.
///
/// This property sets the [`HEADER_BACKGROUND_FN_VAR`].
#[property(CONTEXT, default(HEADER_BACKGROUND_FN_VAR), widget_impl(Wizard))]
pub fn header_background_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> UiNode {
    with_context_var(child, HEADER_BACKGROUND_FN_VAR, wgt_fn)
}

/// Widget function that converts a [`SideFnArgs`] into a page side container widget.
///
/// This property sets the [`SIDE_FN_VAR`].
#[property(CONTEXT, default(SIDE_FN_VAR), widget_impl(Wizard))]
pub fn side_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<SideFnArgs>>) -> UiNode {
    with_context_var(child, SIDE_FN_VAR, wgt_fn)
}

/// Widget function that makes a background visual for the page side container widget.
///
/// This property sets the [`SIDE_BACKGROUND_FN_VAR`].
#[property(CONTEXT, default(SIDE_BACKGROUND_FN_VAR), widget_impl(Wizard))]
pub fn side_background_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<()>>) -> UiNode {
    with_context_var(child, SIDE_BACKGROUND_FN_VAR, wgt_fn)
}

/// Widget function that converts a [`ContentFnArgs`] into a page main content container container widget.
///
/// This property sets the [`CONTENT_FN_VAR`].
#[property(CONTEXT, default(CONTENT_FN_VAR), widget_impl(Wizard))]
pub fn content_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<ContentFnArgs>>) -> UiNode {
    with_context_var(child, CONTENT_FN_VAR, wgt_fn)
}

/// Widget function that converts a [`FooterFnArgs`] into a page side container widget.
///
/// This property sets the [`FOOTER_FN_VAR`].
#[property(CONTEXT, default(FOOTER_FN_VAR), widget_impl(Wizard))]
pub fn footer_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<FooterFnArgs>>) -> UiNode {
    with_context_var(child, FOOTER_FN_VAR, wgt_fn)
}

/// Widget function that converts a [`PanelFnArgs`] into a wizard.
///
/// This property sets the [`PANEL_FN_VAR`].
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(Wizard))]
pub fn panel_fn(child: impl IntoUiNode, wgt_fn: impl IntoVar<WidgetFn<PanelFnArgs>>) -> UiNode {
    with_context_var(child, PANEL_FN_VAR, wgt_fn)
}
