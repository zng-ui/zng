use std::{
    cmp::Ordering,
    ffi::{OsStr, OsString},
    fmt, ops,
    path::{Path, PathBuf},
};

type GlobItem = Result<walkdir::DirEntry, walkdir::Error>;

/// Walk all `pattern` matches.
///
/// The `pattern` is split into path prefix and actual glob, for example, `"some/path/*/*.txt"` is
/// split into path `"some/path"` and glob `"*/*.txt"`, the syntax rules apply only to the glob part.
///
/// The iterator sorts by file name, using the system sorting function. See [`FileNameSortKey`] for more details.
///
/// ## Syntax
///
/// Standard Unix-style glob syntax is supported (provided by [`globset`] crate):
///
/// * `?` matches any single character.
/// * `*` matches zero or more characters.
/// * `**` recursively matches directories but are only legal in three situations.
///   First, if the glob starts with <code>\*\*&#x2F;</code>, then it matches
///   all directories. For example, <code>\*\*&#x2F;foo</code> matches `foo`
///   and `bar/foo` but not `foo/bar`. Secondly, if the glob ends with
///   <code>&#x2F;\*\*</code>, then it matches all sub-entries. For example,
///   <code>foo&#x2F;\*\*</code> matches `foo/a` and `foo/a/b`, but not `foo`.
///   Thirdly, if the glob contains <code>&#x2F;\*\*&#x2F;</code> anywhere within
///   the pattern, then it matches zero or more directories. Using `**` anywhere
///   else is illegal (Note that the glob `**` is allowed and means "match everything").
/// * `{a,b}` matches `a` or `b` where `a` and `b` are arbitrary glob patterns or empty.
///   (Note that nesting `{...}` is allowed.)
/// * `[ab]` matches `a` or `b` where `a` and `b` are characters. Use
///   `[!ab]` to match any character except for `a` and `b`.
/// * `[[:alpha:]]`, `[[:digit:]]`, `[[:space:]]` etc. match POSIX character
///   classes using ASCII-only definitions. Supported classes: `alnum`, `alpha`,
///   `blank`, `cntrl`, `digit`, `graph`, `lower`, `print`, `punct`, `space`,
///   `upper`, `xdigit`. These can be combined with regular ranges, e.g.,
///   `[[:digit:]a-f]` matches hexadecimal digits.
/// * Metacharacters such as `*` and `?` can be escaped with character class
///   notation. e.g., `[*]` matches `*`. Backslash does not escape, it matches path separators on all platforms.
/// * `/` and `\\` matches path separators.
///
/// [`globset`]: https://docs.rs/globset/latest/globset/
///
/// ## Performance
///
/// A [`walkdir`] iterator is configured to iterator over only the file depths that might match and each file is
/// checked against the compiled pattern, this is generally pretty fast, but note that if `**` is in the pattern
/// the entire directory depth is searched.
///
/// ## Configuration
///
/// Use [`glob_build`] to modify the [`walkdir::WalkDir`] and [`globset::GlobBuilder`] before iteration starts.
///
/// [`walkdir`]: https://docs.rs/walkdir/latest/walkdir/
/// [`walkdir::WalkDir`]: https://docs.rs/walkdir/latest/walkdir/struct.WalkDir.html
/// [`globset::GlobBuilder`]: https://docs.rs/globset/latest/globset/struct.GlobBuilder.html
pub fn glob(pattern: impl AsRef<Path>, case_insensitive: bool) -> Result<impl Iterator<Item = GlobItem> + Send + Sync, globset::Error> {
    glob_build_impl(
        pattern.as_ref(),
        &mut |w| w.sort_by(|a, b| FileNameSortKey::cmp_ref(a.file_name(), b.file_name())),
        &mut move |g| g.case_insensitive(case_insensitive).literal_separator(true).empty_alternates(true),
    )
}

