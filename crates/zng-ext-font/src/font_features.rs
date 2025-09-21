//! Font features and variation types.

use std::{
    collections::{HashMap, HashSet, hash_map},
    fmt,
    marker::PhantomData,
    num::NonZeroU32,
    ops,
};

use num_enum::FromPrimitive;

/// Name of a font feature.
///
/// # Examples
///
/// ```
/// # use zng_ext_font::font_features::*;
/// let historical_lig: FontFeatureName = b"hlig".into();
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FontFeatureName(pub [u8; 4]);
impl FontFeatureName {
    /// As UTF-8.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}
impl From<&'static [u8; 4]> for FontFeatureName {
    fn from(name: &'static [u8; 4]) -> Self {
        FontFeatureName(*name)
    }
}
impl From<FontFeatureName> for ttf_parser::Tag {
    fn from(value: FontFeatureName) -> Self {
        ttf_parser::Tag::from_bytes(&value.0)
    }
}
impl ops::Deref for FontFeatureName {
    type Target = [u8; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl fmt::Debug for FontFeatureName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.as_str().is_empty() {
            write!(f, "{:?}", self.0)
        } else {
            write!(f, "{}", self.as_str())
        }
    }
}
impl fmt::Display for FontFeatureName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// The raw value used when a feature is set to `true`.
pub const FEATURE_ENABLED: u32 = 1;
/// The raw value used when a feature is set to `false`.
pub const FEATURE_DISABLED: u32 = 0;

type FontFeaturesMap = HashMap<FontFeatureName, u32>;

/// Font features configuration.
#[derive(Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct FontFeatures(FontFeaturesMap);
impl FontFeatures {
    /// New default.
    pub fn new() -> FontFeatures {
        FontFeatures::default()
    }

    /// New builder.
    pub fn builder() -> FontFeaturesBuilder {
        FontFeaturesBuilder::default()
    }

    /// Set or override the features of `self` from `other`.
    ///
    /// Returns the previous state of all affected names.
    pub fn set_all(&mut self, other: &FontFeatures) -> Vec<(FontFeatureName, Option<u32>)> {
        let mut prev = Vec::with_capacity(other.0.len());
        for (&name, &state) in other.0.iter() {
            prev.push((name, self.0.insert(name, state)));
        }
        prev
    }

    /// Restore feature states that where overridden in [`set_all`](Self::set_all).
    pub fn restore(&mut self, prev: Vec<(FontFeatureName, Option<u32>)>) {
        for (name, state) in prev {
            match state {
                Some(state) => {
                    self.0.insert(name, state);
                }
                None => {
                    self.0.remove(&name);
                }
            }
        }
    }

    /// Access to the named feature.
    pub fn feature(&mut self, name: FontFeatureName) -> FontFeature<'_> {
        FontFeature(self.0.entry(name))
    }

    /// Access to a set of named features that are managed together.
    ///
    /// # Panics
    ///
    /// If `names` has less than 2 names.
    pub fn feature_set(&mut self, names: &'static [FontFeatureName]) -> FontFeatureSet<'_> {
        assert!(names.len() >= 2);
        FontFeatureSet {
            features: &mut self.0,
            names,
        }
    }

    /// Access to a set of named features where only one of the features can be enabled at a time.
    ///
    /// # Panics
    ///
    /// If `S::names()` has less than 2 names.
    pub fn feature_exclusive_set<S: FontFeatureExclusiveSetState>(&mut self) -> FontFeatureExclusiveSet<'_, S> {
        assert!(S::names().len() >= 2);
        FontFeatureExclusiveSet {
            features: &mut self.0,
            _t: PhantomData,
        }
    }

    /// Access to a set of named features where only one or more features can be enabled but each combination
    /// represents a single distinct *state*.
    ///
    /// # Panics
    ///
    /// If `S::names()` has less than 2 entries.
    pub fn feature_exclusive_sets<S: FontFeatureExclusiveSetsState>(&mut self) -> FontFeatureExclusiveSets<'_, S> {
        assert!(S::names().len() >= 2);
        FontFeatureExclusiveSets {
            features: &mut self.0,
            _t: PhantomData,
        }
    }

    /// Generate the harfbuzz font features.
    pub fn finalize(&self) -> RFontFeatures {
        self.0
            .iter()
            .map(|(&n, &s)| rustybuzz::Feature::new(ttf_parser::Tag::from(n), s, 0..usize::MAX))
            .collect()
    }
}
impl fmt::Debug for FontFeatures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        for (name, state) in self.0.iter() {
            map.entry(&name.as_str(), state);
        }
        map.finish()
    }
}

/// Finalized [`FontFeatures`].
///
/// This is a vec of [harfbuzz features](https://docs.rs/rustybuzz/0.17.0/rustybuzz/struct.Feature.html).
pub type RFontFeatures = Vec<rustybuzz::Feature>;

/// A builder for [`FontFeatures`].
///
/// # Examples
///
/// ```
/// # use zng_ext_font::font_features::*;
/// let features = FontFeatures::builder().kerning(false).build();
/// ```
#[derive(Default)]
pub struct FontFeaturesBuilder(FontFeatures);
impl FontFeaturesBuilder {
    /// Finish building.
    pub fn build(self) -> FontFeatures {
        self.0
    }

    /// Set the named feature.
    pub fn feature(mut self, name: FontFeatureName, state: impl Into<FontFeatureState>) -> Self {
        self.0.feature(name).set(state);
        self
    }

    /// Sets all the named features to the same value.
    ///
    /// # Panics
    ///
    /// If `names` has less than 2 names.
    pub fn feature_set(mut self, names: &'static [FontFeatureName], state: impl Into<FontFeatureState>) -> Self {
        self.0.feature_set(names).set(state);
        self
    }

    /// Sets a single feature of a set of features.
    ///
    /// # Panics
    ///
    /// If `S::names()` has less than 2 names.
    pub fn feature_exclusive_set<S: FontFeatureExclusiveSetState>(mut self, state: impl Into<S>) -> Self {
        self.0.feature_exclusive_set::<S>().set(state);
        self
    }

    /// Sets the features that represent the `state`.
    ///
    /// # Panics
    ///
    /// If `S::names()` has less than 2 entries.
    pub fn feature_exclusive_sets<S: FontFeatureExclusiveSetsState>(mut self, state: impl Into<S>) -> Self {
        self.0.feature_exclusive_sets::<S>().set(state);
        self
    }
}

