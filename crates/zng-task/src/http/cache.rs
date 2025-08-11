use std::{
    fmt,
    time::{Duration, SystemTime},
};

use super::{Body, Error};
use async_trait::async_trait;
use serde::*;
use zng_unit::*;

use http_cache_semantics as hcs;

pub(super) use hcs::BeforeRequest;

/// Represents a serializable configuration for a cache entry in a [`CacheDb`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy(PolicyInner);
impl CachePolicy {
    pub(super) fn new(request: &isahc::Request<super::Body>, response: &isahc::Response<isahc::AsyncBody>) -> Self {
        let p = hcs::CachePolicy::new_options(
            request,
            response,
            SystemTime::now(),
            hcs::CacheOptions {
                shared: false,
                ignore_cargo_cult: true,
                ..Default::default()
            },
        );
        Self(PolicyInner::Policy(p))
    }

    pub(super) fn should_store(&self) -> bool {
        match &self.0 {
            PolicyInner::Policy(p) => p.is_storable() && p.time_to_live(SystemTime::now()) > 5.secs(),
            PolicyInner::Permanent(_) => true,
        }
    }

    pub(super) fn new_permanent(response: &isahc::Response<isahc::AsyncBody>) -> Self {
        let p = PermanentHeader {
            res: response.headers().clone(),
            status: response.status(),
        };
        Self(PolicyInner::Permanent(p))
    }

    pub(super) fn is_permanent(&self) -> bool {
        matches!(self.0, PolicyInner::Permanent(_))
    }

    pub(super) fn before_request(&self, request: &isahc::Request<super::Body>) -> BeforeRequest {
        match &self.0 {
            PolicyInner::Policy(p) => p.before_request(request, SystemTime::now()),
            PolicyInner::Permanent(p) => BeforeRequest::Fresh(p.parts()),
        }
    }

    pub(super) fn after_response(
        &self,
        request: &isahc::Request<super::Body>,
        response: &isahc::Response<isahc::AsyncBody>,
    ) -> AfterResponse {
        match &self.0 {
            PolicyInner::Policy(p) => p.after_response(request, response, SystemTime::now()).into(),
            PolicyInner::Permanent(_) => unreachable!(), // don't call `after_response` for `Fresh` `before_request`
        }
    }

    /// Returns how long the response has been sitting in cache.
    pub fn age(&self, now: SystemTime) -> Duration {
        match &self.0 {
            PolicyInner::Policy(p) => p.age(now),
            PolicyInner::Permanent(_) => Duration::MAX,
        }
    }

    /// Returns approximate time in milliseconds until the response becomes stale.
    pub fn time_to_live(&self, now: SystemTime) -> Duration {
        match &self.0 {
            PolicyInner::Policy(p) => p.time_to_live(now),
            PolicyInner::Permanent(_) => Duration::MAX,
        }
    }

    /// Returns `true` if the cache entry has expired.
    pub fn is_stale(&self, now: SystemTime) -> bool {
        match &self.0 {
            PolicyInner::Policy(p) => p.is_stale(now),
            PolicyInner::Permanent(_) => false,
        }
    }
}
impl From<hcs::CachePolicy> for CachePolicy {
    fn from(p: hcs::CachePolicy) -> Self {
        CachePolicy(PolicyInner::Policy(p))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::large_enum_variant)]