/// Build a [`glob`] iterator with custom walker and glob build.
///
/// The `walker` and `glob` are called after the pattern is split into path prefix and glob.
pub fn glob_build(
    pattern: impl AsRef<Path>,
    walker: impl FnOnce(walkdir::WalkDir) -> walkdir::WalkDir,
    glob: impl for<'a> FnOnce(&'a mut globset::GlobBuilder<'a>) -> &'a mut globset::GlobBuilder<'a>,
) -> Result<impl Iterator<Item = GlobItem> + Send + Sync, globset::Error> {
    let mut walker = Some(walker);
    let mut glob = Some(glob);
    glob_build_impl(pattern.as_ref(), &mut |w| walker.take().unwrap()(w), &mut |g| {
        glob.take().unwrap()(g)
    })
}
fn glob_build_impl(
    pattern: &Path,
    walker: &mut dyn FnMut(walkdir::WalkDir) -> walkdir::WalkDir,
    glob: &mut dyn for<'a> FnMut(&'a mut globset::GlobBuilder<'a>) -> &'a mut globset::GlobBuilder<'a>,
) -> Result<impl Iterator<Item = GlobItem> + Send + Sync + use<>, globset::Error> {
    const META: [char; 4] = ['*', '?', '[', '{'];
    // split root/glob
    let mut root = PathBuf::new();
    let mut glob_parts: Vec<&str> = Vec::new();
    for comp in pattern.components() {
        let s = comp.as_os_str().to_str().unwrap();
        if glob_parts.is_empty() && !s.contains(META) {
            root.push(comp);
        } else {
            glob_parts.push(s);
        }
    }
    if root.as_os_str().is_empty() {
        root.push(".");
    }

    let mut walker = walker(walkdir::WalkDir::new(&root));

    let matcher = if glob_parts.is_empty() {
        // pattern is a path
        walker = walker.max_depth(0);
        None
    } else {
        if !glob_parts.iter().any(|c| c.contains("**")) {
            // glob has known depth
            walker = walker.max_depth(glob_parts.len());
        }
        walker = walker.min_depth(1);
        let glob_str = glob_parts.join("/");
        let mut builder = globset::GlobBuilder::new(&glob_str);
        Some(glob(&mut builder).backslash_escape(false).build()?.compile_matcher())
    };

    Ok(walker.into_iter().filter(move |entry| {
        let (Some(matcher), Ok(entry)) = (&matcher, entry) else {
            return true; // literal path, or an error
        };
        let rel = entry.path().strip_prefix(&root).unwrap_or(entry.path());
        matcher.is_match(rel)
    }))
}

/// Represents a comparison key for sorting file names using the system name order.
///
/// On Windows uses [`StrCmpLogicalW`], on macOS uses [`localizedStandardCompare`], on Linux
/// uses [`natord::compare_ignore_case`] as fallback.
///
/// If two names are equal according with the system function the original name is compared.
///
/// [`natord::compare_ignore_case`]: https://docs.rs/natord/latest/natord/fn.compare_ignore_case.html
/// [`StrCmpLogicalW`]: https://learn.microsoft.com/en-us/windows/win32/api/shlwapi/nf-shlwapi-strcmplogicalw
/// [`localizedStandardCompare`]: https://developer.apple.com/documentation/foundation/nsstring/localizedstandardcompare(_:)
#[derive(Clone)]
pub struct FileNameSortKey {
    native_key: NativeFileNameSortKey,
    original: OsString,
}
impl FileNameSortKey {
    /// New key.
    ///
    /// Note that if you already have an `OsString` it will convert to a key without heap copy.
    pub fn new(file_name: &OsStr) -> Self {
        file_name.to_os_string().into()
    }
}
impl From<OsString> for FileNameSortKey {
    fn from(file_name: OsString) -> Self {
        Self {
            native_key: NativeFileNameSortKey::new(&file_name),
            original: file_name,
        }
    }
}
impl From<FileNameSortKey> for OsString {
    fn from(k: FileNameSortKey) -> Self {
        k.original
    }
}
impl ops::Deref for FileNameSortKey {
    type Target = OsStr;

    fn deref(&self) -> &Self::Target {
        &self.original
    }
}
impl PartialEq for FileNameSortKey {
    fn eq(&self, other: &Self) -> bool {
        self.original == other.original
    }
}
impl Eq for FileNameSortKey {}
impl Ord for FileNameSortKey {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.native_key.cmp(&other.native_key) {
            Ordering::Equal => self.original.cmp(&other.original),
            ord => ord,
        }
    }
}
impl PartialOrd for FileNameSortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl std::hash::Hash for FileNameSortKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.original.hash(state);
    }
}
impl fmt::Debug for FileNameSortKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FileNameSortKey")
            .field("file_name", &self.original)
            .finish_non_exhaustive()
    }
}
impl AsRef<OsStr> for FileNameSortKey {
    fn as_ref(&self) -> &OsStr {
        &self.original
    }
}
impl AsRef<Path> for FileNameSortKey {
    fn as_ref(&self) -> &Path {
        Path::new(self)
    }
}

impl FileNameSortKey {
    /// Compare two names with zero or minimal allocation.
    ///
    /// On Windows it will only allocate if the name is longer than 260 chars, but note that
    /// it will re-encode to UTF-16 for every comparison.
    ///
    /// On macOS a small fixed size header is still allocated.
    ///
    /// For very large lists it might be faster to allocate all keys upfront, but in cases where
    /// the keys are discarded after each comparison this is faster.
    pub fn cmp_ref(a: &OsStr, b: &OsStr) -> Ordering {
        match NativeFileNameSortKey::cmp_ref(a, b) {
            Ordering::Equal => a.cmp(b),
            ord => ord,
        }
    }
}

