//! Settings are the config the user can directly edit, this module implements a basic settings data model.
//!
//! The settings editor widget is not implemented here, this module bridges config implementers with settings UI implementers.

use core::fmt;
use std::{any::TypeId, cmp::Ordering, mem, ops, sync::Arc};

use zng_app_context::app_local;
use zng_state_map::{OwnedStateMap, StateId, StateMapMut, StateMapRef, StateValue};
use zng_txt::Txt;
use zng_var::{AnyVar, AnyVarHookArgs, BoxAnyVarValue, IntoVar, Var, const_var, impl_from_and_into_var, var};

use crate::{CONFIG, Config, ConfigKey, ConfigValue, FallbackConfigReset};

/// Settings metadata service.
pub struct SETTINGS;

impl SETTINGS {
    /// Register a closure that provides settings metadata.
    ///
    /// The closure can be called multiple times.
    pub fn register(&self, f: impl Fn(&mut SettingsBuilder) + Send + Sync + 'static) {
        SETTINGS_SV.write().sources.push(Box::new(f))
    }

    /// Register a closure that provides category metadata.
    ///
    /// The closure can be called multiple times.
    pub fn register_categories(&self, f: impl Fn(&mut CategoriesBuilder) + Send + Sync + 'static) {
        SETTINGS_SV.write().sources_cat.push(Box::new(f))
    }

    /// Select and sort settings matched by `filter`.
    ///
    /// This calls all registered closures that are not excluded by `filter`.
    pub fn get(&self, mut filter: impl FnMut(&ConfigKey, &CategoryId) -> bool, sort: bool) -> Vec<(Category, Vec<Setting>)> {
        self.get_impl(&mut filter, sort)
    }

    fn get_impl(&self, filter: &mut dyn FnMut(&ConfigKey, &CategoryId) -> bool, sort: bool) -> Vec<(Category, Vec<Setting>)> {
        let sv = SETTINGS_SV.read();

        let mut settings = SettingsBuilder { settings: vec![], filter };
        for source in sv.sources.iter() {
            source(&mut settings);
        }
        let settings = settings.settings;

        let mut categories = CategoriesBuilder {
            categories: vec![],
            filter: &mut |cat| settings.iter().any(|s| &s.category == cat),
        };
        for source in sv.sources_cat.iter() {
            source(&mut categories);
        }
        let categories = categories.categories;

        let mut result: Vec<_> = categories.into_iter().map(|c| (c, vec![])).collect();
        for s in settings {
            if let Some(i) = result.iter().position(|(c, _)| c.id == s.category) {
                result[i].1.push(s);
            } else {
                tracing::warn!("missing category metadata for {}", s.category);
                result.push((
                    Category {
                        id: s.category.clone(),
                        order: u16::MAX,
                        name: const_var(s.category.0.clone()),
                        meta: Arc::new(OwnedStateMap::new()),
                    },
                    vec![s],
                ));
            }
        }

        if sort {
            self.sort(&mut result);
        }
        result
    }

    /// Gets if there are any setting matched by `filter`.
    pub fn any(&self, mut filter: impl FnMut(&ConfigKey, &CategoryId) -> bool) -> bool {
        self.any_impl(&mut filter)
    }
    fn any_impl(&self, filter: &mut dyn FnMut(&ConfigKey, &CategoryId) -> bool) -> bool {
        let sv = SETTINGS_SV.read();

        let mut any = false;

        for source in sv.sources.iter() {
            source(&mut SettingsBuilder {
                settings: vec![],
                filter: &mut |k, i| {
                    if filter(k, i) {
                        any = true;
                    }
                    false
                },
            });
            if any {
                break;
            }
        }

        any
    }

