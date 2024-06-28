#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//! Settings widgets.

zng_wgt::enable_widget_macros!();

mod view_fn;
pub use view_fn::*;

use zng_ext_config::{
    settings::{Category, CategoryId, Setting, SETTINGS},
    ConfigKey,
};
use zng_ext_window::{WINDOW_Ext as _, WINDOWS};
use zng_wgt::prelude::*;
use zng_wgt_container::Container;
use zng_wgt_input::cmd::SETTINGS_CMD;
use zng_wgt_window::{save_state_node, SaveState, Window};

/// Settings editor widget.
#[widget($crate::SettingsEditor)]
pub struct SettingsEditor(WidgetBase);
impl SettingsEditor {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            save_state = SaveState::enabled();
            zng_wgt_fill::background_color = color_scheme_pair((rgb(0.15, 0.15, 0.15), rgb(0.85, 0.85, 0.85)));
            zng_wgt_container::padding = 10;
        }
        self.widget_builder().push_build_action(|wgt| {
            wgt.set_child(settings_editor_node());
            wgt.push_intrinsic(NestGroup::CONTEXT, "editor-vars", |child| {
                let child = with_context_var(child, EDITOR_SEARCH_VAR, var(Txt::from("")));
                with_context_var(child, EDITOR_SELECTED_CATEGORY_VAR, var(CategoryId::from("")))
            });
        });
    }
}

