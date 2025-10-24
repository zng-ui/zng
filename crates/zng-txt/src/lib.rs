#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! String type optimized for sharing.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::{borrow::Cow, fmt, hash::Hash, mem, ops::Deref, sync::Arc};

const INLINE_MAX: usize = mem::size_of::<usize>() * 3;

fn inline_to_str(d: &[u8; INLINE_MAX]) -> &str {
    let utf8 = if let Some(i) = d.iter().position(|&b| b == b'\0') {
        &d[..i]
    } else {
        &d[..]
    };
    std::str::from_utf8(utf8).unwrap()
}
fn str_to_inline(s: &str) -> [u8; INLINE_MAX] {
    let mut inline = [b'\0'; INLINE_MAX];
    inline[..s.len()].copy_from_slice(s.as_bytes());
    inline
}

#[derive(Clone)]
enum TxtData {
    Static(&'static str),
    Inline([u8; INLINE_MAX]),
    String(String),
    Arc(Arc<str>),
}
impl fmt::Debug for TxtData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            match self {
                Self::Static(s) => write!(f, "Static({s:?})"),
                Self::Inline(d) => write!(f, "Inline({:?})", inline_to_str(d)),
                Self::String(s) => write!(f, "String({s:?})"),
                Self::Arc(s) => write!(f, "Arc({s:?})"),
            }
        } else {
            write!(f, "{:?}", self.deref())
        }
    }
}
impl fmt::Display for TxtData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.deref())
    }
}
impl PartialEq for TxtData {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}
impl Eq for TxtData {}
impl Hash for TxtData {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Hash::hash(&self.deref(), state)
    }
}
impl Deref for TxtData {
    type Target = str;

    fn deref(&self) -> &str {
        match self {
            TxtData::Static(s) => s,
            TxtData::Inline(d) => inline_to_str(d),
            TxtData::String(s) => s,
            TxtData::Arc(s) => s,
        }
    }
}

/// Identifies how a [`Txt`] is currently storing the string data.
///
/// Use [`Txt::repr`] to retrieve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxtRepr {
    /// Text data is stored as a `&'static str`.
    Static,
    /// Text data is a small string stored as a null terminated `[u8; {size_of::<usize>() * 3}]`.
    Inline,
    /// Text data is stored as a `String`.
    String,
    /// Text data is stored as an `Arc<str>`.
    Arc,
}

/// Text string type, can be one of multiple internal representations, mostly optimized for sharing and one for editing.
///
/// This type dereferences to [`str`] so you can use all methods of that type.
///
/// For editing some mutable methods are provided, you can also call [`Txt::to_mut`]
/// to access all mutating methods of [`String`]. After editing you can call [`Txt::end_mut`] to convert
/// back to an inner representation optimized for sharing.
///
/// See [`Txt::repr`] for more details about the inner representations.
#[derive(PartialEq, Eq, Hash)]
pub struct Txt(TxtData);
/// Clones the text.
///
/// If the inner representation is [`TxtRepr::String`] the returned value is in a representation optimized
/// for sharing, either a static empty, an inlined short or an `Arc<str>` long string.
impl Clone for Txt {
    fn clone(&self) -> Self {
        Self(match &self.0 {
            TxtData::Static(s) => TxtData::Static(s),
            TxtData::Inline(d) => TxtData::Inline(*d),
            TxtData::String(s) => return Self::from_str(s),
            TxtData::Arc(s) => TxtData::Arc(Arc::clone(s)),
        })
    }
}
impl Txt {
    /// New text that is a `&'static str`.
    pub const fn from_static(s: &'static str) -> Txt {
        Txt(TxtData::Static(s))
    }

    /// New text from a [`String`] optimized for editing.
    ///
    /// If you don't plan to edit the text after this call consider using [`from_str`] instead.
    ///
    /// [`from_str`]: Self::from_str
    pub const fn from_string(s: String) -> Txt {
        Txt(TxtData::String(s))
    }

    /// New cloned from `s`.
    ///
    /// The text will be internally optimized for sharing, if you plan to edit the text after this call
    /// consider using [`from_string`] instead.
    ///
    /// [`from_string`]: Self::from_string
    #[expect(clippy::should_implement_trait)] // have implemented trait, this one is infallible.
    pub fn from_str(s: &str) -> Txt {
        if s.is_empty() {
            Self::from_static("")
        } else if s.len() <= INLINE_MAX && !s.contains('\0') {
            Self(TxtData::Inline(str_to_inline(s)))
        } else {
            Self(TxtData::Arc(Arc::from(s)))
        }
    }

