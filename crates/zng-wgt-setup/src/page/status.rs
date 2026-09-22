use std::fmt::Write as _;

use zng_ext_l10n::l10n;
use zng_ext_setup::{SETUP, SetupOpStatus, SetupStatus, task::SetupTaskError};
use zng_wgt::{ICONS, Wgt, align, prelude::*};
use zng_wgt_container::{Container, child_out_bottom};
use zng_wgt_fill::background;
use zng_wgt_filter::opacity;
use zng_wgt_progress::{Progress, ProgressView};
use zng_wgt_size_offset::{min_height, size, y};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::{Text, icon::ico_color};
use zng_wgt_text_input::selectable::SelectableText;
use zng_wgt_wizard::{Page, PageArgs};

use crate::{APP_NAME_VAR, SETUP_OP_VAR, SetupOp, page::TaskInfo};

/// Setup operation status page.
#[non_exhaustive]
pub struct StatusPage {
    /// The status displayed by the page.
    ///
    /// Is [`SETUP::status`] by default.
    pub status: Var<SetupStatus>,

    /// Display info about setup task components of the operation that will run.
    ///
    /// This task list is displayed on the same order as defined, as a *check list* with
    /// individual progress indicator.
    ///
    /// If this list is empty or a task is not present only the overall operation progress
    /// indicator is shown.
    pub task_infos: Vec<TaskInfo>,

    /// Widget that generates the status item for each item in `task_infos`.
    pub task_info_fn: WidgetFn<StatusTaskInfoFnArgs>,
}
impl Default for StatusPage {
    fn default() -> Self {
        Self {
            status: SETUP.status(),
            task_infos: vec![],
            task_info_fn: WidgetFn::new(default_task_info_fn),
        }
    }
}
impl StatusPage {
    /// New default page.
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert display info about a setup task component.
    pub fn push_info(&mut self, info: TaskInfo) {
        if let Some(i) = self.task_infos.iter().position(|t| t.id == info.id) {
            tracing::debug!("task info for {:?} replaced", info.id);
            self.task_infos.remove(i);
        }
        self.task_infos.push(info);
    }

    /// Insert display info about a setup task component.
    pub fn with_info(mut self, info: TaskInfo) -> Self {
        self.push_info(info);
        self
    }

    /// Build page.
    pub fn build(self) -> Page {
        let title = SETUP_OP_VAR.flat_map(|op| match op {
            SetupOp::Install => l10n!("status/title.install", "Installing"),
            SetupOp::Update => l10n!("status/title.update", "Updating"),
            SetupOp::Repair => l10n!("status/title.repair", "Repairing"),
            SetupOp::Uninstall => l10n!("status/title.uninstall", "Uninstalling"),
        });
        let info = SETUP_OP_VAR.flat_map(|op| match op {
            SetupOp::Install => l10n!(
                "status/info.install",
                "Please wait while {$app} is installed on your computer.",
                app = APP_NAME_VAR
            ),
            SetupOp::Update => l10n!(
                "status/info.update",
                "Please wait while {$app} is updated on your computer.",
                app = APP_NAME_VAR
            ),
            SetupOp::Repair => l10n!(
                "status/info.repair",
                "Please wait while {$app} is repaired on your computer.",
                app = APP_NAME_VAR
            ),
            SetupOp::Uninstall => l10n!(
                "status/info.uninstall",
                "Please wait while {$app} is uninstalled from your computer.",
                app = APP_NAME_VAR
            ),
        });
        let mut pg = Page::new(
            title,
            info,
            wgt_fn!(|_| build(self.status.clone(), self.task_infos.clone(), self.task_info_fn.clone())),
        );
        pg.side = WidgetFn::nil();
        pg.footer = wgt_fn!(|a: PageArgs| {
            let id = a.wizard_id();
            zng_wgt_wizard::default_page_footer_cancel(id)
        });
        pg
    }
}

