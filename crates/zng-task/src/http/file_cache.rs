use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    pin::Pin,
    time::SystemTime,
};

use crate::{
    self as task,
    http::util::{lock_exclusive, lock_shared, unlock_ok},
};
use zng_unit::TimeUnits;

use super::*;

type Fut<O> = Pin<Box<dyn Future<Output = O> + Send>>;

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
    /// New from cache dir.
    pub fn new(dir: PathBuf) -> io::Result<Self> {
        std::fs::create_dir_all(&dir)?;

        Ok(FileSystemCache { dir })
    }

    async fn entry(&self, key: CacheKey, write: bool) -> Option<CacheEntry> {
        let dir = self.dir.clone();
        let key = key.sha_str();
        task::wait(move || CacheEntry::open(dir.join(key), write)).await
    }
}
impl HttpCache for FileSystemCache {
    fn policy(&'static self, key: CacheKey) -> Fut<Option<CachePolicy>> {
        Box::pin(async { self.entry(key, false).await.map(|mut e| e.policy.take().unwrap()) })
    }
    fn set_policy(&'static self, key: CacheKey, policy: CachePolicy) -> Fut<bool> {
        Box::pin(async {
            if let Some(entry) = self.entry(key, true).await {
                task::wait(move || entry.write_policy(policy)).await
            } else {
                false
            }
        })
    }

    fn body(&'static self, key: CacheKey) -> Fut<Option<IpcBytes>> {
        Box::pin(async { self.entry(key, false).await?.open_body().await })
    }
    fn set(&'static self, key: CacheKey, policy: CachePolicy, body: IpcBytes) -> Fut<()> {
        Box::pin(async {
            if let Some(entry) = self.entry(key, true).await {
                let (entry, ok) = task::wait(move || {
                    let ok = entry.write_policy(policy);
                    (entry, ok)
                })
                .await;
                if ok {
                    entry.write_body(body).await;
                }
            }
        })
    }

    fn remove(&'static self, key: CacheKey) -> Fut<()> {
        Box::pin(async {
            if let Some(entry) = self.entry(key, true).await {
                task::wait(move || {
                    CacheEntry::try_delete_locked_dir(&entry.dir, &entry.lock);
                })
                .await
            }
        })
    }

    fn purge(&'static self) -> Fut<()> {
        Box::pin(async {
            let dir = self.dir.clone();
            task::wait(move || {
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let entry = entry.path();
                        if entry.is_dir()
                            && let Ok(lock) = File::open(entry.join(CacheEntry::LOCK))
                            && lock.try_lock_shared().is_ok()
                        {
                            CacheEntry::try_delete_locked_dir(&entry, &lock);
                        }
                    }
                }
            })
            .await
        })
    }

    fn prune(&'static self) -> Fut<()> {
        Box::pin(async {
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
        })
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
    pub async fn open_body(&self) -> Option<IpcBytes> {
        let path = self.dir.join(Self::BODY);
        match task::wait(move || IpcBytes::from_file(&path)).await {
            Ok(b) => Some(b),
            Err(e) => {
                tracing::error!("cache open body error, {e:?}");
                Self::try_delete_locked_dir(&self.dir, &self.lock);
                None
            }
        }
    }

    /// Start downloading and writing a copy of the body to the cache entry.
    pub async fn write_body(self, body: IpcBytes) {
        let w_tag = if let Some(t) = self.writing_tag() {
            t
        } else {
            return;
        };

        if let Err(e) = task::fs::write(self.dir.join(Self::BODY), body).await {
            tracing::error!("cache body create error, {e:?}");
            Self::try_delete_locked_dir(&self.dir, &self.lock);
        } else {
            let _ = fs::remove_file(w_tag);
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

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use zng_clone_move::async_clmv;

    use crate::{
        self as task,
        http::{file_cache::FileSystemCache, header::*, util::*, *},
    };
    use zng_unit::*;

    macro_rules! test_cache {
        ($test:tt, $tmp:tt, $tmp_file:tt) => {
            test_log();
            let $tmp = TestTempDir::new($tmp_file);
            let $test: &'static FileSystemCache = Box::leak(Box::new(FileSystemCache::new($tmp.path().to_owned()).unwrap()));
        };
        ($test:tt, $tmp:tt) => {
            test_cache!($test, tmp, $tmp)
        };
    }

    #[test]
    pub fn file_cache_miss() {
        test_cache!(test, "file_cache_miss");

        let request = Request::get("https://file_cache_miss.invalid/content").unwrap();
        let key = CacheKey::from_request(&request);

        let r = async_test(async move { test.policy(key).await });

        assert!(r.is_none());
    }

    #[test]
    pub fn file_cache_set_get() {
        test_cache!(test, "file_cache_set");

        let request = Request::get("https://file_cache_set.invalid/content").unwrap();
        let key = CacheKey::from_request(&request);

        let mut response = Response::from_msg(StatusCode::OK, "test content.");

        let policy = CachePolicy::new(&request, &response);

        let body = async_test(async move {
            test.set(key.clone(), policy, response.bytes().await.unwrap()).await;
            let body = test.body(key).await.unwrap();
            Response::from_done(StatusCode::OK, HeaderMap::new(), Uri::from_static("/"), Metrics::zero(), body)
                .text()
                .await
                .unwrap()
        });

        assert_eq!(body, "test content.");
    }

    #[test]
    pub fn file_cache_get_cached() {
        test_cache!(test, tmp, "file_cache_get_cached");

        let request = Request::get("https://file_cache_get_cached.invalid/content").unwrap();
        let key = CacheKey::from_request(&request);

        let mut response = Response::from_msg(StatusCode::OK, "test content.");

        let policy = CachePolicy::new(&request, &response);

        async_test(async_clmv!(key, {
            test.set(key.clone(), policy, response.bytes().await.unwrap()).await;
        }));

        let test: &'static FileSystemCache = Box::leak(Box::new(FileSystemCache::new(tmp.path().to_owned()).unwrap()));

        let body = async_test(async move {
            let body = test.body(key).await.unwrap();
            Response::from_done(StatusCode::OK, HeaderMap::new(), Uri::from_static("/"), Metrics::zero(), body)
                .text()
                .await
                .unwrap()
        });

        assert_eq!(body, "test content.");
    }

    #[test]
    pub fn file_cache_get_policy() {
        test_cache!(test, tmp, "get_etag");

        let request = Request::get("https://get_etag.invalid/content").unwrap();
        let key = CacheKey::from_request(&request);

        let mut response = Response::from_msg(StatusCode::OK, "test content.");
        let policy = CachePolicy::new(&request, &response);

        let r_policy = async_test(async_clmv!(policy, {
            test.set(key.clone(), policy, response.bytes().await.unwrap()).await;

            let test: &'static FileSystemCache = Box::leak(Box::new(FileSystemCache::new(tmp.path().to_owned()).unwrap()));

            test.policy(key).await.unwrap()
        }));

        let now = SystemTime::now();
        assert_eq!(policy.age(now), r_policy.age(now));
    }

    #[track_caller]
    fn async_test<F>(test: F) -> F::Output
    where
        F: Future,
    {
        task::block_on(task::with_deadline(test, 30.secs())).unwrap()
    }
}
