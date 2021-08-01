//! Font features and variation types.

use crate::{
    crate_util::{FxHashMap, FxHashSet},
    var::impl_from_and_into_var,
};
use std::{collections::hash_map::Entry as HEntry, fmt, marker::PhantomData, mem, num::NonZeroU32};

// TODO
// main: https://developer.mozilla.org/en-US/docs/Web/CSS/font-feature-settings
// 5 - https://helpx.adobe.com/pt/fonts/user-guide.html/pt/fonts/using/open-type-syntax.ug.html#calt
// review - https://harfbuzz.github.io/shaping-opentype-features.html

/// Name of a font feature.
///
/// # Example
///
/// ```
/// # use zero_ui_core::text::font_features::FontFeatureName;
/// let historical_lig: FontFeatureName = b"hlig";
/// ```
pub type FontFeatureName = &'static [u8; 4];

/// The raw value used when a feature is set to `true`.
pub const FEATURE_ENABLED: u32 = 1;
/// The raw value used when a feature is set to `false`.
pub const FEATURE_DISABLED: u32 = 0;

type FontFeaturesMap = FxHashMap<FontFeatureName, u32>;

/// Font features configuration.
#[derive(Default, Clone)]
pub struct FontFeatures(FontFeaturesMap);
impl FontFeatures {
    /// New default.
    #[inline]
    pub fn new() -> FontFeatures {
        FontFeatures::default()
    }

    /// New builder.
    #[inline]
    pub fn builder() -> FontFeaturesBuilder {
        FontFeaturesBuilder::default()
    }

    /// Set or override the features of `self` from `other`.
    ///
    /// Returns the previous state of all affected names.
    #[inline]
    pub fn set_all(&mut self, other: &FontFeatures) -> Vec<(FontFeatureName, Option<u32>)> {
        let mut prev = Vec::with_capacity(other.0.len());
        for (&name, &state) in other.0.iter() {
            prev.push((name, self.0.insert(name, state)));
        }
        prev
    }

    /// Restore feature states that where overridden in [`set_all`](Self::set_all).
    #[inline]
    pub fn restore(&mut self, prev: Vec<(FontFeatureName, Option<u32>)>) {
        for (name, state) in prev {
            match state {
                Some(state) => {
                    self.0.insert(name, state);
                }
                None => {
                    self.0.remove(name);
                }
            }
        }
    }

    /// Access to the named feature.
    #[inline]
    pub fn feature(&mut self, name: FontFeatureName) -> FontFeature {
        FontFeature(self.0.entry(name))
    }

    /// Access to a set of named features that are managed together.
    ///
    /// # Panics
    ///
    /// If `names` has less than 2 names.
    #[inline]
    pub fn feature_set(&mut self, names: &'static [FontFeatureName]) -> FontFeatureSet {
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
    pub fn feature_exclusive_set<S: FontFeatureExclusiveSetState>(&mut self) -> FontFeatureExclusiveSet<S> {
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
    pub fn feature_exclusive_sets<S: FontFeatureExclusiveSetsState>(&mut self) -> FontFeatureExclusiveSets<S> {
        assert!(S::names().len() >= 2);
        FontFeatureExclusiveSets {
            features: &mut self.0,
            _t: PhantomData,
        }
    }

    /// Generate the rustybuzz font features.
    #[inline]
    pub fn finalize(&self) -> RFontFeatures {
        self.0
            .iter()
            .map(|(&n, &s)| rustybuzz::Feature::new(rustybuzz::Tag::from_bytes(n), s, 0..usize::MAX))
            .collect()
    }
}
impl fmt::Debug for FontFeatures {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        for (name, state) in self.0.iter() {
            map.entry(&name_to_str(name), state);
        }
        map.finish()
    }
}

/// Finalized [`FontFeatures`].
///
/// This is a vec of [rustybuzz features](rustybuzz::Feature).
pub type RFontFeatures = Vec<rustybuzz::Feature>;

fn name_to_str(name: &[u8; 4]) -> &str {
    std::str::from_utf8(name).unwrap_or_default()
}

/// A builder for [`FontFeatures`].
///
/// # Example
///
/// ```
/// # use zero_ui_core::text::FontFeatures;
/// let features = FontFeatures::builder().kerning(false).build();
/// ```
#[derive(Default)]
pub struct FontFeaturesBuilder(FontFeatures);
impl FontFeaturesBuilder {
    /// Finish building.
    #[inline]
    pub fn build(self) -> FontFeatures {
        self.0
    }