    /// New from a shared arc str.
    ///
    /// Note that the text can outlive the `Arc`, by cloning the string data when modified or
    /// to use a more optimal representation, you cannot use the reference count of `s` to track
    /// the lifetime of the text.
    ///
    /// [`from_string`]: Self::from_string
    pub fn from_arc(s: Arc<str>) -> Txt {
        if s.is_empty() {
            Self::from_static("")
        } else if s.len() <= INLINE_MAX && !s.contains('\0') {
            Self(TxtData::Inline(str_to_inline(&s)))
        } else {
            Self(TxtData::Arc(s))
        }
    }

    /// New text that is an inlined `char`.
    pub fn from_char(c: char) -> Txt {
        #[allow(clippy::assertions_on_constants)]
        const _: () = assert!(4 <= INLINE_MAX, "cannot inline char");

        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);

        if s.contains('\0') {
            return Txt(TxtData::Arc(Arc::from(&*s)));
        }

        Txt(TxtData::Inline(str_to_inline(s)))
    }

    /// New text from [`format_args!`], avoids allocation if the text is static (no args) or can fit the inlined representation.
    pub fn from_fmt(args: std::fmt::Arguments) -> Txt {
        if let Some(s) = args.as_str() {
            Txt::from_static(s)
        } else {
            let mut r = Txt(TxtData::Inline([b'\0'; INLINE_MAX]));
            std::fmt::write(&mut r, args).unwrap();
            r
        }
    }

    /// Identifies how the text is currently stored.
    pub const fn repr(&self) -> TxtRepr {
        match &self.0 {
            TxtData::Static(_) => TxtRepr::Static,
            TxtData::Inline(_) => TxtRepr::Inline,
            TxtData::String(_) => TxtRepr::String,
            TxtData::Arc(_) => TxtRepr::Arc,
        }
    }

    /// Acquires a mutable reference to a [`String`] buffer.
    ///
    /// Converts the text to an internal representation optimized for editing, you can call [`end_mut`] after
    /// editing to re-optimize the text for sharing.
    ///
    /// [`end_mut`]: Self::end_mut
    pub fn to_mut(&mut self) -> &mut String {
        self.0 = match mem::replace(&mut self.0, TxtData::Static("")) {
            TxtData::String(s) => TxtData::String(s),
            TxtData::Static(s) => TxtData::String(s.to_owned()),
            TxtData::Inline(d) => TxtData::String(inline_to_str(&d).to_owned()),
            TxtData::Arc(s) => TxtData::String((*s).to_owned()),
        };

        if let TxtData::String(s) = &mut self.0 { s } else { unreachable!() }
    }

    /// Convert the inner representation of the string to not be [`String`]. After
    /// this call the text can be cheaply cloned.
    pub fn end_mut(&mut self) {
        match mem::replace(&mut self.0, TxtData::Static("")) {
            TxtData::String(s) => {
                *self = Self::from_str(&s);
            }
            already => self.0 = already,
        }
    }

    /// Extracts the owned string.
    ///
    /// Turns the text to owned if it was borrowed.
    pub fn into_owned(self) -> String {
        match self.0 {
            TxtData::String(s) => s,
            TxtData::Static(s) => s.to_owned(),
            TxtData::Inline(d) => inline_to_str(&d).to_owned(),
            TxtData::Arc(s) => (*s).to_owned(),
        }
    }

    /// Calls [`String::clear`] if the text is owned, otherwise
    /// replaces `self` with an empty str (`""`).
    pub fn clear(&mut self) {
        match &mut self.0 {
            TxtData::String(s) => s.clear(),
            d => *d = TxtData::Static(""),
        }
    }

    /// Removes the last character from the text and returns it.
    ///
    /// Returns None if this `Txt` is empty.
    ///
    /// This method only converts to [`TxtRepr::String`] if the
    /// internal representation is [`TxtRepr::Arc`], other representations are reborrowed.
    pub fn pop(&mut self) -> Option<char> {
        match &mut self.0 {
            TxtData::String(s) => s.pop(),
            TxtData::Static(s) => {
                if let Some((i, c)) = s.char_indices().last() {
                    *s = &s[..i];
                    Some(c)
                } else {
                    None
                }
            }
            TxtData::Inline(d) => {
                let s = inline_to_str(d);
                if let Some((i, c)) = s.char_indices().last() {
                    if i > 0 {
                        *d = str_to_inline(&s[..i]);
                    } else {
                        self.0 = TxtData::Static("");
                    }
                    Some(c)
                } else {
                    None
                }
            }
            TxtData::Arc(_) => self.to_mut().pop(),
        }
    }

    /// Shortens this `Txt` to the specified length.
    ///
    /// If `new_len` is greater than the text's current length, this has no
    /// effect.
    ///
    /// This method only converts to [`TxtRepr::String`] if the
    /// internal representation is [`TxtRepr::Arc`], other representations are reborrowed.
    pub fn truncate(&mut self, new_len: usize) {
        match &mut self.0 {
            TxtData::String(s) => s.truncate(new_len),
            TxtData::Static(s) => {
                if new_len <= s.len() {
                    assert!(s.is_char_boundary(new_len));
                    *s = &s[..new_len];
                }
            }
            TxtData::Inline(d) => {
                if new_len == 0 {
                    self.0 = TxtData::Static("");
                } else {
                    let s = inline_to_str(d);
                    if new_len < s.len() {
                        assert!(s.is_char_boundary(new_len));
                        d[new_len..].iter_mut().for_each(|b| *b = b'\0');
                    }
                }
            }
            TxtData::Arc(_) => self.to_mut().truncate(new_len),
        }
    }

    /// Splits the text into two at the given index.
    ///
    /// Returns a new `Txt`. `self` contains bytes `[0, at)`, and
    /// the returned `Txt` contains bytes `[at, len)`. `at` must be on the
    /// boundary of a UTF-8 code point.
    ///
    /// This method only converts to [`TxtRepr::String`] if the
    /// internal representation is [`TxtRepr::Arc`], other representations are reborrowed.
    pub fn split_off(&mut self, at: usize) -> Txt {
        match &mut self.0 {
            TxtData::String(s) => Txt::from_string(s.split_off(at)),
            TxtData::Static(s) => {
                assert!(s.is_char_boundary(at));
                let other = &s[at..];
                *s = &s[..at];
                Txt(TxtData::Static(other))
            }
            TxtData::Inline(d) => {
                let s = inline_to_str(d);
                assert!(s.is_char_boundary(at));
                let a_len = at;
                let b_len = s.len() - at;

                let r = Txt(if b_len == 0 {
                    TxtData::Static("")
                } else {
                    TxtData::Inline(str_to_inline(&s[at..]))
                });

                if a_len == 0 {
                    self.0 = TxtData::Static("");
                } else {
                    *d = str_to_inline(&s[..at]);
                }

                r
            }
            TxtData::Arc(_) => Txt::from_string(self.to_mut().split_off(at)),
        }
    }

    /// Push the character to the end of the text.
    ///
    /// This method avoids converting to [`TxtRepr::String`] when the current text
    /// plus char can fit inlined.
    pub fn push(&mut self, c: char) {
        match &mut self.0 {
            TxtData::String(s) => s.push(c),
            TxtData::Inline(inlined) => {
                if let Some(len) = inlined.iter().position(|&c| c == b'\0') {
                    let c_len = c.len_utf8();
                    if len + c_len <= INLINE_MAX && c != '\0' {
                        let mut buf = [0u8; 4];
                        let s = c.encode_utf8(&mut buf);
                        inlined[len..len + c_len].copy_from_slice(s.as_bytes());
                        return;
                    }
                }
                self.to_mut().push(c)
            }
            _ => {
                let len = self.len();
                let c_len = c.len_utf8();
                if len + c_len <= INLINE_MAX && c != '\0' {
                    let mut inlined = str_to_inline(self.as_str());
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    inlined[len..len + c_len].copy_from_slice(s.as_bytes());

                    self.0 = TxtData::Inline(inlined);
                } else {
                    self.to_mut().push(c)
                }
            }
        }
    }

    /// Push the string to the end of the text.
    ///
    /// This method avoids converting to [`TxtRepr::String`] when the current text
    /// plus char can fit inlined.
    pub fn push_str(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }

        match &mut self.0 {
            TxtData::String(str) => str.push_str(s),
            TxtData::Inline(inlined) => {
                if let Some(len) = inlined.iter().position(|&c| c == b'\0')
                    && len + s.len() <= INLINE_MAX
                    && !s.contains('\0')
                {
                    inlined[len..len + s.len()].copy_from_slice(s.as_bytes());
                    return;
                }
                self.to_mut().push_str(s)
            }
            _ => {
                let len = self.len();
                if len + s.len() <= INLINE_MAX && !s.contains('\0') {
                    let mut inlined = str_to_inline(self.as_str());
                    inlined[len..len + s.len()].copy_from_slice(s.as_bytes());

                    self.0 = TxtData::Inline(inlined);
                } else {
                    self.to_mut().push_str(s)
                }
            }
        }
    }

    /// Borrow the text as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.deref()
    }

    /// Copy the inner static `str` if this text represents one.
    pub fn as_static_str(&self) -> Option<&'static str> {
        match self.0 {
            TxtData::Static(s) => Some(s),
            _ => None,
        }
    }
}
impl fmt::Debug for Txt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}
impl fmt::Display for Txt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl Default for Txt {
    /// Empty.
    fn default() -> Self {
        Self::from_static("")
    }
}
impl std::str::FromStr for Txt {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Txt::from_str(s))
    }
}
impl From<&'static str> for Txt {
    fn from(value: &'static str) -> Self {
        Txt(TxtData::Static(value))
    }
}
impl From<String> for Txt {
    fn from(value: String) -> Self {
        Txt(TxtData::String(value))
    }
}
impl From<Cow<'static, str>> for Txt {
    fn from(value: Cow<'static, str>) -> Self {
        match value {
            Cow::Borrowed(s) => Txt(TxtData::Static(s)),
            Cow::Owned(s) => Txt(TxtData::String(s)),
        }
    }
}
impl From<char> for Txt {
    fn from(value: char) -> Self {
        Txt::from_char(value)
    }
}
impl From<Txt> for String {
    fn from(value: Txt) -> Self {
        value.into_owned()
    }
}
impl From<Txt> for Cow<'static, str> {
    fn from(value: Txt) -> Self {
        match value.0 {
            TxtData::Static(s) => Cow::Borrowed(s),
            TxtData::String(s) => Cow::Owned(s),
            TxtData::Inline(d) => Cow::Owned(inline_to_str(&d).to_owned()),
            TxtData::Arc(s) => Cow::Owned((*s).to_owned()),
        }
    }
}
impl From<Txt> for std::path::PathBuf {
    fn from(value: Txt) -> Self {
        value.into_owned().into()
    }
}
impl From<Txt> for Box<dyn std::error::Error> {
    fn from(err: Txt) -> Self {
        err.into_owned().into()
    }
}
impl From<Txt> for Box<dyn std::error::Error + Send + Sync> {
    fn from(err: Txt) -> Self {
        err.into_owned().into()
    }
}
impl From<Txt> for std::ffi::OsString {
    fn from(value: Txt) -> Self {
        String::from(value).into()
    }
}
impl std::ops::Deref for Txt {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}
impl AsRef<str> for Txt {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
impl AsRef<std::path::Path> for Txt {
    fn as_ref(&self) -> &std::path::Path {
        self.0.as_ref()
    }
}
impl AsRef<std::ffi::OsStr> for Txt {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}
impl std::borrow::Borrow<str> for Txt {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}
impl<'a> std::ops::Add<&'a str> for Txt {
    type Output = Txt;