enum PolicyInner {
    Policy(hcs::CachePolicy),
    Permanent(PermanentHeader),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PermanentHeader {
    #[serde(with = "http_serde::header_map")]
    res: super::header::HeaderMap,
    #[serde(with = "http_serde::status_code")]
    status: super::StatusCode,
}
impl PermanentHeader {
    pub fn parts(&self) -> isahc::http::response::Parts {
        let mut r = isahc::Response::<()>::default().into_parts().0;
        r.headers = self.res.clone();
        r.status = self.status;
        r
    }
}

/// New policy and flags to act on `after_response()`
pub(super) enum AfterResponse {
    /// You can use the cached body! Make sure to use these updated headers
    NotModified(CachePolicy, isahc::http::response::Parts),
    /// You need to update the body in the cache
    Modified(CachePolicy, isahc::http::response::Parts),
}
impl From<hcs::AfterResponse> for AfterResponse {
    fn from(s: hcs::AfterResponse) -> Self {
        match s {
            hcs::AfterResponse::NotModified(po, pa) => AfterResponse::NotModified(po.into(), pa),
            hcs::AfterResponse::Modified(po, pa) => AfterResponse::Modified(po.into(), pa),
        }
    }
}

/// Represents a download cache in a [`Client`].
///
/// Cache implementers must store a [`CachePolicy`] and [`Body`] for a given [`CacheKey`].
///
/// [`Client`]: crate::http::Client
#[async_trait]
pub trait CacheDb: Send + Sync + 'static {
    /// Dynamic clone.
    fn clone_boxed(&self) -> Box<dyn CacheDb>;

    /// Retrieves the cache-policy for the given `key`.
    async fn policy(&self, key: &CacheKey) -> Option<CachePolicy>;

    /// Replaces the cache-policy for the given `key`.
    ///
    /// Returns `false` if the entry does not exist.
    async fn set_policy(&self, key: &CacheKey, policy: CachePolicy) -> bool;

    /// Read/clone the cached body for the given `key`.
    async fn body(&self, key: &CacheKey) -> Option<Body>;

    /// Caches the `policy` and `body` for the given `key`.
    ///
    /// The `body` is fully downloaded and stored into the cache, this method can await for the full download
    /// before returning or return immediately with a body that updates as data is cached.
    ///
    /// In case of error the cache entry is removed, the returned body may continue downloading data if possible.
    /// In case of a cache entry creation error the input `body` may be returned if it was not lost in the error.
    async fn set(&self, key: &CacheKey, policy: CachePolicy, body: Body) -> Option<Body>;

    /// Remove cached policy and body for the given `key`.
    async fn remove(&self, key: &CacheKey);

    /// Remove all cached entries that are not locked in a `set*` operation.
    async fn purge(&self);

    /// Remove cache entries to reduce pressure.
    ///
    /// What entries are removed depends on the cache DB implementer.
    async fn prune(&self);
}

/// Cache mode selected for a [`Uri`].
///
/// See [`ClientBuilder::cache_mode`] for more information.
///
/// [`Uri`]: crate::http::Uri
///
/// [`ClientBuilder::cache_mode`]: crate::http::ClientBuilder::cache_mode
#[derive(Debug, Clone, Default)]
pub enum CacheMode {
    /// Always requests the server, never caches the response.
    NoCache,

    /// Follow the standard cache policy as computed by [`http-cache-semantics`].
    ///
    /// [`http-cache-semantics`]: https://docs.rs/http-cache-semantics
    #[default]
    Default,

    /// Always caches the response, overwriting cache control configs.
    ///
    /// If the response is cached returns it instead of requesting an update.
    Permanent,

    /// Returns the error.
    Error(Error),
}

/// Represents a SHA-512/256 hash computed from a normalized request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey([u8; 32]);
impl CacheKey {
    /// Compute key from request.
    pub fn from_request(request: &super::Request) -> Self {
        Self::new(&request.req)
    }

    pub(super) fn new(request: &isahc::Request<super::Body>) -> Self {
        let mut headers: Vec<_> = request.headers().iter().map(|(n, v)| (n.clone(), v.clone())).collect();

        headers.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

        use sha2::Digest;

        let mut m = sha2::Sha512_256::new();
        m.update(request.uri().to_string().as_bytes());
        m.update(request.method().as_str());
        for (name, value) in headers {
            m.update(name.as_str().as_bytes());
            m.update(value.as_bytes());
        }
        let hash = m.finalize();

        CacheKey(hash.into())
    }

    /// Returns the SHA-512/256 hash.
    pub fn sha(&self) -> [u8; 32] {
        self.0
    }