#[cfg(target_os = "windows")]
#[derive(Clone)]
struct NativeFileNameSortKey {
    stem_wide: Vec<u16>,
    ext_wide: Vec<u16>,
}
#[cfg(target_os = "windows")]
impl NativeFileNameSortKey {
    fn new(file_name: &OsStr) -> Self {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        let path = Path::new(file_name);

        fn to_wide_null(s: Option<&OsStr>) -> Vec<u16> {
            s.into_iter().flat_map(|os| os.encode_wide()).chain(std::iter::once(0)).collect()
        }

        Self {
            stem_wide: to_wide_null(path.file_stem()),
            ext_wide: to_wide_null(path.extension()),
        }
    }

    fn cmp(&self, other: &Self) -> Ordering {
        use windows::Win32::UI::Shell::StrCmpLogicalW;
        use windows::core::PCWSTR;

        // SAFETY: stem_wide is a valid conversion to utf-16, null terminated
        let stem_ord = unsafe { StrCmpLogicalW(PCWSTR(self.stem_wide.as_ptr()), PCWSTR(other.stem_wide.as_ptr())) }.cmp(&0);

        // SAFETY: ext_wide is a valid conversion to utf-16, null terminated
        match stem_ord {
            Ordering::Equal => unsafe { StrCmpLogicalW(PCWSTR(self.ext_wide.as_ptr()), PCWSTR(other.ext_wide.as_ptr())) }.cmp(&0),
            ord => ord,
        }
    }

    fn cmp_ref(a: &OsStr, b: &OsStr) -> Ordering {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::UI::Shell::StrCmpLogicalW;
        use windows::core::PCWSTR;

        #[allow(clippy::large_enum_variant)] // we want the Stack alloc
        enum WideBuf {
            Stack([u16; 260]),
            Heap(Vec<u16>),
        }

        impl WideBuf {
            fn new(s: Option<&OsStr>) -> Self {
                let mut stack = [0u16; 260];
                let Some(s) = s else { return Self::Stack(stack) };

                let mut i = 0;
                for unit in s.encode_wide() {
                    if i < 259 {
                        stack[i] = unit;
                        i += 1;
                    } else {
                        // Rare fallback for >259 char filenames
                        let mut heap = Vec::with_capacity(261);
                        heap.extend_from_slice(&stack[..i]);
                        heap.extend(s.encode_wide().skip(i));
                        heap.push(0);
                        return Self::Heap(heap);
                    }
                }
                stack[i] = 0;
                Self::Stack(stack)
            }

            fn as_pcwstr(&self) -> PCWSTR {
                match self {
                    Self::Stack(arr) => PCWSTR(arr.as_ptr()),
                    Self::Heap(vec) => PCWSTR(vec.as_ptr()),
                }
            }
        }

        let cmp_part = |part_a: Option<&OsStr>, part_b: Option<&OsStr>| -> Ordering {
            if part_a == part_b {
                return Ordering::Equal;
            }
            let buf_a = WideBuf::new(part_a);
            let buf_b = WideBuf::new(part_b);
            // SAFETY: Both buffers are guaranteed null-terminated UTF-16.
            unsafe { StrCmpLogicalW(buf_a.as_pcwstr(), buf_b.as_pcwstr()) }.cmp(&0)
        };

        let (path_a, path_b) = (Path::new(a), Path::new(b));
        match cmp_part(path_a.file_stem(), path_b.file_stem()) {
            Ordering::Equal => cmp_part(path_a.extension(), path_b.extension()),
            ord => ord,
        }
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone)]
struct NativeFileNameSortKey {
    stem_ns: objc2::rc::Retained<objc2_foundation::NSString>,
    ext_ns: objc2::rc::Retained<objc2_foundation::NSString>,
}
#[cfg(target_os = "macos")]
impl NativeFileNameSortKey {
    fn new(file_name: &OsStr) -> Self {
        use objc2::ClassType;
        use objc2_foundation::{NSString, NSUTF8StringEncoding};
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        use std::ptr::NonNull;

        let path = Path::new(file_name);

        fn to_ns_string(s: Option<&OsStr>) -> objc2::rc::Retained<NSString> {
            let bytes = s.map(OsStr::as_bytes).unwrap_or(&[]);
            if !bytes.is_empty() {
                // SAFETY: `bytes` is valid for `bytes.len()` and `NonNull` is non-null.
                let ptr = unsafe { NonNull::new_unchecked(bytes.as_ptr() as *mut std::ffi::c_void) };
                if let Some(ns) =
                    unsafe { NSString::initWithBytes_length_encoding(NSString::alloc(), ptr, bytes.len(), NSUTF8StringEncoding) }
                {
                    return ns;
                }
            }
            NSString::from_str(&s.unwrap_or_default().to_string_lossy())
        }

        Self {
            stem_ns: to_ns_string(path.file_stem()),
            ext_ns: to_ns_string(path.extension()),
        }
    }

