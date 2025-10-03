//! Key combination types.
//!
//! This is declared on this crate mostly to support shortcuts in commands, shortcut events
//! are implemented in the input crate.

use std::fmt;

use bitflags::bitflags;
use zng_txt::{ToTxt, Txt};
use zng_unique_id::static_id;
use zng_var::{Var, impl_from_and_into_var};

#[doc(hidden)]
pub use zng_view_api::keyboard::{Key, KeyCode};

use crate::event::{Command, CommandMetaVar, CommandMetaVarId};

/// A keyboard key used in a gesture.
///
/// Gesture keys are case-insensitive, [`Key::Char`] is matched as case-insensitive.
///
/// Note that not all keys work well as gesture keys, you can use `try_into` to filter [`Key`] or [`KeyCode`] values
/// that do not work.
///
/// [`Key::Char`]: zng_view_api::keyboard::Key::Char
/// [`Key`]: zng_view_api::keyboard::Key
/// [`KeyCode`]: zng_view_api::keyboard::KeyCode
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum GestureKey {
    /// Gesture key identified by the semantic key.
    Key(Key),
    /// Gesture key identified by the physical key.
    Code(KeyCode),
}
impl std::hash::Hash for GestureKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            GestureKey::Key(k) => match k {
                Key::Char(c) => {
                    for c in c.to_uppercase() {
                        c.hash(state);
                    }
                }
                Key::Str(s) => {
                    unicase::UniCase::new(s).hash(state);
                }
                k => k.hash(state),
            },
            GestureKey::Code(c) => c.hash(state),
        }
    }
}
impl Eq for GestureKey {}
impl PartialEq for GestureKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Key(l0), Self::Key(r0)) => match (l0, r0) {
                (Key::Char(l), Key::Char(r)) => {
                    let mut l = l.to_uppercase();
                    let mut r = r.to_uppercase();

                    while let (Some(l), Some(r)) = (l.next(), r.next()) {
                        if l != r {
                            return false;
                        }
                    }

                    l.next().is_none() && r.next().is_none()
                }
                (Key::Str(l), Key::Str(r)) => unicase::eq(l, r),
                (l0, r0) => l0 == r0,
            },
            (Self::Code(l0), Self::Code(r0)) => l0 == r0,
            _ => false,
        }
    }
}
impl fmt::Display for GestureKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GestureKey::Key(k) => match k {
                Key::Char(c) => write!(f, "{}", c.to_uppercase()),
                Key::Str(s) => write!(f, "{s}"),
                Key::ArrowLeft => write!(f, "←"),
                Key::ArrowRight => write!(f, "→"),
                Key::ArrowUp => write!(f, "↑"),
                Key::ArrowDown => write!(f, "↑"),
                k => write!(f, "{k:?}"),
            },
            GestureKey::Code(c) => write!(f, "{c:?}"),
        }
    }
}
impl GestureKey {
    /// If is not any:
    ///
    /// * [`Key::is_modifier`]
    /// * [`Key::is_composition`]
    /// * [`Key::Unidentified`]
    /// * [`KeyCode::is_modifier`]
    /// * [`KeyCode::is_composition`]
    pub fn is_valid(&self) -> bool {
        match self {
            GestureKey::Key(k) => !k.is_modifier() && !k.is_composition() && *k != Key::Unidentified,
            GestureKey::Code(k) => k.is_modifier() && !k.is_composition(),
        }
    }
}

/// Accepts only keys that are not [`is_modifier`] and not [`is_composition`].
///
/// [`is_modifier`]: Key::is_modifier
/// [`is_composition`]: Key::is_composition
impl TryFrom<Key> for GestureKey {
    type Error = Key;

    fn try_from(key: Key) -> Result<Self, Self::Error> {
        if key.is_modifier() || key.is_composition() || key == Key::Unidentified {
            Err(key)
        } else {
            Ok(Self::Key(key))
        }
    }
}
/// Accepts only keys that are not [`is_modifier`] and not [`is_composition`].
///
/// [`is_modifier`]: KeyCode::is_modifier
/// [`is_composition`]: KeyCode::is_composition
impl TryFrom<KeyCode> for GestureKey {
    type Error = KeyCode;