    fn add(mut self, rhs: &'a str) -> Self::Output {
        self += rhs;
        self
    }
}
impl std::ops::AddAssign<&str> for Txt {
    fn add_assign(&mut self, rhs: &str) {
        self.push_str(rhs);
    }
}
impl PartialEq<&str> for Txt {
    fn eq(&self, other: &&str) -> bool {
        self.as_str().eq(*other)
    }
}
impl PartialEq<str> for Txt {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}
impl PartialEq<String> for Txt {
    fn eq(&self, other: &String) -> bool {
        self.as_str().eq(other)
    }
}
impl PartialEq<Txt> for &str {
    fn eq(&self, other: &Txt) -> bool {
        other.as_str().eq(*self)
    }
}
impl PartialEq<Txt> for str {
    fn eq(&self, other: &Txt) -> bool {
        other.as_str().eq(self)
    }
}
impl PartialEq<Txt> for String {
    fn eq(&self, other: &Txt) -> bool {
        other.as_str().eq(self)
    }
}
impl serde::Serialize for Txt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> serde::Deserialize<'de> for Txt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Txt::from)
    }
}
impl AsRef<[u8]> for Txt {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_ref()
    }
}
impl std::fmt::Write for Txt {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}
impl PartialOrd for Txt {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Txt {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}
impl FromIterator<char> for Txt {
    fn from_iter<I: IntoIterator<Item = char>>(iter: I) -> Txt {
        String::from_iter(iter).into()
    }
}
impl<'a> FromIterator<&'a char> for Txt {
    fn from_iter<I: IntoIterator<Item = &'a char>>(iter: I) -> Txt {
        String::from_iter(iter).into()
    }
}
impl<'a> FromIterator<&'a str> for Txt {
    fn from_iter<I: IntoIterator<Item = &'a str>>(iter: I) -> Txt {
        String::from_iter(iter).into()
    }
}
impl FromIterator<String> for Txt {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Txt {
        String::from_iter(iter).into()
    }
}
impl<'a> FromIterator<Cow<'a, str>> for Txt {
    fn from_iter<I: IntoIterator<Item = Cow<'a, str>>>(iter: I) -> Txt {
        String::from_iter(iter).into()
    }
}