    /// Count how many settings match the `filter`.
    pub fn count(&self, mut filter: impl FnMut(&ConfigKey, &CategoryId) -> bool) -> usize {
        self.count_impl(&mut filter)
    }
    fn count_impl(&self, filter: &mut dyn FnMut(&ConfigKey, &CategoryId) -> bool) -> usize {
        let sv = SETTINGS_SV.read();

        let mut count = 0;

        for source in sv.sources.iter() {
            source(&mut SettingsBuilder {
                settings: vec![],
                filter: &mut |k, i| {
                    if filter(k, i) {
                        count += 1;
                    }
                    false
                },
            });
        }

        count
    }

    /// Select and sort categories matched by `filter`.
    ///
    /// If `include_empty` is `true` includes categories that have no settings.
    pub fn categories(&self, mut filter: impl FnMut(&CategoryId) -> bool, include_empty: bool, sort: bool) -> Vec<Category> {
        self.categories_impl(&mut filter, include_empty, sort)
    }
    fn categories_impl(&self, filter: &mut dyn FnMut(&CategoryId) -> bool, include_empty: bool, sort: bool) -> Vec<Category> {
        let sv = SETTINGS_SV.read();

        let mut categories = CategoriesBuilder {
            categories: vec![],
            filter,
        };
        for source in sv.sources_cat.iter() {
            source(&mut categories);
        }
        let mut result = categories.categories;

        if !include_empty {
            let mut non_empty = vec![];
            for source in sv.sources.iter() {
                source(&mut SettingsBuilder {
                    settings: vec![],
                    filter: &mut |_, cat| {
                        if !non_empty.contains(cat) {
                            non_empty.push(cat.clone());
                        }
                        false
                    },
                });
            }

            result.retain(|c| {
                if let Some(i) = non_empty.iter().position(|id| &c.id == id) {
                    non_empty.swap_remove(i);
                    true
                } else {
                    false
                }
            });

            for missing in non_empty {
                tracing::warn!("missing category metadata for {}", missing);
                result.push(Category::unknown(missing));
            }
        }

        if sort {
            self.sort_categories(&mut result)
        }

        result
    }

    /// Sort `settings`.
    pub fn sort_settings(&self, settings: &mut [Setting]) {
        settings.sort_by(|a, b| {
            let c = a.order.cmp(&b.order);
            if matches!(c, Ordering::Equal) {
                return a.name.with(|a| b.name.with(|b| a.cmp(b)));
            }
            c
        });
    }

    /// Sort `categories`.
    pub fn sort_categories(&self, categories: &mut [Category]) {
        categories.sort_by(|a, b| {
            let c = a.order.cmp(&b.order);
            if matches!(c, Ordering::Equal) {
                return a.name.with(|a| b.name.with(|b| a.cmp(b)));
            }
            c
        });
    }

    /// Sort categories and settings.
    pub fn sort(&self, settings: &mut [(Category, Vec<Setting>)]) {
        settings.sort_by(|a, b| {
            let c = a.0.order.cmp(&b.0.order);
            if matches!(c, Ordering::Equal) {
                return a.0.name.with(|a| b.0.name.with(|b| a.cmp(b)));
            }
            c
        });
        for (_, s) in settings {
            self.sort_settings(s);
        }
    }
}

/// Unique ID of a [`Category`].
#[derive(PartialEq, Eq, Clone, Debug, Hash, Default, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CategoryId(pub Txt);
impl_from_and_into_var! {
    fn from(id: Txt) -> CategoryId {
        CategoryId(id)
    }
    fn from(id: String) -> CategoryId {
        CategoryId(id.into())
    }
    fn from(id: &'static str) -> CategoryId {
        CategoryId(id.into())
    }
}
impl ops::Deref for CategoryId {
    type Target = Txt;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl fmt::Display for CategoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Settings category.
#[derive(Clone)]
pub struct Category {
    id: CategoryId,
    order: u16,
    name: Var<Txt>,
    meta: Arc<OwnedStateMap<Category>>,
}
impl Category {
    /// Unique ID.
    pub fn id(&self) -> &CategoryId {
        &self.id
    }

    /// Position of the category in a list of categories.
    ///
    /// Lower numbers are listed first, two categories with the same order are sorted by display name.
    pub fn order(&self) -> u16 {
        self.order
    }