    fn try_from(key: KeyCode) -> Result<Self, Self::Error> {
        if key.is_modifier() || key.is_composition() || key.is_unidentified() {
            Err(key)
        } else {
            Ok(Self::Code(key))
        }
    }
}
impl std::str::FromStr for GestureKey {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Key::from_str(s) {
            Key::Str(s) => match KeyCode::from_str(&s) {
                Ok(k) => {
                    let key = k
                        .try_into()
                        .map_err(|e| ParseError::new(format!("key `{e:?}` cannot be used in gestures")))?;

                    Ok(key)
                }
                Err(_) => Ok(Self::Key(Key::Str(s))),
            },
            k => {
                let key = k
                    .try_into()
                    .map_err(|e| ParseError::new(format!("key `{e:?}` cannot be used in gestures")))?;

                Ok(key)
            }
        }
    }
}

/// A keyboard combination.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyGesture {
    /// The key modifiers.
    ///
    /// Equality of key gestures matches the [`ambit`] modifiers, so a `L_CTRL` is equal to a `R_CTRL` in a key gesture,
    /// the actual bit flag is preserved in the state and can be extracted from the shortcut.
    ///
    /// [`ambit`]: ModifiersState::ambit
    pub modifiers: ModifiersState,
    /// The key.
    pub key: GestureKey,
}
impl PartialEq for KeyGesture {
    fn eq(&self, other: &Self) -> bool {
        self.modifiers.ambit() == other.modifiers.ambit() && self.key == other.key
    }
}
impl Eq for KeyGesture {}
impl std::hash::Hash for KeyGesture {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.modifiers.ambit().hash(state);
        self.key.hash(state);
    }
}
impl KeyGesture {
    /// New from modifiers and key.
    pub fn new(modifiers: ModifiersState, key: GestureKey) -> Self {
        KeyGesture { modifiers, key }
    }

    /// New key gesture without modifiers.
    pub fn new_key(key: GestureKey) -> Self {
        KeyGesture {
            modifiers: ModifiersState::empty(),
            key,
        }
    }

    /// Gets if  [`GestureKey::is_valid`].
    pub fn is_valid(&self) -> bool {
        self.key.is_valid()
    }
}
impl fmt::Debug for KeyGesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("KeyGesture")
                .field("modifiers", &self.modifiers)
                .field("key", &self.key)
                .finish()
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for KeyGesture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.modifiers.has_super() {
            write!(f, "Super+")?
        }
        if self.modifiers.has_ctrl() {
            write!(f, "Ctrl+")?
        }
        if self.modifiers.has_shift() {
            write!(f, "Shift+")?
        }
        if self.modifiers.has_alt() {
            write!(f, "Alt+")?
        }

        write!(f, "{}", self.key)
    }
}