fn build(status: Var<SetupStatus>, tasks: Vec<TaskInfo>, item_fn: WidgetFn<StatusTaskInfoFnArgs>) -> UiNode {
    // get (is_prepare, SetupOpStatus), but only if it matches SETUP_OP_VAR
    let op_status = expr_var! {
        match #{SETUP_OP_VAR} {
            SetupOp::Install | SetupOp::Update | SetupOp::Repair => match #{status} {
                SetupStatus::PrepareInstall(op) => Some((true, op.clone())),
                SetupStatus::CommitInstall(op) => Some((false, op.clone())),
                _ => None,
            },
            SetupOp::Uninstall => match #{status} {
                SetupStatus::ValidateUninstall(op) => Some((true, op.clone())),
                SetupStatus::Uninstall(op) => Some((false, op.clone())),
                _ => None,
            },
        }
    };

    if tasks.len() <= 1 {
        Stack! {
            direction = StackDirection::top_to_bottom();
            children = ui_vec![
                Text! {
                    txt = op_status.map(|s| match s {
                        Some((_, s)) if s.progress.1 > 1 => formatx!("{} / {}", s.progress.0, s.progress.1),
                        _ => "".into(),
                    });
                    txt_align = Align::CENTER;
                    font_size = 1.2.em();
                },
                ProgressView! {
                    align = Align::FILL_TOP;
                    progress = op_status.flat_map(|s| match s {
                        Some((_, s)) => s.task_progress.0.clone(),
                        None => const_var(Progress::indeterminate()),
                    });
                }
            ];
            spacing = 5;
        }
    } else {
        Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 5;
            children = list_presenter_from_iter(
                tasks,
                wgt_fn!(|a| item_fn(StatusTaskInfoFnArgs::new(a, &op_status))),
            );
        }
    }
}

/// Arguments for generating a task status UI.
#[derive(Clone)]
#[non_exhaustive]
pub struct StatusTaskInfoFnArgs {
    /// Task info.
    pub info: TaskInfo,

    status: Var<Option<(bool, SetupOpStatus)>>,
}
impl StatusTaskInfoFnArgs {
    fn new(info: TaskInfo, status: &Var<Option<(bool, SetupOpStatus)>>) -> Self {
        Self {
            info,
            status: status.clone(),
        }
    }

    /// Task progress.
    pub fn progress(&self) -> Var<Progress> {
        let mut progress = const_var(Progress::from_n_of(0, 1));
        let id = self.info.id.clone();
        self.status.flat_map(move |s| {
            if let Some((_, s)) = s
                && s.task == id
            {
                progress = s.task_progress.0.clone();
            }
            progress.clone()
        })
    }

    /// When `true` progress is for [`SetupStatus::PrepareInstall`] or [`SetupStatus::ValidateUninstall`].
    ///
    /// When `false` progress is for [`SetupStatus::CommitInstall`] or [`SetupStatus::Uninstall`].
    pub fn is_preparing(&self) -> Var<bool> {
        self.status.map(|s| if let Some((is_prep, _)) = s { *is_prep } else { true })
    }

    /// Progress is for cancel operation.
    pub fn is_cancelling(&self) -> Var<bool> {
        self.status.map(|s| if let Some((_, s)) = s { s.cancel } else { false })
    }

    /// Task errors.
    pub fn errors(&self) -> Var<Vec<SetupTaskError>> {
        let id = self.info.id.clone();
        self.status.map(move |s| {
            if let Some((_, s)) = s {
                s.errors
                    .iter()
                    .filter_map(|((_, ty, name), e)| if *ty == id.0 && *name == id.1 { Some(e.clone()) } else { None })
                    .collect()
            } else {
                vec![]
            }
        })
    }

    /// If task has completed successfully.
    pub fn is_success(&self) -> Var<bool> {
        let mut progress = const_var(false);
        let id = self.info.id.clone();
        self.status.flat_map(move |s| {
            if let Some((is_prep, s)) = s
                && !is_prep
            {
                if s.errors.iter().any(|((_, ty, name), _)| *ty == id.0 && *name == id.1) {
                    progress = const_var(false);
                } else if s.task == id {
                    progress = s.task_progress.map(|p| p.is_complete());
                }
            }
            progress.clone()
        })
    }
}

pub fn default_task_info_fn(args: StatusTaskInfoFnArgs) -> UiNode {
    let errors = args.errors().map(|e| {
        let mut r = String::new();
        let mut sep = "";
        for e in e {
            write!(&mut r, "{sep}{e}").unwrap();
            sep = "\n";
        }
        Txt::from(r)
    });
    Container! {
        child_start = Wgt! {
            background = ICONS.req("circle");
            y = 5;
            size = 20;

            #[easing(300.ms())]
            opacity = 0.1.fct();

            when #{args.is_success()} {
                opacity = 1.fct();
                background = ICONS.req("check-circle");
            }
            when !#{errors.clone()}.is_empty() {
                opacity = 1.fct();
                background = ICONS.req("error");
                ico_color = light_dark(colors::RED.darken(50.pct()), colors::RED.lighten(50.pct()));
            }
        };
        child_bottom = ProgressView! {
            progress = args.progress();
            // reserve space for progress message
            min_height = 24;

            when !#{errors.clone()}.is_empty() {
                // replace progress message
                child_out_bottom = SelectableText! {
                    txt = errors;
                    font_color = light_dark(colors::RED.darken(70.pct()), colors::RED.lighten(70.pct()));
                };
            }
        };
        child = Text! {
            txt = args.info.info.0;
            font_size = 1.1.em();
        };
        child_spacing = 5;
    }
}
