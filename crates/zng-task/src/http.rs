#![cfg(feature = "http")]

//! HTTP client.
//!
//! This module provides an HTTP client API that is backend agnostic. By default it uses the system `curl` command
//! line utility with a simple cache, this can be replaced by implementing [`HttpClient`] and [`HttpCache`].
//!

mod cache;
mod ctx;
mod curl;
mod file_cache;
mod util;

pub use cache::{CacheKey, CacheMode, CachePolicy};
pub use ctx::{HttpCache, HttpClient, http_cache, http_client, set_http_cache, set_http_client};
pub use curl::CurlProcessClient;
pub use file_cache::FileSystemCache;

/// Any error during request or response.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub use http::{
    StatusCode, header,
    method::{self, Method},
    uri::{self, Uri},
};
use serde::{Deserialize, Serialize};
use zng_var::impl_from_and_into_var;

use std::time::Duration;
use std::{fmt, mem};

use crate::{Progress, channel::IpcBytes};

use super::io::AsyncRead;

use zng_txt::{ToTxt, Txt, formatx};
use zng_unit::*;

/// HTTP request.
///
/// Use [`send`] to send a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Request {
    /// The URI.
    #[serde(with = "http_serde::uri")]
    pub uri: Uri,
    /// The HTTP method.
    #[serde(with = "http_serde::method")]
    pub method: Method,

    /// Header values.
    ///
    /// Is empty by default.
    #[serde(with = "http_serde::header_map")]
    pub headers: http::HeaderMap,

    /// Maximum amount of time that a complete request/response cycle is allowed to
    /// take before being aborted. This includes DNS resolution, connecting to the server,
    /// writing the request, and reading the response.
    ///
    /// Note that this includes the response read operation, so if you get a response but don't
    /// read-it within this timeout you will get a [`TimedOut`] IO error.
    ///
    /// By default no timeout is used, [`Duration::MAX`].
    ///
    /// [`TimedOut`]: https://doc.rust-lang.org/nightly/std/io/enum.ErrorKind.html#variant.TimedOut
    pub timeout: Duration,

    /// Maximum amount of time to await for establishing connections to a host.
    ///
    /// Is 90 seconds by default.
    pub connect_timeout: Duration,

    /// Maximum amount of time allowed when transfer speed is under the given speed in bytes per second.
    ///
    /// By default not timeout is used, `(Duration::MAX, 0)`.
    pub low_speed_timeout: (Duration, ByteLength),

    /// Maximum redirects to follow.
    ///
    /// When redirecting the `Referer` header is updated automatically.
    ///
    /// Is `20` by default.
    pub redirect_limit: u16,

    /// If should auto decompress received data.
    ///
    /// If enabled the "Accept-Encoding" will also be set automatically, if it was not set on the header.
    ///
    /// This is enabled by default.
    pub auto_decompress: bool,

    /// Maximum upload speed in bytes per second.
    ///
    /// No maximum by default, [`ByteLength::MAX`].
    pub max_upload_speed: ByteLength,

    /// Maximum download speed in bytes per second.
    ///
    /// No maximum by default, [`ByteLength::MAX`].
    pub max_download_speed: ByteLength,

    /// If the `Content-Length` header must be present in the response.
    ///
    /// By default this is not required.
    pub require_length: bool,

    /// Set the maximum response content length allowed.
    ///
    /// If the `Content-Length` is present on the response and it exceeds this limit an error is
    /// returned immediately, otherwise if [`require_length`] is not enabled an error will be returned
    /// only when the downloaded body length exceeds the limit.
    ///
    /// By default no limit is set, [`ByteLength::MAX`].
    ///
    /// [`require_length`]: Request::require_length
    pub max_length: ByteLength,

    /// Response cache mode.
    ///
    /// Is [`CacheMode::Default`] by default.
    pub cache: CacheMode,

    // !!: TODO cookies
    /// If transfer metrics should be measured.
    ///
    /// When enabled you can get the information using the [`Response::metrics`] method.
    ///
    /// This is enabled by default.
    pub metrics: bool,

    /// Request body content.
    ///
    /// Is empty by default.
    pub body: IpcBytes,
}
impl Request {
    /// Starts building a request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zng_task::http;
    ///
    /// # fn try_example() -> Result<(), http::Error> {
    /// let request = http::Request::new(http::Method::PUT, "https://httpbin.org/put".try_into()?);
    /// # Ok(()) }
    /// ```
    pub fn new(method: Method, uri: Uri) -> Self {
        Self {
            uri,
            method,
            require_length: false,
            max_length: ByteLength::MAX,
            headers: header::HeaderMap::new(),
            timeout: Duration::MAX,
            connect_timeout: 90.secs(),
            low_speed_timeout: (Duration::MAX, 0.bytes()),
            redirect_limit: 20,
            auto_decompress: true,
            max_upload_speed: ByteLength::MAX,
            max_download_speed: ByteLength::MAX,
            cache: CacheMode::Default,
            metrics: true,
            body: IpcBytes::default(),
        }
    }