/// A modifier key press and release without any other key press in between.
#[derive(Clone, Copy, Eq, PartialEq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ModifierGesture {
    /// Any of the Windows/Apple keys.
    Super,
    /// Any of the CTRL keys.
    Ctrl,
    /// Any of the SHIFT keys.
    Shift,
    /// Any of the ALT keys.
    Alt,
}
impl fmt::Debug for ModifierGesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "ModifierGesture::")?;
        }
        write!(f, "{self}")
    }
}
impl fmt::Display for ModifierGesture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModifierGesture::Super => write!(f, "Super"),
            ModifierGesture::Ctrl => write!(f, "Ctrl"),
            ModifierGesture::Shift => write!(f, "Shift"),
            ModifierGesture::Alt => write!(f, "Alt"),
        }
    }
}
impl<'a> TryFrom<&'a Key> for ModifierGesture {
    type Error = &'a Key;
    fn try_from(value: &'a Key) -> Result<Self, Self::Error> {
        match value {
            Key::Alt | Key::AltGraph => Ok(ModifierGesture::Alt),
            Key::Ctrl => Ok(ModifierGesture::Ctrl),
            Key::Shift => Ok(ModifierGesture::Shift),
            Key::Super => Ok(ModifierGesture::Super),
            key => Err(key),
        }
    }
}
impl TryFrom<KeyCode> for ModifierGesture {
    type Error = KeyCode;
    fn try_from(value: KeyCode) -> Result<Self, Self::Error> {
        match value {
            KeyCode::AltLeft | KeyCode::AltRight => Ok(ModifierGesture::Alt),
            KeyCode::CtrlLeft | KeyCode::CtrlRight => Ok(ModifierGesture::Ctrl),
            KeyCode::ShiftLeft | KeyCode::ShiftRight => Ok(ModifierGesture::Shift),
            KeyCode::SuperLeft | KeyCode::SuperRight => Ok(ModifierGesture::Super),
            key => Err(key),
        }
    }
}
impl ModifierGesture {
    /// Left modifier key.
    pub fn left_key(&self) -> (KeyCode, Key) {
        match self {
            ModifierGesture::Super => (KeyCode::SuperLeft, Key::Super),
            ModifierGesture::Ctrl => (KeyCode::CtrlLeft, Key::Ctrl),
            ModifierGesture::Shift => (KeyCode::ShiftLeft, Key::Shift),
            ModifierGesture::Alt => (KeyCode::AltLeft, Key::Alt),
        }
    }
    /// To modifiers state.
    pub fn modifiers_state(&self) -> ModifiersState {
        match self {
            ModifierGesture::Super => ModifiersState::LOGO,
            ModifierGesture::Ctrl => ModifiersState::CTRL,
            ModifierGesture::Shift => ModifiersState::SHIFT,
            ModifierGesture::Alt => ModifiersState::ALT,
        }
    }
}

/// A sequence of two keyboard combinations.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct KeyChord {
    /// The first key gesture.
    pub starter: KeyGesture,

    /// The second key gesture.
    pub complement: KeyGesture,
}
impl fmt::Debug for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("KeyChord")
                .field("starter", &self.starter)
                .field("complement", &self.complement)
                .finish()
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for KeyChord {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.starter, self.complement)
    }
}

/// Keyboard gesture or chord associated with a command.
///
/// See the [`shortcut!`] macro for declaring a shortcut.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Shortcut {
    /// Key-press plus modifiers.
    Gesture(KeyGesture),
    /// Sequence of two key gestures.
    Chord(KeyChord),
    /// Modifier press and release.
    Modifier(ModifierGesture),
}
impl fmt::Debug for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                Shortcut::Gesture(g) => f.debug_tuple("Shortcut::Gesture").field(g).finish(),
                Shortcut::Chord(c) => f.debug_tuple("Shortcut::Chord").field(c).finish(),
                Shortcut::Modifier(m) => f.debug_tuple("Shortcut::Modifier").field(m).finish(),
            }
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Shortcut::Gesture(g) => fmt::Display::fmt(g, f),
            Shortcut::Chord(c) => fmt::Display::fmt(c, f),
            Shortcut::Modifier(m) => fmt::Display::fmt(m, f),
        }
    }
}
impl Shortcut {
    /// Modifiers state required by shortcut.
    pub fn modifiers_state(&self) -> ModifiersState {
        match self {
            Shortcut::Gesture(g) => g.modifiers,
            Shortcut::Chord(c) => c.complement.modifiers,
            Shortcut::Modifier(m) => m.modifiers_state(),
        }
    }

