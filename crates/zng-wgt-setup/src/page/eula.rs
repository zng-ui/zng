use zng_ext_l10n::l10n;
use zng_wgt::{enabled, margin, on_init, prelude::*};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_markdown::Markdown;
use zng_wgt_scroll::{SCROLL, Scroll, ScrollMode};
use zng_wgt_text::Text;
use zng_wgt_text_input::selectable::SelectableText;
use zng_wgt_toggle as toggle;
use zng_wgt_wizard::Page;

use crate::{APP_NAME_VAR, SETUP_OP_VAR};

/// End-user agreement page.
#[non_exhaustive]
pub struct EulaPage {
    /// The agreement text.
    pub license: Var<EulaTxt>,

    /// Variable that is `true` when the user affirms
    /// they have read and agreed the `license`.
    ///
    /// Is `var(false)` by default.
    pub user_accepts: Var<bool>,

    /// If user must horizontally scroll the `license` text to at
    /// least this amount or more to enable the option to accept.
    ///
    /// This value is checked against the [`SCROLL::horizontal_offset`], it must
    /// be equal or greater than this value to enable the accept option.
    ///
    /// Is `0.fct()` by default.
    pub required_scroll: Factor,
}
impl EulaPage {
    /// New with agreement text.
    pub fn new(license: impl IntoVar<EulaTxt>) -> Self {
        Self {
            license: license.into_var(),
            user_accepts: var(false),
            required_scroll: 0.fct(),
        }
    }

    /// Build the page.
    pub fn build(self) -> Page {
        let title = l10n!("eula/title", "License Agreement");
        let info = SETUP_OP_VAR.flat_map(|op| match op {
            crate::SetupOp::Install => l10n!(
                "eula/info.install",
                "Please review the license terms before installing {$app}",
                app = APP_NAME_VAR
            ),
            crate::SetupOp::Update => l10n!(
                "eula/info.update",
                "Please review the license terms before updating {$app}",
                app = APP_NAME_VAR
            ),
            crate::SetupOp::Repair | crate::SetupOp::Uninstall => const_var("".into()),
        });

        let Self {
            license,
            user_accepts,
            required_scroll,
        } = self;

        let mut pg = Page::new(
            title,
            info,
            wgt_fn!(user_accepts, |_| {
                let accept_enabled = var(required_scroll == 0.fct());
                Container! {
                    child_top = Text! {
                        margin = 10;
                        txt = if required_scroll >= 0.01.fct() {
                            l10n!(
                                "eula/message.requires_scroll",
                                "You must read and accept the terms of this agreement before continuing."
                            )
                        } else {
                            l10n!("eula/message", "You must accept the terms of this agreement before continuing.")
                        };
                    };
                    child = Scroll! {
                        padding = 10;
                        background_color = light_dark(rgb(0.87, 0.87, 0.87), rgb(0.13, 0.13, 0.13));
                        mode = license.map(|l| {
                            let mut mode = ScrollMode::VERTICAL;
                            if matches!(l, EulaTxt::PlainMono(_)) {
                                // no text wrap
                                mode |= ScrollMode::HORIZONTAL;
                            }
                            mode
                        });
                        child = license.present(wgt_fn!(|l| match l {
                            EulaTxt::Plain(txt) => SelectableText! {
                                txt;
                            },
                            EulaTxt::PlainMono(txt) => SelectableText! {
                                txt;
                                font_family = "monospace";
                                txt_wrap = false;
                            },
                            EulaTxt::Markdown(txt) => Markdown! {
                                txt;
                            },
                        }));
                        on_init = hn!(accept_enabled, |_| {
                            // enable accept choice once scroll >= 95%
                            let offset = SCROLL.horizontal_offset();
                            if offset.get() >= required_scroll {
                                accept_enabled.set(true);
                            } else {
                                let sub = offset.hook(clmv!(accept_enabled, |args| {
                                    if *args.value() >= required_scroll {
                                        accept_enabled.set(true);
                                        return false;
                                    }
                                    true
                                }));
                                WIDGET.push_var_handle(sub);
                            }
                        });
                    };
                    child_bottom = Container! {
                        margin = 10;
                        child_spacing = 5;
                        toggle::selector = toggle::Selector::single(user_accepts.clone());
                        toggle::style_fn = toggle::RadioStyle!();
                        child_top = toggle::Toggle! {
                            child = Text!(l10n!("eula/accept.true", "I accept the agreement"));
                            enabled = accept_enabled;
                            value::<bool> = true;
                        };
                        child_bottom = toggle::Toggle! {
                            child = Text!(l10n!("eula/accept.false", "I do not accept the agreement"));
                            value::<bool> = false;
                        };
                    };
                }
            }),
        );
        pg.can_next.0 = user_accepts.read_only();
        pg.content_fill = true;
        pg
    }
}

/// Supported formats for [`EulaPage::license`].
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum EulaTxt {
    /// Plain text, with normal text font and line wrapping.
    Plain(Txt),
    /// Plain text, monospace font, no line wrapping.
    PlainMono(Txt),
    /// Markdown formatted text.
    Markdown(Txt),
}
impl_from_and_into_var! {
    /// `EulaTxt::Plain`.
    fn from(plain: Txt) -> EulaTxt {
        EulaTxt::Plain(plain)
    }
    /// `EulaTxt::Plain`.
    fn from(plain: &'static str) -> EulaTxt {
        EulaTxt::Plain(Txt::from_static(plain))
    }

    fn from(eula: EulaTxt) -> Txt {
        match eula {
            EulaTxt::Plain(txt) | EulaTxt::PlainMono(txt) | EulaTxt::Markdown(txt) => txt,
        }
    }
}