    /// Starts building a GET request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zng_task::http;
    ///
    /// # fn try_example() -> Result<(), http::Error> {
    /// let get = http::Request::get("https://httpbin.org/get")?;
    /// # Ok(()) }
    /// ```
    pub fn get<U: TryInto<Uri>>(uri: U) -> Result<Self, <U as TryInto<Uri>>::Error> {
        Ok(Self::new(Method::GET, uri.try_into()?))
    }

    /// Starts building a PUT request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zng_task::http;
    ///
    /// # fn try_example() -> Result<(), http::Error> {
    /// let put = http::Request::put("https://httpbin.org/put")?.header("accept", "application/json")?;
    /// # Ok(()) }
    /// ```
    pub fn put<U: TryInto<Uri>>(uri: U) -> Result<Self, <U as TryInto<Uri>>::Error> {
        Ok(Self::new(Method::PUT, uri.try_into()?))
    }

    /// Starts building a POST request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zng_task::http;
    ///
    /// # fn try_example() -> Result<(), http::Error> {
    /// let post = http::Request::post("https://httpbin.org/post")?.header("accept", "application/json")?;
    /// # Ok(()) }
    /// ```
    pub fn post<U: TryInto<Uri>>(uri: U) -> Result<Self, <U as TryInto<Uri>>::Error> {
        Ok(Self::new(Method::POST, uri.try_into()?))
    }

    /// Starts building a DELETE request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zng_task::http;
    ///
    /// # fn try_example() -> Result<(), http::Error> {
    /// let delete = http::Request::delete("https://httpbin.org/delete")?.header("accept", "application/json")?;
    /// # Ok(()) }
    /// ```
    pub fn delete<U: TryInto<Uri>>(uri: U) -> Result<Self, <U as TryInto<Uri>>::Error> {
        Ok(Self::new(Method::DELETE, uri.try_into()?))
    }

    /// Starts building a PATCH request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zng_task::http;
    ///
    /// # fn try_example() -> Result<(), http::Error> {
    /// let patch = http::Request::patch("https://httpbin.org/patch")?.header("accept", "application/json")?;
    /// # Ok(()) }
    /// ```
    pub fn patch<U: TryInto<Uri>>(uri: U) -> Result<Self, <U as TryInto<Uri>>::Error> {
        Ok(Self::new(Method::PATCH, uri.try_into()?))
    }

    /// Starts building a HEAD request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zng_task::http;
    ///
    /// # fn try_example() -> Result<(), http::Error> {
    /// let head = http::Request::head("https://httpbin.org")?;
    /// # Ok(()) }
    /// ```
    pub fn head<U: TryInto<Uri>>(uri: U) -> Result<Self, <U as TryInto<Uri>>::Error> {
        Ok(Self::new(Method::HEAD, uri.try_into()?))
    }