    /// Gets if all [`KeyGesture::is_valid`].
    pub fn is_valid(&self) -> bool {
        match self {
            Shortcut::Gesture(k) => k.is_valid(),
            Shortcut::Chord(c) => c.starter.is_valid() && c.complement.is_valid(),
            Shortcut::Modifier(_) => true,
        }
    }
}
impl_from_and_into_var! {
    fn from(shortcut: Shortcut) -> Shortcuts {
        Shortcuts(vec![shortcut])
    }

    fn from(key_gesture: KeyGesture) -> Shortcut {
        Shortcut::Gesture(key_gesture)
    }

    fn from(key_chord: KeyChord) -> Shortcut {
        Shortcut::Chord(key_chord)
    }

    fn from(modifier: ModifierGesture) -> Shortcut {
        Shortcut::Modifier(modifier)
    }

    fn from(gesture_key: GestureKey) -> Shortcut {
        KeyGesture::new_key(gesture_key).into()
    }

    fn from(gesture_key: GestureKey) -> Shortcuts {
        Shortcuts(vec![gesture_key.into()])
    }

    fn from(key_gesture: KeyGesture) -> Shortcuts {
        Shortcuts(vec![key_gesture.into()])
    }

    fn from(key_chord: KeyChord) -> Shortcuts {
        Shortcuts(vec![key_chord.into()])
    }

    fn from(modifier: ModifierGesture) -> Shortcuts {
        Shortcuts(vec![modifier.into()])
    }

    fn from(shortcuts: Vec<Shortcut>) -> Shortcuts {
        Shortcuts(shortcuts)
    }
}
impl<const N: usize> From<[Shortcut; N]> for Shortcuts {
    fn from(a: [Shortcut; N]) -> Self {
        Shortcuts(a.into())
    }
}
impl<const N: usize> crate::var::IntoVar<Shortcuts> for [Shortcut; N] {
    fn into_var(self) -> Var<Shortcuts> {
        crate::var::const_var(self.into())
    }
}

/// Multiple shortcuts.
#[derive(Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Shortcuts(pub Vec<Shortcut>);
impl Shortcuts {
    /// New default (empty).
    pub const fn new() -> Self {
        Self(vec![])
    }

    /// Try to generate shortcuts that produce the `character`.
    ///
    /// Returns at least one shortcut or error the char back if it cannot
    /// be generated by a single shortcut.
    ///
    /// Note chords are not generated. Caps lock is assumed to be off.
    pub fn from_char(character: char) -> Result<Self, char> {
        if character.is_control() {
            Err(character)
        } else {
            Ok(Self(vec![Shortcut::Gesture(KeyGesture {
                modifiers: ModifiersState::empty(),
                key: GestureKey::Key(Key::Char(character)),
            })]))
        }
    }

    /// If the `shortcut` is present in the shortcuts.
    pub fn contains(&self, shortcut: &Shortcut) -> bool {
        self.0.contains(shortcut)
    }
}
impl TryFrom<char> for Shortcuts {
    type Error = char;

