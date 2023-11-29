//! Crate visible macros and utilities.

use crate::units::Deadline;

use rand::Rng;
use rustc_hash::FxHasher;
use std::{
    fmt,
    hash::{BuildHasher, Hash, Hasher},
    ops,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub use zero_ui_handle::*;
pub use zero_ui_unique_id::*;

/// Asserts the `size_of` a type at compile time.
#[allow(unused)]
macro_rules! assert_size_of {
    ($Type:ty, $n:expr) => {
        const _: () = assert!(std::mem::size_of::<$Type>() == $n);
    };
}

/// Asserts the `size_of` an `Option<T>` is the same as the size of `T` a type at compile time.
#[allow(unused)]
macro_rules! assert_non_null {
    ($Type:ty) => {
        const _: () = assert!(std::mem::size_of::<$Type>() == std::mem::size_of::<Option<$Type>>());
    };
}

/// Runs a cleanup action once on drop.
pub(crate) struct RunOnDrop<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> RunOnDrop<F> {
    pub fn new(clean: F) -> Self {
        RunOnDrop(Some(clean))
    }
}
impl<F: FnOnce()> Drop for RunOnDrop<F> {
    fn drop(&mut self) {
        if let Some(clean) = self.0.take() {
            clean();
        }
    }
}

/// Converts a [`std::panic::catch_unwind`] payload to a str.
pub fn panic_str<'s>(payload: &'s Box<dyn std::any::Any + Send + 'static>) -> &'s str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s
    } else {
        "<unknown-panic-message-type>"
    }
}

/// Type alias for the *error* of [`PanicResult`].
pub type PanicPayload = Box<dyn std::any::Any + Send + 'static>;

/// The result that is returned by [`std::panic::catch_unwind`].
pub type PanicResult<R> = thread::Result<R>;

// this is the FxHasher with random const init.
#[derive(Clone)]
pub struct BuildFxHasher(usize);
impl BuildFxHasher {
    #[allow(clippy::double_parens)] // const_random expands to ({n} as usize)
    const fn new() -> Self {
        BuildFxHasher(const_random::const_random!(usize))
    }
}
impl BuildHasher for BuildFxHasher {
    type Hasher = rustc_hash::FxHasher;

    fn build_hasher(&self) -> Self::Hasher {
        let mut hasher = FxHasher::default();
        hasher.write_usize(self.0);
        hasher
    }
}
impl Default for BuildFxHasher {
    fn default() -> Self {
        Self(rand::thread_rng().gen())
    }
}

/// Like [`rustc_hash::FxHashMap`] but faster deserialization and access to the raw_entry API.
///
/// Use [`fx_map_new`] for const init.
pub type FxHashMap<K, V> = hashbrown::HashMap<K, V, BuildFxHasher>;
/// Like [`rustc_hash::FxHashSet`] but faster deserialization.
///
/// Use [`fx_set_new`] for const init.
pub type FxHashSet<V> = hashbrown::HashSet<V, BuildFxHasher>;

/// Entry in [`FxHashMap`].
pub type FxEntry<'a, K, V> = hashbrown::hash_map::Entry<'a, K, V, BuildFxHasher>;

pub const fn fx_map_new<K, V>() -> FxHashMap<K, V> {
    hashbrown::HashMap::with_hasher(BuildFxHasher::new())
}
#[allow(unused)]
pub const fn fx_set_new<K>() -> FxHashSet<K> {
    hashbrown::HashSet::with_hasher(BuildFxHasher::new())
}

/// Resolves `..` components, without any system request.
///
/// Source: https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

/// Resolves relative paths in the `root` and normalizes then.
///
/// The `base` is only evaluated if the `path` is relative.
///
/// If `allow_escape` is `false`, relative paths with `..` cannot reference outside of `base`.
pub fn absolute_path(path: &Path, base: impl FnOnce() -> PathBuf, allow_escape: bool) -> PathBuf {
    if path.is_absolute() {
        normalize_path(path)
    } else {
        let mut dir = base();
        if allow_escape {
            dir.push(path);
            normalize_path(&dir)
        } else {
            dir.push(normalize_path(path));
            dir
        }
    }
}

