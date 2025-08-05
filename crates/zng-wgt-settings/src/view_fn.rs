use zng_app::{
    static_id,
    widget::{
        node::{BoxedUiNode, UiNode, UiVec},
        property,
    },
};
use zng_ext_config::{
    ConfigKey,
    settings::{Category, CategoryId, SETTINGS, Setting, SettingBuilder},
};
use zng_ext_font::FontWeight;
use zng_ext_l10n::l10n;
use zng_var::ContextInitHandle;
use zng_wgt::{EDITORS, ICONS, Wgt, WidgetFn, node::with_context_var, prelude::*};
use zng_wgt_container::Container;
use zng_wgt_filter::opacity;
use zng_wgt_markdown::Markdown;
use zng_wgt_rule_line::{hr::Hr, vr::Vr};
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_style::Style;
use zng_wgt_text::Text;
use zng_wgt_text_input::TextInput;
use zng_wgt_toggle::{Selector, Toggle};
use zng_wgt_tooltip::{Tip, disabled_tooltip, tooltip};

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
    /// Settings search area.
    pub static SETTINGS_SEARCH_FN_VAR: WidgetFn<SettingsSearchArgs> = WidgetFn::new(default_settings_search_fn);
    /// Editor layout.
    pub static PANEL_FN_VAR: WidgetFn<PanelArgs> = WidgetFn::new(default_panel_fn);
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
/// Note that the widget must set [`setting`] or some features will not work.
///
/// Sets the [`SETTING_FN_VAR`].
///
/// [`setting`]: fn@setting
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

/// Widget that defines the editor layout, bringing together the other component widgets.
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(SettingsEditor))]
pub fn panel_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<PanelArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_FN_VAR, wgt_fn)
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
        font_size = 1.5.em();
        zng_wgt::margin = (10, 10, 10, 28);
    }
}

/// Default categories list view on `actual_width > 400`.
///
/// See [`CATEGORIES_LIST_FN_VAR`] for more details.
pub fn default_categories_list_fn(args: CategoriesListArgs) -> impl UiNode {
    Container! {
        child = categories_list(args.items.boxed());
        child_end = Vr!(zng_wgt::margin = 0), 0;
    }
}
fn categories_list(items: BoxedUiNodeList) -> impl UiNode {
    let list = Stack! {
        zng_wgt_toggle::selector = Selector::single(SETTINGS.editor_selected_category());
        direction = StackDirection::top_to_bottom();
        children = items;
        zng_wgt_toggle::style_fn = Style! {
            replace = true;
            opacity = 70.pct();
            zng_wgt_size_offset::height = 2.em();
            zng_wgt_container::child_align = Align::START;
            zng_wgt_input::cursor = zng_wgt_input::CursorIcon::Pointer;

            when *#zng_wgt_input::is_cap_hovered {
                zng_wgt_text::font_weight = FontWeight::MEDIUM;
            }

            when *#zng_wgt_toggle::is_checked {
                zng_wgt_text::font_weight = FontWeight::BOLD;
                opacity = 100.pct();
            }
        };
    };
    Scroll! {
        mode = ScrollMode::VERTICAL;
        child_align = Align::FILL_TOP;
        padding = (10, 20);
        child = list;
    }
}

/// Default categories list view on `actual_width <= 400`.
pub fn default_categories_list_mobile_fn(args: CategoriesListArgs) -> impl UiNode {
    let items = ArcNodeList::new(args.items);
    Toggle! {
        zng_wgt::margin = 4;
        style_fn = zng_wgt_toggle::ComboStyle!();
        child = Text! {
            txt = SETTINGS
                .editor_state()
                .flat_map(|e| e.as_ref().unwrap().selected_cat.name().clone());
            font_weight = FontWeight::BOLD;
            zng_wgt_container::padding = 5;
        };
        checked_popup = wgt_fn!(|_| zng_wgt_layer::popup::Popup! {
            child = categories_list(items.take_on_init().boxed());
        });
    }
}

