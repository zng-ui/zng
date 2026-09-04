use zng_ext_l10n::l10n;
use zng_wgt::prelude::*;
use zng_wgt_markdown::Markdown;
use zng_wgt_wizard::{Page, PageArgs};

use crate::{APP_NAME_VAR, APP_VERSION_VAR, INSTALLED_VERSION_VAR, SETUP_OP_VAR, SetupOp};

/// Page that shows a markdown message that introduces the purpose of the wizard.
#[non_exhaustive]
pub struct WelcomePage {
    /// Title.
    ///
    /// Displayed inside the content, unlike other pages.
    pub title: Var<Txt>,
    /// Default message markdown.
    pub msg: Var<Txt>,
    /// Custom message markdown, appended after the `msg` paragraph.
    pub msg_extra: Var<Txt>,
}
impl Default for WelcomePage {
    fn default() -> Self {
        Self {
            title: SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install => l10n!("welcome/title.install", "Welcome to the {$app} Install Wizard", app = APP_NAME_VAR),
                SetupOp::Update => l10n!("welcome/title.update", "Welcome to the {$app} Update Wizard", app = APP_NAME_VAR),
                SetupOp::Repair => l10n!("welcome/title.repair", "Welcome to the {$app} Repair Wizard", app = APP_NAME_VAR),
                SetupOp::Uninstall => l10n!(
                    "welcome/title.uninstall",
                    "Welcome to the {$app} Uninstall Wizard",
                    app = APP_NAME_VAR
                ),
            }),
            msg: SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install => l10n!(
                    "welcome/message.install",
                    "This will install {$app} {$version} on your computer.",
                    app = APP_NAME_VAR,
                    version = APP_VERSION_VAR,
                ),
                SetupOp::Update => l10n!(
                    "welcome/message.update",
                    "This will update {$app} from {$current_version} to {$new_version} on your computer.",
                    app = APP_NAME_VAR,
                    current_version = INSTALLED_VERSION_VAR,
                    new_version = APP_VERSION_VAR,
                ),
                SetupOp::Repair => l10n!(
                    "welcome/message.install",
                    "This will repair {$app} {$version} installation on your computer.",
                    app = APP_NAME_VAR,
                    version = APP_VERSION_VAR,
                ),
                SetupOp::Uninstall => l10n!(
                    "welcome/message.uninstall",
                    "This will uninstall {$app} {$version} from your your computer.",
                    app = APP_NAME_VAR,
                    version = APP_VERSION_VAR,
                ),
            }),
            msg_extra: const_var("".into()),
        }
    }
}
impl WelcomePage {
    /// New with message extra.
    pub fn new(msg_extra: impl IntoVar<Txt>) -> Self {
        Self {
            msg_extra: msg_extra.into_var(),
            ..Self::default()
        }
    }

    /// Build the page.
    pub fn build(self) -> Page {
        let Self { title, msg, msg_extra } = self;
        let msg = expr_var! {
            formatx!("## {}\n\n{}\n\n{}", #{title}, #{msg}, #{msg_extra})
        };
        let mut pg = Page::new(
            "",
            "",
            wgt_fn!(|args: PageArgs| {
                if !args.is_first() {
                    tracing::warn!("welcome page is not first");
                }
                Markdown! {
                    txt = msg.clone();
                }
            }),
        );
        pg.header = WidgetFn::nil();
        pg
    }
}
