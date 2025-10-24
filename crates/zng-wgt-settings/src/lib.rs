#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//! Settings widgets.

zng_wgt::enable_widget_macros!();

mod view_fn;
pub use view_fn::*;

use zng_ext_config::settings::{Category, CategoryId, SETTINGS};
use zng_ext_input::focus::FOCUS;
use zng_ext_l10n::l10n;
use zng_ext_window::{WINDOW_Ext as _, WINDOWS};
use zng_wgt::{node::VarPresent as _, prelude::*};
use zng_wgt_input::cmd::SETTINGS_CMD;
use zng_wgt_size_offset::actual_width;
use zng_wgt_window::{SaveState, Window, save_state_node};

/// Settings editor widget.
#[widget($crate::SettingsEditor)]
pub struct SettingsEditor(WidgetBase);
impl SettingsEditor {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            save_state = SaveState::enabled();
            zng_wgt_fill::background_color = light_dark(rgb(0.85, 0.85, 0.85), rgb(0.15, 0.15, 0.15));
            zng_wgt_container::padding = 10;

            when *#actual_width <= 400 && *#actual_width > 1 {
                panel_fn = WidgetFn::new(default_panel_mobile_fn);
                categories_list_fn = WidgetFn::new(default_categories_list_mobile_fn);
            }
        }
        self.widget_builder().push_build_action(|wgt| {
            wgt.set_child(settings_editor_node());
            wgt.push_intrinsic(NestGroup::EVENT, "command-handler", command_handler);
            wgt.push_intrinsic(NestGroup::CONTEXT, "editor-vars", |child| {
                let child = with_context_var_init(child, EDITOR_STATE_VAR, editor_state);
                let child = with_context_var(child, EDITOR_SEARCH_VAR, var(Txt::from("")));
                with_context_var(child, EDITOR_SELECTED_CATEGORY_VAR, var(CategoryId::from("")))
            });
        });
    }
}

