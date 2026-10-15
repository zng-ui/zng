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
mod stream_decoder;
mod util;

pub use cache::{CacheKey, CacheMode, CachePolicy};
pub use ctx::{HttpCache, HttpClient, http_cache, http_client, set_http_cache, set_http_client, set_request_default};
pub use curl::CurlProcessClient;
pub use file_cache::FileSystemCache;

/// Any error sending request or receiving response.
///
/// # HTTP Error
///
/// In functions that return `Result<Response, Error>` successfully received HTTP errors are not converted to `Err`. Use
/// [`Response::error`] to get a [`HttpError`] if the response represents an error.
///
/// In helper functions that convert the response body to a value the [`HttpError`] is converted into this type. You
/// can downcast to get the HTTP error in those cases.
pub type Error = Box<dyn std::error::Error + Send + Sync>;

use futures_lite::AsyncBufReadExt;
pub use http::{
    StatusCode, header,
    method::{self, Method},
    uri::{self, Uri},
};
use serde::{Deserialize, Serialize};
use zng_var::{Var, const_var};

use std::{borrow::Cow, time::Duration};
use std::{fmt, mem};

use crate::{channel::IpcBytes, http::ctx::REQUEST_DEFAULT, io::Metrics};

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
    #[cfg(feature = "http_compression")]
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

    /// If cookies should be send and stored.
    ///
    /// When enabled the [`http_cache`] is used to retrieve and store cookies.
    ///
    /// Is not enabled by default.
    #[cfg(feature = "http_cookie")]
    pub cookies: bool,

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
        match REQUEST_DEFAULT.lock().clone() {
            Some(mut r) => {
                r.method = method;
                r.uri = uri;
                r
            }
            None => Self {
                uri,
                method,
                require_length: false,
                max_length: ByteLength::MAX,
                headers: header::HeaderMap::new(),
                timeout: Duration::MAX,
                connect_timeout: 90.secs(),
                low_speed_timeout: (Duration::MAX, 0.bytes()),
                redirect_limit: 20,
                #[cfg(feature = "http_compression")]
                auto_decompress: true,
                max_upload_speed: ByteLength::MAX,
                max_download_speed: ByteLength::MAX,
                cache: CacheMode::Default,
                #[cfg(feature = "http_cookie")]
                cookies: false,
                metrics: true,
                body: IpcBytes::default(),
            },
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

    /// Appends a header to [`headers`] to this request.
    ///
    /// [`headers`]: field@Request::headers
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

    /// Append Server-Sent Events (SSE) headers and disables cache.
    ///
    /// Set the `last_event_id` if this request is to reconnect a previous stream that tracks messages received.
    ///
    /// # Examples
    ///
    /// ```
    /// use zng_task::http;
    ///
    /// # fn try_example() -> Result<(), http::Error> {
    /// let sse = http::Request::get("https://httpbun.com/sse")?.sse("")?;
    /// # Ok(()) }
    /// ```
    pub fn sse(mut self, last_event_id: &str) -> Result<Self, Error> {
        self.cache = CacheMode::NoCache;
        let r = self.header("accept", "text/event-stream")?;
        if last_event_id.is_empty() {
            Ok(r)
        } else {
            r.header("Last-Event-ID", last_event_id)
        }
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
    #[cfg(feature = "http_compression")]
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

    /// Set the [`cookies`].
    ///
    /// [`cookies`]: field@Request::cookies
    #[cfg(feature = "http_cookie")]
    pub fn cookies(mut self, enable: bool) -> Self {
        self.cookies = enable;
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

    /// Set the [`body`] to a plain text UTF-8 payload.  Also sets the `Content-Type` header if it is not set.
    ///
    /// [`body`]: field@Request::body
    pub fn body_text(mut self, body: &str) -> Result<Self, Error> {
        if !self.headers.contains_key("Content-Type") {
            self = self.header("Content-Type", "text/plain; charset=utf-8")?;
        }
        Ok(self.body(IpcBytes::from_slice_blocking(body.as_bytes())?))
    }

    /// Set the [`body`] to a JSON payload. Also sets the `Content-Type` header if it is not set.
    ///
    /// [`body`]: field@Request::body
    pub fn body_json<T: Serialize>(mut self, body: &T) -> Result<Self, Error> {
        if !self.headers.contains_key("Content-Type") {
            self = self.header("Content-Type", "text/json; charset=utf-8")?;
        }
        let body = serde_json::to_vec(body)?;
        Ok(self.body(IpcBytes::from_vec_blocking(body)?))
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

/// HTTP response.
pub struct Response {
    status: StatusCode,
    headers: header::HeaderMap,
    effective_uri: Uri,
    body: ResponseBody,
    metrics: Var<Metrics>,
}
impl fmt::Debug for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("effective_uri", &self.effective_uri)
            .field("header", &self.headers)
            .field("metrics", &self.metrics.get())
            .finish_non_exhaustive()
    }
}
enum ResponseBody {
    Done { bytes: IpcBytes },
    Read { read: Box<dyn AsyncRead + Send> },
}
impl Response {
    /// New with body download pending or ongoing.
    pub fn from_read(
        status: StatusCode,
        header: header::HeaderMap,
        effective_uri: Uri,
        metrics: Var<Metrics>,
        read: Box<dyn AsyncRead + Send>,
    ) -> Self {
        Self {
            status,
            headers: header,
            effective_uri,
            metrics,
            body: ResponseBody::Read { read },
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
            metrics: const_var(metrics),
            body: ResponseBody::Done { bytes: body },
        }
    }

    /// New with status and message body.
    pub fn from_msg(status: StatusCode, msg: impl ToTxt) -> Self {
        Self::from_done(
            status,
            header::HeaderMap::new(),
            Uri::from_static("/"),
            Metrics::zero(),
            IpcBytes::from_slice_blocking(msg.to_txt().as_bytes()).unwrap(),
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
            ResponseBody::Done { bytes, .. } => Some((bytes.len() as u64).bytes()),
            ResponseBody::Read { .. } => {
                let len = self
                    .headers
                    .get(header::CONTENT_LENGTH)?
                    .to_str()
                    .ok()?
                    .parse::<u64>()
                    .ok()?
                    .bytes();
                Some(len)
            }
        }
    }

    /// Receive the full body to memory.
    pub async fn download(&mut self) -> Result<(), Error> {
        if let ResponseBody::Done { .. } = &self.body {
            return Ok(());
        }

        let downloader = match mem::replace(
            &mut self.body,
            ResponseBody::Done {
                bytes: IpcBytes::default(),
            },
        ) {
            ResponseBody::Read { read: downloader } => downloader,
            ResponseBody::Done { .. } => unreachable!(),
        };
        let mut downloader = Box::into_pin(downloader);
        let body = IpcBytes::from_read(downloader.as_mut()).await?;

        self.body = ResponseBody::Done { bytes: body };

        Ok(())
    }

    /// Download the full body memory cache and returns a reference to it.
    pub async fn body(&mut self) -> Result<IpcBytes, Error> {
        self.download().await?;
        match &self.body {
            ResponseBody::Done { bytes } => Ok(bytes.clone()),
            ResponseBody::Read { .. } => unreachable!(),
        }
    }

    /// Download the full body to memory and decode a clone to text.
    ///
    /// The text encoding is guessed from header `"content-type"` and *BOM sniffing*, otherwise
    /// utf-8 is assumed and the replacement character (U+FFFD) is used for invalid chars.
    pub async fn body_text(&mut self) -> Result<Txt, Error> {
        let content_type = self
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<mime::Mime>().ok());
        let encoding_name = content_type
            .as_ref()
            .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
            .unwrap_or("utf-8");

        let bytes = self.body().await?;

        let encoding = encoding_rs::Encoding::for_label(encoding_name.as_bytes()).unwrap_or(encoding_rs::UTF_8);
        let (text, _, _) = encoding.decode(&bytes);
        Ok(Txt::from_str(&text))
    }

    /// Download the full body to memory and deserialize it from JSON.
    pub async fn body_json<O>(&mut self) -> Result<O, Error>
    where
        O: serde::de::DeserializeOwned + std::marker::Unpin,
    {
        let bytes = self.body().await?;
        let r = serde_json::from_slice(&bytes)?;
        Ok(r)
    }

    /// Take the body stream.
    ///
    /// After this call the `body_*` methods will return empty.
    ///
    /// If the body was already downloaded reads that data from the start.
    ///
    /// Note that the [`metrics`] var will continue updating for the stream.
    ///
    /// [`metrics`]: Self::metrics
    pub fn take_stream(&mut self) -> Box<dyn AsyncRead + Send> {
        match mem::replace(
            &mut self.body,
            ResponseBody::Done {
                bytes: IpcBytes::default(),
            },
        ) {
            ResponseBody::Done { bytes } => Box::new(crate::io::Cursor::new(bytes)),
            ResponseBody::Read { read } => read,
        }
    }

    /// Take the body stream and decodes it to UTF-8.
    ///
    /// The text encoding is guessed from header `"content-type"` and *BOM sniffing*, otherwise
    /// utf-8 is assumed and the replacement character (U+FFFD) is used for invalid chars.
    pub fn stream_decode(&mut self) -> impl AsyncRead + Send + 'static {
        let content_type = self
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<mime::Mime>().ok());
        let encoding_name = content_type
            .as_ref()
            .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
            .unwrap_or("utf-8");
        let encoding = encoding_rs::Encoding::for_label(encoding_name.as_bytes()).unwrap_or(encoding_rs::UTF_8);

        stream_decoder::StreamDecoder::new(self.take_stream(), encoding.new_decoder())
    }

    /// Take the body stream and decode Server-Sent Events (SSE).
    ///
    /// The `handler` is called for each [`SseArgs`] and once at the end with [`SseDisconnected`].
    /// The `handler` is dropped after disconnect, the caller must make a new request to retry if desired.
    ///
    /// If the `handler` returns `false` it is dropped and disconnected.
    pub fn on_sse<F>(&mut self, mut handler: impl FnMut(Result<SseArgs, SseDisconnected>) -> F + Send + 'static)
    where
        F: Future<Output = bool> + Send,
    {
        // no need to decode, sse is always utf-8
        let mut stream = crate::io::BufReader::new(Box::into_pin(self.take_stream()));
        let mut first = true;

        crate::spawn(async move {
            let mut buf = vec![];
            let mut last_event_id = String::new();
            let mut last_retry = String::new();
            let mut error = None;
            loop {
                // read_line has a bug https://github.com/smol-rs/futures-lite/issues/143
                // it also allocates a second buf for some reason, read_until is more efficient
                let mut result = stream.read_until(b'\n', &mut buf).await;
                if let &Ok(n) = &result
                    && n > 0
                {
                    // validate the line
                    let start = buf.len() - n;
                    if let Err(e) = str::from_utf8(&buf[start..]) {
                        result = Err(std::io::Error::new(std::io::ErrorKind::InvalidData, e));
                    }
                }
                match result {
                    Ok(n) => {
                        // SAFETY: we validate by line as they are received
                        let mut buf_str = unsafe { str::from_utf8_unchecked(&buf) };
                        if mem::take(&mut first) {
                            // strip utf-8 BOM
                            buf_str = buf_str.strip_prefix('\u{FEFF}').unwrap_or(buf_str);
                        }

                        if matches!(buf_str.lines().next_back(), Some(l) if l.is_empty()) {
                            // received empty line

                            let mut id = None;
                            let mut event = None;
                            let mut retry = None;
                            let mut data = None::<Cow<str>>;
                            for line in buf_str.lines() {
                                // spec says a line without : is a field with no value,
                                // so "data\n\n" is the same as "data:\n\n"
                                let (field, s) = line.split_once(':').unwrap_or((line, ""));
                                let s = s.strip_prefix(' ').unwrap_or(s);
                                match field {
                                    "data" => {
                                        if let Some(d) = &mut data {
                                            let d = d.to_mut();
                                            d.push('\n');
                                            d.push_str(s);
                                        } else {
                                            data = Some(Cow::Borrowed(s));
                                        }
                                    }
                                    "event" => event = Some(s),
                                    "id" => {
                                        if !s.contains('\0') {
                                            id = Some(s)
                                        }
                                    }
                                    "retry" if !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()) => retry = Some(s),
                                    _ => {}
                                }
                            }

                            if let Some(id) = id {
                                last_event_id.clear();
                                last_event_id.push_str(id);
                            }
                            if let Some(ms) = retry {
                                last_retry.clear();
                                last_retry.push_str(ms);
                            }
                            if let Some(data) = data {
                                let event = event.unwrap_or("");
                                let continue_ = handler(Ok(SseArgs { event, data: &data, id })).await;
                                if !continue_ {
                                    // disconnect by user, skip `Disconnected` event
                                    return;
                                }
                            }

                            buf.clear();
                        }

                        if n == 0 {
                            // disconnected by EOF, goto `Disconnected` event
                            if !buf.is_empty() {
                                // spec says last event is terminated by \n\n too
                                error = Some(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "EOF before last \\n\\n"))
                            }
                            break;
                        }
                    }
                    Err(e) => {
                        error = Some(e);
                        // disconnected by error, goto `Disconnected` event
                        break;
                    }
                }
            }
            let mut retry_delay = Duration::ZERO;
            if !last_retry.is_empty()
                && let Ok(ms) = last_retry.parse()
            {
                retry_delay = Duration::from_millis(ms);
            }
            handler(Err(SseDisconnected {
                last_event_id: &last_event_id,
                retry_delay,
                error,
            }))
            .await;
        });
    }

    /// Metrics for the task transfer, if it was enabled in the request.
    pub fn metrics(&self) -> Var<Metrics> {
        self.metrics.read_only()
    }

    /// Convert failed status to an error.
    ///
    /// If the [`status`] is not success this will attempt to download the body as text, otherwise
    /// the body is not accessed.
    ///
    /// [`status`]: Response::status
    pub async fn error(&mut self) -> Result<(), HttpError> {
        if self.status.is_success() {
            Ok(())
        } else {
            let body = match self.body_text().await {
                Ok(b) => b,
                Err(e) => formatx!("could not receive error body, {e}"),
            };
            Err(HttpError { status: self.status, body })
        }
    }
}