/// Default setting item view.
pub fn default_setting_fn(args: SettingArgs) -> impl UiNode {
    let name = args.setting.name().clone();
    let description = args.setting.description().clone();
    let can_reset = args.setting.can_reset();
    Container! {
        setting = args.setting.clone();

        zng_wgt_input::focus::focus_scope = true;
        zng_wgt_input::focus::focus_scope_behavior = zng_ext_input::focus::FocusScopeOnFocus::FirstDescendant;

        child_start = {
            let s = args.setting;
            Wgt! {
                zng_wgt::align = Align::TOP;
                zng_wgt::visibility = can_reset.map(|c| match c {
                    true => Visibility::Visible,
                    false => Visibility::Hidden,
                });
                zng_wgt_input::gesture::on_click = hn!(|_| {
                    s.reset();
                });

                zng_wgt_fill::background = ICONS.req_or(["settings-reset", "settings-backup-restore"], || Text!("R"));
                zng_wgt_size_offset::size = 18;

                tooltip = Tip!(Text!("reset"));
                disabled_tooltip = Tip!(Text!("is default"));

                zng_wgt_input::focus::tab_index = zng_ext_input::focus::TabIndex::SKIP;

                opacity = 70.pct();
                when *#zng_wgt_input::is_cap_hovered {
                    opacity = 100.pct();
                }
                when *#zng_wgt::is_disabled {
                    opacity = 30.pct();
                }
            }
        }, 4;
        child_top = Container! {
            child_top = Text! {
                txt = name;
                font_weight = FontWeight::BOLD;
            }, 4;
            child = Markdown! {
                txt = description;
                opacity = 70.pct();
            };
        }, 5;
        child = args.editor;
    }
}

/// Default settings for a category view.
pub fn default_settings_fn(args: SettingsArgs) -> impl UiNode {
    Container! {
        child_top = args.header, 5;
        child = Scroll! {
            mode = ScrollMode::VERTICAL;
            padding = (0, 20, 20, 10);
            child_align = Align::FILL_TOP;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                spacing = 10;
                children = args.items;
            };
        };
    }
}

/// Default settings search box.
pub fn default_settings_search_fn(_: SettingsSearchArgs) -> impl UiNode {
    Container! {
        child = TextInput! {
            txt = SETTINGS.editor_search();
            style_fn = zng_wgt_text_input::SearchStyle!();
            zng_wgt_input::focus::focus_shortcut = [shortcut![CTRL+'F'], shortcut![Find]];
            placeholder_txt = l10n!("search.placeholder", "search settings ({$shortcut})", shortcut = "Ctrl+F");
        };
        child_bottom = Hr!(zng_wgt::margin = (10, 10, 0, 10)), 0;
    }
}

/// Default editor layout on `actual_width > 400`.
pub fn default_panel_fn(args: PanelArgs) -> impl UiNode {
    Container! {
        child_top = args.search, 0;
        child = Container! {
            child_start = args.categories, 0;
            child = args.settings;
        };
    }
}

/// Default editor layout on `actual_width <= 400`.
pub fn default_panel_mobile_fn(args: PanelArgs) -> impl UiNode {
    Container! {
        child_top = args.search, 0;
        child = Container! {
            child_top = args.categories, 0;
            child = args.settings;
        };
    }
}

/// Arguments for a widget function that makes a category item for a categories list.
#[non_exhaustive]
pub struct CategoryItemArgs {
    /// Index on the list.
    pub index: usize,
    /// The category.
    pub category: Category,
}
impl CategoryItemArgs {
    /// New args.
    pub fn new(index: usize, category: Category) -> Self {
        Self { index, category }
    }
}

/// Arguments for a widget function that makes a category header in a settings list.
#[non_exhaustive]
pub struct CategoryHeaderArgs {
    /// The category.
    pub category: Category,
}
impl CategoryHeaderArgs {
    /// New args.
    pub fn new(category: Category) -> Self {
        Self { category }
    }
}

/// Arguments for a widget function that makes a list of category items that can be selected.
///
/// The selected category variable is in [`SETTINGS.editor_selected_category`](SettingsCtxExt::editor_selected_category).
#[non_exhaustive]
pub struct CategoriesListArgs {
    /// The item views.
    pub items: UiVec,
}
impl CategoriesListArgs {
    /// New args.
    pub fn new(items: UiVec) -> Self {
        Self { items }
    }
}

/// Arguments for a widget function that makes a setting container.
#[non_exhaustive]
pub struct SettingArgs {
    /// Index of the setting on the list.
    pub index: usize,
    /// The setting.
    pub setting: Setting,
    /// The setting value editor.
    pub editor: BoxedUiNode,
}
impl SettingArgs {
    /// New args.
    pub fn new(index: usize, setting: Setting, editor: BoxedUiNode) -> Self {
        Self { index, setting, editor }
    }
}

/// Arguments for a widget function that makes a settings for a category list.
#[non_exhaustive]
pub struct SettingsArgs {
    /// The category header.
    pub header: BoxedUiNode,
    /// The items.
    pub items: UiVec,
}
impl SettingsArgs {
    /// New args.
    pub fn new(header: BoxedUiNode, items: UiVec) -> Self {
        Self { header, items }
    }
}

/// Arguments for a search box widget.
///
/// The search variable is in [`SETTINGS.editor_search`](SettingsCtxExt::editor_search).
#[derive(Default)]
#[non_exhaustive]
pub struct SettingsSearchArgs {}