/// A temporary directory for unit tests.
///
/// Directory is "target/tmp/unit_tests/<name>" with fallback to system temporary if the target folder is not found.
///
/// Auto cleanup on drop.
#[cfg(test)]
pub struct TestTempDir {
    path: Option<PathBuf>,
}
#[cfg(test)]
impl Drop for TestTempDir {
    fn drop(&mut self) {
        if let Some(path) = self.path.take() {
            let _ = remove_dir_all::remove_dir_all(path);
        }
    }
}
#[cfg(test)]
impl TestTempDir {
    /// Create temporary directory for the unique teste name.
    pub fn new(name: &str) -> Self {
        let path = Self::try_target().unwrap_or_else(Self::fallback).join(name);
        std::fs::create_dir_all(&path).unwrap_or_else(|e| panic!("failed to create temp `{}`, {e:?}", path.display()));
        TestTempDir { path: Some(path) }
    }
    fn try_target() -> Option<PathBuf> {
        let p = std::env::current_exe().ok()?;
        // target/debug/deps/../../..
        let target = p.parent()?.parent()?.parent()?;
        if target.file_name()?.to_str()? != "target" {
            return None;
        }
        Some(target.join("tmp/unit_tests"))
    }
    fn fallback() -> PathBuf {
        tracing::warn!("using fallback temporary directory");
        std::env::temp_dir().join("zero_ui/unit_tests")
    }

    /// Dereferences the temporary directory path.
    pub fn path(&self) -> &Path {
        self.path.as_deref().unwrap()
    }

    /// Drop `self` without removing the temporary files.
    ///
    /// Returns the path to the temporary directory.
    pub fn keep(mut self) -> PathBuf {
        self.path.take().unwrap()
    }
}
#[cfg(test)]
impl std::ops::Deref for TestTempDir {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.path()
    }
}
#[cfg(test)]
impl std::convert::AsRef<Path> for TestTempDir {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}
#[cfg(test)]
impl<'a> From<&'a TestTempDir> for std::path::PathBuf {
    fn from(a: &'a TestTempDir) -> Self {
        a.path.as_ref().unwrap().clone()
    }
}

/// Sets a `tracing` subscriber that writes warnings to stderr and panics on errors.
///
/// Panics if another different subscriber is already set.
#[cfg(any(test, feature = "test_util"))]
pub fn test_log() {
    use std::sync::atomic::*;

    use tracing::*;

    struct TestSubscriber;
    impl Subscriber for TestSubscriber {
        fn enabled(&self, metadata: &Metadata<'_>) -> bool {
            metadata.is_event() && metadata.level() < &Level::WARN
        }

        fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
            unimplemented!()
        }

        fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {
            unimplemented!()
        }

        fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {
            unimplemented!()
        }

        fn event(&self, event: &Event<'_>) {
            struct MsgCollector<'a>(&'a mut String);
            impl<'a> field::Visit for MsgCollector<'a> {
                fn record_debug(&mut self, field: &field::Field, value: &dyn fmt::Debug) {
                    use std::fmt::Write;
                    write!(self.0, "\n  {} = {:?}", field.name(), value).unwrap();
                }
            }

            let meta = event.metadata();
            let file = meta.file().unwrap_or("");
            let line = meta.line().unwrap_or(0);

            let mut msg = format!("[{file}:{line}]");
            event.record(&mut MsgCollector(&mut msg));

            if meta.level() == &Level::ERROR {
                panic!("[LOG-ERROR]{msg}");
            } else {
                eprintln!("[LOG-WARN]{msg}");
            }
        }

        fn enter(&self, _span: &span::Id) {
            unimplemented!()
        }
        fn exit(&self, _span: &span::Id) {
            unimplemented!()
        }
    }

    static IS_SET: AtomicBool = AtomicBool::new(false);

    if !IS_SET.swap(true, Ordering::Relaxed) {
        if let Err(e) = subscriber::set_global_default(TestSubscriber) {
            panic!("failed to set test log subscriber, {e:?}");
        }
    }
}

/// Calls [`fs4::FileExt::unlock`] and ignores "already unlocked" errors.
#[allow(unused)] // http only
pub fn unlock_ok(file: &impl fs4::FileExt) -> std::io::Result<()> {
    if let Err(e) = file.unlock() {
        if let Some(code) = e.raw_os_error() {
            #[cfg(windows)]
            if code == 158 {
                // ERROR_NOT_LOCKED
                return Ok(());
            }

            #[cfg(unix)]
            if code == 22 {
                // EINVAL
                return Ok(());
            }
        }

        Err(e)
    } else {
        Ok(())
    }
}