/// Represents an HTTP response error.
///
/// See [`Response::error`] for more details.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct HttpError {
    /// The error code.
    pub status: StatusCode,
    /// The error body.
    pub body: Txt,
}

impl HttpError {
    /// New error.
    pub fn new(status: StatusCode, body: Txt) -> Self {
        Self { status, body }
    }
}
/// Alternate writes the `body`.
impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HTTP error {}", self.status.as_str())?;
        if let Some(r) = self.status.canonical_reason() {
            write!(f, " {r}")?;
        }
        if f.alternate() && self.body.is_empty() {
            for line in self.body.lines() {
                write!(f, "\n   {line}")?;
            }
        }
        Ok(())
    }
}
impl std::error::Error for HttpError {}

/// Send a GET request to the `uri`.
///
/// The [`http_client`] is used to send the request.
pub async fn get<U>(uri: U) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::get(uri)?).await
}

/// Send a GET request to the `uri` and read the response as a string.
///
/// The [`http_client`] is used to send the request.
pub async fn get_txt<U>(uri: U) -> Result<Txt, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    let mut r = send(Request::get(uri)?).await?;
    r.error().await?;
    r.body_text().await
}

/// Send a GET request to the `uri` and read the response as raw bytes.
///
/// The [`http_client`] is used to send the request.
pub async fn get_bytes<U>(uri: U) -> Result<IpcBytes, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    let mut r = send(Request::get(uri)?).await?;
    r.error().await?;
    r.body().await
}