    /// See [`from_char`](Self::from_char).
    fn try_from(value: char) -> Result<Self, Self::Error> {
        Shortcuts::from_char(value)
    }
}
impl fmt::Debug for Shortcuts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_tuple("Shortcuts").field(&self.0).finish()
        } else {
            write!(f, "[")?;
            if !self.0.is_empty() {
                if let Shortcut::Chord(c) = &self.0[0] {
                    write!(f, "({c:?})")?;
                } else {
                    write!(f, "{:?}", self.0[0])?;
                }
                for shortcut in &self.0[1..] {
                    if let Shortcut::Chord(c) = shortcut {
                        write!(f, ", ({c:?})")?;
                    } else {
                        write!(f, ", {shortcut:?}")?;
                    }
                }
            }
            write!(f, "]")
        }
    }
}
impl fmt::Display for Shortcuts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.0.is_empty() {
            write!(f, "{}", self.0[0])?;
            for shortcut in &self.0[1..] {
                write!(f, " | {shortcut}")?;
            }
        }
        Ok(())
    }
}
impl std::ops::Deref for Shortcuts {
    type Target = Vec<Shortcut>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Shortcuts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Shortcut, gesture parsing error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseError {
    /// Error message, usually in the pattern "`{invalid-input}` is not a {shortcut/modifier}".
    pub error: String,
}
impl ParseError {
    /// New from any error message.
    pub fn new(error: impl ToString) -> Self {
        ParseError { error: error.to_string() }
    }
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}
impl std::error::Error for ParseError {}

bitflags! {
    /// Represents the current state of the keyboard modifiers.
    ///
    /// Each flag represents a modifier and is set if this modifier is active.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct ModifiersState: u8 {
        /// The left "shift" key.
        const L_SHIFT = 0b0000_0001;
        /// The right "shift" key.
        const R_SHIFT = 0b0000_0010;
        /// Any "shift" key.
        const SHIFT = 0b0000_0011;

        /// The left "control" key.
        const L_CTRL = 0b0000_0100;
        /// The right "control" key.
        const R_CTRL = 0b0000_1000;
        /// Any "control" key.
        const CTRL = 0b0000_1100;

        /// The left "alt" key.
        const L_ALT = 0b0001_0000;
        /// The right "alt" key.
        const R_ALT = 0b0010_0000;
        /// Any "alt" key.
        const ALT = 0b0011_0000;

        /// The left "logo" key.
        const L_SUPER = 0b0100_0000;
        /// The right "logo" key.
        const R_SUPER = 0b1000_0000;
        /// Any "logo" key.
        ///
        /// This is the "windows" key on PC and "command" key on Mac.
        const LOGO = 0b1100_0000; // TODO(breaking) rename to SUPER
    }
}
impl ModifiersState {
    /// Returns `true` if any shift key is pressed.
    pub fn has_shift(self) -> bool {
        self.intersects(Self::SHIFT)
    }
    /// Returns `true` if any control key is pressed.
    pub fn has_ctrl(self) -> bool {
        self.intersects(Self::CTRL)
    }
    /// Returns `true` if any alt key is pressed.
    pub fn has_alt(self) -> bool {
        self.intersects(Self::ALT)
    }
    /// Returns `true` if any logo key is pressed.
    pub fn has_super(self) -> bool {
        self.intersects(Self::LOGO)
    }

    /// Returns `true` if only any flag in `part` is pressed.
    pub fn is_only(self, part: ModifiersState) -> bool {
        !self.is_empty() && (self - part).is_empty()
    }

    /// Returns `true` if only any shift key is pressed.
    pub fn is_only_shift(self) -> bool {
        self.is_only(ModifiersState::SHIFT)
    }
    /// Returns `true` if only any control key is pressed.
    pub fn is_only_ctrl(self) -> bool {
        self.is_only(ModifiersState::CTRL)
    }
    /// Returns `true` if only any alt key is pressed.
    pub fn is_only_alt(self) -> bool {
        self.is_only(ModifiersState::ALT)
    }
    /// Returns `true` if only any logo key is pressed.
    pub fn is_only_logo(self) -> bool {
        self.is_only(ModifiersState::LOGO)
    }

    /// Removes `part` and returns if it was removed.
    pub fn take(&mut self, part: ModifiersState) -> bool {
        let r = self.intersects(part);
        if r {
            self.remove(part);
        }
        r
    }

    /// Removes `SHIFT` and returns if it was removed.
    pub fn take_shift(&mut self) -> bool {
        self.take(ModifiersState::SHIFT)
    }

    /// Removes `CTRL` and returns if it was removed.
    pub fn take_ctrl(&mut self) -> bool {
        self.take(ModifiersState::CTRL)
    }

    /// Removes `ALT` and returns if it was removed.
    pub fn take_alt(&mut self) -> bool {
        self.take(ModifiersState::ALT)
    }

    /// Removes `LOGO` and returns if it was removed.
    pub fn take_logo(&mut self) -> bool {
        self.take(ModifiersState::LOGO)
    }

    /// Returns modifiers that set both left and right flags if any side is set in `self`.
    pub fn ambit(self) -> Self {
        let mut r = Self::empty();
        if self.has_alt() {
            r |= Self::ALT;
        }
        if self.has_ctrl() {
            r |= Self::CTRL;
        }
        if self.has_shift() {
            r |= Self::SHIFT;
        }
        if self.has_super() {
            r |= Self::LOGO;
        }
        r
    }

