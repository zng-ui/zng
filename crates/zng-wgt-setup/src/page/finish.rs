use std::mem;

use zng_ext_l10n::l10n;
use zng_wgt::{
    enabled, margin,
    prelude::{task::parking_lot::Mutex, *},
};
use zng_wgt_container::Container;
use zng_wgt_markdown::Markdown;
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::Text;
use zng_wgt_toggle::{CheckStyle, Toggle};
use zng_wgt_wizard::{CAN_FINISH_VAR, FINISH_CMD, Page, PageArgs, default_page_footer_finish};

use crate::{APP_NAME_VAR, APP_VERSION_VAR, INSTALLED_VERSION_VAR, SETUP_OP_VAR, SetupOp};

/// Setup operation finish without errors page.
pub struct FinishPage {
    /// Title.
    ///
    /// Displayed inside the content, unlike other pages.
    pub title: Var<Txt>,

    /// Default message markdown.
    pub msg: Var<Txt>,

    /// Custom message markdown, appended after the `msg` paragraph.
    pub msg_extra: Var<Txt>,

    /// Actions to run once the wizard finishes.
    pub actions: Vec<FinishAction>,
}
impl Default for FinishPage {
    fn default() -> Self {
        Self {
            title: SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install => l10n!("finish/title.install", "{$app} Installed", app = APP_NAME_VAR),
                SetupOp::Update => l10n!("finish/title.update", "{$app} Updated", app = APP_NAME_VAR),
                SetupOp::Repair => l10n!("finish/title.repair", "{$app} Repaired", app = APP_NAME_VAR),
                SetupOp::Uninstall => l10n!("finish/title.uninstall", "{$app} Uninstalled", app = APP_NAME_VAR),
            }),
            msg: SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install => l10n!(
                    "finish/message.install",
                    "Wizard has installed {$app} {$version} on your computer.",
                    app = APP_NAME_VAR,
                    version = APP_VERSION_VAR,
                ),
                SetupOp::Update => l10n!(
                    "finish/message.update",
                    "Wizard has updated {$app} from {$current_version} to {$new_version} on your computer.",
                    app = APP_NAME_VAR,
                    current_version = INSTALLED_VERSION_VAR,
                    new_version = APP_VERSION_VAR,
                ),
                SetupOp::Repair => l10n!(
                    "finish/message.install",
                    "Wizard has repaired {$app} {$version} installation on your computer.",
                    app = APP_NAME_VAR,
                    version = APP_VERSION_VAR,
                ),
                SetupOp::Uninstall => l10n!(
                    "finish/message.uninstall",
                    "Wizard has uninstalled {$app} {$version} from your your computer.",
                    app = APP_NAME_VAR,
                    version = APP_VERSION_VAR,
                ),
            }),
            msg_extra: const_var("".into()),
            actions: vec![],
        }
    }
}
impl FinishPage {
    /// New default, without finish actions.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the page.
    pub fn build(self) -> Page {
        let Self {
            title,
            msg,
            msg_extra,
            actions,
        } = self;

        let msg = expr_var! {
            formatx!("## {}\n\n{}\n\n{}", #{title}, #{msg}, #{msg_extra})
        };

        let (action_infos, actions): (Vec<_>, Vec<_>) = actions.into_iter().map(|a| ((a.selected, a.info), a.action)).unzip();
        let actions = Mutex::new(actions);

        let content = wgt_fn!(|args: PageArgs| {
            if !args.is_last() && !args.is_custom() {
                tracing::warn!("finish page is not last");
            }

            // subscribe to run actions on finish
            let actions = mem::take(&mut *actions.lock());
            if !actions.is_empty() {
                let selected: Vec<_> = action_infos.iter().map(|(s, _)| s.clone()).collect();
                let handle = FINISH_CMD.scoped(args.wizard_id()).on_pre_event(
                    false,
                    true,
                    false,
                    hn_once!(|_| {
                        for (action, selected) in actions.into_iter().zip(selected) {
                            if selected.get()
                                && let Err(e) = action()
                            {
                                tracing::error!("finish action failed, {e}");
                            }
                        }
                    }),
                );
                // let SetupWizard! define if can finish
                CAN_FINISH_VAR.set_bind(handle.enabled()).perm();
                WIDGET.push_any_handle(handle);
            }

            Container! {
                child = Scroll! {
                    child_align = Align::FILL_TOP;
                    padding = 20;
                    child = Markdown! {
                        txt = msg.clone();
                    };
                    mode = ScrollMode::VERTICAL;
                };
                child_bottom = Stack! {
                    margin = (0, 20, 20, 20);
                    direction = StackDirection::top_to_bottom();
                    spacing = 5;
                    children =
                        action_infos
                            .iter()
                            .map(|(checked, txt)| {
                                Toggle! {
                                    checked = checked.clone();
                                    child = Text!(txt.clone());
                                    style_fn = CheckStyle!();
                                    enabled = checked.capabilities().can_modify();
                                }
                            })
                            .collect::<UiVec>(),
                    ;
                };
            }
        });

        let mut pg = Page::new("", "", content);
        pg.content_fill = true;
        pg.header = WidgetFn::nil();
        pg.footer = wgt_fn!(|a: PageArgs| default_page_footer_finish(a.wizard_id()));
        pg
    }
}

/// Represents an action that [`FinishPage`] runs once the wizard finishes.
#[non_exhaustive]
pub struct FinishAction {
    /// Short display info about the action.
    pub info: Var<Txt>,
    /// If the action should run.
    ///
    /// If this var is modifiable the user will see an enabled checkbox for the action.
    pub selected: Var<bool>,
    /// Action to run.
    ///
    /// The action will run blocking on the app context, on a [`FINISH_CMD`] preview handler. Note
    /// that the setup app will likely exit just after this call, starting async tasks is not recommended.
    pub action: Box<dyn FnOnce() -> std::io::Result<()> + Send + 'static>,
}
impl FinishAction {
    /// New action the user can enable/disable.
    pub fn new(info: impl IntoVar<Txt>, enabled_default: bool, action: impl FnOnce() -> std::io::Result<()> + Send + 'static) -> Self {
        Self {
            info: info.into_var(),
            selected: var(enabled_default),
            action: Box::new(action),
        }
    }

    /// Force `enabled`, the user will not be allowed to disable.
    pub fn required(mut self) -> Self {
        self.selected = const_var(true);
        self
    }

    /// Action that runs the installed app.
    pub fn run_app(mut run_app_cmd: std::process::Command) -> Self {
        Self::new(l10n!("finish/action-run.info", "Run {$app}", app = APP_NAME_VAR), true, move || {
            run_app_cmd.spawn()?;
            Ok(())
        })
    }
}