/// Calls [`fs4::FileExt::lock_exclusive`] with a timeout.
pub fn lock_exclusive(file: &impl fs4::FileExt, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_exclusive(), timeout)
}

/// Calls [`fs4::FileExt::lock_shared`] with a timeout.
pub fn lock_shared(file: &impl fs4::FileExt, timeout: Duration) -> std::io::Result<()> {
    lock_timeout(file, |f| f.try_lock_shared(), timeout)
}

fn lock_timeout<F: fs4::FileExt>(file: &F, try_lock: impl Fn(&F) -> std::io::Result<()>, mut timeout: Duration) -> std::io::Result<()> {
    let mut locked_error = None;
    loop {
        match try_lock(file) {
            Ok(()) => return Ok(()),
            Err(e) => {
                if e.raw_os_error() != locked_error.get_or_insert_with(fs4::lock_contended_error).raw_os_error() {
                    return Err(e);
                }

                const INTERVAL: Duration = Duration::from_millis(10);
                timeout = timeout.saturating_sub(INTERVAL);
                if timeout.is_zero() {
                    return Err(e);
                } else {
                    thread::sleep(INTERVAL.min(timeout));
                }
            }
        }
    }
}

/// Like [`std::ops::Range<usize>`], but implements [`Copy`].
#[derive(Clone, Copy)]
pub struct IndexRange(pub usize, pub usize);
impl fmt::Debug for IndexRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.0, self.1)
    }
}
impl IntoIterator for IndexRange {
    type Item = usize;

    type IntoIter = std::ops::Range<usize>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl From<IndexRange> for std::ops::Range<usize> {
    fn from(c: IndexRange) -> Self {
        c.iter()
    }
}
impl From<std::ops::Range<usize>> for IndexRange {
    fn from(r: std::ops::Range<usize>) -> Self {
        IndexRange(r.start, r.end)
    }
}
impl IndexRange {
    pub fn from_bounds(bounds: impl ops::RangeBounds<usize>) -> Self {
        // start..end
        let start = match bounds.start_bound() {
            ops::Bound::Included(&i) => i,
            ops::Bound::Excluded(&i) => i + 1,
            ops::Bound::Unbounded => 0,
        };
        let end = match bounds.end_bound() {
            ops::Bound::Included(&i) => i + 1,
            ops::Bound::Excluded(&i) => i,
            ops::Bound::Unbounded => 0,
        };
        Self(start, end)
    }

    /// Into `Range<usize>`.
    pub fn iter(self) -> std::ops::Range<usize> {
        self.0..self.1
    }

    /// `self.0`
    pub fn start(self) -> usize {
        self.0
    }

    /// `self.1`
    pub fn end(self) -> usize {
        self.1
    }

    /// `self.1.saturating_sub(1)`
    pub fn inclusive_end(self) -> usize {
        self.1.saturating_sub(1)
    }

    /// `self.end - self.start`
    pub fn len(self) -> usize {
        self.end() - self.start()
    }

    /// Gets if `i` is in range.
    pub fn contains(self, i: usize) -> bool {
        i >= self.0 && i < self.1
    }
}
impl std::ops::RangeBounds<usize> for IndexRange {
    fn start_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Included(&self.0)
    }

    fn end_bound(&self) -> std::ops::Bound<&usize> {
        std::ops::Bound::Excluded(&self.1)
    }
}