/// Send a GET request to the `uri` and de-serializes the response.
///
/// The [`http_client`] is used to send the request.
pub async fn get_json<U, O>(uri: U) -> Result<O, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
    O: serde::de::DeserializeOwned + std::marker::Unpin,
{
    let mut r = send(Request::get(uri)?).await?;
    r.error().await?;
    r.body_json().await
}

/// Send a HEAD request to the `uri`.
///
/// The [`http_client`] is used to send the request.
pub async fn head<U>(uri: U) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::head(uri)?).await
}

/// Send a PUT request to the `uri` with a given request body.
///
/// The [`http_client`] is used to send the request.
pub async fn put<U>(uri: U, body: IpcBytes) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::put(uri)?.body(body)).await
}

/// Send a POST request to the `uri` with a given request body.
///
/// The [`http_client`] is used to send the request.
pub async fn post<U>(uri: U, body: IpcBytes) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::post(uri)?.body(body)).await
}

/// Send a DELETE request to the `uri`.
///
/// The [`http_client`] is used to send the request.
pub async fn delete<U>(uri: U) -> Result<Response, Error>
where
    U: TryInto<Uri>,
    Error: From<<U as TryInto<Uri>>::Error>,
{
    send(Request::delete(uri)?).await
}

/// Send a custom [`Request`].
///
/// The [`http_client`] is used to send the request.
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

