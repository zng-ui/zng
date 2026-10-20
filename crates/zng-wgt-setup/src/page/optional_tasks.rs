use zng_ext_l10n::l10n;
use zng_var::MergeVarBuilder;
use zng_wgt::{margin, prelude::*};
use zng_wgt_container::Container;
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::Text;
use zng_wgt_toggle::Toggle;
use zng_wgt_wizard::Page;

use crate::APP_NAME_VAR;

/// Page for reviewing and enabling optional setup tasks.
///
/// The *tasks* represented here do not need to match a `SetupTask` each, usually
/// the entire page configures a single `SetupTask`.
#[non_exhaustive]
pub struct OptionalTasksPage {
    /// Page title.
    ///
    /// This is the [`Page::title`] value.
    pub title: Var<Txt>,

    /// Page info.
    ///
    /// This is the [`Page::info`] value.
    pub info: Var<Txt>,

    /// Message shown at the top of the page content.
    pub message: Var<Txt>,

    /// List of optional tasks.
    ///
    /// Tasks are presented as a list of check boxes, in the same order as this vec.
    /// In case of grouping the first task of that group defines the group position.
    pub tasks: Vec<OptionalTask>,

    /// Info about `tasks` groups.
    ///
    /// Groups are presented as a check box header that can enable or disable all child
    /// items at the same time.
    pub groups: Vec<OptionalTaskGroup>,
}
impl OptionalTasksPage {
    /// New generic "Select Adicional Tasks" page.
    pub fn new_generic() -> Self {
        Self {
            title: l10n!("optional_tasks/generic.title", "Select Additional Tasks"),
            info: l10n!("optional_tasks/generic.info", "Which additional tasks should be performed?"),
            message: l10n!(
                "optional_tasks/generic.message",
                "Select additional tasks you would like to setup to perform."
            ),
            tasks: vec![],
            groups: vec![],
        }
    }

    /// New "Associate File Types" page.
    pub fn new_associate_files() -> Self {
        Self {
            title: l10n!("optional_tasks/associate_files.title", "Associate File Types"),
            info: l10n!(
                "optional_tasks/associate_files.info",
                "Which file types should open with {$app} by default?",
                app = APP_NAME_VAR
            ),
            message: l10n!(
                "optional_tasks/associate_files.message",
                "Select file types you would like associate with {$app}.",
                app = APP_NAME_VAR
            ),
            tasks: vec![],
            groups: vec![],
        }
    }

    /// Build the page.
    ///
    /// # Panics
    ///
    /// Panics if no tasks are defined or if a task requests an undefined group.
    pub fn build(self) -> Page {
        assert!(!self.tasks.is_empty());
        // groups and items are displayed on the same list, with margin adjusted to define group children
        let mut items = Vec::with_capacity(self.tasks.len() + self.groups.len());
        enum Item {
            Task(Var<Txt>, Var<bool>),
            GroupHeader(Var<Txt>, Var<Option<bool>>),
            GroupTask(Var<Txt>, Var<bool>),
            GroupReserved,
        }
        let mut taken = vec![false; self.tasks.len()];
        for i in 0..self.tasks.len() {
            if taken[i] {
                continue;
            }
            taken[i] = true;

            let task = &self.tasks[i];
            if task.group_id.is_empty() {
                items.push(Item::Task(task.name.clone(), task.enabled.clone()));
            } else {
                let group = self
                    .groups
                    .iter()
                    .find(|g| g.id == task.group_id)
                    .unwrap_or_else(|| panic!("group {:?} not defined", task.group_id));
                let group_i = items.len();
                let mut group_enabled = MergeVarBuilder::new();

                items.push(Item::GroupReserved);

                let mut modifiable = task.enabled.capabilities().can_modify();

                group_enabled.push(task.enabled.clone());
                items.push(Item::GroupTask(task.name.clone(), task.enabled.clone()));

                #[allow(clippy::needless_range_loop)] // false positive
                for i in (i + 1)..self.tasks.len() {
                    let task = &self.tasks[i];
                    if task.group_id == group.id {
                        taken[i] = true;
                        modifiable |= task.enabled.capabilities().can_modify();
                        group_enabled.push(task.enabled.clone());
                        items.push(Item::GroupTask(task.name.clone(), task.enabled.clone()));
                    }
                }

                fn merge(i: zng_var::MergeVarInputs<'_, bool>) -> Option<bool> {
                    let enabled = i.get(0);
                    if i.iter().all(|i| i == enabled) { Some(enabled) } else { None }
                }
                let group_enabled = if modifiable {
                    group_enabled.build_bidi_modify(merge, |m, _, mut v| {
                        if let Some(enabled) = *m {
                            v.set(enabled);
                        }
                    })
                } else {
                    group_enabled.build(merge)
                };

                items[group_i] = Item::GroupHeader(group.name.clone(), group_enabled);
            }
        }

        let message = self.message;
        let mut pg = Page::new(
            self.title,
            self.info,
            wgt_fn!(|_| {
                let items = Stack! {
                    direction = StackDirection::top_to_bottom();
                    spacing = 8;
                    zng_wgt_toggle::style_fn = zng_wgt_style::style_fn!(|_| zng_wgt_toggle::CheckStyle!());
                    children =
                        items
                            .iter()
                            .map(|it| match it {
                                Item::Task(name, enabled) => Toggle! {
                                    child = Text!(name.clone());
                                    checked = enabled.clone();
                                },
                                Item::GroupHeader(name, enabled) => Toggle! {
                                    child = Text!(name.clone());
                                    checked_opt = enabled.clone();
                                },
                                Item::GroupTask(name, enabled) => Toggle! {
                                    child = Text!(name.clone());
                                    checked = enabled.clone();
                                    margin = (10, 0);
                                },
                                Item::GroupReserved => unreachable!(),
                            })
                            .collect::<UiVec>(),
                    ;
                };
                Container! {
                    child_top = Text! {
                        txt = message.clone();
                        margin = 10;
                    };
                    child = Scroll! {
                        padding = 10;
                        mode = ScrollMode::VERTICAL;
                        child = items;
                        child_align = Align::FILL;
                    };
                }
            }),
        );
        pg.content_fill = true;
        pg
    }
}

/// Represents a task group in [`OptionalTasksPage`].
#[non_exhaustive]
pub struct OptionalTaskGroup {
    /// Group ID.
    ///
    /// Cannot be empty.
    pub id: Txt,
    /// Group display name.
    pub name: Var<Txt>,
}
impl OptionalTaskGroup {
    /// New from id and name.
    ///
    /// # Panics
    ///
    /// Panics if `id` is empty.
    pub fn new(id: impl Into<Txt>, name: Var<Txt>) -> Self {
        let r = Self { id: id.into(), name };
        assert!(r.id.is_empty());
        r
    }
}

/// Represents a task in [`OptionalTasksPage`].
#[non_exhaustive]
pub struct OptionalTask {
    /// Optional group ID.
    ///
    /// If this is not empty the task will be listed under the group section.
    pub group_id: Txt,
    /// Task display name.
    pub name: Var<Txt>,
    /// Variable that defines if the task must run.
    ///
    /// Set this to a [`const_var`] to show a disabled check box.
    pub enabled: Var<bool>,
}
impl OptionalTask {
    /// New without group, with modifiable `enabled` that is enabled by default.
    pub fn new(name: Var<Txt>) -> Self {
        Self {
            group_id: "".into(),
            name,
            enabled: var(true),
        }
    }
}