    /// Set the named feature.
    #[inline]
    pub fn feature(mut self, name: FontFeatureName, state: impl Into<FontFeatureState>) -> Self {
        self.0.feature(name).set(state);
        self
    }

    /// Sets all the named features to the same value.
    ///
    /// # Panics
    ///
    /// If `names` has less than 2 names.
    #[inline]
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
        #[inline]
        pub fn $name(&mut self) -> FontFeatureSet {
            self.feature_set(&[$feat0, $feat1])
        }

    };

    (feature $(#[$docs:meta])* fn $name:ident($feat0:tt);) => {
        $(#[$docs])*
        #[inline]
        pub fn $name(&mut self) -> FontFeature {
            self.feature($feat0)
        }

    };

    (feature $(#[$docs:meta])* fn $name:ident($Enum:ident) -> $Helper:ident;) => {
        $(#[$docs])*
        #[inline]
        pub fn $name(&mut self) -> $Helper<$Enum> {
            $Helper { features: &mut self.0, _t: PhantomData }
        }

    };

    (builder $(#[$docs:meta])* fn $name:ident();) => {
        $(#[$docs])*
        #[inline]
        pub fn $name(mut self, state: impl Into<FontFeatureState>) -> Self {
            self.0.$name().set(state);
            self
        }

    };

    (builder $(#[$docs:meta])* fn $name:ident($Enum:ident -> $Helper:ident);) => {
        $(#[$docs])*
        #[inline]
        pub fn $name(mut self, state: impl Into<$Enum>) -> Self {
            self.0.$name().set(state);
            self
        }

    };
}

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
pub struct FontFeature<'a>(HEntry<'a, FontFeatureName, u32>);
impl<'a> FontFeature<'a> {
    /// Gets the OpenType name of the feature.
    #[inline]
    pub fn name(&self) -> FontFeatureName {
        self.0.key()
    }

    /// Gets the current state of the feature.
    pub fn state(&self) -> FontFeatureState {
        match &self.0 {
            HEntry::Occupied(e) => FontFeatureState(Some(*e.get())),
            HEntry::Vacant(_) => FontFeatureState::auto(),
        }
    }

    /// If the feature is explicitly enabled.
    pub fn is_enabled(&self) -> bool {
        self.state().is_enabled()
    }

    /// If the feature is explicitly disabled.
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.state().is_disabled()
    }

    /// If the feature is auto enabled zero-ui.
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.state().is_auto()
    }

    /// Set the feature state.
    ///
    /// Returns the previous state.
    #[inline]
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
            HEntry::Occupied(mut e) => {
                e.insert(state);
            }
            HEntry::Vacant(e) => {
                e.insert(state);
            }
        }
    }

    /// Enable the feature.
    #[inline]
    pub fn enable(self) {
        self.set_explicit(FEATURE_ENABLED);
    }

    /// Enable the feature with alternative selection.
    #[inline]
    pub fn enable_alt(self, alt: NonZeroU32) {
        self.set_explicit(alt.get())
    }

    /// Disable the feature.
    #[inline]
    pub fn disable(self) {
        self.set_explicit(FEATURE_DISABLED);
    }

    /// Set the feature to auto.
    #[inline]
    pub fn auto(self) {
        if let HEntry::Occupied(e) = self.0 {
            e.remove();
        }
    }
}
impl<'a> fmt::Debug for FontFeature<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b\"{}\": {:?}", name_to_str(self.name()), self.state())
    }
}

/// Represents a set of features in a [`FontFeatures`] configuration, the features state is managed together.
pub struct FontFeatureSet<'a> {
    features: &'a mut FontFeaturesMap,
    names: &'static [FontFeatureName],
}
impl<'a> FontFeatureSet<'a> {
    /// Gets the OpenType name of the features.
    #[inline]
    pub fn names(&self) -> &'static [FontFeatureName] {
        self.names
    }

    /// Gets the current state of the features.
    ///
    /// Returns `Auto` if the features are mixed.
    #[inline]
    pub fn state(&self) -> FontFeatureState {
        if let Some(&a) = self.features.get(self.names[0]) {
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
    #[inline]
    pub fn is_disabled(&self) -> bool {
        self.state().is_disabled()
    }

    /// If the features are auto enabled zero-ui, or in a mixed state.
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.state().is_auto()
    }

    /// Set the feature state.
    ///
    /// Returns the previous state.
    #[inline]
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
            self.features.insert(name, state);
        }
    }

    /// Enable the feature.
    #[inline]
    pub fn enable(self) {
        self.set_explicit(FEATURE_ENABLED);
    }

    /// Disable the feature.
    #[inline]
    pub fn disable(self) {
        self.set_explicit(FEATURE_DISABLED);
    }

    /// Set the feature to auto.
    #[inline]
    pub fn auto(self) {
        for name in self.names {
            self.features.remove(name);
        }
    }
}
impl<'a> fmt::Debug for FontFeatureSet<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?}: {:?}",
            self.names.iter().map(|s| name_to_str(s)).collect::<Vec<_>>(),
            self.state()
        )
    }
}