    /// Appends a [`header`] to this request.
    ///
    /// [`header`]: field@Request::header
    pub fn header<K, V>(mut self, name: K, value: V) -> Result<Self, Error>
    where
        K: TryInto<header::HeaderName>,
        V: TryInto<header::HeaderValue>,
        Error: From<<K as TryInto<header::HeaderName>>::Error>,
        Error: From<<V as TryInto<header::HeaderValue>>::Error>,
    {
        self.headers.insert(name.try_into()?, value.try_into()?);
        Ok(self)
    }

    /// Set the [`timeout`].
    ///
    /// [`timeout`]: field@Request::timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the [`connect_timeout`].
    ///
    /// [`connect_timeout`]: field@Request::connect_timeout
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the [`low_speed_timeout`].
    ///
    /// [`low_speed_timeout`]: field@Request::low_speed_timeout
    pub fn low_speed_timeout(mut self, timeout: Duration, bytes_per_sec: ByteLength) -> Self {
        self.low_speed_timeout = (timeout, bytes_per_sec);
        self
    }

    /// Set the [`redirect_limit`].
    ///
    /// [`redirect_limit`]: field@Request::redirect_limit
    pub fn redirect_limit(mut self, count: u16) -> Self {
        self.redirect_limit = count;
        self
    }

    /// Set the [`auto_decompress`].
    ///
    /// [`auto_decompress`]: field@Request::auto_decompress
    pub fn auto_decompress(mut self, enabled: bool) -> Self {
        self.auto_decompress = enabled;
        self
    }

    /// Set [`require_length`].
    ///
    /// [`require_length`]: field@Request::require_length
    pub fn require_length(mut self, enabled: bool) -> Self {
        self.require_length = enabled;
        self
    }

    /// Set [`max_length`].
    ///
    /// [`max_length`]: field@Request::max_length
    pub fn max_length(mut self, max: ByteLength) -> Self {
        self.max_length = max;
        self
    }

    /// Set the [`max_upload_speed`].
    ///
    /// [`max_upload_speed`]: field@Request::max_upload_speed
    pub fn max_upload_speed(mut self, bytes_per_sec: ByteLength) -> Self {
        self.max_upload_speed = bytes_per_sec;
        self
    }

    /// Set the [`max_download_speed`].
    ///
    /// [`max_download_speed`]: field@Request::max_download_speed
    pub fn max_download_speed(mut self, bytes_per_sec: ByteLength) -> Self {
        self.max_download_speed = bytes_per_sec;
        self
    }

    /// Set the [`metrics`].
    ///
    /// [`metrics`]: field@Request::metrics
    pub fn metrics(mut self, enabled: bool) -> Self {
        self.metrics = enabled;
        self
    }

    /// Set the [`body`].
    ///
    /// [`body`]: field@Request::body
    pub fn body(mut self, body: IpcBytes) -> Self {
        self.body = body;
        self
    }

    /// Set the [`body`] to a JSON payload. Also sets the `Content-Type` header.
    ///
    /// [`body`]: field@Request::body
    pub fn body_json<T: Serialize>(self, body: &T) -> std::io::Result<Self> {
        let body = serde_json::to_vec(body)?;
        Ok(self.body(IpcBytes::from_vec(body)?))
    }
}
impl From<Request> for http::Request<IpcBytes> {
    fn from(mut r: Request) -> Self {
        let mut b = http::Request::builder().uri(mem::take(&mut r.uri)).method(r.method.clone());
        if !r.headers.is_empty() {
            *b.headers_mut().unwrap() = mem::take(&mut r.headers);
        }
        let body = mem::take(&mut r.body);
        let b = b.extension(r);
        b.body(body).unwrap()
    }
}
impl From<http::Request<IpcBytes>> for Request {
    fn from(value: http::Request<IpcBytes>) -> Self {
        let (mut parts, body) = value.into_parts();
        if let Some(mut r) = parts.extensions.remove::<Request>() {
            r.method = parts.method;
            r.uri = parts.uri;
            r.headers = parts.headers;
            r.body = body;
            r
        } else {
            let mut r = Request::new(parts.method, parts.uri);
            r.headers = parts.headers;
            r.body = body;
            r
        }
    }
}