    /// Returns only the alt flags in `self`.
    pub fn into_alt(self) -> Self {
        self & Self::ALT
    }

    /// Returns only the control flags in `self`.
    pub fn into_ctrl(self) -> Self {
        self & Self::CTRL
    }

    /// Returns only the shift flags in `self`.
    pub fn into_shift(self) -> Self {
        self & Self::SHIFT
    }

    /// Returns only the logo flags in `self`.
    pub fn into_logo(self) -> Self {
        self & Self::LOGO
    }

    /// Modifier from `code`, returns empty if the key is not a modifier.
    pub fn from_code(code: KeyCode) -> ModifiersState {
        match code {
            KeyCode::AltLeft => Self::L_ALT,
            KeyCode::AltRight => Self::R_ALT,
            KeyCode::CtrlLeft => Self::L_CTRL,
            KeyCode::CtrlRight => Self::R_CTRL,
            KeyCode::ShiftLeft => Self::L_SHIFT,
            KeyCode::ShiftRight => Self::R_SHIFT,
            KeyCode::SuperLeft => Self::L_SUPER,
            KeyCode::SuperRight => Self::R_SUPER,
            _ => Self::empty(),
        }
    }

    /// Modifier from `key`, returns empty if the key is not a modifier.
    pub fn from_key(key: Key) -> ModifiersState {
        match key {
            Key::Alt => Self::L_ALT,
            Key::AltGraph => Self::R_ALT,
            Key::Shift => Self::SHIFT,
            Key::Ctrl => Self::CTRL,
            Key::Super => Self::LOGO,
            _ => Self::empty(),
        }
    }

    /// All key codes that when pressed form the modifiers state.
    ///
    /// In case of multiple keys the order is `LOGO`, `CTRL`, `SHIFT`, `ALT`.
    ///
    /// In case both left and right keys are flagged for a modifier, the left key is used.
    pub fn codes(self) -> Vec<KeyCode> {
        let mut r = vec![];

        if self.contains(Self::L_SUPER) {
            r.push(KeyCode::SuperLeft);
        } else if self.contains(Self::R_SUPER) {
            r.push(KeyCode::SuperRight);
        }

        if self.contains(Self::L_CTRL) {
            r.push(KeyCode::CtrlLeft);
        } else if self.contains(Self::R_CTRL) {
            r.push(KeyCode::CtrlRight);
        }

        if self.contains(Self::L_SHIFT) {
            r.push(KeyCode::ShiftLeft);
        } else if self.contains(Self::R_SHIFT) {
            r.push(KeyCode::ShiftRight);
        }

        if self.contains(Self::L_ALT) {
            r.push(KeyCode::AltLeft);
        } else if self.contains(Self::R_ALT) {
            r.push(KeyCode::AltRight);
        }

        r
    }

    /// All keys that when pressed form the modifiers state.
    ///
    /// In case of multiple keys the order is `LOGO`, `CTRL`, `SHIFT`, `ALT`.
    pub fn keys(self) -> Vec<Key> {
        let mut r = vec![];

        if self.intersects(Self::LOGO) {
            r.push(Key::Super);
        }

        if self.intersects(Self::CTRL) {
            r.push(Key::Ctrl);
        }

        if self.intersects(Self::SHIFT) {
            r.push(Key::Shift);
        }

        if self.contains(Self::R_ALT) {
            r.push(Key::AltGraph);
        } else if self.contains(Self::R_ALT) {
            r.push(Key::Alt);
        }

        r
    }
}

/// Adds the [`shortcut`] metadata.
///
/// If a command has a shortcut the `GestureManager` will invoke the command when the shortcut is pressed
/// the command is enabled, if the command target is a widget it will also be focused. See the `GESTURES`
/// service documentation for details on how shortcuts are resolved.
///
/// [`shortcut`]: CommandShortcutExt::shortcut
pub trait CommandShortcutExt {
    /// Gets a read-write variable that is zero-or-more shortcuts that invoke the command.
    fn shortcut(self) -> CommandMetaVar<Shortcuts>;