/// Generate `FontFeature` methods in `FontFeatures` and builder methods in `FontFeaturesBuilder`
/// that set the feature.
macro_rules! font_features {
    ($(
        $(#[$docs:meta])*
        fn $name:ident($feat0_or_Enum:tt $(, $feat1:tt)?) $(-> $Helper:ident)?;
    )+) => {
        impl FontFeatures {$(
            font_features!{feature $(#[$docs])* fn $name($feat0_or_Enum $(, $feat1)?) $(-> $Helper)?; }
        )+}

        impl FontFeaturesBuilder {$(
            font_features!{builder $(#[$docs])* fn $name($($feat0_or_Enum -> $Helper)?); }
        )+}
    };

    (feature $(#[$docs:meta])* fn $name:ident($feat0:tt, $feat1:tt); ) => {
        $(#[$docs])*

        pub fn $name(&mut self) -> FontFeatureSet<'_> {
            static FEATS: [FontFeatureName; 2] = [FontFeatureName(*$feat0), FontFeatureName(*$feat1)];
            self.feature_set(&FEATS)
        }

    };

    (feature $(#[$docs:meta])* fn $name:ident($feat0:tt);) => {
        $(#[$docs])*

        pub fn $name(&mut self) -> FontFeature<'_> {
            self.feature(FontFeatureName(*$feat0))
        }

    };

    (feature $(#[$docs:meta])* fn $name:ident($Enum:ident) -> $Helper:ident;) => {
        $(#[$docs])*

        pub fn $name(&mut self) -> $Helper<'_, $Enum> {
            $Helper { features: &mut self.0, _t: PhantomData }
        }

    };

    (builder $(#[$docs:meta])* fn $name:ident();) => {
        $(#[$docs])*

        pub fn $name(mut self, state: impl Into<FontFeatureState>) -> Self {
            self.0.$name().set(state);
            self
        }

    };

    (builder $(#[$docs:meta])* fn $name:ident($Enum:ident -> $Helper:ident);) => {
        $(#[$docs])*

        pub fn $name(mut self, state: impl Into<$Enum>) -> Self {
            self.0.$name().set(state);
            self
        }

    };
}

#[rustfmt::skip]// zng fmt can't handle this syntax and is slightly slower because it causes rustfmt errors
font_features! {
    /// Font capital glyph variants.
    ///
    /// See [`CapsVariant`] for more details.
    fn caps(CapsVariant) -> FontFeatureExclusiveSets;

    /// Allow glyphs boundaries to overlap for a more pleasant reading.
    ///
    /// This corresponds to the `kern` feature.
    ///
    /// `Auto` always activates these kerning.
    fn kerning(b"kern");

    /// The most common ligatures, like for `fi`, `ffi`, `th` or similar.
    ///
    /// This corresponds to OpenType `liga` and `clig` features.
    ///
    /// `Auto` always activates these ligatures.
    fn common_lig(b"liga", b"clig");

    /// Ligatures specific to the font, usually decorative.
    ///
    /// This corresponds to OpenType `dlig` feature.
    ///
    /// `Auto` usually disables these ligatures.
    fn discretionary_lig(b"dlig");

    /// Ligatures used historically, in old books, like the German tz digraph being displayed ß.
    ///
    /// This corresponds to OpenType `hlig` feature.
    ///
    /// `Auto` usually disables these ligatures.
    fn historical_lig(b"hlig");

    /// Alternative letters that adapt to their surrounding letters.
    ///
    /// This corresponds to OpenType `calt` feature.
    ///
    /// `Auto` usually activates this feature.
    fn contextual_alt(b"calt");

    /// Force usage of ordinal special glyphs, 1a becomes 1ª.
    ///
    /// This corresponds to OpenType `ordn` feature.
    ///
    /// `Auto` deactivates this feature.
    fn ordinal(b"ordn");

    /// Force use of a slashed zero for `0`.
    ///
    /// This corresponds to OpenType `zero` feature.
    ///
    /// `Auto` deactivates this feature.
    fn slashed_zero(b"zero");

    /// Use swashes flourish style.
    ///
    /// Fonts can have alternative swash styles, you can select then by enabling a number.
    ///
    /// This corresponds to OpenType `swsh` and `cswh` feature.
    ///
    /// `Auto` does not use swashes.
    fn swash(b"swsh", b"cswh");

    /// Use stylistic alternatives.
    ///
    /// Fonts can have multiple alternative styles, you can select then by enabling a number.
    ///
    /// This corresponds to OpenType `salt` feature.
    ///
    /// `Auto` does not use alternative styles.
    fn stylistic(b"salt");

    /// Use glyphs that were common in the past but not today.
    ///
    /// This corresponds to OpenType `hist` feature.
    ///
    /// `Auto` does not use alternative styles.
    fn historical_forms(b"hist");

    /// Replace letter with fleurons, dingbats and border elements.
    ///
    /// Fonts can have multiple alternative styles, you can select then by enabling a number.
    ///
    /// This corresponds to OpenType `ornm` feature.
    ///
    /// `Auto` does not enable this by default, but some fonts are purely dingbats glyphs.
    fn ornaments(b"ornm");

    /// Font annotation alternatives, like circled digits or inverted characters.
    ///
    /// Fonts can have multiple alternative styles, you can select then by enabling a number.
    ///
    /// This corresponds to OpenType `nalt` feature.
    ///
    /// `Auto` does not use alternative styles.
    fn annotation(b"nalt");

    /// Font numeric glyph variants.
    ///
    /// See [`NumVariant`] for more details.
    fn numeric(NumVariant) -> FontFeatureExclusiveSet;

    /// Font numeric spacing variants.
    ///
    /// See [`NumSpacing`] for more details.
    fn num_spacing(NumSpacing) -> FontFeatureExclusiveSet;

    /// Font numeric spacing variants.
    ///
    /// See [`NumSpacing`] for more details.
    fn num_fraction(NumFraction) -> FontFeatureExclusiveSet;

    /// Font stylistic alternatives for sets of characters.
    ///
    /// See [`FontStyleSet`] for more details.
    fn style_set(FontStyleSet) -> FontFeatureExclusiveSet;

    /// Font stylistic alternatives for individual characters.
    ///
    /// See [`CharVariant`] for more details.
    fn char_variant(CharVariant) -> FontFeatureExclusiveSet;

    /// Font sub/super script alternatives.
    ///
    /// See [`FontPosition`] for more details.
    fn position(FontPosition) -> FontFeatureExclusiveSet;

    /// Force the use of ruby (rubi) glyph variants.
    ///
    /// This corresponds to OpenType `ruby` feature.
    ///
    /// `Auto` does not force the use of ruby variants.
    fn ruby(b"ruby");

    /// Japanese logographic set selection.
    ///
    /// See [`JpVariant`] for more details.
    fn jp_variant(JpVariant) -> FontFeatureExclusiveSet;

    /// Use kana glyphs optimized for horizontal writing.
    ///
    /// This corresponds to OpenType `hkna` feature.
    fn horizontal_kana(b"hkna");

    /// Chinese logographic set selection.
    ///
    /// See [`CnVariant`] for more details.
    fn cn_variant(CnVariant) -> FontFeatureExclusiveSet;

    /// East Asian figure width control
    ///
    /// See [`EastAsianWidth`] for more details.
    fn ea_width(EastAsianWidth) -> FontFeatureExclusiveSet;
}

/// Represents a feature in a [`FontFeatures`] configuration.
pub struct FontFeature<'a>(hash_map::Entry<'a, FontFeatureName, u32>);
impl FontFeature<'_> {
    /// Gets the OpenType name of the feature.
    pub fn name(&self) -> FontFeatureName {
        *self.0.key()
    }

    /// Gets the current state of the feature.
    pub fn state(&self) -> FontFeatureState {
        match &self.0 {
            hash_map::Entry::Occupied(e) => FontFeatureState(Some(*e.get())),
            hash_map::Entry::Vacant(_) => FontFeatureState::auto(),
        }
    }

    /// If the feature is explicitly enabled.
    pub fn is_enabled(&self) -> bool {
        self.state().is_enabled()
    }

    /// If the feature is explicitly disabled.
    pub fn is_disabled(&self) -> bool {
        self.state().is_disabled()
    }

    /// If the feature is auto enabled.
    pub fn is_auto(&self) -> bool {
        self.state().is_auto()
    }

    /// Set the feature state.
    ///
    /// Returns the previous state.
    pub fn set(self, state: impl Into<FontFeatureState>) -> FontFeatureState {
        let prev = self.state();
        match state.into().0 {
            Some(n) => self.set_explicit(n),
            None => self.auto(),
        }
        prev
    }

    fn set_explicit(self, state: u32) {
        match self.0 {
            hash_map::Entry::Occupied(mut e) => {
                e.insert(state);
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(state);
            }
        }
    }

    /// Enable the feature.
    pub fn enable(self) {
        self.set_explicit(FEATURE_ENABLED);
    }

    /// Enable the feature with alternative selection.
    pub fn enable_alt(self, alt: NonZeroU32) {
        self.set_explicit(alt.get())
    }

    /// Disable the feature.
    pub fn disable(self) {
        self.set_explicit(FEATURE_DISABLED);
    }

    /// Set the feature to auto.
    pub fn auto(self) {
        if let hash_map::Entry::Occupied(e) = self.0 {
            e.remove();
        }
    }
}
impl fmt::Debug for FontFeature<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"{}\": {:?}", self.name(), self.state())
    }
}

/// Represents a set of features in a [`FontFeatures`] configuration, the features state is managed together.
pub struct FontFeatureSet<'a> {
    features: &'a mut FontFeaturesMap,
    names: &'static [FontFeatureName],
}
impl FontFeatureSet<'_> {
    /// Gets the OpenType name of the features.
    pub fn names(&self) -> &'static [FontFeatureName] {
        self.names
    }

    /// Gets the current state of the features.
    ///
    /// Returns `Auto` if the features are mixed.
    pub fn state(&self) -> FontFeatureState {
        if let Some(&a) = self.features.get(&self.names[0]) {
            for name in &self.names[1..] {
                if self.features.get(name) != Some(&a) {
                    return FontFeatureState::auto();
                }
            }
            FontFeatureState(Some(a))
        } else {
            FontFeatureState::auto()
        }
    }

    /// If the features are explicitly enabled.
    pub fn is_enabled(&self) -> bool {
        self.state().is_enabled()
    }

    /// If the features are explicitly disabled.
    pub fn is_disabled(&self) -> bool {
        self.state().is_disabled()
    }

    /// If the features are auto enabled , or in a mixed state.
    pub fn is_auto(&self) -> bool {
        self.state().is_auto()
    }

    /// Set the feature state.
    ///
    /// Returns the previous state.
    pub fn set(self, state: impl Into<FontFeatureState>) -> FontFeatureState {
        let prev = self.state();
        match state.into().0 {
            Some(n) => self.set_explicit(n),
            None => self.auto(),
        }
        prev
    }

    fn set_explicit(self, state: u32) {
        for name in self.names {
            self.features.insert(*name, state);
        }
    }

    /// Enable the feature.
    pub fn enable(self) {
        self.set_explicit(FEATURE_ENABLED);
    }

    /// Disable the feature.
    pub fn disable(self) {
        self.set_explicit(FEATURE_DISABLED);
    }

    /// Set the feature to auto.
    pub fn auto(self) {
        for name in self.names {
            self.features.remove(name);
        }
    }
}
impl fmt::Debug for FontFeatureSet<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {:?}", self.names, self.state())
    }
}

/// Represents a set of exclusive boolean in a [`FontFeatures`] configuration, only one
/// of the feature is enabled at a time.
pub struct FontFeatureExclusiveSet<'a, S: FontFeatureExclusiveSetState> {
    features: &'a mut FontFeaturesMap,
    _t: PhantomData<S>,
}
impl<S: FontFeatureExclusiveSetState> FontFeatureExclusiveSet<'_, S> {
    /// Gets the OpenType names of all the features affected.
    pub fn names(&self) -> &'static [FontFeatureName] {
        S::names()
    }

    /// Gets the current state of the features.
    pub fn state(&self) -> S {
        let mut state = 0;

        for (i, name) in S::names().iter().enumerate() {
            if let Some(&s) = self.features.get(name)
                && s == FEATURE_ENABLED
                && state == 0
            {
                state = i + 1; // found state.
                continue;
            }
            // found `auto`, a custom state set externally or a second feature activated externally.
            return S::auto();
        }
        S::from_variant(state as u32)
    }
    fn take_state(&mut self) -> S {
        let mut state = 0;
        let mut skip = false;

        for (i, name) in S::names().iter().enumerate() {
            if let Some(s) = self.features.remove(name) {
                if skip {
                    continue;
                }

                if s == FEATURE_ENABLED && state == 0 {
                    state = i + 1; // found state.
                    continue;
                }
            }
            // found `auto`, a custom state set externally or a second feature activated externally.
            skip = true;
        }

        S::from_variant(state as u32)
    }

    /// If state is `Auto`.
    pub fn is_auto(&self) -> bool {
        self.state() == S::auto()
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    pub fn set(&mut self, state: impl Into<S>) -> S {
        let prev = self.take_state();
        if let Some(state) = state.into().variant() {
            self.features.insert(self.names()[state as usize - 1], FEATURE_ENABLED);
        }
        prev
    }
}
impl<S: FontFeatureExclusiveSetState + fmt::Debug> fmt::Debug for FontFeatureExclusiveSet<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.state(), f)
    }
}