/// Backend reader for [`Response`].
pub trait HttpResponseDownloader: AsyncRead + Send + 'static {
    /// Metrics of header and body download, if it was requested.
    fn metrics(&self) -> &Metrics;
}

/// HTTP response.
pub struct Response {
    status: StatusCode,
    headers: header::HeaderMap,
    effective_uri: Uri,
    body: ResponseBody,
}
impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("effective_uri", &self.effective_uri)
            .field("header", &self.headers)
            .field("metrics", self.metrics())
            .finish_non_exhaustive()
    }
}
enum ResponseBody {
    Done { metrics: Metrics, bytes: IpcBytes },
    Read { downloader: Box<dyn HttpResponseDownloader> },
}
impl Response {
    /// New with body download pending or ongoing.
    pub fn from_downloader(
        status: StatusCode,
        header: header::HeaderMap,
        effective_uri: Uri,
        downloader: Box<dyn HttpResponseDownloader>,
    ) -> Self {
        Self {
            status,
            headers: header,
            effective_uri,
            body: ResponseBody::Read { downloader },
        }
    }

    /// New with body already downloaded.
    pub fn from_done(status: StatusCode, mut headers: header::HeaderMap, effective_uri: Uri, metrics: Metrics, body: IpcBytes) -> Self {
        if !headers.contains_key(header::CONTENT_LENGTH) {
            headers.insert(header::CONTENT_LENGTH, body.len().into());
        }
        Self {
            status,
            headers,
            effective_uri,
            body: ResponseBody::Done { metrics, bytes: body },
        }
    }

    /// New with status and message body.
    pub fn from_msg(status: StatusCode, msg: impl ToTxt) -> Self {
        Self::from_done(
            status,
            header::HeaderMap::new(),
            Uri::from_static("/"),
            Metrics::zero(),
            IpcBytes::from_slice(msg.to_txt().as_bytes()).unwrap(),
        )
    }

    /// Returns the [`StatusCode`].
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Returns a reference to the associated header field map.
    pub fn header(&self) -> &header::HeaderMap {
        &self.headers
    }

    /// Get the effective URI of this response. This value differs from the
    /// original URI provided when making the request if at least one redirect
    /// was followed.
    pub fn effective_uri(&self) -> &Uri {
        &self.effective_uri
    }

    /// Get the body bytes length if it is downloaded or `Content-Length` value if it is present in the headers.
    pub fn content_len(&self) -> Option<ByteLength> {
        match &self.body {
            ResponseBody::Done { bytes, .. } => Some(bytes.len().bytes()),
            ResponseBody::Read { .. } => {
                let len = self
                    .headers
                    .get(header::CONTENT_LENGTH)?
                    .to_str()
                    .ok()?
                    .parse::<usize>()
                    .ok()?
                    .bytes();
                Some(len)
            }
        }
    }

    /// Receive the entire body.
    pub async fn download(&mut self) -> Result<(), Error> {
        if let ResponseBody::Done { .. } = &self.body {
            return Ok(());
        }

        let mut downloader = match mem::replace(
            &mut self.body,
            ResponseBody::Done {
                metrics: Metrics::zero(),
                bytes: IpcBytes::default(),
            },
        ) {
            ResponseBody::Read { downloader } => downloader,
            ResponseBody::Done { metrics, bytes } => unreachable!(),
        };

        let body = IpcBytes::from_read(data);

        Ok(())
    }

    // !!: TODO
    // /// Get the configured cookie jar used for persisting cookies from this response, if any.
    // ///
    // /// Only returns `None` if the [`default_client`] was replaced by one with cookies disabled.
    // pub fn cookie_jar(&self) -> Option<&CookieJar> {
    //     self.0.cookie_jar()
    // }

