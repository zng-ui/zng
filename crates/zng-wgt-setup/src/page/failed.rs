use zng_ext_l10n::l10n;
use zng_ext_setup::{
    SetupError, SetupErrorState,
    task::{SetupTaskError, TaskTypeId},
};
use zng_wgt::{ICONS, Wgt, WidgetFn, is_inited, prelude::*, wgt_fn, visibility};
use zng_wgt_container::Container;
use zng_wgt_filter::opacity;
use zng_wgt_size_offset::{y, size};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::{self as text, Text};
use zng_wgt_wizard::{Page, PageArgs};
use zng_wgt_fill::background;
use std::fmt::Write as _;
use zng_wgt_text::icon::ico_color;
use zng_wgt_input::{cursor, CursorIcon};

use crate::{APP_NAME_VAR, INSTALLED_VERSION_VAR, SETUP_OP_VAR, SetupOp, page::TaskInfo};

/// Setup operation failed page.
pub struct FailedPage {
    /// The error.
    pub error: SetupError,

    /// Display info about setup task components of the operation that failed.
    ///
    /// This task list is displayed on the same order as defined, as a *check list* with
    /// individual progress indicator.
    ///
    /// If this list is empty or a task is not present only the overall operation error is shown.
    ///
    /// This is usually a clone of [`StatusPage::task_infos`].
    ///
    /// [`StatusPage::task_infos`]: crate::page::StatusPage::task_infos
    pub task_infos: Vec<TaskInfo>,

    /// Widget that generates the status item for each item in `task_infos`.
    pub task_info_fn: WidgetFn<FailedTaskInfoFnArgs>,
}
impl FailedPage {
    /// New from error, without task infos.
    pub fn new(error: SetupError) -> Self {
        Self {
            error,
            task_infos: vec![],
            task_info_fn: WidgetFn::new(default_task_info_fn),
        }
    }

    /// Build page.
    pub fn build(self) -> Page {
        let title = SETUP_OP_VAR.flat_map(|op| match op {
            SetupOp::Install => l10n!("failed/title.install", "Install Failed"),
            SetupOp::Update => l10n!("failed/title.update", "Update Failed"),
            SetupOp::Repair => l10n!("failed/title.repair", "Repair Failed"),
            SetupOp::Uninstall => l10n!("failed/title.uninstall", "Uninstall Failed"),
        });

        let info = match &self.error.state {
            SetupErrorState::Canceled | SetupErrorState::PartialPrepareInstall => {
                l10n!("failed/info.clean", "Operation failed, all changes undone.")
            }
            SetupErrorState::PartialInstall { .. } | SetupErrorState::PartialUninstall => {
                l10n!("failed/info.corrupted", "Operation failed, installation corrupted.")
            }
        };

        let message = match &self.error.state {
            SetupErrorState::Canceled | SetupErrorState::PartialPrepareInstall => SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install => l10n!(
                    "failed/message.install-clean",
                    "Install operation failed, {$app} was not installed.",
                    app = APP_NAME_VAR
                ),
                SetupOp::Update => l10n!(
                    "failed/message.update-clean",
                    "Update operation failed, {$app} was not updated. Current installed version {$current_version} remains and can still be used.",
                    app = APP_NAME_VAR,
                    current_version = INSTALLED_VERSION_VAR,
                ),
                SetupOp::Repair => l10n!(
                    "failed/message.repair-clean",
                    "Repair operation failed, {$app} installation was not repaired.",
                    app = APP_NAME_VAR,
                ),
                SetupOp::Uninstall => l10n!("failed/message.uninstall-clean", "Uninstall operation failed, {$app} remains installed.", app = APP_NAME_VAR),
            }),
            SetupErrorState::PartialInstall { .. }|
            SetupErrorState::PartialUninstall => SETUP_OP_VAR.flat_map(|op| match op {
                SetupOp::Install => l10n!("failed/message.install-corrupted", "Install operation failed, {$app} was only partially installed.", app = APP_NAME_VAR),
                SetupOp::Update => l10n!("failed/message.update-corrupted", "Update operation failed, {$app} was only partially updated.", app = APP_NAME_VAR),
                SetupOp::Repair => l10n!("failed/message.repair-corrupted", "Repair operation failed, {$app} was only partially repaired.", app = APP_NAME_VAR),
                SetupOp::Uninstall => l10n!("failed/message.uninstall-corrupted", "Uninstall operation failed, {$app} was only partially uninstalled.", app = APP_NAME_VAR),
            }),
        };

        let mut pg = Page::new(
            title,
            info,
            wgt_fn!(|_| build(
                message.clone(),
                self.error.clone(),
                self.task_infos.clone(),
                self.task_info_fn.clone()
            )),
        );
        pg.side = WidgetFn::nil();
        pg.footer = wgt_fn!(|a: PageArgs| {
            let id = a.wizard_id();
            zng_wgt_wizard::default_page_footer_cancel(id)
        });
        pg
    }
}