/// Implements the [`SettingsEditor!`] inner widgets.
///
/// [`SettingsEditor!`]: struct@SettingsEditor
pub fn settings_editor_node() -> impl UiNode {
    match_node(NilUiNode.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&SETTINGS_FN_VAR)
                .sub_var(&SETTING_FN_VAR)
                .sub_var(&SETTINGS_SEARCH_FN_VAR)
                .sub_var(&CATEGORIES_LIST_FN_VAR)
                .sub_var(&CATEGORY_HEADER_FN_VAR)
                .sub_var(&CATEGORY_ITEM_FN_VAR);
            *c.child() = settings_view_fn().boxed();
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = NilUiNode.boxed();
        }
        UiNodeOp::Update { .. } => {
            if SETTINGS_FN_VAR.is_new()
                || SETTING_FN_VAR.is_new()
                || SETTINGS_SEARCH_FN_VAR.is_new()
                || CATEGORIES_LIST_FN_VAR.is_new()
                || CATEGORY_HEADER_FN_VAR.is_new()
                || CATEGORY_ITEM_FN_VAR.is_new()
            {
                c.delegated();
                c.child().deinit();
                *c.child() = settings_view_fn().boxed();
                c.child().init();
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

fn settings_view_fn() -> impl UiNode {
    let search_box = SETTINGS_SEARCH_FN_VAR.get()(SettingsSearchArgs {});

    // avoids rebuilds for ignored search changes
    let clean_search = SETTINGS.editor_search().actual_var().map(|s| {
        let s = s.trim();
        if !s.starts_with('@') {
            s.to_lowercase().into()
        } else {
            Txt::from_str(s)
        }
    });

    // live query
    #[derive(PartialEq, Debug, Clone)]
    struct Results {
        categories: Vec<Category>,
        selected_cat: Category,
        selected_settings: Vec<Setting>,
        top_match: ConfigKey,
    }
    let sel_cat = SETTINGS.editor_selected_category().actual_var().clone();
    let search_results = expr_var! {
        if #{clean_search}.is_empty() {
            // no search, does not need to load settings of other categories
            let (cat, settings) = SETTINGS.get(|_, cat| cat == #{sel_cat}, true)
                            .pop().unwrap_or_else(|| (Category::unknown(#{sel_cat}.clone()), vec![]));
            Results {
                categories: SETTINGS.categories(|_| true, false, true),
                selected_cat: cat,
                top_match: settings.first().map(|s| s.key().clone()).unwrap_or_default(),
                selected_settings: settings,
            }
        } else {
            // has search, just load everything
            let mut r = SETTINGS.get(|_, _| true, false);

            // apply search filter, get best match key (top_match), actual selected_cat.
            let mut top_match = (usize::MAX, Txt::from(""));
            let mut actual_cat = None;
            r.retain_mut(|(c, s)| if c.id() == #{sel_cat} {
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
            });
            let mut r = Results {
                categories: r.iter().map(|(c, _)| c.clone()).collect(),
                selected_cat: actual_cat.unwrap_or_else(|| Category::unknown(#{sel_cat}.clone())),
                selected_settings: r.into_iter().find_map(|(c, s)| if c.id() == #{sel_cat} { Some(s) } else { None }).unwrap_or_default(),
                top_match: top_match.1,
            };
            SETTINGS.sort_categories(&mut r.categories);
            SETTINGS.sort_settings(&mut r.selected_settings);
            r
        }
    };

    // select first category when previous selection is removed
    let sel = SETTINGS.editor_selected_category().actual_var();
    let wk_sel_cat = sel.downgrade();
    fn correct_sel(options: &[Category], sel: &BoxedVar<CategoryId>) {
        if sel.with(|s| !options.iter().any(|c| c.id() == s)) {
            if let Some(first) = options.first() {
                let _ = sel.set(first.id().clone());
            }
        }
    }
    search_results
        .hook(move |r| {
            if let Some(sel) = wk_sel_cat.upgrade() {
                correct_sel(&r.value().categories, &sel);
                true
            } else {
                false
            }
        })
        .perm();
    search_results.with(|r| {
        correct_sel(&r.categories, &sel);
    });

    let categories = presenter(
        search_results.map_ref(|r| &r.categories),
        wgt_fn!(|categories: Vec<Category>| {
            let cat_fn = CATEGORY_ITEM_FN_VAR.get();
            let categories: UiNodeVec = categories
                .into_iter()
                .enumerate()
                .map(|(i, c)| cat_fn(CategoryItemArgs { index: i, category: c }))
                .collect();

            CATEGORIES_LIST_FN_VAR.get()(CategoriesListArgs { items: categories })
        }),
    );

    let settings = presenter(
        search_results,
        wgt_fn!(|Results {
                     selected_cat,
                     selected_settings,
                     ..
                 }: Results| {
            let setting_fn = SETTING_FN_VAR.get();

            let settings: UiNodeVec = selected_settings
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
        }),
    );

    Container! {
        child_top = search_box, 0;
        child = Container! {
            child_start = categories, 0;
            child = settings
        };
    }
}

/// Save and restore settings search and selected category.
///
/// This property is enabled by default in the `SettingsEditor!` widget, without a key. Note that without a config key
/// this feature only actually enables if the settings widget ID has a name.
#[property(CONTEXT, widget_impl(SettingsEditor))]
pub fn save_state(child: impl UiNode, enabled: impl IntoValue<SaveState>) -> impl UiNode {
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
                let _ = search.set(c.search);
                let _ = cat.set(c.selected_category);
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

/// Set a [`SETTINGS_CMD`] handler that shows the settings window.
pub fn handle_settings_cmd() {
    use zng_app::{
        app_hn,
        event::AnyEventArgs as _,
        window::{WindowId, WINDOW},
    };

    let id = WindowId::named("zng-config-settings-default");
    SETTINGS_CMD
        .on_event(
            true,
            app_hn!(|args: &zng_app::event::AppCommandArgs, _| {
                if args.propagation().is_stopped() || !SETTINGS.any(|_, _| true) {
                    return;
                }

                args.propagation().stop();

                let parent = WINDOWS.focused_window_id();

                WINDOWS.focus_or_open(id, async move {
                    if let Some(p) = parent {
                        if let Ok(p) = WINDOWS.vars(p) {
                            let v = WINDOW.vars();
                            p.icon().set_bind(&v.icon()).perm();
                        }
                    }

                    Window! {
                        title = formatx!("{} - Settings", zng_env::about().app);
                        parent;
                        child = SettingsEditor! {
                            id = "zng-config-settings-default-editor";
                        };
                    }
                });
            }),
        )
        .perm();
}