/// Represents a set of exclusive boolean in a [`FontFeatures`] configuration, only one
/// of the feature is enabled at a time.
pub struct FontFeatureExclusiveSet<'a, S: FontFeatureExclusiveSetState> {
    features: &'a mut FontFeaturesMap,
    _t: PhantomData<S>,
}
impl<'a, S: FontFeatureExclusiveSetState> FontFeatureExclusiveSet<'a, S> {
    /// Gets the OpenType names of all the features affected.
    #[inline]
    pub fn names(&self) -> &'static [FontFeatureName] {
        S::names()
    }

    /// Gets the current state of the features.
    #[inline]
    pub fn state(&self) -> S {
        let mut state = 0;

        for (i, name) in S::names().iter().enumerate() {
            if let Some(&s) = self.features.get(name) {
                if s == FEATURE_ENABLED && state == 0 {
                    state = i + 1; // found state.
                    continue;
                }
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
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.state() == S::auto()
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    #[inline]
    pub fn set(&mut self, state: impl Into<S>) -> S {
        let prev = self.take_state();
        if let Some(state) = state.into().variant() {
            self.features.insert(self.names()[state as usize - 1], FEATURE_ENABLED);
        }
        prev
    }
}
impl<'a, S: FontFeatureExclusiveSetState + fmt::Debug> fmt::Debug for FontFeatureExclusiveSet<'a, S> {
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
impl<'a, S: FontFeatureExclusiveSetsState> FontFeatureExclusiveSets<'a, S> {
    /// Gets the OpenType names of all the features affected.
    #[inline]
    pub fn names(&self) -> &'static [&'static [FontFeatureName]] {
        S::names()
    }

    /// Gets the current state of the features.
    #[inline]
    pub fn state(&self) -> S {
        let mut active = FxHashSet::default();
        for &names in self.names() {
            for &name in names {
                if let Some(&s) = self.features.get(name) {
                    if s != FEATURE_ENABLED {
                        // custom external state, we only set to FEATURE_ENABLED.
                        return S::auto();
                    } else {
                        active.insert(name);
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
        let mut active = FxHashSet::default();
        let mut force_auto = false;

        for &names in self.names() {
            for &name in names {
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
    #[inline]
    pub fn is_auto(&self) -> bool {
        self.state() == S::auto()
    }

    /// Sets the features.
    ///
    /// Returns the previous state.
    #[inline]
    pub fn set(&mut self, state: impl Into<S>) -> S {
        let prev = self.take_state();
        if let Some(state) = state.into().variant() {
            for name in self.names()[state as usize - 1] {
                self.features.insert(name, FEATURE_ENABLED);
            }
        }
        prev
    }
}
impl<'a, S: FontFeatureExclusiveSetsState + fmt::Debug> fmt::Debug for FontFeatureExclusiveSets<'a, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.state(), f)
    }
}

/// An `enum` like type that represents a exclusive set of features + `Auto`.
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

/// An `enum` like type that represents a exclusive set of features + `Auto`.
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
    #[inline]
    pub const fn auto() -> Self {
        FontFeatureState(None)
    }

    /// Enabled state.
    #[inline]
    pub const fn enabled() -> Self {
        FontFeatureState(Some(1))
    }

    /// Enabled state with alternative selected.
    #[inline]
    pub const fn enabled_alt(alt: NonZeroU32) -> Self {
        FontFeatureState(Some(alt.get()))
    }

    /// Disabled state.
    #[inline]
    pub const fn disabled() -> Self {
        FontFeatureState(Some(0))
    }

    /// Is [`auto`](Self::auto).
    #[inline]
    pub fn is_auto(self) -> bool {
        self.0.is_none()
    }

    /// Is [`enabled`](Self::enabled) or [`enabled_alt`](Self::enabled_alt).
    #[inline]
    pub fn is_enabled(self) -> bool {
        if let Some(n) = self.0 {
            if n >= 1 {
                return true;
            }
        }
        false
    }

    /// Is [`disabled`](Self::disabled).
    #[inline]
    pub fn is_disabled(self) -> bool {
        self == Self::disabled()
    }

    /// Gets the enabled alternative.
    #[inline]
    pub fn alt(self) -> Option<u32> {
        if let Some(n) = self.0 {
            if n >= 1 {
                return Some(n);
            }
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
                    write!(f, "FontFeatureState::enabled_alt({})", n)
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
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum CapsVariant {
    /// No caps variant.
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
    #[inline]
    fn names() -> &'static [&'static [FontFeatureName]] {
        &[
            &[b"smcp"],
            &[b"c2sc", b"smcp"],
            &[b"pcap"],
            &[b"c2pc", b"pcap"],
            &[b"unic"],
            &[b"titl"],
        ]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        if self == CapsVariant::Auto {
            None
        } else {
            Some(self as u32)
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        if v as usize > Self::names().len() {
            CapsVariant::Auto
        } else {
            // SAFETY:
            unsafe { mem::transmute(v as u8) }
        }
    }

    #[inline]
    fn auto() -> Self {
        CapsVariant::Auto
    }
}

/// Font numeric variant features.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumVariant {
    /// Uses the default numeric glyphs, in most fonts this is the same as `Lining`, some fonts use the `OldStyle`.
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
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[b"lnum", b"onum"]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        match self {
            NumVariant::Auto => None,
            NumVariant::Lining => Some(1),
            NumVariant::OldStyle => Some(2),
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        match v {
            1 => NumVariant::Lining,
            2 => NumVariant::OldStyle,
            _ => NumVariant::Auto,
        }
    }

    #[inline]
    fn auto() -> Self {
        NumVariant::Auto
    }
}

/// Font numeric spacing features.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumSpacing {
    /// Uses the default numeric width, usually this is `Tabular` for *monospace* fonts and `Proportional` for the others.
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
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[b"pnum", b"tnum"]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        match self {
            NumSpacing::Auto => None,
            NumSpacing::Proportional => Some(1),
            NumSpacing::Tabular => Some(2),
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        match v {
            1 => NumSpacing::Proportional,
            2 => NumSpacing::Tabular,
            _ => NumSpacing::Auto,
        }
    }

    #[inline]
    fn auto() -> Self {
        NumSpacing::Auto
    }
}

/// Font numeric fraction features.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum NumFraction {
    /// Don't use fraction variants.
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
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[b"frac", b"afrc"]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        match self {
            NumFraction::Auto => None,
            NumFraction::Diagonal => Some(1),
            NumFraction::Stacked => Some(2),
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        match v {
            1 => NumFraction::Diagonal,
            2 => NumFraction::Stacked,
            _ => NumFraction::Auto,
        }
    }

    #[inline]
    fn auto() -> Self {
        NumFraction::Auto
    }
}

/// All possible [style_set](FontFeatures::style_set) features.
///
/// The styles depend on the font, it is recommended you create an `enum` with named sets that
/// converts into this one for each font you wish to use.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum FontStyleSet {
    /// Don't use alternative style set.
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
        if n == 0 {
            write!(f, "Auto")
        } else {
            write!(f, "S{:0<2}", n)
        }
    }
}
impl_from_and_into_var! {
    /// `set == 0 || set > 20` is Auto, `set >= 1 && set <= 20` maps to their variant.
    fn from(set: u8) -> FontStyleSet {
        if set > 20 {
            FontStyleSet::Auto
        } else {
            // SAFETY: We eliminated the bad values in the `if`.
            unsafe { mem::transmute(set) }
        }
    }
}
impl FontFeatureExclusiveSetState for FontStyleSet {
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[
            b"ss01", b"ss02", b"ss03", b"ss04", b"ss05", b"ss06", b"ss07", b"ss08", b"ss09", b"ss10", b"ss11", b"ss12", b"ss13", b"ss14",
            b"ss15", b"ss16", b"ss17", b"ss18", b"ss19", b"ss20",
        ]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        if self == FontStyleSet::Auto {
            None
        } else {
            Some(self as u32)
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        if v > 20 {
            FontStyleSet::Auto
        } else {
            // SAFETY: we validated the input in the `if`.
            unsafe { mem::transmute(v as u8) }
        }
    }

    #[inline]
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
    #[inline]
    pub const fn new(v: u8) -> Self {
        if v > 99 {
            CharVariant(0)
        } else {
            CharVariant(v)
        }
    }

    /// New auto.
    #[inline]
    pub const fn auto() -> Self {
        CharVariant(0)
    }

    /// Is auto.
    #[inline]
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
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[
            b"cv01", b"cv02", b"cv03", b"cv04", b"cv05", b"cv06", b"cv07", b"cv08", b"cv09", b"cv20", b"cv21", b"cv22", b"cv23", b"cv24",
            b"cv25", b"cv26", b"cv27", b"cv28", b"cv29", b"cv30", b"cv31", b"cv32", b"cv33", b"cv34", b"cv35", b"cv36", b"cv37", b"cv38",
            b"cv39", b"cv40", b"cv41", b"cv42", b"cv43", b"cv44", b"cv45", b"cv46", b"cv47", b"cv48", b"cv49", b"cv50", b"cv51", b"cv52",
            b"cv53", b"cv54", b"cv55", b"cv56", b"cv57", b"cv58", b"cv59", b"cv60", b"cv61", b"cv62", b"cv63", b"cv64", b"cv65", b"cv66",
            b"cv67", b"cv68", b"cv69", b"cv70", b"cv71", b"cv72", b"cv73", b"cv74", b"cv75", b"cv76", b"cv77", b"cv78", b"cv79", b"cv70",
            b"cv71", b"cv72", b"cv73", b"cv74", b"cv75", b"cv76", b"cv77", b"cv78", b"cv79", b"cv80", b"cv81", b"cv82", b"cv83", b"cv84",
            b"cv85", b"cv86", b"cv87", b"cv88", b"cv89", b"cv90", b"cv91", b"cv92", b"cv93", b"cv94", b"cv95", b"cv96", b"cv97", b"cv98",
            b"cv99", b"cv99",
        ]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        if self.is_auto() {
            None
        } else {
            Some(self.0 as u32)
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        if v > 99 {
            CharVariant::auto()
        } else {
            CharVariant(v as u8)
        }
    }

    #[inline]
    fn auto() -> Self {
        CharVariant::auto()
    }
}

/// Sub-script and super-script variants.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum FontPosition {
    /// Don't use sub/super script positions.
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
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[b"subs", b"sups"]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        match self {
            FontPosition::Auto => None,
            FontPosition::Sub => Some(1),
            FontPosition::Super => Some(2),
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        match v {
            1 => FontPosition::Sub,
            2 => FontPosition::Super,
            _ => FontPosition::Auto,
        }
    }

    #[inline]
    fn auto() -> Self {
        FontPosition::Auto
    }
}

/// Logographic glyph variants for Japanese fonts.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum JpVariant {
    /// Uses the font default glyphs.
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
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[b"jp78", b"jp83", b"jp90", b"jp04", b"nlck"]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        if self == JpVariant::Auto {
            None
        } else {
            Some(self as u32)
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        if v > Self::names().len() as u32 {
            JpVariant::Auto
        } else {
            // SAFETY: we validated the input in the `if`.
            unsafe { mem::transmute(v as u8) }
        }
    }

    #[inline]
    fn auto() -> Self {
        JpVariant::Auto
    }
}
/// Logographic glyph variants for Chinese fonts.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum CnVariant {
    /// Uses the font default glyphs.
    Auto,
    /// Simplified Chinese glyphs.
    ///
    /// This corresponds to OpenType `smpl` feature.
    Simplified,
    /// Traditional Chinese glyphs.
    ///
    /// This corresponds to OpenType `trad` feature.
    Tradicional,
}
impl fmt::Debug for CnVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "CnVariant")?;
        }
        match self {
            CnVariant::Auto => write!(f, "Auto"),
            CnVariant::Simplified => write!(f, "Simplified"),
            CnVariant::Tradicional => write!(f, "Tradicional"),
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
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[b"smpl", b"trad"]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        match self {
            CnVariant::Auto => None,
            CnVariant::Simplified => Some(1),
            CnVariant::Tradicional => Some(2),
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        match v {
            1 => CnVariant::Simplified,
            2 => CnVariant::Tradicional,
            _ => CnVariant::Auto,
        }
    }

    #[inline]
    fn auto() -> Self {
        CnVariant::Auto
    }
}