    /// Gets a read-only variable that is the display text for the first shortcut.
    fn shortcut_txt(self) -> Var<Txt>
    where
        Self: Sized,
    {
        self.shortcut().map(|c| if c.is_empty() { Txt::from("") } else { c[0].to_txt() })
    }

    /// Gets a read-write variable that sets a filter for when the [`shortcut`] is valid.
    ///
    /// [`shortcut`]: CommandShortcutExt::shortcut
    fn shortcut_filter(self) -> CommandMetaVar<ShortcutFilter>;

    /// Sets the initial shortcuts.
    fn init_shortcut(self, shortcut: impl Into<Shortcuts>) -> Self;

    /// Sets the initial shortcut filters.
    fn init_shortcut_filter(self, filter: impl Into<ShortcutFilter>) -> Self;
}
impl CommandShortcutExt for Command {
    fn shortcut(self) -> CommandMetaVar<Shortcuts> {
        self.with_meta(|m| m.get_var_or_default(*COMMAND_SHORTCUT_ID))
    }

    fn shortcut_filter(self) -> CommandMetaVar<ShortcutFilter> {
        self.with_meta(|m| m.get_var_or_default(*COMMAND_SHORTCUT_FILTER_ID))
    }

    fn init_shortcut(self, shortcut: impl Into<Shortcuts>) -> Self {
        self.with_meta(|m| m.init_var(*COMMAND_SHORTCUT_ID, shortcut.into()));
        self
    }

    fn init_shortcut_filter(self, filter: impl Into<ShortcutFilter>) -> Self {
        self.with_meta(|m| m.init_var(*COMMAND_SHORTCUT_FILTER_ID, filter.into()));
        self
    }
}

bitflags! {
    /// Conditions that must be met for the shortcut to apply.
    #[derive(Default, Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    #[serde(transparent)]
    pub struct ShortcutFilter: u8 {
        /// Shortcut only applies if the scope is enabled.
        const ENABLED = 0b001;
        /// Shortcut only applies if the scope is in the focused path.
        const FOCUSED = 0b010;
        /// Shortcut only applies if the command is enabled.
        const CMD_ENABLED = 0b100;
    }
}

static_id! {
    static ref COMMAND_SHORTCUT_ID: CommandMetaVarId<Shortcuts>;
    static ref COMMAND_SHORTCUT_FILTER_ID: CommandMetaVarId<ShortcutFilter>;
}

#[doc(hidden)]
#[macro_export]
macro_rules! __shortcut {
    (-> + $Key:tt) => {
        $crate::shortcut::KeyGesture {
            key: $crate::__shortcut!(@key $Key),
            modifiers: $crate::shortcut::ModifiersState::empty(),
        }
    };

    (-> $($MODIFIER:ident)|+ + $Key:tt) => {
        $crate::shortcut::KeyGesture {
            key: $crate::__shortcut!(@key $Key),
            modifiers: $($crate::shortcut::ModifiersState::$MODIFIER)|+,
        }
    };

    (=> $($STARTER_MODIFIER:ident)|* + $StarterKey:tt, $($COMPLEMENT_MODIFIER:ident)|* + $ComplementKey:tt) => {
        $crate::shortcut::KeyChord {
            starter: $crate::__shortcut!(-> $($STARTER_MODIFIER)|* + $StarterKey),
            complement: $crate::__shortcut!(-> $($COMPLEMENT_MODIFIER)|* + $ComplementKey)
        }
    };

    (@key $Key:ident) => { $crate::shortcut::GestureKey::Key($crate::shortcut::Key::$Key) };
    (@key $key_char:literal) => { $crate::shortcut::GestureKey::Key($crate::shortcut::Key::Char($key_char)) };
}

