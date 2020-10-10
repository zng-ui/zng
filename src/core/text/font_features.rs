//! Font features config types.

use fnv::FnvHashMap;
use std::{collections::hash_map::Entry as HEntry, marker::PhantomData, num::NonZeroU32};
use std::{fmt, mem};

// TODO
// main: https://developer.mozilla.org/en-US/docs/Web/CSS/font-feature-settings
// 1 - https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-east-asian
// 5 - https://helpx.adobe.com/pt/fonts/user-guide.html/pt/fonts/using/open-type-syntax.ug.html#calt
// review - https://harfbuzz.github.io/shaping-opentype-features.html

/// Name of a font feature.
///
/// # Example
///
/// ```
/// # use zero_ui::core::text::FontFeatureName;
/// let historical_lig: FontFeatureName = b"hlig";
/// ```
pub type FontFeatureName = &'static [u8; 4];

/// The raw value used when a feature is set to `true`.
pub const FEATURE_ENABLED: u32 = 1;
/// The raw value used when a feature is set to `false`.
pub const FEATURE_DISABLED: u32 = 0;

/// Font features configuration.
#[derive(Default, Clone)]
pub struct FontFeatures(FnvHashMap<FontFeatureName, u32>);
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
    /// If `names` has less then 2 names.
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
    /// If `S::names()` has less then 2 names.
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
    /// If `S::names()` has less then 2 entries.
    pub fn feature_exclusive_sets<S: FontFeatureExclusiveSetsState>(&mut self) -> FontFeatureExclusiveSets<S> {
        assert!(S::names().len() >= 2);
        FontFeatureExclusiveSets {
            features: &mut self.0,
            _t: PhantomData,
        }
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

fn name_to_str(name: FontFeatureName) -> &'static str {
    std::str::from_utf8(name).unwrap_or_default()
}

/// A builder for [`FontFeatures`].
///
/// # Example
///
/// ```
/// # use zero_ui::core::text::FontFeatures;
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
    /// If `names` has less then 2 names.
    #[inline]
    pub fn feature_set(mut self, names: &'static [FontFeatureName], state: impl Into<FontFeatureState>) -> Self {
        self.0.feature_set(names).set(state);
        self
    }

    /// Sets a single feature of a set of features.
    ///
    /// # Panics
    ///
    /// If `S::names()` has less then 2 names.
    pub fn feature_exclusive_set<S: FontFeatureExclusiveSetState>(mut self, state: impl Into<S>) -> Self {
        self.0.feature_exclusive_set::<S>().set(state);
        self
    }

    /// Sets the features that represent the `state`.
    ///
    /// # Panics
    ///
    /// If `S::names()` has less then 2 entries.
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
        $(
            font_features!{feature $(#[$docs])* fn $name($feat0_or_Enum $(, $feat1)?) $(-> $Helper)?; }
            font_features!{builder $(#[$docs])* fn $name($($feat0_or_Enum -> $Helper)?); }
        )+
    };

    (feature $(#[$docs:meta])* fn $name:ident($feat0:tt, $feat1:tt); ) => {
        impl FontFeatures {
            $(#[$docs])*
            #[inline]
            pub fn $name(&mut self) -> FontFeatureSet {
                self.feature_set(&[$feat0, $feat1])
            }
        }
    };

    (feature $(#[$docs:meta])* fn $name:ident($feat0:tt);) => {
        impl FontFeatures {
            $(#[$docs])*
            #[inline]
            pub fn $name(&mut self) -> FontFeature {
                self.feature($feat0)
            }
        }
    };

    (feature $(#[$docs:meta])* fn $name:ident($Enum:ident) -> $Helper:ident;) => {
        impl FontFeatures {
            $(#[$docs])*
            #[inline]
            pub fn $name(&mut self) -> $Helper<$Enum> {
                $Helper { features: &mut self.0, _t: PhantomData }
            }
        }
    };

    (builder $(#[$docs:meta])* fn $name:ident();) => {
        impl FontFeaturesBuilder {
            $(#[$docs])*
            #[inline]
            pub fn $name(mut self, state: impl Into<FontFeatureState>) -> Self {
                self.0.$name().set(state);
                self
            }
        }
    };

    (builder $(#[$docs:meta])* fn $name:ident($Enum:ident -> $Helper:ident);) => {
        impl FontFeaturesBuilder {
            $(#[$docs])*
            #[inline]
            pub fn $name(mut self, state: impl Into<$Enum>) -> Self {
                self.0.$name().set(state);
                self
            }
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
    /// See [`StyleSet`] for more details.
    fn style_set(StyleSet) -> FontFeatureExclusiveSet;

    /// Font stylistic alternatives for individual characters.
    ///
    /// See [`StyleSet`] for more details.
    fn char_variant(CharVariant) -> FontFeatureExclusiveSet;

    /// Font sub/super script alternatives.
    ///
    /// See [`FontPosition`] for more details.
    fn position(FontPosition) -> FontFeatureExclusiveSet;
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
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
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
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
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
    features: &'a mut FnvHashMap<FontFeatureName, u32>,
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
        let mut active = fnv::FnvHashSet::default();
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
        let mut active = fnv::FnvHashSet::default();
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
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
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
        self == Self::auto()
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
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
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
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
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
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
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
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum StyleSet {
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
impl_from_and_into_var! {
    /// `set == 0 || set > 20` is Auto, `set >= 1 && set <= 20` maps to their variant.
    fn from(set: u8) -> StyleSet {
        if set > 20 {
            StyleSet::Auto
        } else {
            // SAFETY: We eliminated the bad values in the `if`.
            unsafe { mem::transmute(set) }
        }
    }
}
impl FontFeatureExclusiveSetState for StyleSet {
    #[inline]
    fn names() -> &'static [FontFeatureName] {
        &[
            b"ss01", b"ss02", b"ss03", b"ss04", b"ss05", b"ss06", b"ss07", b"ss08", b"ss09", b"ss10", b"ss11", b"ss12", b"ss13", b"ss14",
            b"ss15", b"ss16", b"ss17", b"ss18", b"ss19", b"ss20",
        ]
    }

    #[inline]
    fn variant(self) -> Option<u32> {
        if self == StyleSet::Auto {
            None
        } else {
            Some(self as u32)
        }
    }

    #[inline]
    fn from_variant(v: u32) -> Self {
        if v > 20 {
            StyleSet::Auto
        } else {
            // SAFETY: we validated the input in the `if`.
            unsafe { mem::transmute(v as u8) }
        }
    }

    #[inline]
    fn auto() -> Self {
        StyleSet::Auto
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
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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