/// Represents a set of exclusive boolean in a [`FontFeatures`] configuration, one or more
/// of the features can be active at the same time but they always map to a single *state*.
pub struct FontFeatureExclusiveSets<'a, S: FontFeatureExclusiveSetsState> {
    features: &'a mut FontFeaturesMap,
    _t: PhantomData<S>,
}
impl<S: FontFeatureExclusiveSetsState> FontFeatureExclusiveSets<'_, S> {
    /// Gets the OpenType names of all the features affected.
    pub fn names(&self) -> &'static [&'static [FontFeatureName]] {
        S::names()
    }

    /// Gets the current state of the features.
    pub fn state(&self) -> S {
        let mut active = HashSet::new();
        for &names in self.names() {
            for name in names {
                if let Some(&s) = self.features.get(name) {
                    if s != FEATURE_ENABLED {
                        // custom external state, we only set to FEATURE_ENABLED.
                        return S::auto();
                    } else {
                        active.insert(*name);
                    }
                }
            }
        }

        if !active.is_empty() {
            'names: for (i, &names) in self.names().iter().enumerate() {
                if names.len() == active.len() {
                    for name in names {
                        if !active.contains(name) {
                            continue 'names;
                        }
                    }
                    return S::from_variant(i as u32 + 1);
                }
            }
        }

        S::auto()
    }
    fn take_state(&mut self) -> S {
        let mut active = HashSet::new();
        let mut force_auto = false;

        for &names in self.names() {
            for name in names {
                if let Some(s) = self.features.remove(name) {
                    if force_auto {
                        continue;
                    }

                    if s != FEATURE_ENABLED {
                        // custom external state, we only set to FEATURE_ENABLED.
                        force_auto = true;
                    } else {
                        active.insert(name);
                    }
                }
            }
        }

        if !force_auto && !active.is_empty() {
            'names: for (i, &names) in self.names().iter().enumerate() {
                if names.len() == active.len() {
                    for name in names {
                        if !active.contains(name) {
                            continue 'names;
                        }
                    }
                    return S::from_variant(i as u32 + 1);
                }
            }
        }

        S::auto()
    }

    /// If state is `Auto`.
    pub fn is_auto(&self) -> bool {
        self.state() == S::auto()
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    pub fn set(&mut self, state: impl Into<S>) -> S {
        let prev = self.take_state();
        if let Some(state) = state.into().variant() {
            for name in self.names()[state as usize - 1] {
                self.features.insert(*name, FEATURE_ENABLED);
            }
        }
        prev
    }
}
impl<S: FontFeatureExclusiveSetsState + fmt::Debug> fmt::Debug for FontFeatureExclusiveSets<'_, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.state(), f)
    }
}