/// The sizing and spacing of figures used for East Asian characters.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum EastAsianWidth {
    /// Uses the font default glyphs and spacing.
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
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[b"pwid", b"palt", b"pkna", b"fwid", b"hwid", b"halt", b"twid", b"qwid"]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        if self == EastAsianWidth::Auto {
            None
        } else {
            Some(self as u32)
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        if v > Self::names().len() as u32 {
            EastAsianWidth::Auto
        } else {
            // SAFETY: we validated the input in the `if`.
            unsafe { mem::transmute(v as u8) }
        }
    }

    #[inline]
    fn auto() -> Self {
        EastAsianWidth::Auto
    }
}

/// Name of a font variation axis.
///
/// # Example
///
/// ```
/// # use zero_ui_core::text::font_features::FontVariationName;
/// let devocar_worm: FontVariationName = b"BLDB";
/// ```
pub type FontVariationName = &'static [u8; 4];

/// A small map of font variations.
///
/// Use [`font_variations!`] to manually initialize.
#[derive(Default, Clone)]
pub struct FontVariations(Vec<rustybuzz::Variation>);
impl FontVariations {
    /// New empty.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// New empty with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(Vec::with_capacity(capacity))
    }

    /// New font variations from pairs of name, value.
    #[inline]
    pub fn from_pairs(pairs: &[(FontVariationName, f32)]) -> Self {
        let mut r = Self::with_capacity(pairs.len());
        for (name, value) in pairs {
            r.insert(name, *value);
        }
        r
    }

    /// Insert the font variation, returns the previous value if the variation was already set.
    #[inline]
    pub fn insert(&mut self, name: FontVariationName, value: f32) -> Option<f32> {
        let name = rustybuzz::Tag::from_bytes(name);
        if let Some(entry) = self.0.iter_mut().find(|v| v.tag == name) {
            let prev = Some(entry.value);
            entry.value = value;
            prev
        } else {
            self.0.push(rustybuzz::Variation { tag: name, value });
            None
        }
    }

    /// Remove the font variation, returns the value if the variation was set.
    #[inline]
    pub fn remove(&mut self, name: FontVariationName) -> Option<f32> {
        let name = rustybuzz::Tag::from_bytes(name);
        if let Some(i) = self.0.iter().position(|v| v.tag == name) {
            Some(self.0.swap_remove(i).value)
        } else {
            None
        }
    }

    /// If the variation is set.
    #[inline]
    pub fn contains(&self, name: FontVariationName) -> bool {
        let name = rustybuzz::Tag::from_bytes(name);
        self.0.iter().any(|v| v.tag == name)
    }

    /// Gets a copy of the variation value if it is set.
    #[inline]
    pub fn get(&self, name: FontVariationName) -> Option<f32> {
        let name = rustybuzz::Tag::from_bytes(name);
        self.0.iter().find(|v| v.tag == name).map(|v| v.value)
    }

    /// Exclusive borrow the variation value if it is set.
    #[inline]
    pub fn get_mut(&mut self, name: FontVariationName) -> Option<&mut f32> {
        let name = rustybuzz::Tag::from_bytes(name);
        self.0.iter_mut().find(|v| v.tag == name).map(|v| &mut v.value)
    }

    /// Count of font variations set.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// If not font variation is set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Finalize variations config for use in a font.
    #[inline]
    pub fn finalize(&self) -> RFontVariations {
        let mut r = self.0.clone();
        r.shrink_to_fit();
        r
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
                write!(f, r#", b"{}": {}"#, name_to_str(&entry.tag.to_bytes()), entry.value)?;
            }
            write!(f, "]")
        }
    }
}

///<span data-inline></span> Initialize a [`FontVariations`] map.
///
/// # Example
///
/// ```
/// # use zero_ui_core::text::font_features::{FontVariations, font_variations};
/// # fn assert_type(_: FontVariations) { }
/// let variations = font_variations! {
///     b"SKLA": 1000.0,
///     b"TRMG": 750.0
/// };
/// # assert_type(variations);
/// ```
#[macro_export]
macro_rules! font_variations {
    [$(
        $name:tt : $value: expr
    ),* $(,)?] => {
        $crate::text::font_features::FontVariations::from_pairs(&[
            $(
                ($name, $value),
            )*
        ])
    }
}
#[doc(inline)]
pub use font_variations;

/// Finalized [`FontVariations`].
///
/// This is a vec of [rustybuzz variations](rustybuzz::Variation).
pub type RFontVariations = Vec<rustybuzz::Variation>;
