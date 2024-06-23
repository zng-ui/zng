use std::sync::Arc;

use zng_app::{
    static_id,
    widget::{
        node::{BoxedUiNode, UiNode, UiNodeVec},
        property,
    },
};
use zng_ext_config::settings::{Category, CategoryId, Setting, SettingBuilder};
use zng_ext_font::FontWeight;
use zng_wgt::{node::with_context_var, prelude::*, WidgetFn, VAR_EDITOR};
use zng_wgt_button::Button;
use zng_wgt_container::Container;
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::Text;
use zng_wgt_text_input::TextInput;
use zng_wgt_toggle::{Selector, Toggle};

use crate::SettingsEditor;

context_var! {
    /// Category in a category list.
    pub static CATEGORY_ITEM_FN_VAR: WidgetFn<CategoryItemArgs> = WidgetFn::new(default_category_item_fn);
    /// Categories list.
    pub static CATEGORIES_LIST_FN_VAR: WidgetFn<CategoriesListArgs> = WidgetFn::new(default_categories_list_fn);
    /// Category header on the settings list.
    pub static CATEGORY_HEADER_FN_VAR: WidgetFn<CategoryHeaderArgs> = WidgetFn::new(default_category_header_fn);
    /// Setting item.
    pub static SETTING_FN_VAR: WidgetFn<SettingArgs> = WidgetFn::new(default_setting_fn);
    /// Settings list for a category.
    pub static SETTINGS_FN_VAR: WidgetFn<SettingsArgs> = WidgetFn::new(default_settings_fn);
    /// Settings search box.
    pub static SETTINGS_SEARCH_FN_VAR: WidgetFn<SettingsSearchArgs> = WidgetFn::new(default_settings_search_fn);
}

/// Widget function that converts [`CategoryItemArgs`] to a category item on a category list.
///
/// Sets the [`CATEGORY_ITEM_FN_VAR`].
#[property(CONTEXT, default(CATEGORY_ITEM_FN_VAR), widget_impl(SettingsEditor))]
pub fn category_item_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<CategoryItemArgs>>) -> impl UiNode {
    with_context_var(child, CATEGORY_ITEM_FN_VAR, wgt_fn)
}

/// Widget function that converts [`CategoriesListArgs`] to a category list.
///
/// Sets the [`CATEGORIES_LIST_FN_VAR`].
#[property(CONTEXT, default(CATEGORIES_LIST_FN_VAR), widget_impl(SettingsEditor))]
pub fn categories_list_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<CategoriesListArgs>>) -> impl UiNode {
    with_context_var(child, CATEGORIES_LIST_FN_VAR, wgt_fn)
}

/// Widget function that converts [`CategoryHeaderArgs`] to a category settings header.
///
/// Sets the [`CATEGORY_HEADER_FN_VAR`].
#[property(CONTEXT, default(CATEGORY_HEADER_FN_VAR), widget_impl(SettingsEditor))]
pub fn category_header_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<CategoryHeaderArgs>>) -> impl UiNode {
    with_context_var(child, CATEGORY_HEADER_FN_VAR, wgt_fn)
}

/// Widget function that converts [`SettingArgs`] to a setting editor entry on a settings list.
///
/// Sets the [`SETTING_FN_VAR`].
#[property(CONTEXT, default(SETTING_FN_VAR), widget_impl(SettingsEditor))]
pub fn setting_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<SettingArgs>>) -> impl UiNode {
    with_context_var(child, SETTING_FN_VAR, wgt_fn)
}

/// Widget function that converts [`SettingsArgs`] to a settings list.
///
/// Sets the [`SETTINGS_FN_VAR`].
#[property(CONTEXT, default(SETTINGS_FN_VAR), widget_impl(SettingsEditor))]
pub fn settings_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<SettingsArgs>>) -> impl UiNode {
    with_context_var(child, SETTINGS_FN_VAR, wgt_fn)
}

/// Widget function that converts [`SettingsSearchArgs`] to a search box.
///
/// Sets the [`SETTINGS_SEARCH_FN_VAR`].
#[property(CONTEXT, default(SETTINGS_SEARCH_FN_VAR), widget_impl(SettingsEditor))]
pub fn settings_search_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<SettingsSearchArgs>>) -> impl UiNode {
    with_context_var(child, SETTINGS_SEARCH_FN_VAR, wgt_fn)
}

/// Default category item view.
///
/// See [`CATEGORY_ITEM_FN_VAR`] for more details.
pub fn default_category_item_fn(args: CategoryItemArgs) -> impl UiNode {
    Toggle! {
        child = Text!(args.category.name().clone());
        value::<CategoryId> = args.category.id().clone();
    }
}

/// Default category item view.
///
/// See [`CATEGORY_HEADER_FN_VAR`] for more details.
pub fn default_category_header_fn(args: CategoryHeaderArgs) -> impl UiNode {
    Text! {
        txt = args.category.name().clone();
        font_size = 1.1.em();
    }
}

/// Default categories list view.
///
/// See [`CATEGORIES_LIST_FN_VAR`] for more details.
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
        child = args.editor;
        child_bottom = Text! {
            txt = args.setting.description().clone();
        }, 4;
        child_end = {
            let s = args.setting;
            Button! {
                zng_wgt::enabled = s.can_reset();
                on_click = hn!(|_| s.reset());
                child = Text!("R"); // !!: TODO
            }
        }, 4;
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

/// Default settings search box.
pub fn default_settings_search_fn(args: SettingsSearchArgs) -> impl UiNode {
    TextInput! {
        txt = args.search;
    }
}

/// Arguments for a widget function that makes a category item for a categories list.
pub struct CategoryItemArgs {
    /// Index on the list.
    pub index: usize,
    /// The category.
    pub category: Category,
}

/// Arguments for a widget function that makes a category header in a settings list.
pub struct CategoryHeaderArgs {
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
    /// Index of the setting on the list.
    pub index: usize,
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

/// Arguments for a search box widget.
pub struct SettingsSearchArgs {
    /// Search that matches setting name and descriptions.
    pub search: ArcVar<Txt>,
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