    /// Await for the full body and returns a referent to it.
    pub async fn bytes(&mut self) -> Result<IpcBytes, Error> {
        self.download().await?;
        match &self.body {
            ResponseBody::Done { bytes, .. } => Ok(bytes.clone()),
            ResponseBody::Read { .. } => unreachable!(),
        }
    }

    /// Read the response body as a string.
    pub async fn text(&mut self) -> Result<Txt, Error> {
        let content_type = self
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<mime::Mime>().ok());
        let encoding_name = content_type
            .as_ref()
            .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
            .unwrap_or("utf-8");

        let bytes = self.bytes().await?;

        let encoding = encoding_rs::Encoding::for_label(encoding_name.as_bytes()).unwrap_or(encoding_rs::UTF_8);
        let (text, _, _) = encoding.decode(&bytes);
        Ok(Txt::from_str(&text))
    }

    /// Deserialize the response body as JSON.
    pub async fn json<O>(&mut self) -> Result<O, Error>
    where
        O: serde::de::DeserializeOwned + std::marker::Unpin,
    {
        let bytes = self.bytes().await?;
        let r = serde_json::from_slice(&bytes)?;
        Ok(r)
    }

    /// Metrics for the task transfer, if it was enabled in the request.
    pub fn metrics(&self) -> &Metrics {
        match &self.body {
            ResponseBody::Done { metrics, .. } => metrics,
            ResponseBody::Read { downloader: consumer, .. } => consumer.metrics(),
        }
    }
}

/// Send a GET request to the `uri`.
///
/// The [`default_client`] is used to send the request.
pub async fn get<U>(uri: U) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::get(uri)?).await
}

/// Send a GET request to the `uri` and read the response as a string.
///
/// The [`default_client`] is used to send the request.
pub async fn get_txt<U>(uri: U) -> Result<Txt, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::get(uri)?).await?.text().await
}

/// Send a GET request to the `uri` and read the response as raw bytes.
///
/// The [`default_client`] is used to send the request.
pub async fn get_bytes<U>(uri: U) -> Result<IpcBytes, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::get(uri)?).await?.bytes().await
}

/// Send a GET request to the `uri` and de-serializes the response.
///
/// The [`default_client`] is used to send the request.
pub async fn get_json<U, O>(uri: U) -> Result<O, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
    O: serde::de::DeserializeOwned + std::marker::Unpin,
{
    send(Request::get(uri)?).await?.json().await
}

/// Send a HEAD request to the `uri`.
///
/// The [`default_client`] is used to send the request.
pub async fn head<U>(uri: U) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::head(uri)?).await
}

/// Send a PUT request to the `uri` with a given request body.
///
/// The [`default_client`] is used to send the request.
pub async fn put<U>(uri: U, body: IpcBytes) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::put(uri)?.body(body)).await
}

/// Send a POST request to the `uri` with a given request body.
///
/// The [`default_client`] is used to send the request.
pub async fn post<U>(uri: U, body: IpcBytes) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::post(uri)?.body(body)).await
}

/// Send a DELETE request to the `uri`.
///
/// The [`default_client`] is used to send the request.
pub async fn delete<U>(uri: U) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::delete(uri)?).await
}

/// Send a custom [`Request`].
///
/// The [`default_client`] is used to send the request.
pub async fn send(request: Request) -> Result<Response, Error> {
    let client = http_client();
    if client.is_cache_manager() {
        client.send(request).await
    } else {
        match request.cache {
            CacheMode::NoCache => client.send(request).await,
            CacheMode::Default => cache::send_cache(client, request).await,
            CacheMode::Permanent => cache::send_cache_perm(client, request).await,
        }
    }
}