///<span data-del-macro-root></span> Creates a [`Shortcut`].
///
/// This macro input can be:
///
/// * A single [`ModifierGesture`] variant defines a [`Shortcut::Modifier`].
/// * A single [`Key`] variant defines a [`Shortcut::Gesture`] without modifiers.
/// * A single [`char`] literal that translates to a [`Key::Char`].
/// * [`ModifiersState`] followed by `+` followed by a `Key` or `char` defines a gesture with modifiers. Modifier
///   combinations must be joined by `|`.
/// * A gesture followed by `,` followed by another gesture defines a [`Shortcut::Chord`].
///
/// Note that not all shortcuts can be declared with this macro, in particular there is no support for [`Key::Str`]
/// and [`KeyCode`], these shortcuts must be declared manually. Also note that some keys are not recommended in shortcuts,
/// in particular [`Key::is_modifier`] and [`Key::is_composition`] keys will not work right.
///
///
/// # Examples
///
/// ```
/// use zng_app::shortcut::{Shortcut, shortcut};
///
/// fn single_key() -> Shortcut {
///     shortcut!(Enter)
/// }
///
/// fn modified_key() -> Shortcut {
///     shortcut!(CTRL + 'C')
/// }
///
/// fn multi_modified_key() -> Shortcut {
///     shortcut!(CTRL | SHIFT + 'C')
/// }
///
/// fn chord() -> Shortcut {
///     shortcut!(CTRL + 'E', 'A')
/// }
///
/// fn modifier_release() -> Shortcut {
///     shortcut!(Alt)
/// }
/// ```
///
/// [`Key`]: zng_view_api::keyboard::Key
/// [`Key::Char`]: zng_view_api::keyboard::Key::Char
/// [`Key::Str`]: zng_view_api::keyboard::Key::Str
/// [`KeyCode`]: zng_view_api::keyboard::KeyCode
/// [`Key::is_modifier`]: zng_view_api::keyboard::Key::is_modifier
/// [`Key::is_composition`]: zng_view_api::keyboard::Key::is_composition
#[macro_export]
macro_rules! shortcut_macro {
    (Super) => {
        $crate::shortcut::Shortcut::Modifier($crate::shortcut::ModifierGesture::Super)
    };
    (Shift) => {
        $crate::shortcut::Shortcut::Modifier($crate::shortcut::ModifierGesture::Shift)
    };
    (Ctrl) => {
        $crate::shortcut::Shortcut::Modifier($crate::shortcut::ModifierGesture::Ctrl)
    };
    (Alt) => {
        $crate::shortcut::Shortcut::Modifier($crate::shortcut::ModifierGesture::Alt)
    };

    ($Key:tt) => {
        $crate::shortcut::Shortcut::Gesture($crate::__shortcut!(-> + $Key))
    };
    ($($MODIFIER:ident)|+ + $Key:tt) => {
        $crate::shortcut::Shortcut::Gesture($crate::__shortcut!(-> $($MODIFIER)|+ + $Key))
    };

    ($StarterKey:tt, $ComplementKey:tt) => {
        $crate::shortcut::Shortcut::Chord($crate::__shortcut!(=>
            + $StarterKey,
            + $ComplementKey
        ))
    };

    ($StarterKey:tt, $($COMPLEMENT_MODIFIER:ident)|+ + $ComplementKey:tt) => {
        $crate::shortcut::Shortcut::Chord($crate::__shortcut!(=>
            + $StarterKey,
            $(COMPLEMENT_MODIFIER)|* + $ComplementKey
        ))
    };

    ($($STARTER_MODIFIER:ident)|+ + $StarterKey:tt, $ComplementKey:tt) => {
        $crate::shortcut::Shortcut::Chord($crate::__shortcut!(=>
            $($STARTER_MODIFIER)|* + $StarterKey,
            + $ComplementKey
        ))
    };

    ($($STARTER_MODIFIER:ident)|+ + $StarterKey:tt, $($COMPLEMENT_MODIFIER:ident)|+ + $ComplementKey:tt) => {
        $crate::shortcut::Shortcut::Chord($crate::__shortcut!(=>
            $($STARTER_MODIFIER)|* + $StarterKey,
            $($COMPLEMENT_MODIFIER)|* + $ComplementKey
        ))
    };
}
#[doc(inline)]
pub use crate::shortcut_macro as shortcut;