/// Represents `enum` like types that represents a exclusive set of features + `Auto`.
pub trait FontFeatureExclusiveSetState: Copy + PartialEq + 'static {
    /// All the names of features, must have more then one name.
    fn names() -> &'static [FontFeatureName];
    /// `None` if `Auto` or `Some(NonZeroUsize)` if is a feature.
    fn variant(self) -> Option<u32>;
    /// New from feature variant.
    ///
    /// Returns `Auto` if `v == 0 || v > Self::names().len()`.
    fn from_variant(v: u32) -> Self;
    /// New `Auto`.
    fn auto() -> Self;
}

/// Represents `enum` like types that represents a exclusive set of features + `Auto`.
/// Some variants can have multiple features.
pub trait FontFeatureExclusiveSetsState: Copy + PartialEq + 'static {
    /// All the names of features, must have more then one sub-set.
    fn names() -> &'static [&'static [FontFeatureName]];
    /// `None` if `Auto` or `Some(NonZeroUsize)` if is a feature.
    fn variant(self) -> Option<u32>;
    /// New from feature variant.
    ///
    /// Returns `Auto` if `v == 0 || v > Self::names().len()`.
    fn from_variant(v: u32) -> Self;
    /// New `Auto`.
    fn auto() -> Self;
}

/// State of a [font feature](FontFeatures).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default)]
pub struct FontFeatureState(Option<u32>);
impl FontFeatureState {
    /// Automatic state.
    pub const fn auto() -> Self {
        FontFeatureState(None)
    }

    /// Enabled state.
    pub const fn enabled() -> Self {
        FontFeatureState(Some(1))
    }

    /// Enabled state with alternative selected.
    pub const fn enabled_alt(alt: NonZeroU32) -> Self {
        FontFeatureState(Some(alt.get()))
    }

    /// Disabled state.
    pub const fn disabled() -> Self {
        FontFeatureState(Some(0))
    }

    /// Is [`auto`](Self::auto).
    pub fn is_auto(self) -> bool {
        self.0.is_none()
    }

    /// Is [`enabled`](Self::enabled) or [`enabled_alt`](Self::enabled_alt).
    pub fn is_enabled(self) -> bool {
        if let Some(n) = self.0
            && n >= 1
        {
            return true;
        }
        false
    }

    /// Is [`disabled`](Self::disabled).
    pub fn is_disabled(self) -> bool {
        self == Self::disabled()
    }

    /// Gets the enabled alternative.
    pub fn alt(self) -> Option<u32> {
        if let Some(n) = self.0
            && n >= 1
        {
            return Some(n);
        }
        None
    }
}
impl fmt::Debug for FontFeatureState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Some(n) => {
                if n == FEATURE_DISABLED {
                    write!(f, "FontFeatureState::disabled()")
                } else if n == FEATURE_ENABLED {
                    write!(f, "FontFeatureState::enabled()")
                } else {
                    write!(f, "FontFeatureState::enabled_alt({n})")
                }
            }
            None => write!(f, "FontFeatureState::auto()"),
        }
    }
}
impl_from_and_into_var! {
    fn from(enabled: bool) -> FontFeatureState {
        if enabled {
            FontFeatureState::enabled()
        } else {
            FontFeatureState::disabled()
        }
    }

    /// `0` is disabled, `>=1` is enabled with the alt value.
    fn from(alt: u32) -> FontFeatureState {
        FontFeatureState(Some(alt))
    }
}

