//! Settings are the config the user can directly edit, this module implements a basic settings data model.
//!
//! The settings editor widget is not implemented here, this module bridges config implementers with settings UI implementers.

use core::fmt;
use std::{any::TypeId, cmp::Ordering, mem, sync::Arc};

use zng_app_context::app_local;
use zng_state_map::{OwnedStateMap, StateId, StateMapMut, StateMapRef, StateValue};
use zng_txt::Txt;
use zng_var::{var, AnyVar, AnyVarHookArgs, AnyVarValue, BoxedAnyVar, BoxedVar, IntoVar, LocalVar, Var, VarValue};

use crate::{AnyConfig, ConfigKey, ConfigValue, CONFIG};

/// Settings metadata service.
pub struct SETTINGS;

impl SETTINGS {
    /// Register a closure that provides settings metadata.
    pub fn register(&self, f: impl Fn(&mut SettingsBuilder) + Send + Sync + 'static) {
        SETTINGS_SV.write().sources.push(Box::new(f))
    }

    /// Register a closure that provides category metadata.
    pub fn register_cat(&self, f: impl Fn(&mut CategoriesBuilder) + Send + Sync + 'static) {
        SETTINGS_SV.write().sources_cat.push(Box::new(f))
    }

    /// Select and sort settings matched by `filter` that edits configs from [`CONFIG`].
    pub fn get(&self, mut filter: impl FnMut(&ConfigKey, &CategoryId) -> bool, sort: bool) -> Vec<(Category, Vec<Setting>)> {
        self.get_impl(None, &mut filter, sort)
    }

    /// Select and sort settings matched by `filter` that edits configs from a different source.
    pub fn get_for(
        &self,
        config: &mut impl AnyConfig,
        mut filter: impl FnMut(&ConfigKey, &CategoryId) -> bool,
        sort: bool,
    ) -> Vec<(Category, Vec<Setting>)> {
        self.get_impl(Some(config), &mut filter, sort)
    }

    fn get_impl(
        &self,
        config: Option<&mut dyn AnyConfig>,
        filter: &mut dyn FnMut(&ConfigKey, &CategoryId) -> bool,
        sort: bool,
    ) -> Vec<(Category, Vec<Setting>)> {
        let sv = SETTINGS_SV.read();

        let mut settings = SettingsBuilder {
            config,
            settings: vec![],
            filter,
        };
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
                tracing::debug!("missing category metadata for {}", s.category);
                result.push((
                    Category {
                        id: s.category.clone(),
                        order: u16::MAX,
                        name: LocalVar(s.category.clone()).boxed(),
                        meta: Arc::new(OwnedStateMap::new()),
                    },
                    vec![s],
                ));
            }
        }

        if sort {
            result.sort_by(|a, b| {
                let c = a.0.order.cmp(&b.0.order);
                if matches!(c, Ordering::Equal) {
                    return a.0.name.with(|a| b.0.name.with(|b| a.cmp(b)));
                }
                c
            });
            for (_, s) in &mut result {
                s.sort_by(|a, b| {
                    let c = a.order.cmp(&b.order);
                    if matches!(c, Ordering::Equal) {
                        return a.name.with(|a| b.name.with(|b| a.cmp(b)));
                    }
                    c
                });
            }
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
                config: None,
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
                config: None,
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
}

/// Unique ID of a [`Category`].
pub type CategoryId = Txt;

/// Settings category.
#[derive(Clone)]
pub struct Category {
    id: CategoryId,
    order: u16,
    name: BoxedVar<Txt>,
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
    pub fn name(&self) -> &BoxedVar<Txt> {
        &self.name
    }