    /// Display name.
    pub fn name(&self) -> &Var<Txt> {
        &self.name
    }

    /// Custom category metadata.
    pub fn meta(&self) -> StateMapRef<'_, Category> {
        self.meta.borrow()
    }

    /// Category from an ID only, no other metadata.
    pub fn unknown(missing: CategoryId) -> Self {
        Self {
            id: missing.clone(),
            order: u16::MAX,
            name: const_var(missing.0),
            meta: Arc::default(),
        }
    }
}
impl PartialEq for Category {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for Category {}
impl fmt::Debug for Category {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Category").field("id", &self.id).finish_non_exhaustive()
    }
}

#[cfg(test)]
fn _setting_in_var(s: Setting) {
    let _x = const_var(s).get();
}

/// Setting entry.
pub struct Setting {
    key: ConfigKey,
    order: u16,
    name: Var<Txt>,
    description: Var<Txt>,
    category: CategoryId,
    meta: Arc<OwnedStateMap<Setting>>,
    value: AnyVar,
    value_type: TypeId,
    reset: Arc<dyn SettingReset>,
}
impl Clone for Setting {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            order: self.order,
            name: self.name.clone(),
            description: self.description.clone(),
            category: self.category.clone(),
            meta: self.meta.clone(),
            value: self.value.clone(),
            value_type: self.value_type,
            reset: self.reset.clone(),
        }
    }
}
impl Setting {
    /// The config edited by this setting.
    pub fn key(&self) -> &ConfigKey {
        &self.key
    }

    /// Position of the setting in a list of settings.
    ///
    /// Lower numbers are listed first, two settings with the same order are sorted by display name.
    pub fn order(&self) -> u16 {
        self.order
    }

    /// Display name.
    pub fn name(&self) -> &Var<Txt> {
        &self.name
    }
    /// Short help text.
    pub fn description(&self) -> &Var<Txt> {
        &self.description
    }
    /// Settings category.
    pub fn category(&self) -> &CategoryId {
        &self.category
    }

    /// Custom setting metadata.
    pub fn meta(&self) -> StateMapRef<'_, Setting> {
        self.meta.borrow()
    }

    /// If the `value` is set to an actual config variable.
    ///
    /// Setting builders can skip setting the value, this can indicate the config should be edited directly.
    pub fn value_is_set(&self) -> bool {
        self.value_type != TypeId::of::<SettingValueNotSet>()
    }

    /// Config value.
    pub fn value(&self) -> &AnyVar {
        &self.value
    }

    /// Config value type.
    pub fn value_type(&self) -> TypeId {
        self.value_type
    }

    /// Config value, strongly typed.
    pub fn value_downcast<T: ConfigValue>(&self) -> Option<Var<T>> {
        if self.value_type == std::any::TypeId::of::<T>() {
            let v = self.value.clone().downcast::<T>().unwrap_or_else(|_| panic!());
            Some(v)
        } else {
            None
        }
    }

    /// Gets a variable that indicates the current setting value is not the default.
    pub fn can_reset(&self) -> Var<bool> {
        self.reset.can_reset(&self.key, &self.value)
    }

    /// Reset the setting value.
    pub fn reset(&self) {
        self.reset.reset(&self.key, &self.value);
    }

    /// Gets if the setting should be included in the search and how likely it is to be an exact match (0 is exact).
    ///
    /// If `search` starts with `@key:` matches key case sensitive, otherwise matches name or description in lower case. Note
    /// that non-key search is expected to already be lowercase.
    pub fn search_index(&self, search: &str) -> Option<usize> {
        if let Some(key) = search.strip_prefix("@key:") {
            return if self.key.contains(key) {
                Some(self.key.len() - search.len())
            } else {
                None
            };
        }

        let r = self.name.with(|s| {
            let s = s.to_lowercase();
            if s.contains(search) { Some(s.len() - search.len()) } else { None }
        });
        if r.is_some() {
            return r;
        }

        self.description.with(|s| {
            let s = s.to_lowercase();
            if s.contains(search) {
                Some(s.len() - search.len() + usize::MAX / 2)
            } else {
                None
            }
        })
    }
}
impl PartialEq for Setting {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Setting {}
impl fmt::Debug for Setting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Setting").field("key", &self.key).finish_non_exhaustive()
    }
}

