use std::fmt;

use async_trait::async_trait;
use super::{Uri, Response, Error, StatusCode, Body, header::{ HeaderName, HeaderValue, HeaderMap}};


/// Represents a download cache in a [`Client`].
#[async_trait]
pub trait CacheProxy: Send + Sync + 'static {
    /// Dynamic clone.
    fn clone_boxed(&self) -> Box<dyn CacheProxy>;

    /// Gets the `ETAG` for the cached data for `uri`.
    ///
    /// Only returns some if the entry has not expired.
    async fn etag(&self, uri: &Uri) -> Option<String>;

    /// Read/clone the cached data for the `uri`.
    async fn get(&self, uri: &Uri) -> Option<Response>;

    /// Caches the `data` with the given `ETAG` and expiration date.
    ///
    /// The `data` must be consumed as fast as possible writing to the cache, at the same time the returned
    /// reader must be reading a copy of the data.
    ///
    /// In case of error the entry is purged.
    async fn set(&self, uri: Uri, etag: String, expires: ExpireInstant, data: Response) -> Result<Response, CacheError>;

    /// Remove `uri` from cache if it is cached.
    async fn forget(&self, uri: &Uri);

    /// Remove all cached entries that are not in use.
    async fn purge(&self);

    /// Remove all expired cache entries.
    async fn prune(&self);
}

/// Represents the expire instant of a [`CacheProxy`] entry.
///
/// The value is the number of seconds since the Unix epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ExpireInstant(pub u64);
impl ExpireInstant {
    /// Returns the instant that just expired.
    #[inline]
    pub fn now() -> ExpireInstant {
        Self(std::time::SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs())
    }

    /// Returns `true` if the cache entry must be removed.
    #[inline]
    pub fn expired(self) -> bool {
        Self::now() > self
    }
}

/// Error when setting an entry in a [`CacheProxy`].
///
/// The cache entry was purged.
#[derive(Debug, Clone, Copy)]
pub struct CacheError;
impl fmt::Display for CacheError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error setting cache entry, the entry has been purged")
    }
}
impl std::error::Error for CacheError {}
impl From<CacheError> for Error {
    fn from(e: CacheError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Interrupted, e).into()
    }
}

/// Cache mode selected for a [`Uri`].
///
/// See [`ClientBuilder::cache_mode`] for more information.
#[derive(Debug, Clone)]
pub enum CacheMode {
    /// Always requests the server, never caches the response.
    NoCache,

    /// Validate the cached `ETag` against the server, returns the cached response if
    /// the server responds with [`Status::NOT_MODIFIED`], otherwise caches the response.
    ///
    /// This is the default mode.
    ETag,

    /// Always caches the response, ignoring `Cache-Control` or `ETag`.
    ///
    /// If the response is cached returns it instead of requesting an update.
    Permanent,

    /// Returns the error.
    Error(Error),
}
impl Default for CacheMode {
    fn default() -> Self {
        CacheMode::ETag
    }
}

pub use file_cache::FileSystemCache;

mod file_cache {
    use std::{
        fs::{self, File, OpenOptions},
        io::{self, Read, Write},
        mem,
        path::{Path, PathBuf},
    };

    use crate::task::{self, io::McBufReader};
    use async_trait::async_trait;
    use fs2::FileExt;

    use super::*;

    /// A simple [`CacheProxy`] implementation that uses a local directory.
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