/// Font capital letters variant features.
#[derive(Copy, Clone, PartialEq, Eq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum CapsVariant {
    /// No caps variant.
    #[default]
    Auto,

    /// Enable small caps alternative for lowercase letters.
    ///
    /// This corresponds to OpenType `smcp` feature.
    SmallCaps,

    /// Enable small caps alternative for lower and upper case letters.
    ///
    /// This corresponds to OpenType `smcp` and `c2sc` features.
    AllSmallCaps,

    /// Enable petite caps alternative for lowercase letters.
    ///
    /// This corresponds to OpenType `pcap` feature.
    Petite,

    /// Enable petite caps alternative for lower and upper case letters.
    ///
    /// This corresponds to OpenType `pcap` and `c2pc` features.
    AllPetite,

    /// Enables unicase, using small caps for upper case letters mixed with normal lowercase letters.
    ///
    /// This corresponds to OpenType `unic` feature.
    Unicase,

    /// Enable title caps alternatives. This uses alternative uppercase glyphs designed for all uppercase words.
    ///
    /// This corresponds to OpenType `titl` feature.
    TitlingCaps,
}
impl fmt::Debug for CapsVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "CapsVariant::")?;
        }
        match self {
            CapsVariant::Auto => write!(f, "Auto"),
            CapsVariant::SmallCaps => write!(f, "SmallCaps"),
            CapsVariant::AllSmallCaps => write!(f, "AllSmallCaps"),
            CapsVariant::Petite => write!(f, "Petite"),
            CapsVariant::AllPetite => write!(f, "AllPetite"),
            CapsVariant::Unicase => write!(f, "Unicase"),
            CapsVariant::TitlingCaps => write!(f, "TitlingCaps"),
        }
    }
}
impl Default for CapsVariant {
    /// [`CapsVariant::Auto`]
    fn default() -> Self {
        CapsVariant::Auto
    }
}
impl FontFeatureExclusiveSetsState for CapsVariant {
    fn names() -> &'static [&'static [FontFeatureName]] {
        static N0: [FontFeatureName; 1] = [FontFeatureName(*b"smcp")];
        static N1: [FontFeatureName; 2] = [FontFeatureName(*b"c2sc"), FontFeatureName(*b"smcp")];
        static N2: [FontFeatureName; 1] = [FontFeatureName(*b"pcap")];
        static N3: [FontFeatureName; 2] = [FontFeatureName(*b"c2pc"), FontFeatureName(*b"pcap")];
        static N4: [FontFeatureName; 1] = [FontFeatureName(*b"unic")];
        static N5: [FontFeatureName; 1] = [FontFeatureName(*b"titl")];
        static NAMES: [&[FontFeatureName]; 6] = [&N0, &N1, &N2, &N3, &N4, &N5];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        if self == CapsVariant::Auto { None } else { Some(self as u32) }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        CapsVariant::Auto
    }
}

/// Font numeric variant features.
#[derive(Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum NumVariant {
    /// Uses the default numeric glyphs, in most fonts this is the same as `Lining`, some fonts use the `OldStyle`.
    #[default]
    Auto,
    /// Uses numeric glyphs that rest on the baseline.
    ///
    /// This corresponds to OpenType `lnum` feature.
    Lining,
    /// Uses old-style numeric glyphs, where some numbers, like 3, 4, 7, 9 have descenders.
    ///
    /// This corresponds to OpenType `onum` feature.
    OldStyle,
}
impl fmt::Debug for NumVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "NumVariant::")?;
        }
        match self {
            NumVariant::Auto => write!(f, "Auto"),
            NumVariant::Lining => write!(f, "Lining"),
            NumVariant::OldStyle => write!(f, "OldStyle"),
        }
    }
}
impl Default for NumVariant {
    /// [`NumVariant::Auto`]
    fn default() -> Self {
        NumVariant::Auto
    }
}
impl FontFeatureExclusiveSetState for NumVariant {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 2] = [FontFeatureName(*b"lnum"), FontFeatureName(*b"onum")];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        match self {
            NumVariant::Auto => None,
            NumVariant::Lining => Some(1),
            NumVariant::OldStyle => Some(2),
        }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        NumVariant::Auto
    }
}

/// Font numeric spacing features.
#[derive(Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum NumSpacing {
    /// Uses the default numeric width, usually this is `Tabular` for *monospace* fonts and `Proportional` for the others.
    #[default]
    Auto,
    /// Numeric glyphs take different space depending on the design of the glyph.
    ///
    /// This corresponds to OpenType `pnum` feature.
    Proportional,
    /// Numeric glyphs take the same space even if the glyphs design width is different.
    ///
    /// This corresponds to OpenType `tnum` feature.
    Tabular,
}
impl fmt::Debug for NumSpacing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "NumSpacing::")?;
        }
        match self {
            NumSpacing::Auto => write!(f, "Auto"),
            NumSpacing::Proportional => write!(f, "Proportional"),
            NumSpacing::Tabular => write!(f, "Tabular"),
        }
    }
}
impl Default for NumSpacing {
    /// [`NumSpacing::Auto`]
    fn default() -> Self {
        NumSpacing::Auto
    }
}
impl FontFeatureExclusiveSetState for NumSpacing {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 2] = [FontFeatureName(*b"pnum"), FontFeatureName(*b"tnum")];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        match self {
            NumSpacing::Auto => None,
            NumSpacing::Proportional => Some(1),
            NumSpacing::Tabular => Some(2),
        }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        NumSpacing::Auto
    }
}

/// Font numeric fraction features.
#[derive(Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum NumFraction {
    /// Don't use fraction variants.
    #[default]
    Auto,
    /// Variant where the numerator and denominator are made smaller and separated by a slash.
    ///
    /// This corresponds to OpenType `frac` feature.
    Diagonal,
    /// Variant where the numerator and denominator are made smaller, stacked and separated by a horizontal line.
    ///
    /// This corresponds to OpenType `afrc` feature.
    Stacked,
}
impl fmt::Debug for NumFraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "NumFraction::")?;
        }
        match self {
            NumFraction::Auto => write!(f, "Auto"),
            NumFraction::Diagonal => write!(f, "Diagonal"),
            NumFraction::Stacked => write!(f, "Stacked"),
        }
    }
}
impl Default for NumFraction {
    /// [`NumFraction::Auto`]
    fn default() -> Self {
        NumFraction::Auto
    }
}
impl FontFeatureExclusiveSetState for NumFraction {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 2] = [FontFeatureName(*b"frac"), FontFeatureName(*b"afrc")];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        match self {
            NumFraction::Auto => None,
            NumFraction::Diagonal => Some(1),
            NumFraction::Stacked => Some(2),
        }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        NumFraction::Auto
    }
}