fn build(message: Var<Txt>, error: SetupError, task_infos: Vec<TaskInfo>, task_info_fn: WidgetFn<FailedTaskInfoFnArgs>) -> UiNode {
    let SetupError {
        op_error,
        task_errors,
        state,
        ..
    } = error;
    let op_error = match op_error {
        Some(e) => e.to_txt(),
        None => "".into(),
    };
    Container! {
        text::rich_text = true;
        text::txt_selectable = true;
        cursor = CursorIcon::Text;
        child_spacing = 10;
        child_top = Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 10;
            children = ui_vec![
                Text!(message),
                Text! {
                    visibility = !op_error.is_empty();
                    txt = op_error;
                    font_color = error_font_color();
                }
            ];

            #[easing(600.ms())]
            opacity = 0.pct();
            when #is_inited {
                opacity = 100.pct();
            }
        };
        child = Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 5;
            children = list_presenter_from_iter(
                task_infos,
                wgt_fn!(|a| task_info_fn(FailedTaskInfoFnArgs::new(a, &task_errors, &state))),
            );

            #[easing(300.ms())]
            y = -15;
            when #is_inited {
                y = 0;
            }
        };
    }
}

/// Arguments for generating a task status UI.
#[derive(Clone)]
#[non_exhaustive]
pub struct FailedTaskInfoFnArgs {
    /// Task info.
    pub info: TaskInfo,

    /// Task errors.
    pub errors: Vec<SetupTaskError>,

    /// If all task changes where undone or task never committed changes.
    ///
    /// If this is `true` and `errors` is empty the task never started.
    ///
    /// If this is `false` and `errors` is empty the task completed successfully.
    pub clean: bool,
}
impl FailedTaskInfoFnArgs {
    fn new(info: TaskInfo, task_errors: &[((usize, TaskTypeId, Txt), SetupTaskError)], state: &SetupErrorState) -> Self {
        Self {
            errors: task_errors
                .iter()
                .filter_map(|((_, id, name), e)| {
                    if id == &info.id.0 && name == &info.id.1 {
                        Some(e.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            clean: matches!(state, SetupErrorState::Canceled | SetupErrorState::PartialPrepareInstall),
            info,
        }
    }
}

pub fn default_task_info_fn(args: FailedTaskInfoFnArgs) -> UiNode {
    let mut r = String::new();
    let mut sep = "";
    for e in &args.errors {
        write!(&mut r, "{sep}{e}").unwrap();
        sep = "\n";
    }
    let errors = Txt::from(r);
    Container! {
        child_start = Wgt! {
            background = ICONS.req(if args.clean { "circle" } else { "error" });
            y = 5;
            size = 20;

            when !args.clean {
                ico_color = light_dark(colors::RED.darken(50.pct()), colors::RED.lighten(50.pct()));
            }
        };
        child_bottom = Text! {
            txt = errors;
            font_color = error_font_color();
        };
        child = Text! {
            txt = args.info.info.0;
            font_size = 1.1.em();
        };
        child_spacing = (0, 5);
    }
}

fn error_font_color() -> LightDark {
    light_dark(colors::RED.darken(70.pct()), colors::RED.lighten(70.pct()))
}