/// Implements the [`SettingsEditor!`] inner widgets.
///
/// [`SettingsEditor!`]: struct@SettingsEditor
pub fn settings_editor_node() -> UiNode {
    match_node(UiNode::nil(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&SETTINGS_FN_VAR)
                .sub_var(&SETTING_FN_VAR)
                .sub_var(&SETTINGS_SEARCH_FN_VAR)
                .sub_var(&CATEGORIES_LIST_FN_VAR)
                .sub_var(&CATEGORY_HEADER_FN_VAR)
                .sub_var(&CATEGORY_ITEM_FN_VAR)
                .sub_var(&PANEL_FN_VAR);
            *c.node() = settings_view_fn();
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
        }
        UiNodeOp::Update { .. } => {
            if PANEL_FN_VAR.is_new()
                || SETTINGS_FN_VAR.is_new()
                || SETTING_FN_VAR.is_new()
                || SETTINGS_SEARCH_FN_VAR.is_new()
                || CATEGORIES_LIST_FN_VAR.is_new()
                || CATEGORY_HEADER_FN_VAR.is_new()
                || CATEGORY_ITEM_FN_VAR.is_new()
            {
                c.delegated();
                c.node().deinit();
                *c.node() = settings_view_fn();
                c.node().init();
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

fn editor_state() -> Var<Option<SettingsEditorState>> {
    // avoids rebuilds for ignored search changes
    let clean_search = SETTINGS.editor_search().current_context().map(|s| {
        let s = s.trim();
        if !s.starts_with('@') {
            s.to_lowercase().into()
        } else {
            Txt::from_str(s)
        }
    });

    let sel_cat = SETTINGS.editor_selected_category().current_context();
    let r = expr_var! {
        if #{clean_search}.is_empty() {
            // no search, does not need to load settings of other categories
            let (cat, settings) = SETTINGS
                .get(|_, cat| cat == #{sel_cat}, true)
                .pop()
                .unwrap_or_else(|| (Category::unknown(#{sel_cat}.clone()), vec![]));
            Some(SettingsEditorState {
                clean_search: #{clean_search}.clone(),
                categories: SETTINGS.categories(|_| true, false, true),
                selected_cat: cat,
                top_match: settings.first().map(|s| s.key().clone()).unwrap_or_default(),
                selected_settings: settings,
            })
        } else {
            // has search, just load everything
            let mut r = SETTINGS.get(|_, _| true, false);

            // apply search filter, get best match key (top_match), actual selected_cat.
            let mut top_match = (usize::MAX, Txt::from(""));
            let mut actual_cat = None;
            r.retain_mut(|(c, s)| {
                if c.id() == #{sel_cat} {
                    // is selected cat
                    actual_cat = Some(c.clone());
                    // actually filter settings
                    s.retain(|s| match s.search_index(#{clean_search}) {
                        Some(i) => {
                            if i < top_match.0 {
                                top_match = (i, s.key().clone());
                            }
                            true
                        }
                        None => false,
                    });
                    !s.is_empty()
                } else {
                    // is not selected cat, just search, settings will be ignored
                    s.iter().any(|s| match s.search_index(#{clean_search}) {
                        Some(i) => {
                            if i < top_match.0 {
                                top_match = (i, s.key().clone());
                            }
                            true
                        }
                        None => false,
                    })
                }
            });
            let mut r = SettingsEditorState {
                clean_search: #{clean_search}.clone(),
                categories: r.iter().map(|(c, _)| c.clone()).collect(),
                selected_cat: actual_cat.unwrap_or_else(|| Category::unknown(#{sel_cat}.clone())),
                selected_settings: r
                    .into_iter()
                    .find_map(|(c, s)| if c.id() == #{sel_cat} { Some(s) } else { None })
                    .unwrap_or_default(),
                top_match: top_match.1,
            };
            SETTINGS.sort_categories(&mut r.categories);
            SETTINGS.sort_settings(&mut r.selected_settings);
            Some(r)
        }
    };

    // select first category when previous selection is removed
    let sel = SETTINGS.editor_selected_category().current_context();
    let wk_sel_cat = sel.downgrade();
    fn correct_sel(options: &[Category], sel: &Var<CategoryId>) {
        if sel.with(|s| !options.iter().any(|c| c.id() == s))
            && let Some(first) = options.first()
        {
            sel.set(first.id().clone());
        }
    }
    r.hook(move |r| {
        if let Some(sel) = wk_sel_cat.upgrade() {
            correct_sel(&r.value().as_ref().unwrap().categories, &sel);
            true
        } else {
            false
        }
    })
    .perm();
    r.with(|r| {
        correct_sel(&r.as_ref().unwrap().categories, &sel);
    });

    r
}

fn settings_view_fn() -> UiNode {
    let search = SETTINGS_SEARCH_FN_VAR.get()(SettingsSearchArgs {});

    let editor_state = SETTINGS.editor_state().current_context();

    let categories = editor_state
        .map(|r| r.as_ref().unwrap().categories.clone())
        .present(wgt_fn!(|categories: Vec<Category>| {
            let cat_fn = CATEGORY_ITEM_FN_VAR.get();
            let categories: UiVec = categories
                .into_iter()
                .enumerate()
                .map(|(i, c)| cat_fn(CategoryItemArgs { index: i, category: c }))
                .collect();

            CATEGORIES_LIST_FN_VAR.get()(CategoriesListArgs { items: categories })
        }));

    let settings = editor_state.present(wgt_fn!(|state: Option<SettingsEditorState>| {
        let SettingsEditorState {
            selected_cat,
            selected_settings,
            ..
        } = state.unwrap();
        let setting_fn = SETTING_FN_VAR.get();

        let settings: UiVec = selected_settings
            .into_iter()
            .enumerate()
            .map(|(i, s)| {
                let editor = s.editor();
                setting_fn(SettingArgs {
                    index: i,
                    setting: s.clone(),
                    editor,
                })
            })
            .collect();

        let header = CATEGORY_HEADER_FN_VAR.get()(CategoryHeaderArgs { category: selected_cat });

        SETTINGS_FN_VAR.get()(SettingsArgs { header, items: settings })
    }));

    PANEL_FN_VAR.get()(PanelArgs {
        search,
        categories,
        settings,
    })
}

/// Save and restore settings search and selected category.
///
/// This property is enabled by default in the `SettingsEditor!` widget, without a key. Note that without a config key
/// this feature only actually enables if the settings widget ID has a name.
#[property(CONTEXT, widget_impl(SettingsEditor))]
pub fn save_state(child: impl IntoUiNode, enabled: impl IntoValue<SaveState>) -> UiNode {
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct SettingsEditorCfg {
        search: Txt,
        selected_category: CategoryId,
    }
    save_state_node::<SettingsEditorCfg>(
        child,
        enabled,
        |cfg| {
            let search = SETTINGS.editor_search();
            let cat = SETTINGS.editor_selected_category();
            WIDGET.sub_var(&search).sub_var(&cat);
            if let Some(c) = cfg {
                search.set(c.search);
                cat.set(c.selected_category);
            }
        },
        |required| {
            let search = SETTINGS.editor_search();
            let cat = SETTINGS.editor_selected_category();
            if required || search.is_new() || cat.is_new() {
                Some(SettingsEditorCfg {
                    search: search.get(),
                    selected_category: cat.get(),
                })
            } else {
                None
            }
        },
    )
}

/// Intrinsic SETTINGS_CMD handler.
fn command_handler(child: impl IntoUiNode) -> UiNode {
    let mut _handle = CommandHandle::dummy();
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            _handle = SETTINGS_CMD.scoped(WIDGET.id()).subscribe(true);
        }
        UiNodeOp::Deinit => {
            _handle = CommandHandle::dummy();
        }
        UiNodeOp::Event { update } => {
            c.event(update);

            if let Some(args) = SETTINGS_CMD.scoped(WIDGET.id()).on_unhandled(update) {
                args.propagation().stop();

                if let Some(id) = args.param::<CategoryId>() {
                    if SETTINGS
                        .editor_state()
                        .with(|s| s.as_ref().unwrap().categories.iter().any(|c| c.id() == id))
                    {
                        SETTINGS.editor_selected_category().set(id.clone());
                    }
                } else if let Some(key) = args.param::<Txt>() {
                    let search = if SETTINGS.any(|k, _| k == key) {
                        formatx!("@key:{key}")
                    } else {
                        key.clone()
                    };
                    SETTINGS.editor_search().set(search);
                } else if args.param.is_none() && !FOCUS.is_focus_within(WIDGET.id()).get() {
                    // focus top match
                    let s = Some(SETTINGS.editor_state().with(|s| s.as_ref().unwrap().top_match.clone()));
                    let info = WIDGET.info();
                    if let Some(w) = info.descendants().find(|w| w.setting_key() == s) {
                        FOCUS.focus_widget_or_enter(w.id(), false, false);
                    } else {
                        FOCUS.focus_widget_or_enter(info.id(), false, false);
                    }
                }
            }
        }
        _ => {}
    })
}

/// Set a [`SETTINGS_CMD`] handler that shows the settings window.
pub fn handle_settings_cmd() {
    use zng_app::{event::AnyEventArgs as _, window::WINDOW};

    SETTINGS_CMD
        .on_event(
            true,
            async_hn!(|args| {
                if args.propagation().is_stopped() || !SETTINGS.any(|_, _| true) {
                    return;
                }

                args.propagation().stop();

                let parent = WINDOWS.focused_window_id();

                let new_window = WINDOWS.focus_or_open("zng-config-settings-default", async move {
                    if let Some(p) = parent
                        && let Ok(p) = WINDOWS.vars(p)
                    {
                        let v = WINDOW.vars();
                        p.icon().set_bind(&v.icon()).perm();
                    }

                    Window! {
                        title = l10n!("window.title", "{$app} - Settings", app = zng_env::about().app.clone());
                        parent;
                        child = SettingsEditor! {
                            id = "zng-config-settings-default-editor";
                        };
                    }
                });

                if let Some(param) = &args.args.param {
                    if let Some(w) = new_window {
                        WINDOWS.wait_loaded(w.wait_rsp().await, true).await;
                    }
                    SETTINGS_CMD
                        .scoped("zng-config-settings-default-editor")
                        .notify_param(param.clone());
                }
            }),
        )
        .perm();
}
