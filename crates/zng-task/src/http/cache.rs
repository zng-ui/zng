use std::{
    fmt,
    time::{Duration, Instant, SystemTime},
};

use crate::http::{Error, HttpClient, Metrics, Request, Response, http_cache};

use serde::*;
use zng_unit::*;

use http_cache_semantics as hcs;

pub(super) use hcs::BeforeRequest;

impl hcs::RequestLike for Request {
    fn uri(&self) -> http::Uri {
        self.uri.clone()
    }

    fn is_same_uri(&self, other: &http::Uri) -> bool {
        &self.uri == other
    }

    fn method(&self) -> &http::Method {
        &self.method
    }

    fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }
}
impl hcs::ResponseLike for Response {
    fn status(&self) -> http::StatusCode {
        self.status
    }

    fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }
}

/// Represents a serializable configuration for a cache entry in a [`HttpCache`].
///
/// [`HttpCache`]: crate::http::HttpCache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePolicy(PolicyInner);
impl CachePolicy {
    pub(super) fn new(request: &Request, response: &Response) -> Self {
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

    pub(super) fn new_permanent(response: &Response) -> Self {
        let p = PermanentHeader {
            res: response.headers.clone(),
            status: response.status(),
        };
        Self(PolicyInner::Permanent(p))
    }

    pub(super) fn before_request(&self, request: &Request) -> BeforeRequest {
        match &self.0 {
            PolicyInner::Policy(p) => p.before_request(request, SystemTime::now()),
            PolicyInner::Permanent(p) => BeforeRequest::Fresh(p.parts()),
        }
    }

    pub(super) fn after_response(&self, request: &Request, response: &Response) -> AfterResponse {
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
    pub fn parts(&self) -> http::response::Parts {
        let (mut r, ()) = http::response::Response::builder().body(()).unwrap().into_parts();
        r.headers = self.res.clone();
        r.status = self.status;
        r
    }
}

/// New policy and flags to act on `after_response()`
pub(super) enum AfterResponse {
    /// You can use the cached body! Make sure to use these updated headers
    NotModified(CachePolicy, http::response::Parts),
    /// You need to update the body in the cache
    Modified(CachePolicy, http::response::Parts),
}
impl From<hcs::AfterResponse> for AfterResponse {
    fn from(s: hcs::AfterResponse) -> Self {
        match s {
            hcs::AfterResponse::NotModified(po, pa) => AfterResponse::NotModified(po.into(), pa),
            hcs::AfterResponse::Modified(po, pa) => AfterResponse::Modified(po.into(), pa),
        }
    }
}

/// Represents a SHA-512/256 hash computed from a normalized request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey([u8; 32]);
impl CacheKey {
    /// Compute key from request.
    pub fn from_request(request: &super::Request) -> Self {
        let mut headers: Vec<_> = request.headers.iter().map(|(n, v)| (n.clone(), v.clone())).collect();

        headers.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));

        use sha2::Digest;

        let mut m = sha2::Sha512_256::new();
        m.update(request.uri.to_string().as_bytes());
        m.update(request.method.as_str());
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

/// Request cache mode.
#[derive(Default, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum CacheMode {
    /// Always requests the server, never caches the response.
    NoCache,
    /// Follow the standard cache policy as computed by [`http-cache-semantics`].
    ///
    /// [`http-cache-semantics`]: https://docs.rs/http-cache-semantics/
    #[default]
    Default,
    /// Always caches the response, overwriting cache control configs.
    ///
    /// If the response is cached returns it instead of requesting an update.
    Permanent,
}

pub(crate) async fn send_cache(client: &'static dyn HttpClient, request: Request) -> Result<Response, Error> {
    let start_time = Instant::now();

    let cache = http_cache();
    let key = CacheKey::from_request(&request);
    for _retry in 0..3 {
        if let Some(policy) = cache.policy(key.clone()).await {
            if let Some(body) = cache.body(key.clone()).await {
                match policy.before_request(&request) {
                    http_cache_semantics::BeforeRequest::Fresh(parts) => {
                        // valid cache
                        let mut metrics = Metrics::zero();
                        if request.metrics {
                            metrics.total_time = start_time.elapsed();
                        }
                        return Ok(Response::from_done(parts.status, parts.headers, request.uri, metrics, body));
                    }
                    http_cache_semantics::BeforeRequest::Stale { request: parts, matches } => {
                        if !matches {
                            tracing::error!("cache key does match request");
                            cache.remove(key.clone()).await;
                            continue;
                        }

                        let mut request = request;
                        request.uri = parts.uri;
                        request.method = parts.method;
                        request.headers = parts.headers;
                        let mut response = client.send(request.clone()).await?;
                        match policy.after_response(&request, &response) {
                            AfterResponse::NotModified(cache_policy, parts) => {
                                if cache_policy.should_store() {
                                    cache.set_policy(key, cache_policy).await;
                                } else {
                                    cache.remove(key).await;
                                }
                                let mut metrics = response.metrics().get();
                                if request.metrics {
                                    metrics.total_time = start_time.elapsed();
                                }
                                let response = Response::from_done(parts.status, parts.headers, request.uri, metrics, body);
                                return Ok(response);
                            }
                            AfterResponse::Modified(cache_policy, parts) => {
                                if cache_policy.should_store() {
                                    let body = response.body().await?;
                                    response.status = parts.status;
                                    response.headers = parts.headers;
                                    cache.set(key, cache_policy, body).await;
                                } else {
                                    cache.remove(key).await;
                                }
                                return Ok(response);
                            }
                        }
                    }
                }
            } else {
                tracing::error!("found cached policy without body");
                cache.remove(key.clone()).await;
                continue;
            }
        } else {
            // not cached
            let mut response = client.send(request.clone()).await?;
            let cache_policy = CachePolicy::new(&request, &response);
            if cache_policy.should_store() {
                let body = response.body().await?;
                cache.set(key, CachePolicy::new(&request, &response), body).await;
            }

            return Ok(response);
        }
    }
    tracing::error!("skipped caching due to multiple errors");
    client.send(request).await
}
pub(crate) async fn send_cache_perm(client: &'static dyn HttpClient, request: Request) -> Result<Response, Error> {
    let start_time = Instant::now();

    let cache = http_cache();
    let key = CacheKey::from_request(&request);
    for _retry in 0..3 {
        if let Some(policy) = cache.policy(key.clone()).await {
            if let Some(body) = cache.body(key.clone()).await {
                match policy.before_request(&request) {
                    http_cache_semantics::BeforeRequest::Fresh(parts) => {
                        let mut metrics = Metrics::zero();
                        if request.metrics {
                            metrics.total_time = start_time.elapsed();
                        }
                        // found permanent cache
                        return Ok(Response::from_done(parts.status, parts.headers, request.uri, metrics, body));
                    }
                    http_cache_semantics::BeforeRequest::Stale { request: parts, matches } => {
                        if !matches {
                            tracing::error!("cache key does match request");
                            cache.remove(key.clone()).await;
                            continue;
                        }

                        // previous cache policy was not permanent, check
                        let mut request = request;
                        request.uri = parts.uri;
                        request.method = parts.method;
                        request.headers = parts.headers;
                        let mut response = client.send(request.clone()).await?;
                        match policy.after_response(&request, &response) {
                            AfterResponse::NotModified(_, parts) => {
                                cache.set_policy(key, CachePolicy::new_permanent(&response)).await;
                                let mut metrics = response.metrics().get();
                                if request.metrics {
                                    metrics.total_time = start_time.elapsed();
                                }
                                let response = Response::from_done(parts.status, parts.headers, request.uri, metrics, body);
                                return Ok(response);
                            }
                            AfterResponse::Modified(_, parts) => {
                                let body = response.body().await?;
                                response.status = parts.status;
                                response.headers = parts.headers;
                                cache.set(key, CachePolicy::new_permanent(&response), body).await;
                                return Ok(response);
                            }
                        }
                    }
                }
            } else {
                tracing::error!("found cached policy without body");
                cache.remove(key.clone()).await;
                continue;
            }
        } else {
            // not cached
            let mut response = client.send(request).await?;
            let body = response.body().await?;
            cache.set(key, CachePolicy::new_permanent(&response), body).await;

            return Ok(response);
        }
    }
    tracing::error!("skipped caching due to multiple errors");
    client.send(request).await
}