app_local! {
    static SETTINGS_SV: SettingsService = SettingsService {
        sources: vec![],
        sources_cat: vec![],
    };
}
struct SettingsService {
    sources: Vec<Box<dyn Fn(&mut SettingsBuilder) + Send + Sync + 'static>>,
    sources_cat: Vec<Box<dyn Fn(&mut CategoriesBuilder) + Send + Sync + 'static>>,
}

/// Settings builder.
pub struct SettingsBuilder<'a> {
    settings: Vec<Setting>,
    filter: &'a mut dyn FnMut(&ConfigKey, &CategoryId) -> bool,
}
impl SettingsBuilder<'_> {
    /// Calls `builder` for the key and category if it is not filtered by the view query.
    ///
    /// If the setting is already present the builder overrides only the metadata set.
    pub fn entry(
        &mut self,
        config_key: impl Into<ConfigKey>,
        category_id: impl Into<CategoryId>,
        builder: impl for<'a, 'b> FnOnce(&'a mut SettingBuilder<'b>) -> &'a mut SettingBuilder<'b>,
    ) -> &mut Self {
        if let Some(mut e) = self.entry_impl(config_key.into(), category_id.into()) {
            builder(&mut e);
        }
        self
    }
    fn entry_impl(&mut self, config_key: ConfigKey, category_id: CategoryId) -> Option<SettingBuilder<'_>> {
        if (self.filter)(&config_key, &category_id) {
            if let Some(i) = self.settings.iter().position(|s| s.key == config_key) {
                let existing = self.settings.swap_remove(i);
                Some(SettingBuilder {
                    settings: &mut self.settings,
                    config_key,
                    category_id,
                    order: existing.order,
                    name: Some(existing.name),
                    description: Some(existing.description),
                    meta: Arc::try_unwrap(existing.meta).unwrap(),
                    value: None,
                    reset: None,
                })
            } else {
                Some(SettingBuilder {
                    settings: &mut self.settings,
                    config_key,
                    category_id,
                    order: u16::MAX,
                    name: None,
                    description: None,
                    meta: OwnedStateMap::new(),
                    value: None,
                    reset: None,
                })
            }
        } else {
            None
        }
    }
}

