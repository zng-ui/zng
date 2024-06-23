use zng_app::{
    event::app_local,
    static_id,
    widget::node::{BoxedUiNode, NilUiNode, UiNode, UiNodeVec},
};
use zng_ext_config::settings::{Category, CategoryId, Setting, SettingBuilder, SETTINGS};
use zng_ext_font::FontWeight;
use zng_wgt::{
    prelude::{context_var, ArcVar, StateId, Txt},
    WidgetFn,
};
use zng_wgt_container::Container;
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::Text;
use zng_wgt_text_input::TextInput;
use zng_wgt_toggle::{CheckStyle, Selector, Toggle};

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
    fn editor(&mut self, editor: WidgetFn<Setting>) -> &mut Self;
}

/// Extends [`Setting`] to get custom editor metadata.
pub trait SettingEditorExt {
    /// Custom editor for the setting.
    fn editor(&self) -> Option<WidgetFn<Setting>>;
}

static_id! {
    static ref FOO_ID: StateId<WidgetFn<Setting>>;
}

impl<'a> SettingBuilderEditorExt for SettingBuilder<'a> {
    fn editor(&mut self, editor: WidgetFn<Setting>) -> &mut Self {
        self.with_meta(*FOO_ID, editor)
    }
}

impl SettingEditorExt for Setting {
    fn editor(&self) -> Option<WidgetFn<Setting>> {
        self.meta().get_clone(*FOO_ID)
    }
}

/// Extends [`SETTINGS`] to register setting editors.
pub trait SettingsEditorsExt {
    /// Register a settings editor handler.
    ///
    /// The `editor` function must return [`NilUiNode`] if it cannot handle the setting type.
    fn register_editor_fn(&self, editor: WidgetFn<Setting>);

    /// Make an editor for the setting.
    fn editor(&self, s: Setting) -> BoxedUiNode;
}
impl SettingsEditorsExt for SETTINGS {
    fn register_editor_fn(&self, editor: WidgetFn<Setting>) {
        SETTINGS_EDITORS.write().push(editor);
    }

    fn editor(&self, s: Setting) -> BoxedUiNode {
        match s.editor() {
            Some(e) => e(s),
            None => {
                let editors = SETTINGS_EDITORS.read();
                for editor in editors.iter().rev() {
                    let editor = editor(s.clone());
                    if !editor.is_nil() {
                        return editor;
                    }
                }
                NilUiNode.boxed()
            }
        }
    }
}

app_local! {
    static SETTINGS_EDITORS: Vec<WidgetFn<Setting>> = vec![WidgetFn::new(default_editor)];
}

fn default_editor(s: Setting) -> BoxedUiNode {
    if let Some(checked) = s.value_downcast::<bool>() {
        return Toggle! {
            checked;
            style_fn = CheckStyle!();
        }
        .boxed();
    } else if let Some(txt) = s.value_downcast::<Txt>() {
        return TextInput! {
            txt;
        }
        .boxed();
    }
    NilUiNode.boxed()
}
