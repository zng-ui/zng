use std::sync::Arc;

use zng_app::{
    static_id,
    widget::node::{BoxedUiNode, UiNode, UiNodeVec},
};
use zng_ext_config::settings::{Category, CategoryId, Setting, SettingBuilder};
use zng_ext_font::FontWeight;
use zng_wgt::{
    prelude::{context_var, ArcVar, OwnedStateMap, StateId, StateMapRef},
    WidgetFn, VAR_EDITOR,
};
use zng_wgt_container::Container;
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::Text;
use zng_wgt_toggle::{Selector, Toggle};

context_var! {
    /// Category in a category list.
    pub static CATEGORY_ITEM_VAR: WidgetFn<CategoryArgs> = WidgetFn::new(default_category_item_fn);
    /// Categories list.
    pub static CATEGORIES_LIST_VAR: WidgetFn<CategoriesListArgs> = WidgetFn::new(default_categories_list_fn);
    /// Category header on the settings list.
    pub static CATEGORY_HEADER_VAR: WidgetFn<CategoryArgs> = WidgetFn::new(default_category_item_fn);
    /// Setting item.
    pub static SETTING_VAR: WidgetFn<SettingArgs> = WidgetFn::new(default_setting_fn);
    /// Settings list for a category.
    pub static SETTINGS_VAR: WidgetFn<SettingsArgs> = WidgetFn::new(default_settings_fn);

}

/// Default category item view.
///
/// See [`CATEGORY_ITEM_VAR`] for more details.
pub fn default_category_item_fn(args: CategoryArgs) -> impl UiNode {
    Toggle! {
        child = Text!(args.category.name().clone());
        value::<CategoryId> = args.category.id().clone();
    }
}

/// Default categories list view.
///
/// See [`CATEGORIES_LIST_VAR`] for more details.
pub fn default_categories_list_fn(args: CategoriesListArgs) -> impl UiNode {
    Scroll!(
        VERTICAL,
        Stack! {
            zng_wgt_toggle::selector = Selector::single(args.selected);
            direction = StackDirection::top_to_bottom();
            children = args.items;
        }
    )
}

/// Default setting item view.
pub fn default_setting_fn(args: SettingArgs) -> impl UiNode {
    Container! {
        child_start = Text! {
            txt = args.setting.name().clone();
            font_weight = FontWeight::BOLD;
        }, 4;
        child_bottom = Text! {
            txt = args.setting.description().clone();
        }, 4;
        // !!: TODO, editor and reset
    }
}

/// Default settings for a category view.
pub fn default_settings_fn(args: SettingsArgs) -> impl UiNode {
    Container! {
        child_top = args.header, 5;
        child = Scroll! {
            mode = ScrollMode::VERTICAL;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                children = args.items;
            };
        };
    }
}

/// Arguments for a widget function that makes a category item for a categories list.
pub struct CategoryArgs {
    /// Index on the list.
    pub index: usize,
    /// The category.
    pub category: Category,
}

/// Arguments for a widget function that makes a list of category items that can be selected.
pub struct CategoriesListArgs {
    /// The item views.
    pub items: UiNodeVec,
    /// The selected item.
    pub selected: ArcVar<CategoryId>,
}

/// Arguments for a widget function that makes a setting container.
pub struct SettingArgs {
    /// The setting.
    pub setting: Setting,
    /// The setting value editor.
    pub editor: BoxedUiNode,
}

/// Arguments for a widget function that makes a settings for a category list.
pub struct SettingsArgs {
    /// The category header.
    pub header: BoxedUiNode,
    /// The items.
    pub items: UiNodeVec,
}

/// Extends [`SettingBuilder`] to set custom editor metadata.
pub trait SettingBuilderEditorExt {
    /// Custom editor for the setting.
    ///
    /// If an editor is set the `VAR_EDITOR` service is used to instantiate the editor.
    fn editor_fn(&mut self, editor: WidgetFn<Setting>) -> &mut Self;
}

/// Extends [`Setting`] to get custom editor metadata.
pub trait SettingEditorExt {
    /// Custom editor for the setting.
    fn editor_fn(&self) -> Option<WidgetFn<Setting>>;

    /// Instantiate editor.
    ///
    /// If an editor is set the [`VAR_EDITOR`] service is used to instantiate the editor.
    fn editor(&self) -> BoxedUiNode;
}

/// Extends [`StateMapRef<VAR_EDITOR>`] to provide the setting.
pub trait VarEditorSettingExt {
    /// Gets the setting that is requesting an editor.
    fn setting(&self) -> Option<&Setting>;
}

static_id! {
    static ref CUSTOM_EDITOR_ID: StateId<WidgetFn<Setting>>;
    static ref SETTING_ID: StateId<Setting>;
}

impl<'a> SettingBuilderEditorExt for SettingBuilder<'a> {
    fn editor_fn(&mut self, editor: WidgetFn<Setting>) -> &mut Self {
        self.with_meta(*CUSTOM_EDITOR_ID, editor)
    }
}

impl SettingEditorExt for Setting {
    fn editor_fn(&self) -> Option<WidgetFn<Setting>> {
        self.meta().get_clone(*CUSTOM_EDITOR_ID)
    }

    fn editor(&self) -> BoxedUiNode {
        match self.editor_fn() {
            Some(f) => f(self.clone()),
            None => {
                let mut meta = OwnedStateMap::new();
                meta.borrow_mut().set(*SETTING_ID, self.clone());
                VAR_EDITOR.new_with(self.value().clone_any(), Arc::new(meta))
            }
        }
    }
}

impl<'a> VarEditorSettingExt for StateMapRef<'a, VAR_EDITOR> {
    fn setting(&self) -> Option<&Setting> {
        self.get(*SETTING_ID)
    }
}