    /// Computes a URI safe base64 encoded SHA-512/256 from the key data.
    pub fn sha_str(&self) -> String {
        use base64::*;

        let hash = self.sha();
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&hash[..])
    }
}
impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.sha_str())
    }
}

pub use file_cache::FileSystemCache;

mod file_cache {
    use std::{
        fs::{self, File, OpenOptions},
        io::{self, Read, Write},
        path::{Path, PathBuf},
    };

    use crate::http::util::{lock_exclusive, lock_shared, unlock_ok};
    use crate::{
        self as task,
        io::{McBufErrorExt, McBufReader},
    };
    use async_trait::async_trait;
    use fs4::fs_std::FileExt;
    use zng_unit::TimeUnits;

    use super::*;

    /// A simple [`CacheDb`] implementation that uses a local directory.
    ///
    /// # Implementation Details
    ///
    /// A file lock is used to control data access, read operations use a shared lock so concurrent reads can happen,
    /// the [`set`] operation uses a exclusive lock for the duration of the body download, so subsequent requests for
    /// a caching resource will await until the cache is completed to return a body that will then read the cached data.
    ///
    /// The [`set`] operation returns a body as soon as the entry is created, the body will receive data as it is downloaded and cached,
    /// in case of a cache error mid-download the cache entry is removed but the returned body will still download the rest of the data.
    /// In case of an error creating the entry the original body is always returned so the [`Client`] can continue with a normal
    /// download also.
    ///
    /// The cache does not pull data, only data read by the returned body is written to the cache, dropping the body without reading
    /// to end cancels the cache entry.
    ///
    /// [`Client`]: crate::http::Client
    /// [`set`]: crate::http::CacheDb::set
    #[derive(Clone)]
    pub struct FileSystemCache {
        dir: PathBuf,
    }
    impl FileSystemCache {
        /// Open the cache in `dir` or create it.
        pub fn new(dir: impl Into<PathBuf>) -> io::Result<Self> {
            let dir = dir.into();
            std::fs::create_dir_all(&dir)?;

            Ok(FileSystemCache { dir })
        }

        async fn entry(&self, key: &CacheKey, write: bool) -> Option<CacheEntry> {
            let dir = self.dir.clone();
            let key = key.sha_str();
            task::wait(move || CacheEntry::open(dir.join(key), write)).await
        }
    }
    #[async_trait]
    impl CacheDb for FileSystemCache {
        fn clone_boxed(&self) -> Box<dyn CacheDb> {
            Box::new(self.clone())
        }

        async fn policy(&self, key: &CacheKey) -> Option<CachePolicy> {
            self.entry(key, false).await.map(|mut e| e.policy.take().unwrap())
        }
        async fn set_policy(&self, key: &CacheKey, policy: CachePolicy) -> bool {
            if let Some(entry) = self.entry(key, true).await {
                task::wait(move || entry.write_policy(policy)).await
            } else {
                false
            }
        }

        async fn body(&self, key: &CacheKey) -> Option<Body> {
            self.entry(key, false).await?.open_body().await
        }
        async fn set(&self, key: &CacheKey, policy: CachePolicy, body: Body) -> Option<Body> {
            match self.entry(key, true).await {
                Some(entry) => {
                    let (entry, ok) = task::wait(move || {
                        let ok = entry.write_policy(policy);
                        (entry, ok)
                    })
                    .await;

                    if ok { Some(entry.write_body(body).await) } else { Some(body) }
                }
                _ => Some(body),
            }
        }

        async fn remove(&self, key: &CacheKey) {
            if let Some(entry) = self.entry(key, true).await {
                task::wait(move || {
                    CacheEntry::try_delete_locked_dir(&entry.dir, &entry.lock);
                })
                .await
            }
        }