/// `f32` comparison, panics for `NaN`.
pub fn f32_cmp(a: &f32, b: &f32) -> std::cmp::Ordering {
    a.partial_cmp(b).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget_instance::WidgetId;
    use fs4::FileExt;

    #[test]
    pub fn sequential_id() {
        let id0 = WidgetId::new_unique();
        let id1 = WidgetId::new_unique();

        assert!(id0.sequential() < id1.sequential());
    }

    #[test]
    fn unlock_ok_exclusive_already_unlocked() {
        let dir = TestTempDir::new("unlock_ok_exclusive_already_unlocked");

        let file = std::fs::File::create(dir.join(".lock")).unwrap();
        file.lock_exclusive().unwrap();

        file.unlock().unwrap();

        unlock_ok(&file).unwrap();
    }

    #[test]
    fn unlock_ok_shared_already_unlocked() {
        let dir = TestTempDir::new("unlock_ok_shared_already_unlocked");

        let file = std::fs::File::create(dir.join(".lock")).unwrap();
        file.lock_shared().unwrap();

        file.unlock().unwrap();

        unlock_ok(&file).unwrap();
    }

    #[test]
    fn unlock_ok_exclusive_never_locked() {
        let dir = TestTempDir::new("unlock_ok_exclusive_never_locked");

        let file = std::fs::File::create(dir.join(".lock")).unwrap();

        unlock_ok(&file).unwrap();
    }
}

#[allow(unused)]
macro_rules! print_backtrace {
    () => {{
        let bt = std::backtrace::Backtrace::capture();
        println!("[{}:{}] BACKTRACE\n{bt}\n=====\n", file!(), line!())
    }};
}

/// Extension methods for [`flume::Receiver<T>`].
pub trait ReceiverExt<T> {
    /// Receive or precise timeout.
    fn recv_deadline_sp(&self, deadline: Deadline) -> Result<T, flume::RecvTimeoutError>;
}

const WORST_SLEEP_ERR: Duration = Duration::from_millis(if cfg!(windows) { 20 } else { 10 });
const WORST_SPIN_ERR: Duration = Duration::from_millis(if cfg!(windows) { 2 } else { 1 });

impl<T> ReceiverExt<T> for flume::Receiver<T> {
    fn recv_deadline_sp(&self, deadline: Deadline) -> Result<T, flume::RecvTimeoutError> {
        if let Some(d) = deadline.0.checked_duration_since(Instant::now()) {
            if d > WORST_SLEEP_ERR {
                // probably sleeps here.
                match self.recv_deadline(deadline.0.checked_sub(WORST_SLEEP_ERR).unwrap()) {
                    Err(flume::RecvTimeoutError::Timeout) => self.recv_deadline_sp(deadline),
                    interrupt => interrupt,
                }
            } else if d > WORST_SPIN_ERR {
                let spin_deadline = Deadline(deadline.0.checked_sub(WORST_SPIN_ERR).unwrap());

                // try_recv spin
                while !spin_deadline.has_elapsed() {
                    match self.try_recv() {
                        Err(flume::TryRecvError::Empty) => thread::yield_now(),
                        Err(flume::TryRecvError::Disconnected) => return Err(flume::RecvTimeoutError::Disconnected),
                        Ok(msg) => return Ok(msg),
                    }
                }
                self.recv_deadline_sp(deadline)
            } else {
                // last millis spin
                while !deadline.has_elapsed() {
                    std::thread::yield_now();
                }
                Err(flume::RecvTimeoutError::Timeout)
            }
        } else {
            Err(flume::RecvTimeoutError::Timeout)
        }
    }
}

/// Pre-compile generic variation so that dependent crates don't need to.
#[allow(unused)]
macro_rules! share_generics {
    ($f:path) => {
        #[doc(hidden)]
        #[cfg(debug_assertions)]
        pub const _: *const () = (&$f) as *const _ as _;
    };
}

#[allow(unused)]
#[doc(hidden)]
pub(crate) struct MeasureTime {
    msg: &'static str,
    started: std::time::Instant,
}
impl MeasureTime {
    #[allow(unused)]
    pub(crate) fn start(msg: &'static str) -> Self {
        MeasureTime {
            msg,
            started: std::time::Instant::now(),
        }
    }
}
impl Drop for MeasureTime {
    fn drop(&mut self) {
        println!("{}: {:?}", self.msg, self.started.elapsed());
    }
}

/// Time an operation, time elapsed is printed on drop.
#[allow(unused)]
macro_rules! measure_time {
    ($msg:tt) => {
        $crate::crate_util::MeasureTime::start($msg)
    };
}

#[allow(unused)]
pub(crate) struct RecursionCheck {
    count: AtomicUsize,
    limit: usize,
}
#[allow(unused)]
impl RecursionCheck {
    pub const fn new(limit: usize) -> Self {
        RecursionCheck {
            count: AtomicUsize::new(0),
            limit,
        }
    }

