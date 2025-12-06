use std::{any::Any, fmt, mem, pin::Pin};

#[cfg(feature = "http_cookie")]
use http::{HeaderValue, Uri};
use parking_lot::Mutex;

use crate::{
    channel::IpcBytes,
    http::{CacheKey, CachePolicy, Error, Request, Response, curl, file_cache},
};

type Fut<O> = Pin<Box<dyn Future<Output = O> + Send>>;

/// HTTP cache backend.
///
/// Cache implementers must store a [`CachePolicy`] and [`IpcBytes`] body for a given [`CacheKey`].
pub trait HttpCache: Send + Sync + Any {
    /// Get the cache-policy for the given `key`.
    fn policy(&'static self, key: CacheKey) -> Fut<Option<CachePolicy>>;

    /// Replaces the cache-policy for the given `key`.
    ///
    /// Returns `false` if the entry does not exist.
    fn set_policy(&'static self, key: CacheKey, policy: CachePolicy) -> Fut<bool>;

    /// Get the cached body for the given `key`.
    fn body(&'static self, key: CacheKey) -> Fut<Option<IpcBytes>>;

    /// Caches the `policy` and `body` for the given `key`.
    fn set(&'static self, key: CacheKey, policy: CachePolicy, body: IpcBytes) -> Fut<()>;

    /// Remove cache policy and body for the given `key`.
    fn remove(&'static self, key: CacheKey) -> Fut<()>;

    /// Get the Cookie value associated with the `uri`.
    ///
    /// The returned value is validated and ready for sending.
    #[cfg(feature = "http_cookie")]
    fn cookie(&'static self, uri: Uri) -> Fut<Option<HeaderValue>>;

    /// Store the Set-Cookie value associated with the `uri`.
    ///
    /// The uri and cookie must be directly from the response, the cache will parse and property associate the cookie with domain.
    #[cfg(feature = "http_cookie")]
    fn set_cookie(&'static self, uri: Uri, cookie: HeaderValue) -> Fut<()>;

    /// Remove the Cookie value associated with the `uri`.
    #[cfg(feature = "http_cookie")]
    fn remove_cookie(&'static self, uri: Uri) -> Fut<()>;

    /// Remove all cached entries that are not locked in a `set*` operation.
    fn purge(&'static self) -> Fut<()>;

    /// Remove cache entries to reduce pressure.
    ///
    /// What entries are removed depends on the cache DB implementer.
    fn prune(&'static self) -> Fut<()>;
}

/// HTTP client backend.
///
/// See [`http_client`] for more details.
pub trait HttpClient: Send + Sync + Any {
    /// If the client manages cache and cookie storage.
    ///
    /// Is `false` by default. When `false` the [`http_cache`] is used before and after `send`.
    fn is_cache_manager(&self) -> bool {
        true
    }

    /// Send a request and await until response header is received.
    /// Full response body can continue to be received using the [`Response`] value.
    fn send(&'static self, request: Request) -> Fut<Result<Response, Error>>;
}

/// Error returned by [`set_http_client`] and [`set_http_cache`] if the default was already initialized.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct AlreadyInitedError {}
impl fmt::Display for AlreadyInitedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "default client already initialized, can only set before first use")
    }
}
impl std::error::Error for AlreadyInitedError {}

/// The [`HttpClient`] used by the functions in this module.
///
/// You can replace the default client at the start of the process using [`set_http_client`].
///
/// # Defaults
///
/// The default client is a minimal implementation that uses the system `curl` command line executable.
/// You can set the `"ZNG_CURL"` environment variable before the first usage to define the path to the curl executable.
pub fn http_client() -> &'static dyn HttpClient {
    use once_cell::sync::Lazy;

    static SHARED: Lazy<Box<dyn HttpClient>> = Lazy::new(|| {
        let ci = mem::replace(&mut *CLIENT_INIT.lock(), ClientInit::Inited);
        if let ClientInit::Set(init) = ci {
            init()
        } else {
            // browser defaults
            Box::new(curl::CurlProcessClient::default())
        }
    });
    &**SHARED
}
static CLIENT_INIT: Mutex<ClientInit> = Mutex::new(ClientInit::None);
enum ClientInit {
    None,
    Set(Box<dyn FnOnce() -> Box<dyn HttpClient> + Send>),
    Inited,
}

/// Set a custom initialization function for the [`http_client`].
///
/// The [`http_client`] is used by all functions in this module and is initialized on the first usage,
/// you can use this function before any HTTP operation to replace backend implementation.
///
/// Returns an error if the [`http_client`] was already initialized.
///
/// [`isahc`]: https://docs.rs/isahc
pub fn set_http_client<I>(init: I) -> Result<(), AlreadyInitedError>
where
    I: FnOnce() -> Box<dyn HttpClient> + Send + 'static,
{
    let mut ci = CLIENT_INIT.lock();
    if let ClientInit::Inited = &*ci {
        Err(AlreadyInitedError {})
    } else {
        *ci = ClientInit::Set(Box::new(init));
        Ok(())
    }
}

/// The [`HttpCache`] used by the functions in this module.
///
/// You can replace the default cache at the start of the process using [`set_http_cache`].
///
/// # Defaults
///
/// The default cache is a minimal implementation that uses the file system.
pub fn http_cache() -> &'static dyn HttpCache {
    use once_cell::sync::Lazy;

    static SHARED: Lazy<Box<dyn HttpCache>> = Lazy::new(|| {
        let ci = mem::replace(&mut *CACHE_INIT.lock(), CacheInit::Inited);
        if let CacheInit::Set(init) = ci {
            init()
        } else {
            Box::new(file_cache::FileSystemCache::new(zng_env::cache("zng-task-http-cache")).unwrap())
        }
    });
    &**SHARED
}
static CACHE_INIT: Mutex<CacheInit> = Mutex::new(CacheInit::None);
enum CacheInit {
    None,
    Set(Box<dyn FnOnce() -> Box<dyn HttpCache> + Send>),
    Inited,
}

/// Set a custom initialization function for the [`http_client`].
///
/// The [`http_client`] is used by all functions in this module and is initialized on the first usage,
/// you can use this function before any HTTP operation to replace backend implementation.
///
/// Returns an error if the [`http_client`] was already initialized.
///
/// [`isahc`]: https://docs.rs/isahc
pub fn set_http_cache<I>(init: I) -> Result<(), AlreadyInitedError>
where
    I: FnOnce() -> Box<dyn HttpCache> + Send + 'static,
{
    let mut ci = CACHE_INIT.lock();
    if let CacheInit::Inited = &*ci {
        Err(AlreadyInitedError {})
    } else {
        *ci = CacheInit::Set(Box::new(init));
        Ok(())
    }
}

/// Set the default values returned by [`Request::new`].
///
/// The method and uri are ignored in this value, the other fields are used as default in all subsequent requests.
pub fn set_request_default(d: Request) {
    *REQUEST_DEFAULT.lock() = Some(d);
}
pub(super) static REQUEST_DEFAULT: Mutex<Option<Request>> = Mutex::new(None);
