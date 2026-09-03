//! Common setup pages.

use std::path::PathBuf;

use zng_ext_l10n::l10n;
use zng_unit::ByteLength;
use zng_wgt::{align, prelude::*, visibility};
use zng_wgt_button::Button;
use zng_wgt_container::Container;
use zng_wgt_dialog::DIALOG;
use zng_wgt_markdown::Markdown;
use zng_wgt_menu::context::ContextMenu;
use zng_wgt_text::Text;
use zng_wgt_text_input::{TextInput, label::Label};
use zng_wgt_toggle::{self as toggle, Toggle};
use zng_wgt_wizard::{Page, PageArgs};

use crate::{APP_NAME_VAR, APP_ORG_VAR, APP_VERSION_VAR, INSTALLED_VERSION_VAR, SETUP_OP_VAR, SetupOp};

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

/// Page for reviewing and changing the install directory.
#[non_exhaustive]
pub struct InstallDirPage {
    /// Default install directory.
    pub default_dir: Var<PathBuf>,
    /// User selected install directory.
    pub install_dir: Var<PathBuf>,
    /// Minimal required space on the selected disk.
    ///
    /// If this is `0.bytes()` the required space is not shown.
    pub min_required_space: Var<ByteLength>,
}
impl Default for InstallDirPage {
    fn default() -> Self {
        let default_dir = expr_var! {
            let org = #{APP_ORG_VAR};
            let app = #{APP_NAME_VAR};

            if let Ok(pf) = std::env::var("ProgramFiles") {
                let mut dir = PathBuf::from(pf);
                if !org.is_empty() {
                    dir.push(org);
                }
                if !app.is_empty() {
                    dir.push(app);
                }
                dir
            } else {
                PathBuf::new()
            }
        };
        Self {
            install_dir: default_dir.cow(),
            default_dir,
            min_required_space: const_var(0.bytes()),
        }
    }
}
impl InstallDirPage {
    /// New default.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the page.
    pub fn build(self) -> Page {
        let title = SETUP_OP_VAR.flat_map(|op| match op {
            SetupOp::Install => l10n!("install_dir/title.install", "Select Install Location"),
            SetupOp::Update | SetupOp::Repair => l10n!("destination/title.update-repair", "Change Install Location"),
            SetupOp::Uninstall => l10n!("install_dir/title.uninstall", "Install Location"),
        });
        let info = SETUP_OP_VAR.flat_map(|op| match op {
            SetupOp::Install => l10n!("install_dir/info.install", "Where should {$app} be installed?", app = APP_NAME_VAR),
            SetupOp::Update | SetupOp::Repair | SetupOp::Uninstall => {
                l10n!(
                    "install_dir/info.update-repair-uninstall",
                    "Where {$app} is installed.",
                    app = APP_NAME_VAR
                )
            }
        });
        let min_space = self.min_required_space.flat_map(|&s| {
            if s == 0.bytes() {
                const_var(Txt::default())
            } else {
                // l10n-# $bytes is already formatted e.g.: 900kB or 50MB.
                l10n!(
                    "install-dir/min-required-space",
                    "At least {$bytes} of free disk space is required.",
                    bytes = s
                )
            }
        });

        let mut pg = Page::new(
            title.clone(),
            info,
            wgt_fn!(|_| { install_dir_ui(self.default_dir.clone(), self.install_dir.clone(), title.clone(), min_space.clone()) }),
        );
        pg.side = WidgetFn::nil();
        pg
    }
}
fn install_dir_ui(
    default_dir: Var<PathBuf>,
    install_dir: Var<PathBuf>,
    select_dlg_title: Var<Txt>,
    min_required_space: Var<Txt>,
) -> UiNode {
    let can_modify = if install_dir.capabilities().is_always_read_only() {
        const_var(false)
    } else {
        SETUP_OP_VAR.map(|op| !matches!(op, SetupOp::Uninstall))
    };
    let can_reset = expr_var! {
        *#{can_modify.clone()} && #{default_dir.clone()} != #{install_dir.clone()}
    };
    Container! {
        child_spacing = 5;
        child = TextInput! {
            txt = install_dir.map(|p| {
                let p = p.display().to_txt();
                if cfg!(windows) {
                    p.replace('/', "\\")
                } else {
                    p.replace('\\', "/")
                }
                .to_txt()
            });
            txt_editable = false;
        };

        when #{can_modify} {
            child_end = select_dir_btn(install_dir.clone(), select_dlg_title.clone());
            child_bottom = Text! {
                align = Align::BOTTOM_RIGHT;
                visibility = min_required_space.map(|t| (!t.is_empty()).into());
                txt = min_required_space;
            };
        }
        when #{can_reset} {
            child_end = Toggle! {
                style_fn = toggle::ComboStyle!();
                child = select_dir_btn(install_dir.clone(), select_dlg_title);
                checked_popup = wgt_fn!(|_| ContextMenu! {
                    children = ui_vec![
                        Button! {
                            child = Label! {
                                txt = l10n!("install_dir/reset-label", "Default Location");
                            };
                            on_click = hn!(default_dir, install_dir, |args| {
                                args.propagation.stop();
                                install_dir.set(default_dir.get());
                            });
                        }
                    ]
                });
            };
        }
    }
}
fn select_dir_btn(install_dir: Var<PathBuf>, select_dlg_title: Var<Txt>) -> UiNode {
    Button! {
        child = Label! {
            txt = l10n!("install_dir/select-label", "Select Location");
        };
        on_click = async_hn!(install_dir, select_dlg_title, |args| {
            args.propagation.stop();
            select_dir_dlg(install_dir, select_dlg_title).await;
        });
    }
}
async fn select_dir_dlg(install_dir: Var<PathBuf>, title: Var<Txt>) {
    let current = install_dir.get();
    let current_parent = current.parent().map(PathBuf::from).unwrap_or_default();
    let current_name = current.file_name().unwrap_or_default().to_str().unwrap_or_default().to_txt();
    let r = DIALOG.select_folder(title, current_parent, current_name);

    match r.wait_rsp().await {
        zng_wgt_dialog::FileDialogResponse::Selected(mut p) => install_dir.set(p.remove(0)),
        zng_wgt_dialog::FileDialogResponse::Cancel => {}
        zng_wgt_dialog::FileDialogResponse::Error(e) => {
            tracing::error!("cannot select install dir, {e}");
        }
        _ => unreachable!(),
    }
}