/// All possible [style_set](FontFeatures::style_set) features.
///
/// The styles depend on the font, it is recommended you create an `enum` with named sets that
/// converts into this one for each font you wish to use.
#[derive(Copy, Clone, Eq, PartialEq, Hash, FromPrimitive)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum FontStyleSet {
    /// Don't use alternative style set.
    #[default]
    Auto = 0,

    S01,
    S02,
    S03,
    S04,
    S05,
    S06,
    S07,
    S08,
    S09,
    S10,

    S11,
    S12,
    S13,
    S14,
    S15,
    S16,
    S17,
    S18,
    S19,
    S20,
}
impl Default for FontStyleSet {
    /// [`FontStyleSet::Auto`]
    fn default() -> Self {
        FontStyleSet::Auto
    }
}
impl fmt::Debug for FontStyleSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "FontStyleSet::")?;
        }
        let n = *self as u8;
        if n == 0 { write!(f, "Auto") } else { write!(f, "S{n:0<2}") }
    }
}
impl_from_and_into_var! {
    fn from(set: u8) -> FontStyleSet;
}

impl FontFeatureExclusiveSetState for FontStyleSet {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 20] = [
            FontFeatureName(*b"ss01"),
            FontFeatureName(*b"ss02"),
            FontFeatureName(*b"ss03"),
            FontFeatureName(*b"ss04"),
            FontFeatureName(*b"ss05"),
            FontFeatureName(*b"ss06"),
            FontFeatureName(*b"ss07"),
            FontFeatureName(*b"ss08"),
            FontFeatureName(*b"ss09"),
            FontFeatureName(*b"ss10"),
            FontFeatureName(*b"ss11"),
            FontFeatureName(*b"ss12"),
            FontFeatureName(*b"ss13"),
            FontFeatureName(*b"ss14"),
            FontFeatureName(*b"ss15"),
            FontFeatureName(*b"ss16"),
            FontFeatureName(*b"ss17"),
            FontFeatureName(*b"ss18"),
            FontFeatureName(*b"ss19"),
            FontFeatureName(*b"ss20"),
        ];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        if self == FontStyleSet::Auto { None } else { Some(self as u32) }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        FontStyleSet::Auto
    }
}

/// All possible [char_variant](FontFeatures::char_variant) features (`cv00..=cv99`).
///
/// The styles depend on the font, it is recommended you create `const`s with named variants to use with a specific font.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct CharVariant(u8);
impl CharVariant {
    /// New variant.
    ///
    /// `v == 0 || v > 99` is Auto, `v >= 1 && v <= 99` maps to their variant.
    pub const fn new(v: u8) -> Self {
        if v > 99 { CharVariant(0) } else { CharVariant(v) }
    }

    /// New auto.
    pub const fn auto() -> Self {
        CharVariant(0)
    }

    /// Is auto.
    pub const fn is_auto(self) -> bool {
        self.0 == 0
    }
}
impl_from_and_into_var! {
    /// `v == 0 || v > 99` is Auto, `v >= 1 && v <= 99` maps to their variant.
    fn from(v: u8) -> CharVariant {
        CharVariant::new(v)
    }
}
impl FontFeatureExclusiveSetState for CharVariant {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 100] = [
            FontFeatureName(*b"cv01"),
            FontFeatureName(*b"cv02"),
            FontFeatureName(*b"cv03"),
            FontFeatureName(*b"cv04"),
            FontFeatureName(*b"cv05"),
            FontFeatureName(*b"cv06"),
            FontFeatureName(*b"cv07"),
            FontFeatureName(*b"cv08"),
            FontFeatureName(*b"cv09"),
            FontFeatureName(*b"cv20"),
            FontFeatureName(*b"cv21"),
            FontFeatureName(*b"cv22"),
            FontFeatureName(*b"cv23"),
            FontFeatureName(*b"cv24"),
            FontFeatureName(*b"cv25"),
            FontFeatureName(*b"cv26"),
            FontFeatureName(*b"cv27"),
            FontFeatureName(*b"cv28"),
            FontFeatureName(*b"cv29"),
            FontFeatureName(*b"cv30"),
            FontFeatureName(*b"cv31"),
            FontFeatureName(*b"cv32"),
            FontFeatureName(*b"cv33"),
            FontFeatureName(*b"cv34"),
            FontFeatureName(*b"cv35"),
            FontFeatureName(*b"cv36"),
            FontFeatureName(*b"cv37"),
            FontFeatureName(*b"cv38"),
            FontFeatureName(*b"cv39"),
            FontFeatureName(*b"cv40"),
            FontFeatureName(*b"cv41"),
            FontFeatureName(*b"cv42"),
            FontFeatureName(*b"cv43"),
            FontFeatureName(*b"cv44"),
            FontFeatureName(*b"cv45"),
            FontFeatureName(*b"cv46"),
            FontFeatureName(*b"cv47"),
            FontFeatureName(*b"cv48"),
            FontFeatureName(*b"cv49"),
            FontFeatureName(*b"cv50"),
            FontFeatureName(*b"cv51"),
            FontFeatureName(*b"cv52"),
            FontFeatureName(*b"cv53"),
            FontFeatureName(*b"cv54"),
            FontFeatureName(*b"cv55"),
            FontFeatureName(*b"cv56"),
            FontFeatureName(*b"cv57"),
            FontFeatureName(*b"cv58"),
            FontFeatureName(*b"cv59"),
            FontFeatureName(*b"cv60"),
            FontFeatureName(*b"cv61"),
            FontFeatureName(*b"cv62"),
            FontFeatureName(*b"cv63"),
            FontFeatureName(*b"cv64"),
            FontFeatureName(*b"cv65"),
            FontFeatureName(*b"cv66"),
            FontFeatureName(*b"cv67"),
            FontFeatureName(*b"cv68"),
            FontFeatureName(*b"cv69"),
            FontFeatureName(*b"cv70"),
            FontFeatureName(*b"cv71"),
            FontFeatureName(*b"cv72"),
            FontFeatureName(*b"cv73"),
            FontFeatureName(*b"cv74"),
            FontFeatureName(*b"cv75"),
            FontFeatureName(*b"cv76"),
            FontFeatureName(*b"cv77"),
            FontFeatureName(*b"cv78"),
            FontFeatureName(*b"cv79"),
            FontFeatureName(*b"cv70"),
            FontFeatureName(*b"cv71"),
            FontFeatureName(*b"cv72"),
            FontFeatureName(*b"cv73"),
            FontFeatureName(*b"cv74"),
            FontFeatureName(*b"cv75"),
            FontFeatureName(*b"cv76"),
            FontFeatureName(*b"cv77"),
            FontFeatureName(*b"cv78"),
            FontFeatureName(*b"cv79"),
            FontFeatureName(*b"cv80"),
            FontFeatureName(*b"cv81"),
            FontFeatureName(*b"cv82"),
            FontFeatureName(*b"cv83"),
            FontFeatureName(*b"cv84"),
            FontFeatureName(*b"cv85"),
            FontFeatureName(*b"cv86"),
            FontFeatureName(*b"cv87"),
            FontFeatureName(*b"cv88"),
            FontFeatureName(*b"cv89"),
            FontFeatureName(*b"cv90"),
            FontFeatureName(*b"cv91"),
            FontFeatureName(*b"cv92"),
            FontFeatureName(*b"cv93"),
            FontFeatureName(*b"cv94"),
            FontFeatureName(*b"cv95"),
            FontFeatureName(*b"cv96"),
            FontFeatureName(*b"cv97"),
            FontFeatureName(*b"cv98"),
            FontFeatureName(*b"cv99"),
            FontFeatureName(*b"cv99"),
        ];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        if self.is_auto() { None } else { Some(self.0 as u32) }
    }

    fn from_variant(v: u32) -> Self {
        if v > 99 { CharVariant::auto() } else { CharVariant(v as u8) }
    }

    fn auto() -> Self {
        CharVariant::auto()
    }
}