impl FromIterator<Txt> for Txt {
    fn from_iter<I: IntoIterator<Item = Txt>>(iter: I) -> Txt {
        let mut iterator = iter.into_iter();

        match iterator.next() {
            None => Txt::from_static(""),
            Some(mut buf) => {
                buf.extend(iterator);
                buf
            }
        }
    }
}

impl Extend<char> for Txt {
    fn extend<T: IntoIterator<Item = char>>(&mut self, iter: T) {
        if let TxtData::String(s) = &mut self.0 {
            s.extend(iter);
        } else {
            let iter = iter.into_iter();
            let (lower_bound, _) = iter.size_hint();

            if self.len() + lower_bound < INLINE_MAX {
                // avoid alloc
                for c in iter {
                    self.push(c);
                }
            } else {
                self.to_mut().extend(iter);
            }
        }
    }
}
impl<'a> Extend<&'a char> for Txt {
    fn extend<I: IntoIterator<Item = &'a char>>(&mut self, iter: I) {
        self.extend(iter.into_iter().cloned());
    }
}
impl<'a> Extend<&'a str> for Txt {
    fn extend<I: IntoIterator<Item = &'a str>>(&mut self, iter: I) {
        iter.into_iter().for_each(move |s| self.push_str(s));
    }
}
impl Extend<String> for Txt {
    fn extend<I: IntoIterator<Item = String>>(&mut self, iter: I) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}