    fn cmp(&self, other: &Self) -> Ordering {
        use objc2_foundation::NSComparisonResult;

        // SAFETY: Both `NSString` instances are valid Objective-C strings.
        let stem_ord = match unsafe { self.stem_ns.localizedStandardCompare(&other.stem_ns) } {
            NSComparisonResult::Ascending => Ordering::Less,
            NSComparisonResult::Same => Ordering::Equal,
            NSComparisonResult::Descending => Ordering::Greater,
        };

        match stem_ord {
            // SAFETY: Both `NSString` instances are valid Objective-C strings.
            Ordering::Equal => match unsafe { self.ext_ns.localizedStandardCompare(&other.ext_ns) } {
                NSComparisonResult::Ascending => Ordering::Less,
                NSComparisonResult::Same => Ordering::Equal,
                NSComparisonResult::Descending => Ordering::Greater,
            },
            ord => ord,
        }
    }

    fn cmp_ref(a: &OsStr, b: &OsStr) -> Ordering {
        use objc2::ClassType;
        use objc2_foundation::{NSComparisonResult, NSString, NSUTF8StringEncoding};
        use std::borrow::Cow;
        use std::os::unix::ffi::OsStrExt;
        use std::ptr::NonNull;

        fn with_ns_string<R>(s: Option<&OsStr>, f: impl FnOnce(&NSString) -> R) -> R {
            let bytes = s.map(OsStr::as_bytes).unwrap_or(&[]);
            if !bytes.is_empty() {
                // SAFETY: `ns` does not outlive `bytes`, `freeWhenDone` is `false`, and `ptr` is non-null.
                let ptr = unsafe { NonNull::new_unchecked(bytes.as_ptr() as *mut std::ffi::c_void) };
                if let Some(ns) = unsafe {
                    NSString::initWithBytesNoCopy_length_encoding_freeWhenDone(
                        NSString::alloc(),
                        ptr,
                        bytes.len(),
                        NSUTF8StringEncoding,
                        false,
                    )
                } {
                    return f(&ns);
                }
            }

            let lossy: Cow<'_, str> = s.unwrap_or_default().to_string_lossy();
            let ns = NSString::from_str(&lossy);
            f(&ns)
        }

        let cmp_part = |part_a: Option<&OsStr>, part_b: Option<&OsStr>| -> Ordering {
            if part_a == part_b {
                return Ordering::Equal;
            }
            with_ns_string(part_a, |ns_a| {
                with_ns_string(part_b, |ns_b| {
                    // SAFETY: Both `NSString` instances are valid for the duration of the closure.
                    match unsafe { ns_a.localizedStandardCompare(ns_b) } {
                        NSComparisonResult::Ascending => Ordering::Less,
                        NSComparisonResult::Same => Ordering::Equal,
                        NSComparisonResult::Descending => Ordering::Greater,
                    }
                })
            })
        };

        let (path_a, path_b) = (Path::new(a), Path::new(b));
        match cmp_part(path_a.file_stem(), path_b.file_stem()) {
            Ordering::Equal => cmp_part(path_a.extension(), path_b.extension()),
            ord => ord,
        }
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
#[derive(Clone)]
struct NativeFileNameSortKey {
    stem_str: Box<str>,
    ext_str: Box<str>,
}
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
impl NativeFileNameSortKey {
    fn new(file_name: &OsStr) -> Self {
        let path = Path::new(file_name);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        let ext = path.extension().unwrap_or_default().to_string_lossy();
        Self {
            stem_str: stem.into_owned().into_boxed_str(),
            ext_str: ext.into_owned().into_boxed_str(),
        }
    }

    fn cmp(&self, other: &Self) -> Ordering {
        match natord::compare_ignore_case(&self.stem_str, &other.stem_str) {
            Ordering::Equal => natord::compare_ignore_case(&self.ext_str, &other.ext_str),
            ord => ord,
        }
    }

    fn cmp_ref(a: &OsStr, b: &OsStr) -> Ordering {
        let cmp_part = |part_a: Option<&OsStr>, part_b: Option<&OsStr>| -> Ordering {
            if part_a == part_b {
                return Ordering::Equal;
            }
            let str_a = part_a.unwrap_or_default().to_string_lossy();
            let str_b = part_b.unwrap_or_default().to_string_lossy();
            natord::compare_ignore_case(&str_a, &str_b)
        };

        let (path_a, path_b) = (Path::new(a), Path::new(b));
        match cmp_part(path_a.file_stem(), path_b.file_stem()) {
            Ordering::Equal => cmp_part(path_a.extension(), path_b.extension()),
            ord => ord,
        }
    }
}