        async fn entry(&self, uri: Uri, write: bool) -> Option<CacheEntry> {
            let dir = self.dir.clone();
            task::wait(move || {
                use sha2::Digest;

                let mut m = sha2::Sha256::new();
                m.update(uri.to_string().as_bytes());
                let hash = m.finalize();
                let dir = dir.join(base64::encode(&hash[..]));

                CacheEntry::open(dir, write)
            })
            .await
        }
    }
    #[async_trait]
    impl CacheProxy for FileSystemCache {
        fn clone_boxed(&self) -> Box<dyn CacheProxy> {
            Box::new(self.clone())
        }

        async fn etag(&self, uri: &Uri) -> Option<String> {
            self.entry(uri.clone(), false).await.map(|mut e| mem::take(&mut e.etag))
        }

        async fn get(&self, uri: &Uri) -> Option<Response> {
            let entry = self.entry(uri.clone(), false).await?;

            let (entry, headers) = task::wait(move || {
                let headers = entry.read_headers();
                (entry, headers)
            })
            .await;
            let headers = headers?;

            let body = entry.open_body().await?;

            Some(Response::new(StatusCode::OK, headers, body))
        }

        async fn set(&self, uri: Uri, etag: String, expires: ExpireInstant, data: Response) -> Result<Response, CacheError> {
            assert_eq!(data.status(), StatusCode::OK);

            let entry = self.entry(uri, true).await.ok_or(CacheError)?;
            if !entry.write_info(expires, etag.as_str()) {
                return Err(CacheError);
            }

            let (parts, body) = data.into_parts();

            if !entry.write_headers(&parts.headers) {
                return Err(CacheError);
            }

            let body = entry.write_body(body).await;

            Ok(Response::from_parts(parts, body))
        }

        async fn forget(&self, uri: &Uri) {
            if let Some(entry) = self.entry(uri.clone(), true).await {
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
                        if entry.is_dir() {
                            if let Ok(lock) = File::open(entry.join(".lock")) {
                                if lock.try_lock_shared().is_ok() {
                                    CacheEntry::try_delete_locked_dir(&entry, &lock);
                                }
                            }
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
                    for entry in entries.flatten() {
                        let entry = entry.path();
                        if entry.is_dir() {
                            let _ = CacheEntry::open(entry, false);
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

        etag: String,
    }
    impl CacheEntry {
        /// Open or create an entry.
        fn open(dir: PathBuf, write: bool) -> Option<Self> {
            if write && !dir.exists() {
                if let Err(e) = fs::create_dir(&dir) {
                    tracing::error!("cache dir error, {:?}", e);
                    return None;
                }
            }

            let lock = dir.join(".lock");
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
                    tracing::error!("cache lock open error, {:?}", e);
                    Self::try_delete_dir(&dir);
                    return None;
                }
            };

            let lock_r = if write { lock.lock_exclusive() } else { lock.lock_shared() };
            if let Err(e) = lock_r {
                tracing::error!("cache lock error, {:?}", e);
                Self::try_delete_dir(&dir);
                return None;
            }

            let mut version = String::new();
            if let Err(e) = lock.read_to_string(&mut version) {
                tracing::error!("cache lock read error, {:?}", e);
                Self::try_delete_locked_dir(&dir, &lock);
                return None;
            }

            let expected_version = "zero_ui::http::FileCache 1.0";
            if version != expected_version {
                if write && version.is_empty() {
                    if let Err(e) = lock.set_len(0).and_then(|()| lock.write_all(expected_version.as_bytes())) {
                        tracing::error!("cache lock write error, {:?}", e);
                        Self::try_delete_locked_dir(&dir, &lock);
                        return None;
                    }
                } else {
                    tracing::error!("unknown cache version, {:?}", version);
                    Self::try_delete_locked_dir(&dir, &lock);
                    return None;
                }
            }

            let mut expire = ExpireInstant(u64::MAX);
            let mut etag = String::new();

            let info = dir.join(".info");
            if info.exists() {
                let info = match fs::read_to_string(&info) {
                    Ok(i) => i,
                    Err(e) => {
                        tracing::error!("cache info read error, {:?}", e);
                        Self::try_delete_locked_dir(&dir, &lock);
                        return None;
                    }
                };

                let mut info_ok = false;
                if let Some((expire_secs, et)) = info.split_once('\n') {
                    if let Ok(expire_secs) = expire_secs.parse() {
                        expire = ExpireInstant(expire_secs);
                        etag = et.to_owned();
                        info_ok = !et.contains('\n');
                    }
                }

                if !info_ok {
                    tracing::error!("invalid cache info `{}`", info);
                    Self::try_delete_locked_dir(&dir, &lock);
                    return None;
                }

                if expire.expired() {
                    tracing::trace!("cache expired");

                    if write {
                        if let Err(e) = Self::clear(&dir) {
                            tracing::error!("error clearing expired cache, {:?}", e);
                            Self::try_delete_locked_dir(&dir, &lock);
                            return None;
                        }
                    } else {
                        Self::try_delete_locked_dir(&dir, &lock);
                    }
                }
            } else if !write {
                tracing::error!("cache info missing");
                Self::try_delete_locked_dir(&dir, &lock);
                return None;
            }

            Some(Self { dir, lock, etag })
        }

        /// Replace the .info content, returns `true` if the entry still exists.
        pub fn write_info(&self, expire: ExpireInstant, etag: &str) -> bool {
            let info = self.dir.join(".info");

            let content = format!("{}\n{}", expire.0, etag);
            if let Err(e) = fs::write(info, content) {
                tracing::error!("cache info write error, {:?}", e);
                Self::try_delete_locked_dir(&self.dir, &self.lock);
                return false;
            }

            true
        }

        /// Read and parse the cached .headers, returns `Some(_)` if the cache still exists.
        pub fn read_headers(&self) -> Option<HeaderMap> {
            let s = match fs::read_to_string(self.dir.join(".headers")) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!("cache headers read error, {:?}", e);
                    Self::try_delete_locked_dir(&self.dir, &self.lock);
                    return None;
                }
            };

            let mut headers = HeaderMap::new();
            for line in s.lines() {
                if let Some((name, value)) = line.split_once(':') {
                    if let (Ok(name), Ok(value)) = (
                        HeaderName::from_bytes(name.as_bytes()),
                        HeaderValue::from_str(value),
                    ) {
                        headers.insert(name, value);
                    }
                }
            }

            Some(headers)
        }

        /// Replace the .headers content, returns `true` if the entry still exists.
        pub fn write_headers(&self, headers: &HeaderMap) -> bool {
            let mut content = String::new();
            for (name, value) in headers.iter() {
                if let Ok(value) = value.to_str() {
                    content.push_str(name.as_str());
                    content.push(':');
                    content.push_str(value);
                    content.push('\n')
                }
            }

            if let Err(e) = fs::write(self.dir.join(".headers"), content) {
                tracing::error!("cache headers write error, {:?}", e);
                Self::try_delete_locked_dir(&self.dir, &self.lock);
                return false;
            }

            true
        }

        /// Start reading the body content, returns `Some(_)` if the entry still exists.
        pub async fn open_body(&self) -> Option<Body> {
            match task::fs::File::open(self.dir.join(".body")).await {
                Ok(body) => Some(Body::from_reader(task::io::BufReader::new(body))),
                Err(e) => {
                    tracing::error!("cache open body error, {:?}", e);
                    Self::try_delete_locked_dir(&self.dir, &self.lock);
                    None
                }
            }
        }

        /// Start downloading and writing a copy of the body to the cache entry.
        pub async fn write_body(self, body: Body) -> Body {
            match task::fs::File::create(self.dir.join(".body")).await {
                Ok(cache_body) => {
                    let cache_copy = McBufReader::new(body);
                    let body_copy = cache_copy.clone();

                    task::spawn(async move {
                        if let Err(e) = task::io::copy(cache_copy, cache_body).await {
                            tracing::error!("cache body write error, {:?}", e);
                            Self::try_delete_locked_dir(&self.dir, &self.lock);
                        }
                    });

                    Body::from_reader(body_copy)
                }
                Err(e) => {
                    tracing::error!("cache body create error, {:?}", e);
                    Self::try_delete_locked_dir(&self.dir, &self.lock);
                    body
                }
            }
        }

        fn try_delete_locked_dir(dir: &Path, lock: &File) {
            let _ = lock.unlock();
            let _ = lock;
            Self::try_delete_dir(dir);
        }

        fn try_delete_dir(dir: &Path) {
            let _ = remove_dir_all::remove_dir_all(dir);
        }

        fn clear(dir: &Path) -> std::io::Result<()> {
            fs::remove_file(dir.join(".info"))?;
            fs::remove_file(dir.join(".headers"))?;
            fs::remove_file(dir.join(".body"))?;
            Ok(())
        }
    }
    impl Drop for CacheEntry {
        fn drop(&mut self) {
            if let Err(e) = self.lock.unlock() {
                tracing::error!("cache unlock error, {:?}", e);
                Self::try_delete_dir(&self.dir);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        crate_util::{test_log, TestTempDir},
        task::{self, http::header},
        units::*,
    };

    use super::*;

    #[test]
    pub fn file_cache_miss() {
        test_log();
        let tmp = TestTempDir::new("file_cache_miss");

        let test = FileSystemCache::new(&tmp).unwrap();
        let uri = Uri::try_from("https://file_cache_miss.invalid/content").unwrap();

        let r = async_test(async move { test.get(&uri).await });

        assert!(r.is_none());
    }

    #[test]
    pub fn file_cache_set_no_headers() {
        test_log();
        let tmp = TestTempDir::new("file_cache_set_no_headers");

        let test = FileSystemCache::new(&tmp).unwrap();
        let uri = Uri::try_from("https://file_cache_set_no_headers.invalid/content").unwrap();
        let response = Response::new_message(StatusCode::OK, "test content.");

        let (headers, body) = async_test(async move {
            let mut response = test
                .set(uri.clone(), "test-tag".to_owned(), ExpireInstant(u64::MAX), response)
                .await
                .unwrap();

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
        let uri = Uri::try_from("https://file_cache_set.invalid/content").unwrap();

        let mut headers = HeaderMap::default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
        let body = Body::from_reader(task::io::Cursor::new("test content."));
        let response = Response::new(StatusCode::OK, headers, body);

        let (headers, body) = async_test(async move {
            let mut response = test
                .set(uri.clone(), "test-tag".to_owned(), ExpireInstant(u64::MAX), response)
                .await
                .unwrap();

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
        let uri = Uri::try_from("https://file_cache_get_cached.invalid/content").unwrap();

        let test = FileSystemCache::new(&tmp).unwrap();

        let mut headers = HeaderMap::default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
        let body = Body::from_reader(task::io::Cursor::new("test content."));
        let response = Response::new(StatusCode::OK, headers, body);

        async_test(async_clone_move!(uri, {
            let _ = test
                .set(uri, "test-tag".to_owned(), ExpireInstant(u64::MAX), response)
                .await
                .unwrap();

            drop(test);
        }));

        let test = FileSystemCache::new(&tmp).unwrap();

        let (headers, body) = async_test(async move {
            let mut response = test.get(&uri).await.unwrap();

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
    pub fn file_cache_get_etag() {
        test_log();
        let tmp = TestTempDir::new("get_etag");

        let test = FileSystemCache::new(&tmp).unwrap();

        let uri = Uri::try_from("https://get_etag.invalid/content").unwrap();
        let response = Response::new_message(StatusCode::OK, "test content.");

        let etag = async_test(async move {
            let _ = test
                .set(uri.clone(), "test-tag".to_owned(), ExpireInstant(u64::MAX), response)
                .await
                .unwrap();

            let test = FileSystemCache::new(&tmp).unwrap();

            test.etag(&uri).await.unwrap()
        });

        assert_eq!(etag, "test-tag");
    }

    #[test]
    pub fn file_cache_concurrent_get() {
        test_log();
        let tmp = TestTempDir::new("file_cache_concurrent_get");
        let uri = Uri::try_from("https://file_cache_concurrent_get.invalid/content").unwrap();

        let test = FileSystemCache::new(&tmp).unwrap();

        let mut headers = HeaderMap::default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
        let body = Body::from_reader(task::io::Cursor::new("test content."));
        let response = Response::new(StatusCode::OK, headers, body);

        async_test(async_clone_move!(uri, {
            let _ = test
                .set(uri, "test-tag".to_owned(), ExpireInstant(u64::MAX), response)
                .await
                .unwrap();

            drop(test);
        }));

        async_test(async move {
            let a = concurrent_get(tmp.path().to_owned(), uri.clone());
            let b = concurrent_get(tmp.path().to_owned(), uri.clone());
            let c = concurrent_get(tmp.path().to_owned(), uri);

            task::all!(a, b, c).await;
        });
    }
    async fn concurrent_get(tmp: PathBuf, uri: Uri) {
        task::run(async move {
            let test = FileSystemCache::new(&tmp).unwrap();

            let mut response = test.get(&uri).await.unwrap();

            let body = response.text().await.unwrap();

            let (headers, body) = (response.into_parts().0.headers, body);

            assert_eq!(
                headers.get(&header::CONTENT_LENGTH),
                Some(&HeaderValue::from("test content.".len()))
            );
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

            let mut headers = HeaderMap::default();
            headers.insert(header::CONTENT_LENGTH, HeaderValue::from("test content.".len()));
            let body = Body::from_reader(task::io::Cursor::new("test content."));
            let response = Response::new(StatusCode::OK, headers, body);
    
            let (headers, body) = async_test(async move {
                let mut response = test
                    .set(uri.clone(), "test-tag".to_owned(), ExpireInstant(u64::MAX), response)
                    .await
                    .unwrap();
    
                let body = response.text().await.unwrap();
    
                (response.into_parts().0.headers, body)
            });
    
            assert_eq!(
                headers.get(&header::CONTENT_LENGTH),
                Some(&HeaderValue::from("test content.".len()))
            );
            assert_eq!(body, "test content.");
        }).await
    }

    #[track_caller]
    fn async_test<F>(test: F) -> F::Output
    where
        F: std::future::Future,
    {
        task::block_on(task::with_timeout(test, 5.secs())).unwrap()
    }
}