impl<'a> Extend<Cow<'a, str>> for Txt {
    fn extend<I: IntoIterator<Item = Cow<'a, str>>>(&mut self, iter: I) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

impl Extend<Txt> for Txt {
    fn extend<I: IntoIterator<Item = Txt>>(&mut self, iter: I) {
        iter.into_iter().for_each(move |s| self.push_str(&s));
    }
}

/// A trait for converting a value to a [`Txt`].
///
/// This trait is automatically implemented for any type that implements the [`ToString`] trait.
///
/// You can use [`formatx!`](macro.formatx.html) to `format!` a text.
pub trait ToTxt {
    /// Converts the given value to an owned [`Txt`].
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use zng_txt::*;
    ///
    /// let expected = formatx!("10");
    /// let actual = 10.to_txt();
    ///
    /// assert_eq!(expected, actual);
    /// ```
    fn to_txt(&self) -> Txt;
}
impl<T: ToString> ToTxt for T {
    fn to_txt(&self) -> Txt {
        self.to_string().into()
    }
}

///<span data-del-macro-root></span> Creates a [`Txt`] by formatting using the [`format_args!`] syntax.
///
/// Note that this behaves like a [`format!`] for [`Txt`], but it can be more performant because the
/// text type can represent `&'static str` and can i
///
/// # Examples
///
/// ```
/// # use zng_txt::formatx;
/// let text = formatx!("Hello {}", "World!");
/// ```
#[macro_export]
macro_rules! formatx {
    ($($tt:tt)*) => {
        {
            let res = $crate::Txt::from_fmt(format_args!($($tt)*));
            res
        }
    };
}
