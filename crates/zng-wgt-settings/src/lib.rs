#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//! Settings widgets.

zng_wgt::enable_widget_macros!();

mod view_fn;
pub use view_fn::*;

use zng_ext_config::settings::{Category, CategoryId, SETTINGS};
use zng_wgt::prelude::*;
use zng_wgt_container::Container;

/// Settings editor widget.
///
/// The editor
#[widget($crate::SettingsEditor)]
pub struct SettingsEditor(WidgetBase);
impl SettingsEditor {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            let child = settings_editor_node();
            wgt.set_child(child.boxed());
        });
    }
}

/// Implements the [`SettingsEditor!`] inner widgets.
///
/// [`SettingsEditor!`]: struct@SettingsEditor
pub fn settings_editor_node() -> impl UiNode {
    let search = var(Txt::from_static(""));
    let selected_cat = var(Txt::from_static(""));
    match_node(NilUiNode.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&SETTINGS_FN_VAR)
                .sub_var(&SETTING_FN_VAR)
                .sub_var(&SETTINGS_SEARCH_FN_VAR)
                .sub_var(&CATEGORIES_LIST_FN_VAR)
                .sub_var(&CATEGORY_HEADER_FN_VAR)
                .sub_var(&CATEGORY_ITEM_FN_VAR);
            *c.child() = settings_view_fn(search.clone(), selected_cat.clone()).boxed();
            search.set("");
            selected_cat.set("");
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
                *c.child() = settings_view_fn(search.clone(), selected_cat.clone()).boxed();
                c.child().init();
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

fn settings_view_fn(search: ArcVar<Txt>, selected_cat: ArcVar<CategoryId>) -> impl UiNode {
    let search_box = SETTINGS_SEARCH_FN_VAR.get()(SettingsSearchArgs { search: search.clone() });

    // avoids presenter rebuilds for ignored search
    let clean_search = search.map(|s| {
        let s = s.trim();
        if s.len() < 3 {
            Txt::from_static("")
        } else {
            Txt::from_str(s)
        }
    });

    let categories = presenter(
        clean_search.clone(),
        wgt_fn!(selected_cat, |search: Txt| {
            let categories = if search.is_empty() {
                SETTINGS.categories(|_| true, false, true)
            } else {
                todo!("!!: filter")
            };
            let cat_fn = CATEGORY_ITEM_FN_VAR.get();
            let categories: UiNodeVec = categories
                .into_iter()
                .enumerate()
                .map(|(i, c)| cat_fn(CategoryItemArgs { index: i, category: c }))
                .collect();

            CATEGORIES_LIST_FN_VAR.get()(CategoriesListArgs {
                items: categories,
                selected: selected_cat.clone(),
            })
        }),
    );

    let settings = presenter(
        expr_var!((#{selected_cat}.clone(), #{clean_search}.clone())),
        wgt_fn!(|(cat, search)| {
            let (category, settings) = SETTINGS
                .get(|_, c| c == &cat, true)
                .pop()
                .unwrap_or_else(|| (Category::unknown(cat), vec![]));

            let set_fn = SETTING_FN_VAR.get();

            let settings: UiNodeVec = settings
                .into_iter()
                .enumerate()
                .map(|(i, s)| {
                    let editor = s.editor();
                    set_fn(SettingArgs {
                        index: i,
                        setting: s,
                        editor,
                    })
                })
                .collect();

            let header = CATEGORY_HEADER_FN_VAR.get()(CategoryHeaderArgs { category });

            SETTINGS_FN_VAR.get()(SettingsArgs { header, items: settings })
        }),
    );

    Container! {
        child_start = {
            node: Container! {
                child_top = search_box, 5;
                child = categories;
            },
            spacing: 5,
        };
        child = settings;
    }
}