    /// Custom category metadata.
    pub fn meta(&self) -> StateMapRef<Category> {
        self.meta.borrow()
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

/// Setting entry.
pub struct Setting {
    key: ConfigKey,
    order: u16,
    name: BoxedVar<Txt>,
    description: BoxedVar<Txt>,
    category: CategoryId,
    meta: Arc<OwnedStateMap<Setting>>,
    cfg: BoxedAnyVar,
    cfg_type: TypeId,
    cfg_default: Box<dyn AnyVarValue>,
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
            cfg: self.cfg.clone(),
            cfg_type: self.cfg_type,
            cfg_default: self.cfg_default.clone_boxed(),
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
    pub fn name(&self) -> &BoxedVar<Txt> {
        &self.name
    }
    /// Short help text.
    pub fn description(&self) -> &BoxedVar<Txt> {
        &self.description
    }
    /// Settings category.
    pub fn category(&self) -> &CategoryId {
        &self.category
    }

    /// Custom setting metadata.
    pub fn meta(&self) -> StateMapRef<Setting> {
        self.meta.borrow()
    }

    /// If the `cfg` and `cfg_default` values where set by the settings builder.
    pub fn cfg_is_known(&self) -> bool {
        self.cfg_type != TypeId::of::<SettingValueNotSet>()
    }

    /// Config value.
    pub fn cfg(&self) -> &BoxedAnyVar {
        &self.cfg
    }

    /// Config value type.
    pub fn cfg_type(&self) -> TypeId {
        self.cfg_type
    }

    /// Config value, strongly typed.
    pub fn cfg_downcast<T: ConfigValue>(&self) -> Option<BoxedVar<T>> {
        if self.cfg_type == std::any::TypeId::of::<T>() {
            let v = self.cfg.clone().double_boxed_any().downcast::<BoxedVar<T>>().unwrap();
            Some(*v)
        } else {
            None
        }
    }

    /// Config default value.
    pub fn cfg_default(&self) -> &dyn AnyVarValue {
        &*self.cfg_default
    }

    /// Config default value, strongly typed.
    pub fn cfg_default_downcast<T: ConfigValue>(&self) -> Option<T> {
        self.cfg_default.as_any().downcast_ref::<T>().cloned()
    }

    /// Gets a variable that tracks if the config is set to the default value.
    pub fn cfg_is_default(&self) -> BoxedVar<bool> {
        let mut initial = false;
        self.cfg.with_any(&mut |v| {
            initial = v.eq_any(&*self.cfg_default);
        });
        let map = var(initial);

        let map_in = map.clone();
        let dft = self.cfg_default.clone_boxed();
        self.cfg
            .hook_any(Box::new(move |args: &AnyVarHookArgs| {
                map_in.set(args.value().eq_any(&*dft));
                true
            }))
            .perm();

        map.clone().boxed()
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
    config: Option<&'a mut dyn AnyConfig>,
    settings: Vec<Setting>,
    filter: &'a mut dyn FnMut(&ConfigKey, &CategoryId) -> bool,
}
impl<'c> SettingsBuilder<'c> {
    /// Get the setting entry builder for the key and category if it is requested by the view query.
    ///
    /// If the setting is already present the builder overrides only the metadata set.
    pub fn entry<T: VarValue>(&mut self, config_key: impl Into<ConfigKey>, category_id: impl Into<CategoryId>) -> Option<SettingBuilder> {
        self.entry_impl(config_key.into(), category_id.into())
    }
    fn entry_impl(&mut self, config_key: ConfigKey, category_id: CategoryId) -> Option<SettingBuilder> {
        if (self.filter)(&config_key, &category_id) {
            if let Some(i) = self.settings.iter().position(|s| s.key == config_key) {
                let existing = self.settings.swap_remove(i);
                Some(SettingBuilder {
                    config: self.config.as_deref_mut(),
                    settings: &mut self.settings,
                    config_key,
                    category_id,
                    order: existing.order,
                    name: Some(existing.name),
                    description: Some(existing.description),
                    meta: Arc::try_unwrap(existing.meta).unwrap(),
                    cfg: None,
                })
            } else {
                Some(SettingBuilder {
                    config: self.config.as_deref_mut(),
                    settings: &mut self.settings,
                    config_key,
                    category_id,
                    order: u16::MAX,
                    name: None,
                    description: None,
                    meta: OwnedStateMap::new(),
                    cfg: None,
                })
            }
        } else {
            None
        }
    }
}

/// Setting entry builder.
pub struct SettingBuilder<'a> {
    config: Option<&'a mut dyn AnyConfig>,
    settings: &'a mut Vec<Setting>,
    config_key: ConfigKey,
    category_id: CategoryId,
    order: u16,
    name: Option<BoxedVar<Txt>>,
    description: Option<BoxedVar<Txt>>,
    meta: OwnedStateMap<Setting>,
    cfg: Option<(BoxedAnyVar, TypeId, Box<dyn AnyVarValue>)>,
}
impl<'a> SettingBuilder<'a> {
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
    pub fn with_order(&mut self, order: u16) -> &mut Self {
        self.order = order;
        self
    }

    /// Set the setting name.
    pub fn with_name(&mut self, name: impl IntoVar<Txt>) -> &mut Self {
        self.name = Some(name.into_var().read_only().boxed());
        self
    }

    /// Set the setting short help text.
    pub fn with_description(&mut self, description: impl IntoVar<Txt>) -> &mut Self {
        self.description = Some(description.into_var().read_only().boxed());
        self
    }

    /// Set the custom metadata value.
    pub fn with_meta<T: StateValue>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>) -> &mut Self {
        self.meta.borrow_mut().set(id, value);
        self
    }

    /// Set the custom metadata flag.
    pub fn with_meta_flag(&mut self, id: impl Into<StateId<()>>) -> &mut Self {
        self.meta.borrow_mut().flag(id);
        self
    }

    /// Custom setting metadata.
    pub fn meta(&mut self) -> StateMapMut<Setting> {
        self.meta.borrow_mut()
    }

    /// Set the default value and gets the [`CONFIG`] value for the key.
    ///
    ///
    pub fn with_cfg<T: ConfigValue>(&mut self, default: T) -> &mut Self {
        let key = self.key().clone();
        self.cfg = Some((
            match &mut self.config {
                Some(cfg) => cfg.get_raw_serde_bidi(key, || default.clone(), true).boxed_any(),
                None => CONFIG.get(key, || default.clone()).boxed_any(),
            },
            TypeId::of::<T>(),
            Box::new(default),
        ));
        self
    }
}
impl<'a> Drop for SettingBuilder<'a> {
    fn drop(&mut self) {
        let (cfg, cfg_type, cfg_default) = self.cfg.take().unwrap_or_else(|| {
            (
                LocalVar(SettingValueNotSet).boxed_any(),
                TypeId::of::<SettingValueNotSet>(),
                Box::new(SettingValueNotSet),
            )
        });
        self.settings.push(Setting {
            key: mem::take(&mut self.config_key),
            order: self.order,
            name: self.name.take().unwrap_or_else(|| var(Txt::from_static("")).boxed()),
            description: self.description.take().unwrap_or_else(|| var(Txt::from_static("")).boxed()),
            category: mem::take(&mut self.category_id),
            meta: Arc::new(mem::take(&mut self.meta)),
            cfg,
            cfg_type,
            cfg_default,
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
impl<'f> CategoriesBuilder<'f> {
    /// Get the category builder for the id if it is requested by the view query.
    ///
    /// If the category is already present the builder overrides only the metadata set.
    pub fn entry(&mut self, category_id: impl Into<CategoryId>) -> Option<CategoryBuilder> {
        self.entry_impl(category_id.into())
    }
    fn entry_impl(&mut self, category_id: CategoryId) -> Option<CategoryBuilder> {
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
    name: Option<BoxedVar<Txt>>,
    meta: OwnedStateMap<Category>,
}
impl<'a> CategoryBuilder<'a> {
    /// Unique ID.
    pub fn id(&self) -> &CategoryId {
        &self.category_id
    }

    /// Set the position of the category in a list of categories.
    ///
    /// Lower numbers are listed first, two categories with the same order are sorted by display name.
    pub fn with_order(&mut self, order: u16) -> &mut Self {
        self.order = order;
        self
    }

    /// Set the category name.
    pub fn with_name(&mut self, name: impl IntoVar<Txt>) -> &mut Self {
        self.name = Some(name.into_var().read_only().boxed());
        self
    }

    /// Set the custom metadata value.
    pub fn with_meta<T: StateValue>(&mut self, id: impl Into<StateId<T>>, value: impl Into<T>) -> &mut Self {
        self.meta.borrow_mut().set(id, value);
        self
    }

    /// Set the custom metadata flag.
    pub fn with_meta_flag(&mut self, id: impl Into<StateId<()>>) -> &mut Self {
        self.meta.borrow_mut().flag(id);
        self
    }

    /// Custom category metadata.
    pub fn meta(&mut self) -> StateMapMut<Category> {
        self.meta.borrow_mut()
    }
}
impl<'a> Drop for CategoryBuilder<'a> {
    fn drop(&mut self) {
        self.categories.push(Category {
            id: mem::take(&mut self.category_id),
            order: self.order,
            name: self.name.take().unwrap_or_else(|| var(Txt::from_static("")).boxed()),
            meta: Arc::new(mem::take(&mut self.meta)),
        })
    }
}
