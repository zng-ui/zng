use zng_app::{
    static_id,
    widget::{
        node::{BoxedUiNode, UiNode, UiNodeVec},
        property,
    },
};
use zng_ext_config::settings::{Category, CategoryId, Setting, SettingBuilder, SETTINGS};
use zng_ext_font::FontWeight;
use zng_var::{ContextInitHandle, ReadOnlyContextVar};
use zng_wgt::{node::with_context_var, prelude::*, WidgetFn, EDITORS};
use zng_wgt_container::Container;
use zng_wgt_filter::opacity;
use zng_wgt_markdown::Markdown;
use zng_wgt_rule_line::{hr::Hr, vr::Vr};
use zng_wgt_scroll::{Scroll, ScrollMode};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_style::Style;
use zng_wgt_text::{icon::Icon, Text};
use zng_wgt_text_input::TextInput;
use zng_wgt_toggle::{Selector, Toggle};
use zng_wgt_tooltip::{disabled_tooltip, tooltip, Tip};

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
        font_size = 1.5.em();
        zng_wgt::margin = 10;
    }
}

/// Default categories list view.
///
/// See [`CATEGORIES_LIST_FN_VAR`] for more details.
pub fn default_categories_list_fn(args: CategoriesListArgs) -> impl UiNode {
    Container! {
        child = Scroll! {
            mode = ScrollMode::VERTICAL;
            child_align = Align::FILL_TOP;
            padding = (10, 20);
            child = Stack! {
                zng_wgt_toggle::selector = Selector::single(args.selected);
                direction = StackDirection::top_to_bottom();
                children = args.items;
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
        };
        child_end = Vr!(zng_wgt::margin = 0), 0;
    }
}

/// Default setting item view.
pub fn default_setting_fn(args: SettingArgs) -> impl UiNode {
    let name = args.setting.name().clone();
    let description = args.setting.description().clone();
    let can_reset = args.setting.can_reset();
    Container! {
        child_start = {
            let s = args.setting;
            Icon! {
                zng_wgt::align = Align::TOP;
                zng_wgt::enabled = can_reset.clone();
                zng_wgt_input::gesture::on_click = hn!(|_| {
                    s.reset();
                });
                // !!: TODO, some kind of icon intermediary, Icon::Settings that gets fulfilled by the full API from an icon theme.
                ico = zng_wgt_text::icon::GlyphIcon { font: zng_ext_font::FontName::sans_serif(), features: zng_ext_font::font_features::FontFeatures::new(), glyph: zng_wgt_text::icon::GlyphSource::Code('R') };
                tooltip = Tip!(Text!("reset"));
                disabled_tooltip = Tip!(Text!("is default"));

                ico_size = 18;

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
pub fn default_settings_search_fn(args: SettingsSearchArgs) -> impl UiNode {
    Container! {
        child = TextInput! {
            txt = args.search;
        };
        child_bottom = Hr!(zng_wgt::margin = (10, 10, 0, 10)), 0;
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
    /// If an editor is set the [`EDITORS`] service is used to instantiate the editor.
    fn editor(&self) -> BoxedUiNode;
}

static_id! {
    static ref CUSTOM_EDITOR_ID: StateId<WidgetFn<Setting>>;
    static ref SETTING_ID: StateId<Setting>;
}

impl<'a> SettingBuilderEditorExt for SettingBuilder<'a> {
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
                EDITORS.get(self.value().clone_any())
            }),
        }
    }
}

/// Extends [`SETTINGS`] to provide contextual information in an editor.
pub trait SettingsCtxExt {
    /// Gets a context var that tracks the [`Setting`] entry the widget is inside, or will be.
    fn editor_setting(&self) -> ReadOnlyContextVar<Option<Setting>>;
}
impl SettingsCtxExt for SETTINGS {
    fn editor_setting(&self) -> ReadOnlyContextVar<Option<Setting>> {
        EDITOR_SETTING_VAR.read_only()
    }
}

context_var! {
    pub(crate) static EDITOR_SETTING_VAR: Option<Setting> = None;
}