/// Setting entry builder.
pub struct SettingBuilder<'a> {
    settings: &'a mut Vec<Setting>,
    config_key: ConfigKey,
    category_id: CategoryId,
    order: u16,
    name: Option<Var<Txt>>,
    description: Option<Var<Txt>>,
    meta: OwnedStateMap<Setting>,
    value: Option<(AnyVar, TypeId)>,
    reset: Option<Arc<dyn SettingReset>>,
}
impl SettingBuilder<'_> {
    /// The config edited by this setting.
    pub fn key(&self) -> &ConfigKey {
        &self.config_key
    }
    /// Settings category.
    pub fn category(&self) -> &CategoryId {
        &self.category_id
    }

    /// Set the setting order number.
    ///
    /// Lower numbers are listed first, two categories with the same order are sorted by display name.
    pub fn order(&mut self, order: u16) -> &mut Self {
        self.order = order;
        self
    }

    /// Set the setting name.
    pub fn name(&mut self, name: impl IntoVar<Txt>) -> &mut Self {
        self.name = Some(name.into_var().read_only());
        self
    }

    /// Set the setting short help text.
    pub fn description(&mut self, description: impl IntoVar<Txt>) -> &mut Self {
        self.description = Some(description.into_var().read_only());
        self
    }

    /// Set the custom metadata value.
    pub fn set<T: StateValue>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>) -> &mut Self {
        self.meta.borrow_mut().set(id, value);
        self
    }

    /// Set the custom metadata flag.
    pub fn flag(&mut self, id: impl Into<StateId<()>>) -> &mut Self {
        self.meta.borrow_mut().flag(id);
        self
    }

    /// Custom setting metadata.
    pub fn meta(&mut self) -> StateMapMut<'_, Setting> {
        self.meta.borrow_mut()
    }

    /// Get the value variable for editing from [`CONFIG`]. The `default` value is used when getting the variable.
    pub fn value<T: ConfigValue>(&mut self, default: T) -> &mut Self {
        self.cfg_value(&mut CONFIG, default)
    }

    /// Get the value variable for editing from `cfg`. The `default` value is used when getting the variable.
    pub fn cfg_value<T: ConfigValue>(&mut self, cfg: &mut impl Config, default: T) -> &mut Self {
        let value = cfg.get(self.config_key.clone(), default, false);
        self.value = Some((value.into(), TypeId::of::<T>()));
        self
    }

    /// Use a [`FallbackConfigReset`] to reset the settings.
    ///
    /// This is the preferred way of implementing reset as it keeps the user config file clean,
    /// but it does require a config setup with two files.
    ///
    /// The `strip_key_prefix` is removed from config keys before passing to `resetter`, this is
    /// required if the config is setup using a switch over multiple files.
    pub fn reset(&mut self, resetter: Box<dyn FallbackConfigReset>, strip_key_prefix: impl Into<Txt>) -> &mut Self {
        self.reset = Some(Arc::new(FallbackReset {
            resetter,
            strip_key_prefix: strip_key_prefix.into(),
        }));
        self
    }

    /// Use a `default` value to reset the settings.
    ///
    /// The default value is set on the config to reset.
    pub fn default<T: ConfigValue>(&mut self, default: T) -> &mut Self {
        let reset = BoxAnyVarValue::new(default);
        self.reset = Some(Arc::new(reset));
        self
    }
}
impl Drop for SettingBuilder<'_> {
    fn drop(&mut self) {
        let (cfg, cfg_type) = self.value.take().unwrap_or_else(|| {
            tracing::debug!("no value provided for {} settings", self.config_key);
            (const_var(SettingValueNotSet).into(), TypeId::of::<SettingValueNotSet>())
        });
        self.settings.push(Setting {
            key: mem::take(&mut self.config_key),
            order: self.order,
            name: self.name.take().unwrap_or_else(|| var(Txt::from_static(""))),
            description: self.description.take().unwrap_or_else(|| var(Txt::from_static(""))),
            category: mem::take(&mut self.category_id),
            meta: Arc::new(mem::take(&mut self.meta)),
            value: cfg,
            value_type: cfg_type,
            reset: self.reset.take().unwrap_or_else(|| Arc::new(SettingValueNotSet)),
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
struct SettingValueNotSet;

/// Setting categories builder.
pub struct CategoriesBuilder<'f> {
    categories: Vec<Category>,
    filter: &'f mut dyn FnMut(&CategoryId) -> bool,
}
impl CategoriesBuilder<'_> {
    /// Calls `builder` for the id if it is not filtered by the view query.
    ///
    /// If the category is already present the builder overrides only the metadata set.
    pub fn entry(
        &mut self,
        category_id: impl Into<CategoryId>,
        builder: impl for<'a, 'b> FnOnce(&'a mut CategoryBuilder<'b>) -> &'a mut CategoryBuilder<'b>,
    ) -> &mut Self {
        if let Some(mut e) = self.entry_impl(category_id.into()) {
            builder(&mut e);
        }
        self
    }
    fn entry_impl(&mut self, category_id: CategoryId) -> Option<CategoryBuilder<'_>> {
        if (self.filter)(&category_id) {
            if let Some(i) = self.categories.iter().position(|s| s.id == category_id) {
                let existing = self.categories.swap_remove(i);
                Some(CategoryBuilder {
                    categories: &mut self.categories,
                    category_id,
                    order: existing.order,
                    name: Some(existing.name),
                    meta: Arc::try_unwrap(existing.meta).unwrap(),
                })
            } else {
                Some(CategoryBuilder {
                    categories: &mut self.categories,
                    category_id,
                    order: u16::MAX,
                    name: None,
                    meta: OwnedStateMap::new(),
                })
            }
        } else {
            None
        }
    }
}

