#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Wizard widget.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_app::event::CommandArgs;
use zng_ext_input::focus::FOCUS;
use zng_var::MergeVarBuilder;
use zng_view_api::keyboard::Key;
use zng_wgt::prelude::*;
use zng_wgt_input::{gesture, keyboard};
use zng_wgt_style::style_fn;
use zng_wgt_text_input::label;

mod view_fn;

pub use view_fn::*;

/// Paginated widget that
#[widget($crate::Wizard)]
pub struct Wizard(WidgetBase);
impl Wizard {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let pages = wgt.capture_var_or_default(property_id!(pages));
            wgt.set_child(node(pages));
            wgt.push_intrinsic(NestGroup::CONTEXT, "state", |c| {
                with_context_var(c, GET_TITLE_VAR, var(Txt::from_static("")))
            });
        });

        widget_set! {
            self;

            // use mnemonic shortcuts
            gesture::mnemonic_scope = true;
            label::style_fn = style_fn!(|_| {
                label::DefaultStyle! {
                    label::mnemonic_underline = true;
                    zng_wgt_text::underline = 1, LineStyle::Solid;
                }
            });
            keyboard::on_key_up = hn!(|args| {
                if let Key::Char(c) = args.key
                    && c.is_alphanumeric()
                    && !FOCUS.is_highlighting().get()
                {
                    // on unhandled alphanumeric press highlight focus to enable mnemonic keys
                    FOCUS.highlight();
                    args.propagation.stop();
                }
            });
        }
    }
}

context_var! {
    static GET_TITLE_VAR: Txt = Txt::from_static("");
}

/// Defines the wizard pages.
///
/// Pages are built on demand, the [`Page`] value defines [`wgt_fn!`] builders
/// that are used by wizard when the page needs to be instantiated.
#[property(CHILD, widget_impl(Wizard))]
pub fn pages(wgt: &mut WidgetBuilding, pages: impl IntoVar<Vec<Page>>) {
    let _ = pages;
    wgt.expect_property_capture();
}

/// Get the current page title.
#[property(CONTEXT, widget_impl(Wizard))]
pub fn get_title(child: impl IntoUiNode, title: impl IntoVar<Txt>) -> UiNode {
    bind_state(child, GET_TITLE_VAR, title)
}

/// Represents a page builder for [`Wizard!`].
///
/// The widgets defined here must represent only the content that is unique for each page,
/// the wizard widget has properties that define the wizard parts, for example, if the side
/// has an image that is the same for all pages it is defined in [`Wizard::side_fn`].
///
/// The builders care called in the parent [`Wizard!`] widget context.
///
/// [`Wizard!`]: struct@Wizard
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    /// Page title.
    ///
    /// The default `header` presents this for the selected page.
    pub title: VarEq<Txt>,

    /// Page info.
    ///
    /// This is a short explanation about the page. Supports basic markdown span formatting.
    ///
    /// The default `header` presents this for the selected page.
    pub info: VarEq<Txt>,

    /// Page header content.
    ///
    /// Presents the `title`, `info` and any other custom header detail
    /// when the page is selected.
    ///
    /// The header of each page is wrapped by [`Wizard::header_fn`] to form the full header.
    ///
    /// If this builds [`UiNode::nil`] the header panel **is collapsed** for this page.
    ///
    /// Is [`default_page_header`] by default.
    pub header: WidgetFn<PageArgs>,
    /// Page side panel content.
    ///
    /// The side content of each page is wrapped by [`Wizard::side_fn`] to form the full side panel.
    ///
    /// If this is a list node the wizard side builder will generate a layout panel for the items.
    ///
    /// If this builds [`UiNode::nil`] the side panel **is collapsed** for this page. An empty list node
    /// signals the side builder that it should be visible without page content.
    ///
    /// Is [`default_page_side`] by default.
    pub side: WidgetFn<PageArgs>,
    /// Page main content.
    ///
    /// The main content of each page is wrapped by [`Wizard::content_fn`] to form the full content panel.
    pub content: WidgetFn<PageArgs>,
    /// Page footer content.
    ///
    /// The footer of each page is wrapped by [`Wizard::footer_fn`] to form the full footer panel.
    ///
    /// If this is a list node the wizard footer builder will generate a layout panel for the items.
    ///
    /// Is [`default_page_footer`] by default.
    pub footer: WidgetFn<PageArgs>,

    /// When `true` the page is skipped over.
    ///
    /// Is `false` by default.
    pub skip: VarEq<bool>,

    /// When is not the first page controls the wizard `BACK_CMD` handle.
    ///
    /// Is `true` by default.
    pub can_back: VarEq<bool>,

    /// When is not the last page controls the wizard `NEXT_CMD` handle.
    pub can_next: VarEq<bool>,
}