/// Arguments for the entire settings editor layout.
#[non_exhaustive]
pub struct PanelArgs {
    /// Search box widget.
    pub search: BoxedUiNode,
    /// Categories widget.
    pub categories: BoxedUiNode,
    /// Settings widget.
    pub settings: BoxedUiNode,
}
impl PanelArgs {
    /// New args.
    pub fn new(search: BoxedUiNode, categories: BoxedUiNode, settings: BoxedUiNode) -> Self {
        Self {
            search,
            categories,
            settings,
        }
    }
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
    /// If an editor is set the [`EDITORS`] service is used to instantiate the editor.
    fn editor(&self) -> BoxedUiNode;
}

/// Extends [`WidgetInfo`] to provide the setting config key for setting widgets.
pub trait WidgetInfoSettingExt {
    /// Gets the setting config key, if this widget represents a setting item.
    fn setting_key(&self) -> Option<ConfigKey>;
}

static_id! {
    static ref CUSTOM_EDITOR_ID: StateId<WidgetFn<Setting>>;
    static ref SETTING_KEY_ID: StateId<ConfigKey>;
}

impl SettingBuilderEditorExt for SettingBuilder<'_> {
    fn editor_fn(&mut self, editor: WidgetFn<Setting>) -> &mut Self {
        self.set(*CUSTOM_EDITOR_ID, editor)
    }
}

impl SettingEditorExt for Setting {
    fn editor_fn(&self) -> Option<WidgetFn<Setting>> {
        self.meta().get_clone(*CUSTOM_EDITOR_ID)
    }

    fn editor(&self) -> BoxedUiNode {
        match self.editor_fn() {
            Some(f) => f(self.clone()),
            None => EDITOR_SETTING_VAR.with_context_var(ContextInitHandle::current(), Some(self.clone()), || {
                EDITORS.get(self.value().clone())
            }),
        }
    }
}

impl WidgetInfoSettingExt for WidgetInfo {
    fn setting_key(&self) -> Option<ConfigKey> {
        self.meta().get_clone(*SETTING_KEY_ID)
    }
}

/// Extends [`SETTINGS`] to provide contextual information in an editor.
pub trait SettingsCtxExt {
    /// Gets a read-write context var that tracks the search text.
    fn editor_search(&self) -> ContextVar<Txt>;

    /// Gets a read-write context var that tracks the selected category.
    fn editor_selected_category(&self) -> ContextVar<CategoryId>;

    /// Gets a read-only context var that tracks the current editor data state.
    fn editor_state(&self) -> Var<Option<SettingsEditorState>>;

    /// Gets a read-only context var that tracks the [`Setting`] entry the widget is inside, or will be.
    fn editor_setting(&self) -> Var<Option<Setting>>;
}
impl SettingsCtxExt for SETTINGS {
    fn editor_search(&self) -> ContextVar<Txt> {
        EDITOR_SEARCH_VAR
    }

    fn editor_selected_category(&self) -> ContextVar<CategoryId> {
        EDITOR_SELECTED_CATEGORY_VAR
    }

    fn editor_state(&self) -> Var<Option<SettingsEditorState>> {
        EDITOR_STATE_VAR.read_only()
    }

    fn editor_setting(&self) -> Var<Option<Setting>> {
        EDITOR_SETTING_VAR.read_only()
    }
}

context_var! {
    pub(crate) static EDITOR_SEARCH_VAR: Txt = Txt::from_static("");
    pub(crate) static EDITOR_SELECTED_CATEGORY_VAR: CategoryId = CategoryId(Txt::from_static(""));
    pub(crate) static EDITOR_STATE_VAR: Option<SettingsEditorState> = None;
    static EDITOR_SETTING_VAR: Option<Setting> = None;
}

/// Identifies the [`setting_fn`] widget.
///
/// [`setting_fn`]: fn@setting_fn
#[property(CONTEXT)]
pub fn setting(child: impl UiNode, setting: impl IntoValue<Setting>) -> impl UiNode {
    let setting = setting.into();

    let child = match_node(child, |_, op| {
        if let UiNodeOp::Info { info } = op {
            info.set_meta(*SETTING_KEY_ID, EDITOR_SETTING_VAR.with(|s| s.as_ref().unwrap().key().clone()));
        }
    });
    with_context_var(child, EDITOR_SETTING_VAR, Some(setting))
}

/// Represents the current settings data.
///
/// Use [`SETTINGS.editor_state`] to get.
///
/// [`SETTINGS.editor_state`]: SettingsCtxExt::editor_state
#[derive(PartialEq, Debug, Clone)]
#[non_exhaustive]
pub struct SettingsEditorState {
    /// The actual text searched.
    pub clean_search: Txt,
    /// Categories list.
    pub categories: Vec<Category>,
    /// Selected category.
    pub selected_cat: Category,
    /// Settings for the selected category that match the search.
    pub selected_settings: Vec<Setting>,
    /// Top search match.
    pub top_match: ConfigKey,
}