    pub fn enter(&'static self) -> RecursionCheckExitOnDrop {
        let c = self.count.fetch_add(1, Ordering::Relaxed);
        if c >= self.limit {
            panic!("reached {} limit, probably recursing", self.limit);
        }
        RecursionCheckExitOnDrop { check: self }
    }
}

#[must_use = "must be held while calling inner"]
#[allow(unused)]
pub(crate) struct RecursionCheckExitOnDrop {
    check: &'static RecursionCheck,
}
impl Drop for RecursionCheckExitOnDrop {
    fn drop(&mut self) {
        self.check.count.fetch_sub(1, Ordering::Relaxed);
    }
}

/// See [`ParallelSegmentOffsets`].
pub(crate) type ParallelSegmentId = usize;

/// Tracks the position of a range of items in a list that was built in parallel.
#[derive(Debug, Clone)]
pub(crate) struct ParallelSegmentOffsets {
    id: ParallelSegmentId,
    id_gen: Arc<AtomicUsize>,
    used: bool,
    segments: Vec<(ParallelSegmentId, usize)>,
}
impl Default for ParallelSegmentOffsets {
    fn default() -> Self {
        Self {
            id: 0,
            used: false,
            id_gen: Arc::new(AtomicUsize::new(1)),
            segments: vec![],
        }
    }
}
impl ParallelSegmentOffsets {
    /// Gets the segment ID and flags the current tracking offsets as used.
    pub fn id(&mut self) -> ParallelSegmentId {
        self.used = true;
        self.id
    }

    /// Resolve the `id` offset, after build.
    pub fn offset(&self, id: ParallelSegmentId) -> usize {
        self.segments
            .iter()
            .find_map(|&(i, o)| if i == id { Some(o) } else { None })
            .unwrap_or_else(|| {
                if id != 0 {
                    tracing::error!(target: "parallel", "segment offset for `{id}` not found");
                }
                0
            })
    }

    /// Start new parallel segment.
    pub fn parallel_split(&self) -> Self {
        Self {
            used: false,
            id: self.id_gen.fetch_add(1, atomic::Ordering::Relaxed),
            id_gen: self.id_gen.clone(),
            segments: vec![],
        }
    }

    /// Merge parallel segment at the given `offset`.
    pub fn parallel_fold(&mut self, mut split: Self, offset: usize) {
        if !Arc::ptr_eq(&self.id_gen, &split.id_gen) {
            tracing::error!(target: "parallel", "cannot parallel fold segments not split from the same root");
            return;
        }

        if offset > 0 {
            for (_, o) in &mut split.segments {
                *o += offset;
            }
        }

        if self.segments.is_empty() {
            self.segments = split.segments;
        } else {
            self.segments.append(&mut split.segments);
        }
        if split.used {
            self.segments.push((split.id, offset));
        }
    }
}

/// Borrow tuple keys for maps.
///
/// Usage:
/// ```txt
/// map.get(&(a, b) as &dyn KeyPair<A, B>)
/// ```
///
/// Thanks: https://stackoverflow.com/questions/45786717/how-to-implement-hashmap-with-two-keys/45795699#45795699
pub trait KeyPair<A, B> {
    fn a(&self) -> &A;
    fn b(&self) -> &B;
}
impl<'a, A, B> std::borrow::Borrow<dyn KeyPair<A, B> + 'a> for (A, B)
where
    A: Eq + Hash + 'a,
    B: Eq + Hash + 'a,
{
    fn borrow(&self) -> &(dyn KeyPair<A, B> + 'a) {
        self
    }
}
impl<A: Hash, B: Hash> Hash for dyn KeyPair<A, B> + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a().hash(state);
        self.b().hash(state);
    }
}
impl<A: Eq, B: Eq> PartialEq for dyn KeyPair<A, B> + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.a() == other.a() && self.b() == other.b()
    }
}
impl<A: Eq, B: Eq> Eq for dyn KeyPair<A, B> + '_ {}
impl<A, B> KeyPair<A, B> for (A, B) {
    fn a(&self) -> &A {
        &self.0
    }
    fn b(&self) -> &B {
        &self.1
    }
}
impl<A, B> KeyPair<A, B> for (&A, &B) {
    fn a(&self) -> &A {
        self.0
    }
    fn b(&self) -> &B {
        self.1
    }
}