impl Page {
    /// New basic page.
    pub fn new(title: impl IntoVar<Txt>, info: impl IntoVar<Txt>, content: WidgetFn<PageArgs>) -> Self {
        Self {
            title: VarEq(title.into_var()),
            info: VarEq(info.into_var()),
            header: WidgetFn::new(default_page_header),
            side: WidgetFn::new(default_page_side),
            content,
            footer: WidgetFn::new(default_page_footer),
            skip: VarEq(var(false)),
            can_back: VarEq(var(true)),
            can_next: VarEq(var(true)),
        }
    }
}
/// Arguments for [`Page`] builders.
#[non_exhaustive]
#[derive(Clone)]
pub struct PageArgs {
    /// Page index on the pages list.
    pub index: usize,
    /// Count of pages on the list.
    pub pages_len: usize,
    /// The [`Page::title`] var.
    pub title: Var<Txt>,
    /// The [`Page::info`] var.
    pub info: Var<Txt>,
    /// The [`Page::can_back`] var.
    pub can_back: Var<bool>,
    /// The [`Page::can_next`] var.
    pub can_next: Var<bool>,
}
impl PageArgs {
    /// Is first page on the list.
    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    /// Is last page on the list.
    pub fn is_last(&self) -> bool {
        self.index == self.pages_len.saturating_sub(1)
    }

    /// Get `WIDGET.id()`.
    pub fn wizard_id(&self) -> WidgetId {
        WIDGET.id()
    }
}

command! {
    /// Return to previous page.
    pub static BACK_CMD {
        l10n!: true,
        name: "Back",
    };

    /// Advance to next page.
    pub static NEXT_CMD {
        l10n!: true,
        name: "Next",
    };

    /// Cancel wizard operation.
    pub static CANCEL_CMD {
        l10n!: true,
        name: "Cancel",
    };

    /// Finish wizard operation.
    ///
    /// This command also represents the transition from a pages set to another, for example,
    /// a setup wizard starts with only the config pages, the [`finish_cmd_name`]
    /// is set to "Install", on finish the pages are swapped to the progress and results page.
    ///
    /// [`finish_cmd_name`]: fn@finish_cmd_name
    pub static FINISH_CMD {
        l10n!: true,
        name: "Finish",
    };
}
command_property! {
    /// Wizard cancel requested.
    #[property(EVENT, widget_impl(Wizard))]
    pub fn on_cancel<on_pre_cancel, can_cancel>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        CANCEL_CMD
    }

    /// Wizard finish requested.
    #[property(EVENT, widget_impl(Wizard))]
    pub fn on_finish<on_pre_finish, can_finish>(child: impl IntoUiNode, handler: Handler<CommandArgs>) -> UiNode {
        FINISH_CMD
    }
}

/// Set the name for the [`FINISH_CMD`] scoped on this widget.
#[property(CONTEXT, widget_impl(Wizard))]
pub fn finish_cmd_name(child: impl IntoUiNode, name: impl IntoVar<Txt>) -> UiNode {
    let name = name.into_var();
    match_node(child, move |_, op| {
        if let UiNodeOp::Init = op {
            let finish_name = FINISH_CMD.scoped(WIDGET.id()).name();
            let h = name.set_bind(&finish_name);
            WIDGET.push_var_handle(h);
        }
    })
}