/// Sub-script and super-script variants.
#[derive(Copy, Clone, PartialEq, Eq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum FontPosition {
    /// Don't use sub/super script positions.
    #[default]
    Auto,
    /// Uses sub-script position and alternative glyphs.
    ///
    /// This corresponds to OpenType `subs` feature.
    Sub,
    /// Uses super-script position and alternative glyphs.
    ///
    /// This corresponds to OpenType `sups` feature.
    Super,
}
impl fmt::Debug for FontPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "FontPosition::")?;
        }
        match self {
            FontPosition::Auto => write!(f, "Auto"),
            FontPosition::Sub => write!(f, "Sub"),
            FontPosition::Super => write!(f, "Super"),
        }
    }
}
impl Default for FontPosition {
    /// [`FontPosition::Auto`]
    fn default() -> Self {
        FontPosition::Auto
    }
}
impl FontFeatureExclusiveSetState for FontPosition {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 2] = [FontFeatureName(*b"subs"), FontFeatureName(*b"sups")];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        match self {
            FontPosition::Auto => None,
            FontPosition::Sub => Some(1),
            FontPosition::Super => Some(2),
        }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        FontPosition::Auto
    }
}

/// Logographic glyph variants for Japanese fonts.
#[derive(Copy, Clone, PartialEq, Eq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum JpVariant {
    /// Uses the font default glyphs.
    #[default]
    Auto,

    /// JIS X 0208-1978 (first standard)
    ///
    /// This corresponds to OpenType `jp78` feature.
    Jis78,
    /// JIS X 0208-1983 (second standard)
    ///
    /// This corresponds to OpenType `jp83` feature.
    Jis83,
    /// JIS X 0208-1990 (third standard)
    ///
    /// This corresponds to OpenType `jp90` feature.
    Jis90,

    /// JIS X 0213 (2004)
    ///
    /// This corresponds to OpenType `jp04` feature.
    Jis04,
    /// NLC new shapes for JIS (2000).
    ///
    /// This corresponds to OpenType `nlck` feature.
    NlcKanji,
}
impl fmt::Debug for JpVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "JpVariant::")?;
        }
        match self {
            JpVariant::Auto => write!(f, "Auto"),
            JpVariant::Jis78 => write!(f, "Jis78"),
            JpVariant::Jis83 => write!(f, "Jis83"),
            JpVariant::Jis90 => write!(f, "Jis90"),
            JpVariant::Jis04 => write!(f, "Jis04"),
            JpVariant::NlcKanji => write!(f, "NlcKanji"),
        }
    }
}
impl Default for JpVariant {
    /// [`JpVariant::Auto`]
    fn default() -> Self {
        JpVariant::Auto
    }
}
impl FontFeatureExclusiveSetState for JpVariant {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 5] = [
            FontFeatureName(*b"jp78"),
            FontFeatureName(*b"jp83"),
            FontFeatureName(*b"jp90"),
            FontFeatureName(*b"jp04"),
            FontFeatureName(*b"nlck"),
        ];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        if self == JpVariant::Auto { None } else { Some(self as u32) }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        JpVariant::Auto
    }
}
/// Logographic glyph variants for Chinese fonts.
#[derive(Copy, Clone, PartialEq, Eq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum CnVariant {
    /// Uses the font default glyphs.
    #[default]
    Auto,
    /// Simplified Chinese glyphs.
    ///
    /// This corresponds to OpenType `smpl` feature.
    Simplified,
    /// Traditional Chinese glyphs.
    ///
    /// This corresponds to OpenType `trad` feature.
    Traditional,
}
impl fmt::Debug for CnVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "CnVariant")?;
        }
        match self {
            CnVariant::Auto => write!(f, "Auto"),
            CnVariant::Simplified => write!(f, "Simplified"),
            CnVariant::Traditional => write!(f, "Traditional"),
        }
    }
}
impl Default for CnVariant {
    /// [`CnVariant::Auto`]
    fn default() -> Self {
        CnVariant::Auto
    }
}
impl FontFeatureExclusiveSetState for CnVariant {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 2] = [FontFeatureName(*b"smpl"), FontFeatureName(*b"trad")];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        match self {
            CnVariant::Auto => None,
            CnVariant::Simplified => Some(1),
            CnVariant::Traditional => Some(2),
        }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        CnVariant::Auto
    }
}

/// The sizing and spacing of figures used for East Asian characters.
#[derive(Copy, Clone, PartialEq, Eq, Hash, FromPrimitive)]
#[repr(u8)]
pub enum EastAsianWidth {
    /// Uses the font default glyphs and spacing.
    #[default]
    Auto,

    /// Uses the set of glyphs designed for proportional spacing.
    ///
    /// This corresponds to OpenType `pwid` feature.
    Proportional,

    /// Uses the set of glyphs designed for full-width but re-spaced to take proportional space.
    ///
    /// This corresponds to OpenType `palt` feature.
    ProportionalAlt,

    /// Like [`Proportional`](Self::Proportional) but only affects kana and kana related glyphs.
    ///
    /// This corresponds to OpenType `pkna` feature.
    ProportionalKana,

    /// Uses the set of glyphs designed for full-width monospace.
    ///
    /// This corresponds to OpenType `fwid` feature.
    Full,

    /// Uses the set of glyphs designed for half-width monospace.
    ///
    /// This corresponds to OpenType `hwid` feature.
    Half,

    /// Uses the set of glyphs designed for full-width but re-spaced to take half-width monospace.
    ///
    /// This corresponds to OpenType `halt` feature.
    HalfAlt,