/// Represents an update in a Server-Sent Events (SSE) stream.
///
/// See [`Response::on_sse`] for more details.
#[derive(Debug)]
pub struct SseArgs<'a> {
    /// Id.
    pub id: Option<&'a str>,
    /// Event name.
    ///
    /// Empty means *message*.
    pub event: &'a str,
    /// Data.
    pub data: &'a str,
}
impl<'a> SseArgs<'a> {
    /// Gets if is not a custom event.
    ///
    /// Handling `data` only when this is `true` is the equivalent of
    /// subscribing to `EventSource.onmessage` in Javascript.
    pub fn is_message(&self) -> bool {
        self.event.is_empty()
    }

    /// Gets if is custom event.
    pub fn is_event(&self, name: &str) -> bool {
        self.event == name
    }

    /// Try to deserialize `data` from JSON.
    pub fn json<O: serde::Deserialize<'a>>(&self) -> Result<O, serde_json::Error> {
        serde_json::from_str(self.data)
    }
}

/// Represents the end of a Server-Sent Events (SSE) stream.
///
/// See [`Response::on_sse`] for more details.
#[derive(Debug)]
pub struct SseDisconnected<'a> {
    /// Last event ID, use with [`Request::sse`] to reconnect at the same position.
    pub last_event_id: &'a str,
    /// Wait this delay before requesting a new connection.
    pub retry_delay: Duration,
    /// Error.
    ///
    /// This is `None` on EOF.
    pub error: Option<std::io::Error>,
}