fn node(pages: Var<Vec<Page>>) -> UiNode {
    let mut cmds = [CommandHandle::dummy(), CommandHandle::dummy()];
    let mut selected_page = 0usize;
    let mut get_title = VarHandle::dummy();
    match_node(UiNode::nil(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&pages)
                .sub_var(&PANEL_FN_VAR)
                .sub_var(&HEADER_FN_VAR)
                .sub_var(&HEADER_BACKGROUND_FN_VAR)
                .sub_var(&SIDE_FN_VAR)
                .sub_var(&SIDE_BACKGROUND_FN_VAR)
                .sub_var(&SIDE_EXTRA_FN_VAR)
                .sub_var(&CONTENT_FN_VAR)
                .sub_var(&FOOTER_FN_VAR)
                .sub_var(&FOOTER_EXTRA_FN_VAR);
            pages.with(|p| {
                if !p.is_empty() {
                    cmds = subscribe(0, p);
                    *c.node() = build(0, p);
                    get_title = p[0].title.set_bind(&GET_TITLE_VAR);
                }
            });
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
            cmds = [CommandHandle::dummy(), CommandHandle::dummy()];
            selected_page = 0;
            get_title = VarHandle::dummy();
        }
        UiNodeOp::Update { updates } => {
            c.update(updates);

            let mut rebuild = false;
            let id = WIDGET.id();
            BACK_CMD.scoped(id).each_update(true, false, |args| {
                while selected_page > 0 {
                    selected_page -= 1;
                    if !pages.with(|p| p[selected_page].skip.get()) {
                        rebuild = true;
                        break;
                    }
                }
                args.propagation.stop();
            });
            NEXT_CMD.scoped(id).each_update(true, false, |args| {
                let last = pages.with(|p| p.len()).saturating_sub(1);
                while selected_page < last {
                    selected_page += 1;
                    if !pages.with(|p| p[selected_page].skip.get()) {
                        rebuild = true;
                        break;
                    }
                }
                args.propagation.stop();
            });

            if pages.is_new() {
                selected_page = 0;
                rebuild = true;
            } else if PANEL_FN_VAR.is_new()
                || HEADER_FN_VAR.is_new()
                || HEADER_BACKGROUND_FN_VAR.is_new()
                || SIDE_FN_VAR.is_new()
                || SIDE_BACKGROUND_FN_VAR.is_new()
                || SIDE_EXTRA_FN_VAR.is_new()
                || CONTENT_FN_VAR.is_new()
                || FOOTER_FN_VAR.is_new()
                || FOOTER_EXTRA_FN_VAR.is_new()
            {
                rebuild = true;
            }

            if rebuild {
                c.deinit();
                pages.with(|p| {
                    if !p.is_empty() {
                        cmds = subscribe(selected_page, p);
                        *c.node() = build(selected_page, p);
                        get_title = p[selected_page].title.set_bind(&GET_TITLE_VAR);
                        c.init();
                    } else {
                        cmds = [CommandHandle::dummy(), CommandHandle::dummy()];
                        *c.node() = UiNode::nil();
                        get_title = VarHandle::dummy();
                    }
                });
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

fn subscribe(index: usize, pages: &[Page]) -> [CommandHandle; 2] {
    let id = WIDGET.id();
    let cmds = [BACK_CMD.scoped(id).subscribe(false), NEXT_CMD.scoped(id).subscribe(false)];

    let mut skips = MergeVarBuilder::new();
    for p in pages {
        skips.push(p.skip.0.clone());
    }
    let can_dos = skips.build(move |skips| {
        let mut can_back = false;
        for i in 0..index {
            can_back = !skips.get(i);
            if can_back {
                break;
            }
        }
        let mut can_next = false;
        for i in index + 1..skips.len() {
            can_next = !skips.get(i);
            if can_next {
                break;
            }
        }
        [can_back, can_next]
    });

    can_dos.set_bind_map(cmds[0].enabled(), |[b, _]| *b).perm();
    can_dos.set_bind_map(cmds[1].enabled(), |[_, n]| *n).perm();
    cmds[0].enabled().hold(can_dos).perm();

    cmds
}
fn build(index: usize, pages: &[Page]) -> UiNode {
    let page = &pages[index];
    let args = PageArgs {
        index,
        pages_len: pages.len(),
        title: page.title.0.clone(),
        info: page.info.0.clone(),
        can_back: page.can_back.0.clone(),
        can_next: page.can_next.0.clone(),
    };
    let header = (page.header)(args.clone());
    let side = (page.side)(args.clone());
    let content = (page.content)(args.clone());
    let footer = (page.footer)(args.clone());

    let header = if header.is_nil() {
        header
    } else {
        let background = HEADER_BACKGROUND_FN_VAR.get()(());
        HEADER_FN_VAR.get()(HeaderFnArgs {
            header,
            background,
            index,
            pages_len: pages.len(),
            titles: pages.iter().map(|p| p.title.0.clone()).collect(),
            skips: pages.iter().map(|p| p.skip.0.clone()).collect(),
        })
    };
    let side = if side.is_nil() {
        side
    } else {
        let background = SIDE_BACKGROUND_FN_VAR.get()(());
        let side_extra = SIDE_EXTRA_FN_VAR.get()(args.clone());
        SIDE_FN_VAR.get()(SideFnArgs {
            side,
            background,
            side_extra,
            index,
            pages_len: pages.len(),
        })
    };
    let content = CONTENT_FN_VAR.get()(ContentFnArgs {
        content,
        index,
        pages_len: pages.len(),
    });
    let footer_extra = FOOTER_EXTRA_FN_VAR.get()(args);
    let footer = FOOTER_FN_VAR.get()(FooterFnArgs {
        footer,
        footer_extra,
        index,
        pages_len: pages.len(),
    });

    PANEL_FN_VAR.get()(PanelFnArgs {
        header,
        side,
        content,
        footer,
    })
}