/// Information about the state of an HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Metrics {
    /// Number of bytes uploaded / estimated total.
    pub upload_progress: (ByteLength, ByteLength),

    /// Average upload speed so far in bytes/second.
    pub upload_speed: ByteLength,

    /// Number of bytes downloaded / estimated total.
    pub download_progress: (ByteLength, ByteLength),

    /// Average download speed so far in bytes/second.
    pub download_speed: ByteLength,

    /// Total time from the start of the request until DNS name resolving was completed.
    ///
    /// When a redirect is followed, the time from each request is added together.
    pub name_lookup_time: Duration,

    /// Amount of time taken to establish a connection to the server (not including TLS connection time).
    ///
    /// When a redirect is followed, the time from each request is added together.
    pub connect_time: Duration,

    /// Amount of time spent on TLS handshakes.
    ///
    /// When a redirect is followed, the time from each request is added together.
    pub secure_connect_time: Duration,

    /// Time it took from the start of the request until the first byte is either sent or received.
    ///
    /// When a redirect is followed, the time from each request is added together.
    pub transfer_start_time: Duration,

    /// Amount of time spent performing the actual request transfer. The “transfer” includes
    /// both sending the request and receiving the response.
    ///
    /// When a redirect is followed, the time from each request is added together.
    pub transfer_time: Duration,

    /// Total time for the entire request. This will continuously increase until the entire
    /// response body is consumed and completed.
    ///
    /// When a redirect is followed, the time from each request is added together.
    pub total_time: Duration,

    /// If automatic redirect following is enabled, the total time taken for all redirection steps
    /// including name lookup, connect, pre-transfer and transfer before final transaction was started.
    pub redirect_time: Duration,
}
impl Metrics {
    /// All zeros.
    pub fn zero() -> Self {
        Self {
            upload_progress: (0.bytes(), 0.bytes()),
            upload_speed: 0.bytes(),
            download_progress: (0.bytes(), 0.bytes()),
            download_speed: 0.bytes(),
            name_lookup_time: Duration::ZERO,
            connect_time: Duration::ZERO,
            secure_connect_time: Duration::ZERO,
            transfer_start_time: Duration::ZERO,
            transfer_time: Duration::ZERO,
            total_time: Duration::ZERO,
            redirect_time: Duration::ZERO,
        }
    }
}
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ws = false; // written something

        if self.upload_progress.0 != self.upload_progress.1 {
            write!(
                f,
                "↑ {} - {}, {}/s",
                self.upload_progress.0, self.upload_progress.1, self.upload_speed
            )?;
            ws = true;
        }
        if self.download_progress.0 != self.download_progress.1 {
            write!(
                f,
                "{}↓ {} - {}, {}/s",
                if ws { "\n" } else { "" },
                self.download_progress.0,
                self.download_progress.1,
                self.download_speed
            )?;
            ws = true;
        }

        if !ws {
            if self.upload_progress.1.bytes() > 0 {
                write!(f, "↑ {}", self.upload_progress.1)?;
                ws = true;
            }
            if self.download_progress.1.bytes() > 0 {
                write!(f, "{}↓ {}", if ws { "\n" } else { "" }, self.download_progress.1)?;
                ws = true;
            }

            if ws {
                write!(f, "\n{:?}", self.total_time)?;
            }
        }

        Ok(())
    }
}
impl_from_and_into_var! {
    fn from(metrics: Metrics) -> Progress {
        let mut status = Progress::indeterminate();
        if metrics.download_progress.1 > 0.bytes() {
            status = Progress::from_n_of(metrics.download_progress.0.0, metrics.download_progress.1.0);
        }
        if metrics.upload_progress.1 > 0.bytes() {
            let u_status = Progress::from_n_of(metrics.upload_progress.0.0, metrics.upload_progress.1.0);
            if status.is_indeterminate() {
                status = u_status;
            } else {
                status = status.and_fct(u_status.fct());
            }
        }
        status.with_msg(formatx!("{metrics}")).with_meta_mut(|mut m| {
            m.set(*METRICS_ID, metrics);
        })
    }
}
zng_state_map::static_id! {
    /// Metrics in a [`Progress::with_meta`] metadata.
    pub static ref METRICS_ID: zng_state_map::StateId<Metrics>;
}

// !!: TODO chunked downloader/DASH protocol?
// !!: TODO public purge cache function