        async fn purge(&self) {
            let dir = self.dir.clone();
            task::wait(move || {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let entry = entry.path();
                        if entry.is_dir()
                            && let Ok(lock) = File::open(entry.join(CacheEntry::LOCK))
                            && FileExt::try_lock_shared(&lock).is_ok()
                        {
                            CacheEntry::try_delete_locked_dir(&entry, &lock);
                        }
                    }
                }
            })
            .await
        }

        async fn prune(&self) {
            let dir = self.dir.clone();
            task::wait(move || {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    let now = SystemTime::now();
                    let old = (24 * 3).hours();

                    for entry in entries.flatten() {
                        let entry = entry.path();
                        if entry.is_dir()
                            && let Some(entry) = CacheEntry::open(entry, false)
                        {
                            let policy = entry.policy.as_ref().unwrap();
                            if policy.is_stale(now) && policy.age(now) > old {
                                CacheEntry::try_delete_locked_dir(&entry.dir, &entry.lock);
                            }
                        }
                    }
                }
            })
            .await
        }
    }

    struct CacheEntry {
        dir: PathBuf,
        lock: File,

        policy: Option<CachePolicy>,
    }
    impl CacheEntry {
        const LOCK: &'static str = ".lock";
        const WRITING: &'static str = ".w";
        const POLICY: &'static str = ".policy";
        const BODY: &'static str = ".body";

        /// Open or create an entry.
        fn open(dir: PathBuf, write: bool) -> Option<Self> {
            if write
                && !dir.exists()
                && let Err(e) = fs::create_dir_all(&dir)
            {
                tracing::error!("cache dir error, {e:?}");
                return None;
            }

            let lock = dir.join(Self::LOCK);
            let mut opt = OpenOptions::new();
            if write {
                opt.read(true).write(true).create(true);
            } else {
                opt.read(true);
            }

            let mut lock = match opt.open(lock) {
                Ok(l) => l,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound && !dir.exists() => return None,
                Err(e) => {
                    tracing::error!("cache lock open error, {e:?}");
                    Self::try_delete_dir(&dir);
                    return None;
                }
            };

            const TIMEOUT: Duration = Duration::from_secs(10);

            let lock_r = if write {
                lock_exclusive(&lock, TIMEOUT)
            } else {
                lock_shared(&lock, TIMEOUT)
            };
            if let Err(e) = lock_r {
                tracing::error!("cache lock error, {e:?}");
                Self::try_delete_dir(&dir);
                return None;
            }

            let mut version = String::new();
            if let Err(e) = lock.read_to_string(&mut version) {
                tracing::error!("cache lock read error, {e:?}");
                Self::try_delete_locked_dir(&dir, &lock);
                return None;
            }

            let expected_version = "zng::http::FileCache 1.0";
            if version != expected_version {
                if write && version.is_empty() {
                    if let Err(e) = lock.set_len(0).and_then(|()| lock.write_all(expected_version.as_bytes())) {
                        tracing::error!("cache lock write error, {e:?}");
                        Self::try_delete_locked_dir(&dir, &lock);
                        return None;
                    }
                } else {
                    tracing::error!("unknown cache version, {version:?}");
                    Self::try_delete_locked_dir(&dir, &lock);
                    return None;
                }
            }

            let policy_file = dir.join(Self::POLICY);

            if dir.join(Self::WRITING).exists() {
                tracing::error!("cache has partial files, removing");

                if write {
                    if let Err(e) = Self::remove_files(&dir) {
                        tracing::error!("failed to clear partial files, {e:?}");
                        Self::try_delete_locked_dir(&dir, &lock);
                        return None;
                    }
                } else {
                    Self::try_delete_locked_dir(&dir, &lock);
                    return None;
                }
            }

            if policy_file.exists() {
                let policy = match Self::read_policy(&policy_file) {
                    Ok(i) => i,
                    Err(e) => {
                        tracing::error!("cache policy read error, {e:?}");
                        Self::try_delete_locked_dir(&dir, &lock);
                        return None;
                    }
                };

                Some(Self {
                    dir,
                    lock,
                    policy: Some(policy),
                })
            } else if write {
                Some(Self { dir, lock, policy: None })
            } else {
                tracing::error!("cache policy missing");
                Self::try_delete_locked_dir(&dir, &lock);
                None
            }
        }
        fn read_policy(file: &Path) -> Result<CachePolicy, Box<dyn std::error::Error>> {
            let file = std::fs::File::open(file)?;
            let file = std::io::BufReader::new(file);
            let policy = serde_json::from_reader(file)?;
            Ok(policy)
        }

        /// Replace the .policy content, returns `true` if the entry still exists.
        pub fn write_policy(&self, policy: CachePolicy) -> bool {
            let w_tag = if let Some(t) = self.writing_tag() {
                t
            } else {
                return false;
            };

            if let Err(e) = self.write_policy_impl(policy) {
                tracing::error!("cache policy serialize error, {e:?}");
                Self::try_delete_locked_dir(&self.dir, &self.lock);
                return false;
            }

            let _ = fs::remove_file(w_tag);

            true
        }
        fn write_policy_impl(&self, policy: CachePolicy) -> Result<(), Box<dyn std::error::Error>> {
            let file = std::fs::File::create(self.dir.join(Self::POLICY))?;
            serde_json::to_writer(file, &policy)?;
            Ok(())
        }

        /// Start reading the body content, returns `Some(_)` if the entry still exists.
        pub async fn open_body(&self) -> Option<Body> {
            match task::fs::File::open(self.dir.join(Self::BODY)).await {
                Ok(body) => {
                    if let Ok(metadata) = body.metadata().await {
                        Some(Body::from_reader_sized(task::io::BufReader::new(body), metadata.len()))
                    } else {
                        Some(Body::from_reader(task::io::BufReader::new(body)))
                    }
                }
                Err(e) => {
                    tracing::error!("cache open body error, {e:?}");
                    Self::try_delete_locked_dir(&self.dir, &self.lock);
                    None
                }
            }
        }

        /// Start downloading and writing a copy of the body to the cache entry.
        pub async fn write_body(self, body: Body) -> Body {
            let w_tag = if let Some(t) = self.writing_tag() {
                t
            } else {
                return body;
            };

            match task::fs::File::create(self.dir.join(Self::BODY)).await {
                Ok(cache_body) => {
                    let cache_body = task::io::BufWriter::new(cache_body);
                    let len = body.len();
                    let mut cache_copy = McBufReader::new(body);
                    let body_copy = cache_copy.clone();
                    cache_copy.set_lazy(true); // don't read more than body, gets error if body is dropped before EOF.

                    task::spawn(async move {
                        if let Err(e) = task::io::copy(cache_copy, cache_body).await {
                            if e.is_only_lazy_left() {
                                tracing::warn!("cache cancel");
                            } else {
                                tracing::error!("cache body write error, {e:?}");
                            }
                            // cleanup partial download, stopped by error of by user dropping body reader.
                            Self::try_delete_locked_dir(&self.dir, &self.lock);
                        } else {
                            let _ = fs::remove_file(w_tag);
                        }
                    });

                    if let Some(len) = len {
                        Body::from_reader_sized(body_copy, len)
                    } else {
                        Body::from_reader(body_copy)
                    }
                }
                Err(e) => {
                    tracing::error!("cache body create error, {e:?}");
                    Self::try_delete_locked_dir(&self.dir, &self.lock);
                    body
                }
            }
        }

        fn try_delete_locked_dir(dir: &Path, lock: &File) {
            let _ = unlock_ok(lock);
            Self::try_delete_dir(dir);
        }

        fn try_delete_dir(dir: &Path) {
            let _ = remove_dir_all::remove_dir_all(dir);
        }

        fn writing_tag(&self) -> Option<PathBuf> {
            let tag = self.dir.join(Self::WRITING);

            if let Err(e) = fs::write(&tag, "w") {
                tracing::error!("cache write tag error, {e:?}");
                Self::try_delete_locked_dir(&self.dir, &self.lock);
                None
            } else {
                Some(tag)
            }
        }

        fn remove_files(dir: &Path) -> std::io::Result<()> {
            for file in [Self::BODY, Self::POLICY, Self::WRITING] {
                fs::remove_file(dir.join(file))?
            }
            Ok(())
        }
    }
    impl Drop for CacheEntry {
        fn drop(&mut self) {
            if let Err(e) = unlock_ok(&self.lock) {
                tracing::error!("cache unlock error, {e:?}");
                Self::try_delete_dir(&self.dir);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, time::SystemTime};

    use zng_clone_move::async_clmv;

    use crate::{
        self as task,
        http::{header::*, util::*, *},
    };
    use zng_unit::*;

    #[test]
    pub fn file_cache_miss() {
        test_log();
        let tmp = TestTempDir::new("file_cache_miss");

        let test = FileSystemCache::new(&tmp).unwrap();
        let request = Request::get("https://file_cache_miss.invalid/content").unwrap().build();
        let key = CacheKey::from_request(&request);

        let r = async_test(async move { test.policy(&key).await });

        assert!(r.is_none());
    }

    #[test]
    pub fn file_cache_set_no_headers() {
        test_log();
        let tmp = TestTempDir::new("file_cache_set_no_headers");

        let test = FileSystemCache::new(&tmp).unwrap();
        let request = Request::get("https://file_cache_set_no_headers.invalid/content").unwrap().build();
        let response = Response::new_message(StatusCode::OK, "test content.");

        let key = CacheKey::from_request(&request);
        let policy = CachePolicy::new(&request.req, &response.0);

        let (headers, body) = async_test(async move {
            let (parts, body) = response.into_parts();

            let body = test.set(&key, policy, body).await.unwrap();

            let mut response = Response::from_parts(parts, body);

            let body = response.text().await.unwrap();

            (response.into_parts().0.headers, body)
        });

        assert_eq!(body, "test content.");
        assert!(headers.is_empty());
    }

    #[test]
    pub fn file_cache_set() {
        test_log();
        let tmp = TestTempDir::new("file_cache_set");

        let test = FileSystemCache::new(&tmp).unwrap();
        let request = Request::get("https://file_cache_set.invalid/content").unwrap().build();
        let key = CacheKey::from_request(&request);

        let mut headers = HeaderMap::default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
        let body = Body::from_reader(task::io::Cursor::new("test content."));
        let response = Response::new(StatusCode::OK, headers, body);

        let policy = CachePolicy::new(&request.req, &response.0);

        let (headers, body) = async_test(async move {
            let (parts, body) = response.into_parts();

            let body = test.set(&key, policy, body).await.unwrap();

            let mut response = Response::from_parts(parts, body);

            let body = response.text().await.unwrap();

            (response.into_parts().0.headers, body)
        });

        assert_eq!(
            headers.get(&header::CONTENT_LENGTH),
            Some(&HeaderValue::from("test content.".len()))
        );
        assert_eq!(body, "test content.");
    }

    #[test]
    pub fn file_cache_get_cached() {
        test_log();
        let tmp = TestTempDir::new("file_cache_get_cached");
        let request = Request::get("https://file_cache_get_cached.invalid/content").unwrap().build();
        let key = CacheKey::from_request(&request);

        let test = FileSystemCache::new(&tmp).unwrap();

        let mut headers = HeaderMap::default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
        let body = Body::from_reader(task::io::Cursor::new("test content."));
        let response = Response::new(StatusCode::OK, headers, body);

        let policy = CachePolicy::new(&request.req, &response.0);

        async_test(async_clmv!(key, {
            let (_, body) = response.into_parts();

            let mut body = test.set(&key, policy, body).await.unwrap();
            Body::bytes(&mut body).await.unwrap();

            drop(test);
        }));

        let test = FileSystemCache::new(&tmp).unwrap();

        let body = async_test(async move {
            let mut body = test.body(&key).await.unwrap();

            body.text_utf8().await.unwrap()
        });

        assert_eq!(body, "test content.");
    }

    #[test]
    pub fn file_cache_get_policy() {
        test_log();
        let tmp = TestTempDir::new("get_etag");

        let test = FileSystemCache::new(&tmp).unwrap();

        let request = Request::get("https://get_etag.invalid/content").unwrap().build();
        let key = CacheKey::from_request(&request);

        let mut headers = HeaderMap::default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
        let response = Response::new(StatusCode::OK, headers, Body::from_reader(task::io::Cursor::new("test content.")));
        let policy = CachePolicy::new(&request.req, &response.0);

        let r_policy = async_test(async_clmv!(policy, {
            let mut body = test.set(&key, policy, response.into_parts().1).await.unwrap();
            Body::bytes(&mut body).await.unwrap();

            let test = FileSystemCache::new(&tmp).unwrap();

            test.policy(&key).await.unwrap()
        }));

        let now = SystemTime::now();
        assert_eq!(policy.age(now), r_policy.age(now));
    }

    #[test]
    pub fn file_cache_concurrent_get() {
        test_log();
        let tmp = TestTempDir::new("file_cache_concurrent_get");
        let request = Request::get("https://file_cache_concurrent_get.invalid/content").unwrap().build();
        let key = CacheKey::from_request(&request);

        let test = FileSystemCache::new(&tmp).unwrap();

        let mut headers = HeaderMap::default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
        let body = Body::from_reader(task::io::Cursor::new("test content."));
        let response = Response::new(StatusCode::OK, headers, body);
        let policy = CachePolicy::new(&request.req, &response.0);

        async_test(async_clmv!(key, {
            let mut body = test.set(&key, policy, response.into_parts().1).await.unwrap();
            Body::bytes(&mut body).await.unwrap();

            drop(test);
        }));

        async_test(async move {
            let a = concurrent_get(tmp.path().to_owned(), key.clone());
            let b = concurrent_get(tmp.path().to_owned(), key.clone());
            let c = concurrent_get(tmp.path().to_owned(), key);

            task::all!(a, b, c).await;
        });
    }
    async fn concurrent_get(tmp: PathBuf, body: CacheKey) {
        task::run(async move {
            let test = FileSystemCache::new(&tmp).unwrap();

            let mut body = test.body(&body).await.unwrap();

            let body = body.text_utf8().await.unwrap();

            assert_eq!(body, "test content.");
        })
        .await
    }

    #[test]
    pub fn file_cache_concurrent_set() {
        test_log();
        let tmp = TestTempDir::new("file_cache_concurrent_set");
        let uri = Uri::try_from("https://file_cache_concurrent_set.invalid/content").unwrap();

        async_test(async move {
            let a = concurrent_set(tmp.path().to_owned(), uri.clone());
            let b = concurrent_set(tmp.path().to_owned(), uri.clone());
            let c = concurrent_set(tmp.path().to_owned(), uri);

            task::all!(a, b, c).await;
        });
    }
    async fn concurrent_set(tmp: PathBuf, uri: Uri) {
        task::run(async move {
            let test = FileSystemCache::new(&tmp).unwrap();

            let request = Request::get(uri).unwrap().build();
            let key = CacheKey::from_request(&request);

            let mut headers = HeaderMap::default();
            headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
            let body = Body::from_reader(task::io::Cursor::new("test content."));
            let response = Response::new(StatusCode::OK, headers, body);

            let policy = CachePolicy::new(&request.req, &response.0);

            let (headers, body) = async move {
                let (parts, body) = response.into_parts();

                let body = test.set(&key, policy, body).await.unwrap();
                let mut response = Response::from_parts(parts, body);

                let body = response.text().await.unwrap();

                (response.into_parts().0.headers, body)
            }
            .await;

            assert_eq!(
                headers.get(&header::CONTENT_LENGTH),
                Some(&HeaderValue::from("test content.".len()))
            );
            assert_eq!(body, "test content.");
        })
        .await
    }

    #[track_caller]
    fn async_test<F>(test: F) -> F::Output
    where
        F: Future,
    {
        task::block_on(task::with_deadline(test, 30.secs())).unwrap()
    }
}