/// Category entry builder.
pub struct CategoryBuilder<'a> {
    categories: &'a mut Vec<Category>,
    category_id: CategoryId,
    order: u16,
    name: Option<Var<Txt>>,
    meta: OwnedStateMap<Category>,
}
impl CategoryBuilder<'_> {
    /// Unique ID.
    pub fn id(&self) -> &CategoryId {
        &self.category_id
    }

    /// Set the position of the category in a list of categories.
    ///
    /// Lower numbers are listed first, two categories with the same order are sorted by display name.
    pub fn order(&mut self, order: u16) -> &mut Self {
        self.order = order;
        self
    }

    /// Set the category name.
    pub fn name(&mut self, name: impl IntoVar<Txt>) -> &mut Self {
        self.name = Some(name.into_var().read_only());
        self
    }

    /// Set the custom metadata value.
    pub fn set<T: StateValue>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>) -> &mut Self {
        self.meta.borrow_mut().set(id, value);
        self
    }

    /// Set the custom metadata flag.
    pub fn flag(&mut self, id: impl Into<StateId<()>>) -> &mut Self {
        self.meta.borrow_mut().flag(id);
        self
    }

    /// Custom category metadata.
    pub fn meta(&mut self) -> StateMapMut<'_, Category> {
        self.meta.borrow_mut()
    }
}
impl Drop for CategoryBuilder<'_> {
    fn drop(&mut self) {
        self.categories.push(Category {
            id: mem::take(&mut self.category_id),
            order: self.order,
            name: self.name.take().unwrap_or_else(|| var(Txt::from_static(""))),
            meta: Arc::new(mem::take(&mut self.meta)),
        })
    }
}
trait SettingReset: Send + Sync + 'static {
    fn can_reset(&self, key: &ConfigKey, value: &AnyVar) -> Var<bool>;
    fn reset(&self, key: &ConfigKey, value: &AnyVar);
}

struct FallbackReset {
    resetter: Box<dyn FallbackConfigReset>,
    strip_key_prefix: Txt,
}

impl SettingReset for FallbackReset {
    fn can_reset(&self, key: &ConfigKey, _: &AnyVar) -> Var<bool> {
        match key.strip_prefix(self.strip_key_prefix.as_str()) {
            Some(k) => self.resetter.can_reset(ConfigKey::from_str(k)),
            None => self.resetter.can_reset(key.clone()),
        }
    }

    fn reset(&self, key: &ConfigKey, _: &AnyVar) {
        match key.strip_prefix(self.strip_key_prefix.as_str()) {
            Some(k) => self.resetter.reset(&ConfigKey::from_str(k)),
            None => self.resetter.reset(key),
        }
    }
}
impl SettingReset for BoxAnyVarValue {
    fn can_reset(&self, _: &ConfigKey, value: &AnyVar) -> Var<bool> {
        let initial = value.with(|v| v.eq_any(&**self));
        let map = var(initial);

        let map_in = map.clone();
        let dft = (*self).clone_boxed();
        value
            .hook(move |args: &AnyVarHookArgs| {
                map_in.set(args.value().eq_any(&*dft));
                true
            })
            .perm();

        map.clone()
    }

    fn reset(&self, _: &ConfigKey, value: &AnyVar) {
        value.set((*self).clone_boxed());
    }
}
impl SettingReset for SettingValueNotSet {
    fn can_reset(&self, _: &ConfigKey, _: &AnyVar) -> Var<bool> {
        const_var(false)
    }
    fn reset(&self, _: &ConfigKey, _: &AnyVar) {}
}