    /// Uses the set of glyphs designed for a third-width monospace.
    ///
    /// This corresponds to OpenType `twid` feature.
    Third,

    /// Uses the set of glyphs designed for a quarter-width monospace.
    ///
    /// This corresponds to OpenType `qwid` feature.
    Quarter,
}
impl fmt::Debug for EastAsianWidth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "EastAsianWidth::")?;
        }
        match self {
            EastAsianWidth::Auto => write!(f, "Auto"),
            EastAsianWidth::Proportional => write!(f, "Proportional"),
            EastAsianWidth::ProportionalAlt => write!(f, "ProportionalAlt"),
            EastAsianWidth::ProportionalKana => write!(f, "ProportionalKana"),
            EastAsianWidth::Full => write!(f, "Full"),
            EastAsianWidth::Half => write!(f, "Half"),
            EastAsianWidth::HalfAlt => write!(f, "HalfAlt"),
            EastAsianWidth::Third => write!(f, "Third"),
            EastAsianWidth::Quarter => write!(f, "Quarter"),
        }
    }
}
impl Default for EastAsianWidth {
    /// [`EastAsianWidth::Auto`]
    fn default() -> Self {
        EastAsianWidth::Auto
    }
}
impl FontFeatureExclusiveSetState for EastAsianWidth {
    fn names() -> &'static [FontFeatureName] {
        static NAMES: [FontFeatureName; 8] = [
            FontFeatureName(*b"pwid"),
            FontFeatureName(*b"palt"),
            FontFeatureName(*b"pkna"),
            FontFeatureName(*b"fwid"),
            FontFeatureName(*b"hwid"),
            FontFeatureName(*b"halt"),
            FontFeatureName(*b"twid"),
            FontFeatureName(*b"qwid"),
        ];
        &NAMES
    }

    fn variant(self) -> Option<u32> {
        if self == EastAsianWidth::Auto { None } else { Some(self as u32) }
    }

    fn from_variant(v: u32) -> Self {
        Self::from(v as u8)
    }

    fn auto() -> Self {
        EastAsianWidth::Auto
    }
}

/// Name of a font variation axis.
///
/// # Examples
///
/// ```
/// # use zng_ext_font::font_features::*;
/// let historical_lig: FontVariationName = b"BLDB".into();
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FontVariationName(pub [u8; 4]);
impl FontVariationName {
    /// As UTF-8.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}
impl From<&'static [u8; 4]> for FontVariationName {
    fn from(name: &'static [u8; 4]) -> Self {
        FontVariationName(*name)
    }
}
impl From<FontVariationName> for ttf_parser::Tag {
    fn from(value: FontVariationName) -> Self {
        ttf_parser::Tag::from_bytes(&value.0)
    }
}
impl ops::Deref for FontVariationName {
    type Target = [u8; 4];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl fmt::Debug for FontVariationName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.as_str().is_empty() {
            write!(f, "{:?}", self.0)
        } else {
            write!(f, "{}", self.as_str())
        }
    }
}
impl fmt::Display for FontVariationName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

/// A small map of font variations.
///
/// Use [`font_variations!`] to manually initialize.
#[derive(Default, Clone, PartialEq)]
pub struct FontVariations(Vec<(FontVariationName, f32)>);
impl FontVariations {
    /// New empty.
    pub fn new() -> Self {
        Self::default()
    }

    /// New empty with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// New font variations from pairs of name, value.
    pub fn from_pairs(pairs: &[(FontVariationName, f32)]) -> Self {
        let mut r = Self::with_capacity(pairs.len());
        for (name, value) in pairs {
            r.insert(*name, *value);
        }
        r
    }

    /// Insert the font variation, returns the previous value if the variation was already set.
    pub fn insert(&mut self, name: FontVariationName, value: f32) -> Option<f32> {
        if let Some(entry) = self.0.iter_mut().find(|v| v.0 == name) {
            let prev = Some(entry.1);
            entry.1 = value;
            prev
        } else {
            self.0.push((name, value));
            None
        }
    }

    /// Remove the font variation, returns the value if the variation was set.
    pub fn remove(&mut self, name: FontVariationName) -> Option<f32> {
        if let Some(i) = self.0.iter().position(|v| v.0 == name) {
            Some(self.0.swap_remove(i).1)
        } else {
            None
        }
    }

    /// If the variation is set.
    pub fn contains(&self, name: FontVariationName) -> bool {
        self.0.iter().any(|v| v.0 == name)
    }

    /// Gets a copy of the variation value if it is set.
    pub fn get(&self, name: FontVariationName) -> Option<f32> {
        self.0.iter().find(|v| v.0 == name).map(|v| v.1)
    }

    /// Exclusive borrow the variation value if it is set.
    pub fn get_mut(&mut self, name: FontVariationName) -> Option<&mut f32> {
        self.0.iter_mut().find(|v| v.0 == name).map(|v| &mut v.1)
    }

    /// Count of font variations set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// If not font variation is set.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Finalize variations config for use in a font.
    pub fn finalize(&self) -> RFontVariations {
        self.0
            .iter()
            .map(|(name, value)| rustybuzz::Variation {
                tag: (*name).into(),
                value: *value,
            })
            .collect()
    }
}
impl fmt::Debug for FontVariations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("FontVariations").field(&self.0).finish()
        } else {
            write!(f, "[")?;
            let mut first = false;
            for entry in &self.0 {
                if first {
                    first = false;
                } else {
                    write!(f, ", ")?;
                }
                write!(f, r#", b"{}": {}"#, entry.0, entry.1)?;
            }
            write!(f, "]")
        }
    }
}

/// Initialize a [`FontVariations`] map.
///
/// # Examples
///
/// ```rust,no_fmt
/// # use zng_ext_font::font_features::*;
/// # fn assert_type(_: FontVariations) { }
/// let variations = font_variations! {
///     b"SKLA" => 1000.0,
///     b"TRMG" => 750.0
/// };
/// # assert_type(variations);
/// ```
#[macro_export]
macro_rules! font_variations {
    [$(
        $name:tt => $value: expr
    ),* $(,)?] => {
        $crate::font_features::FontVariations::from_pairs(&[
            $(
                ($name.into(), $value),
            )*
        ])
    }
}
#[doc(inline)]
pub use font_variations;
use zng_var::impl_from_and_into_var;

/// Finalized [`FontVariations`].
///
/// This is a vec of [harfbuzz variations](https://docs.rs/rustybuzz/0.17.0/rustybuzz/struct.Variation.html).
pub type RFontVariations = Vec<rustybuzz::Variation>;
