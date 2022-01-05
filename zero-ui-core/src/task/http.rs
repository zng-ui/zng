//! HTTP client.
//!
//! This module is a thin wrapper around the [`isahc`] crate that just limits the API surface to only
//! `async` methods without the async suffix. You can convert from/into that [`isahc`] types and this one.
//!
//! # Examples
//!
//! Get some text:
//!
//! ```
//! # use zero_ui_core::task;
//! # async fn demo() -> Result<(), Box<dyn std::error::Error>> {
//! let text = task::http::get_text("https://httpbin.org/base64/SGVsbG8gV29ybGQ=").await?;
//! println!("{}!", text);
//! # Ok(()) }
//! ```
//!
//! [`isahc`]: https://docs.rs/isahc

mod cache;

pub use cache::*;

use std::convert::TryFrom;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use std::{fmt, mem};

use super::io::AsyncRead;

use isahc::config::Configurable;
pub use isahc::config::RedirectPolicy;
pub use isahc::cookies::{Cookie, CookieJar};
pub use isahc::error::{Error, ErrorKind};
pub use isahc::http::{header, uri, Method, StatusCode, Uri};

use futures_lite::io::{AsyncReadExt, BufReader};
use isahc::{AsyncReadResponseExt, ResponseExt};
use parking_lot::{const_mutex, Mutex};

use crate::text::Text;
use crate::units::*;

/// Marker trait for types that try-to-convert to [`Uri`].
///
/// All types `T` that match `Uri: TryFrom<T>, <Uri as TryFrom<T>>::Error: Into<isahc::http::Error>` implement this trait.
pub trait TryUri {
    /// Tries to convert `self` into [`Uri`].
    fn try_uri(self) -> Result<Uri, Error>;
}
impl<U> TryUri for U
where
    Uri: TryFrom<U>,
    <Uri as TryFrom<U>>::Error: Into<isahc::http::Error>,
{
    fn try_uri(self) -> Result<Uri, Error> {
        Uri::try_from(self).map_err(|e| e.into().into())
    }
}

/// Marker trait for types that try-to-convert to [`Method`].
///
/// All types `T` that match `Method: TryFrom<T>, <Method as TryFrom<T>>::Error: Into<isahc::http::Error>` implement this trait.
pub trait TryMethod {
    /// Tries to convert `self` into [`Method`].
    fn try_method(self) -> Result<Method, Error>;
}
impl<U> TryMethod for U
where
    Method: TryFrom<U>,
    <isahc::http::Method as TryFrom<U>>::Error: Into<isahc::http::Error>,
{
    fn try_method(self) -> Result<Method, Error> {
        Method::try_from(self).map_err(|e| e.into().into())
    }
}

/// Marker trait for types that try-to-convert to [`Body`].
///
/// All types `T` that match `isahc::AsyncBody: TryFrom<T>, <isahc::AsyncBody as TryFrom<T>>::Error: Into<isahc::http::Error>`
/// implement this trait.
pub trait TryBody {
    /// Tries to convert `self` into [`Body`].
    fn try_body(self) -> Result<Body, Error>;
}
impl<U> TryBody for U
where
    isahc::AsyncBody: TryFrom<U>,
    <isahc::AsyncBody as TryFrom<U>>::Error: Into<isahc::http::Error>,
{
    fn try_body(self) -> Result<Body, Error> {
        match isahc::AsyncBody::try_from(self) {
            Ok(r) => Ok(Body(r)),
            Err(e) => Err(e.into().into()),
        }
    }
}

/// Marker trait for types that try-to-convert to [`header::HeaderName`].
///
/// All types `T` that match `header::HeaderName: TryFrom<T>, <header::HeaderName as TryFrom<T>>::Error: Into<isahc::http::Error>`
/// implement this trait.
pub trait TryHeaderName {
    /// Tries to convert `self` into [`Body`].
    fn try_header_name(self) -> Result<header::HeaderName, Error>;
}
impl<U> TryHeaderName for U
where
    header::HeaderName: TryFrom<U>,
    <header::HeaderName as TryFrom<U>>::Error: Into<isahc::http::Error>,
{
    fn try_header_name(self) -> Result<header::HeaderName, Error> {
        header::HeaderName::try_from(self).map_err(|e| e.into().into())
    }
}
impl TryHeaderName for Text {
    fn try_header_name(self) -> Result<header::HeaderName, Error> {
        <header::HeaderName as TryFrom<&str>>::try_from(self.as_str()).map_err(|e| isahc::http::Error::from(e).into())
    }
}

/// Marker trait for types that try-to-convert to [`header::HeaderValue`].
///
/// All types `T` that match `header::HeaderValue: TryFrom<T>, <header::HeaderValue as TryFrom<T>>::Error: Into<isahc::http::Error>`
/// implement this trait.
pub trait TryHeaderValue {
    /// Tries to convert `self` into [`Body`].
    fn try_header_value(self) -> Result<header::HeaderValue, Error>;
}
impl<U> TryHeaderValue for U
where
    header::HeaderValue: TryFrom<U>,
    <header::HeaderValue as TryFrom<U>>::Error: Into<isahc::http::Error>,
{
    fn try_header_value(self) -> Result<header::HeaderValue, Error> {
        header::HeaderValue::try_from(self).map_err(|e| e.into().into())
    }
}
impl TryHeaderValue for Text {
    fn try_header_value(self) -> Result<header::HeaderValue, Error> {
        header::HeaderValue::from_str(self.as_str()).map_err(|e| isahc::http::Error::from(e).into())
    }
}

/// HTTP request.
///
/// Use [`send`] to send a request.
#[derive(Debug)]
pub struct Request(isahc::Request<Body>);
impl Request {
    /// Starts an empty builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use zero_ui_core::task::http;
    ///
    /// # fn try_example() -> Result<(), Box<dyn std::error::Error>> {
    /// let request = http::Request::builder().method(http::Method::PUT)?.uri("https://httpbin.org/put")?.build();
    /// # Ok(()) }
    /// ```
    ///
    /// Call [`build`] or [`body`] to finish building the request, note that there are is also an associated function
    /// to start a builder for each HTTP method and uri.
    ///
    /// [`build`]: RequestBuilder::build
    /// [`body`]: RequestBuilder::body
    pub fn builder() -> RequestBuilder {
        RequestBuilder(isahc::Request::builder())
    }

    /// Starts building a GET request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zero_ui_core::task::http;
    ///
    /// # fn try_example() -> Result<(), Box<dyn std::error::Error>> {
    /// let get = http::Request::get("https://httpbin.org/get")?.build();
    /// # Ok(()) }
    /// ```
    pub fn get(uri: impl TryUri) -> Result<RequestBuilder, Error> {
        Ok(RequestBuilder(isahc::Request::get(uri.try_uri()?)))
    }

    /// Starts building a PUT request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zero_ui_core::task::http;
    ///
    /// # fn try_example() -> Result<(), Box<dyn std::error::Error>> {
    /// let put = http::Request::put("https://httpbin.org/put")?.header("accept", "application/json")?.build();
    /// # Ok(()) }
    /// ```
    pub fn put(uri: impl TryUri) -> Result<RequestBuilder, Error> {
        Ok(RequestBuilder(isahc::Request::put(uri.try_uri()?)))
    }

    /// Starts building a POST request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zero_ui_core::task::http;
    ///
    /// # fn try_example() -> Result<(), Box<dyn std::error::Error>> {
    /// let post = http::Request::post("https://httpbin.org/post")?.header("accept", "application/json")?.build();
    /// # Ok(()) }
    /// ```
    pub fn post(uri: impl TryUri) -> Result<RequestBuilder, Error> {
        Ok(RequestBuilder(isahc::Request::post(uri.try_uri()?)))
    }

    /// Starts building a DELETE request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zero_ui_core::task::http;
    ///
    /// # fn try_example() -> Result<(), Box<dyn std::error::Error>> {
    /// let delete = http::Request::delete("https://httpbin.org/delete")?.header("accept", "application/json")?.build();
    /// # Ok(()) }
    /// ```
    pub fn delete(uri: impl TryUri) -> Result<RequestBuilder, Error> {
        Ok(RequestBuilder(isahc::Request::delete(uri.try_uri()?)))
    }

    /// Starts building a PATCH request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zero_ui_core::task::http;
    ///
    /// # fn try_example() -> Result<(), Box<dyn std::error::Error>> {
    /// let patch = http::Request::patch("https://httpbin.org/patch")?.header("accept", "application/json")?.build();
    /// # Ok(()) }
    /// ```
    pub fn patch(uri: impl TryUri) -> Result<RequestBuilder, Error> {
        Ok(RequestBuilder(isahc::Request::patch(uri.try_uri()?)))
    }

    /// Starts building a HEAD request.
    ///
    /// # Examples
    ///
    /// ```
    /// use zero_ui_core::task::http;
    ///
    /// # fn try_example() -> Result<(), Box<dyn std::error::Error>> {
    /// let head = http::Request::head("https://httpbin.org")?.build();
    /// # Ok(()) }
    /// ```
    pub fn head(uri: impl TryUri) -> Result<RequestBuilder, Error> {
        Ok(RequestBuilder(isahc::Request::head(uri.try_uri()?)))
    }

    /// Returns a reference to the associated URI.
    pub fn uri(&self) -> &Uri {
        self.0.uri()
    }

    /// Returns a reference to the associated HTTP method.
    pub fn method(&self) -> &Method {
        self.0.method()
    }

    /// Returns a reference to the associated header field map.
    pub fn headers(&self) -> &header::HeaderMap {
        self.0.headers()
    }

    /// Create a clone of the request method, URI, version and headers, with a new `body`.
    pub fn clone_with(&self, body: impl TryBody) -> Result<Self, Error> {
        let body = body.try_body()?;

        let mut req = isahc::Request::new(body);
        *req.method_mut() = self.0.method().clone();
        *req.uri_mut() = self.0.uri().clone();
        *req.version_mut() = self.0.version();
        let headers = req.headers_mut();
        for (name, value) in self.headers() {
            headers.insert(name.clone(), value.clone());
        }

        Ok(Self(req))
    }
}

/// A [`Request`] builder.
///
/// You can use [`Request::builder`] to start an empty builder.
#[derive(Debug)]
pub struct RequestBuilder(isahc::http::request::Builder);
impl Default for RequestBuilder {
    fn default() -> Self {
        Request::builder()
    }
}
impl RequestBuilder {
    /// New default request builder.
    pub fn new() -> Self {
        Request::builder()
    }

    /// Set the HTTP method for this request.
    pub fn method(self, method: impl TryMethod) -> Result<Self, Error> {
        Ok(Self(self.0.method(method.try_method()?)))
    }

    /// Set the URI for this request.
    pub fn uri(self, uri: impl TryUri) -> Result<Self, Error> {
        Ok(Self(self.0.uri(uri.try_uri()?)))
    }

    /// Appends a header to this request.
    pub fn header(self, name: impl TryHeaderName, value: impl TryHeaderValue) -> Result<Self, Error> {
        Ok(Self(self.0.header(name.try_header_name()?, value.try_header_value()?)))
    }

    /// Set a cookie jar to use to accept, store, and supply cookies for incoming responses and outgoing requests.
    ///
    /// Note that the [`default_client`] already has a cookie jar.
    pub fn cookie_jar(self, cookie_jar: CookieJar) -> Self {
        Self(self.0.cookie_jar(cookie_jar))
    }

    /// Specify a maximum amount of time that a complete request/response cycle is allowed to
    /// take before being aborted. This includes DNS resolution, connecting to the server,
    /// writing the request, and reading the response.
    ///
    /// Note that this includes the response read operation, so if you get a response but don't
    /// read-it within this timeout you will get a [`TimedOut`] IO error.
    ///
    /// By default no timeout is used.
    ///
    /// [`TimedOut`]: https://doc.rust-lang.org/nightly/std/io/enum.ErrorKind.html#variant.TimedOut
    pub fn timeout(self, timeout: Duration) -> Self {
        Self(self.0.timeout(timeout))
    }

    /// Set a timeout for establishing connections to a host.
    ///
    /// If not set, the [`default_client`] default of 90 seconds will be used.
    pub fn connect_timeout(self, timeout: Duration) -> Self {
        Self(self.0.connect_timeout(timeout))
    }

    /// Specify a maximum amount of time where transfer rate can go below a minimum speed limit.
    ///
    /// The `low_speed` limit is in bytes/s. No low-speed limit is configured by default.
    pub fn low_speed_timeout(self, low_speed: u32, timeout: Duration) -> Self {
        Self(self.0.low_speed_timeout(low_speed, timeout))
    }

    /// Set a policy for automatically following server redirects.
    ///
    /// If enabled the "Referer" header will be set automatically too.
    ///
    /// The [`default_client`] follows up-to 20 redirects.
    pub fn redirect_policy(self, policy: RedirectPolicy) -> Self {
        if !matches!(policy, RedirectPolicy::None) {
            Self(self.0.redirect_policy(policy).auto_referer())
        } else {
            Self(self.0.redirect_policy(policy))
        }
    }

    /// Enable or disable automatic decompression of the response body.
    ///
    /// If enabled the "Accept-Encoding" will also be set automatically, if it was not set using [`header`].
    ///
    /// This is enabled by default.
    ///
    /// [`header`]: Self::header
    pub fn auto_decompress(self, enabled: bool) -> Self {
        Self(self.0.automatic_decompression(enabled))
    }

    /// Set a maximum upload speed for the request body, in bytes per second.
    pub fn max_upload_speed(self, max: u64) -> Self {
        Self(self.0.max_upload_speed(max))
    }

    /// Set a maximum download speed for the response body, in bytes per second.
    pub fn max_download_speed(self, max: u64) -> Self {
        Self(self.0.max_download_speed(max))
    }

    /// Enable or disable metrics collecting.
    ///
    /// When enabled you can get the information using the [`Response::metrics`] method.
    ///
    /// This is enabled by default.
    pub fn metrics(self, enable: bool) -> Self {
        Self(self.0.metrics(enable))
    }

    /// Build the request without a body.
    pub fn build(self) -> Request {
        self.body(()).unwrap()
    }

    /// Build the request with a body.
    pub fn body(self, body: impl TryBody) -> Result<Request, Error> {
        Ok(Request(self.0.body(body.try_body()?).unwrap()))
    }

    /// Build the request with more custom build calls in the [inner builder].
    ///
    /// [inner builder]: isahc::http::request::Builder
    pub fn build_custom<F>(self, custom: F) -> Result<Request, Error>
    where
        F: FnOnce(isahc::http::request::Builder) -> isahc::http::Result<isahc::Request<isahc::AsyncBody>>,
    {
        let req = custom(self.0)?;
        Ok(Request(req.map(Body)))
    }
}

/// Head parts from a split [`Response`].
pub type ResponseParts = isahc::http::response::Parts;

/// HTTP response.
#[derive(Debug)]
pub struct Response(isahc::Response<isahc::AsyncBody>);
impl Response {
    /// Returns the [`StatusCode`].
    #[inline]
    pub fn status(&self) -> StatusCode {
        self.0.status()
    }

    /// Returns a reference to the associated header field map.
    #[inline]
    pub fn headers(&self) -> &header::HeaderMap<header::HeaderValue> {
        self.0.headers()
    }

    /// Decode content-length value if it is present in the headers.
    pub fn content_len(&self) -> Option<ByteLength> {
        self.0.body().len().map(|l| ByteLength(l as usize))
    }

    /// Get the configured cookie jar used for persisting cookies from this response, if any.
    ///
    /// Only returns `None` if the [`default_client`] was replaced by one with cookies disabled.
    pub fn cookie_jar(&self) -> Option<&CookieJar> {
        self.0.cookie_jar()
    }

    /// Read the response body as a string.
    pub async fn text(&mut self) -> std::io::Result<String> {
        self.0.text().await
    }

    /// Get the effective URI of this response. This value differs from the
    /// original URI provided when making the request if at least one redirect
    /// was followed.
    pub fn effective_uri(&self) -> Option<&Uri> {
        self.0.effective_uri()
    }

    /// Read the response body as raw bytes.
    pub async fn bytes(&mut self) -> std::io::Result<Vec<u8>> {
        Body::bytes_impl(self.0.body_mut()).await
    }

    /// Read at most `limit` bytes from the response body.
    pub async fn bytes_limited(&mut self, limit: ByteLength) -> std::io::Result<Vec<u8>> {
        Body::bytes_limited_impl(self.0.body_mut(), limit).await
    }

    /// Read some bytes from the body, returns how many bytes where read.
    pub async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        BufReader::new(self.0.body_mut()).read(buf).await
    }

    /// Read the from the body to exactly fill the buffer.
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        BufReader::new(self.0.body_mut()).read_exact(buf).await
    }

    /// Deserialize the response body as JSON.
    pub async fn json<O>(&mut self) -> Result<O, serde_json::Error>
    where
        O: serde::de::DeserializeOwned + std::marker::Unpin,
    {
        self.0.json().await
    }

    /// Metrics for the task transfer.
    ///
    /// Metrics are enabled in the [`default_client`] and can be toggled for each request using the
    /// [`RequestBuilder::metrics`] method. If disabled returns [`Metrics::zero`].
    pub fn metrics(&self) -> Metrics {
        self.0.metrics().map(Metrics::from_isahc).unwrap_or_else(Metrics::zero)
    }

    /// Drop the request without dropping the connection.
    ///
    /// This receives and discards any remaining bytes in the response stream. When a response
    /// is dropped without finishing the connection is discarded so it cannot be reused for connections
    /// older then HTTP/2.
    ///
    /// You should call this method before dropping if you expect the remaining bytes to be consumed quickly and
    /// don't known that HTTP/2 or newer is being used.
    pub async fn consume(&mut self) -> std::io::Result<()> {
        self.0.consume().await
    }

    /// Create a response with the given status and text body message.
    pub fn new_message(status: impl Into<StatusCode>, msg: impl Into<String>) -> Self {
        let status = status.into();
        let msg = msg.into().into_bytes();
        let msg = futures_lite::io::Cursor::new(msg);
        let mut r = isahc::Response::new(isahc::AsyncBody::from_reader(msg));
        *r.status_mut() = status;
        Self(r)
    }

    /// New response.
    pub fn new(status: StatusCode, headers: header::HeaderMap<header::HeaderValue>, body: Body) -> Self {
        let mut r = isahc::Response::new(body.0);
        *r.status_mut() = status;
        *r.headers_mut() = headers;
        Self(r)
    }

    /// Consumes the response returning the head and body parts.
    pub fn into_parts(self) -> (ResponseParts, Body) {
        let (p, b) = self.0.into_parts();
        (p, Body(b))
    }

    /// New response from given head and body.
    pub fn from_parts(parts: ResponseParts, body: Body) -> Self {
        Self(isahc::Response::from_parts(parts, body.0))
    }
}
impl From<Response> for isahc::Response<isahc::AsyncBody> {
    fn from(r: Response) -> Self {
        r.0
    }
}

/// HTTP request body.
///
/// Use [`TryBody`] to convert types to body.
#[derive(Debug, Default)]
pub struct Body(isahc::AsyncBody);
impl Body {
    /// Create a new empty body.
    ///
    /// An empty body represents the *absence* of a body, which is semantically different than the presence of a body of zero length.
    #[inline]
    pub fn empty() -> Body {
        Body(isahc::AsyncBody::empty())
    }

    /// Create a new body from a potentially static byte buffer.
    ///
    /// The body will have a known length equal to the number of bytes given.
    ///
    /// This will try to prevent a copy if the type passed in can be re-used, otherwise the buffer
    /// will be copied first. This method guarantees to not require a copy for the following types:
    #[inline]
    pub fn from_bytes_static(bytes: impl AsRef<[u8]> + 'static) -> Self {
        Body(isahc::AsyncBody::from_bytes_static(bytes))
    }

    /// Create a streaming body of unknown length.
    #[inline]
    pub fn from_reader(read: impl AsyncRead + Send + Sync + 'static) -> Self {
        Body(isahc::AsyncBody::from_reader(read))
    }

    /// Create a streaming body of with known length.
    #[inline]
    pub fn from_reader_sized(read: impl AsyncRead + Send + Sync + 'static, size: u64) -> Self {
        Body(isahc::AsyncBody::from_reader_sized(read, size))
    }

    /// Report if this body is empty.
    ///
    /// This is not necessarily the same as checking for zero length, since HTTP message bodies are optional,
    /// there is a semantic difference between the absence of a body and the presence of a zero-length body.
    /// This method will only return `true` for the former.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the size of the body, if known.
    #[inline]
    pub fn len(&self) -> Option<u64> {
        self.0.len()
    }

    /// If this body is repeatable, reset the body stream back to the start of the content.
    ///
    /// Returns false if the body cannot be reset.
    #[inline]
    pub fn reset(&mut self) -> bool {
        self.0.reset()
    }

    /// Read the body as raw bytes.
    pub async fn bytes(&mut self) -> std::io::Result<Vec<u8>> {
        Self::bytes_impl(&mut self.0).await
    }
    async fn bytes_impl(body: &mut isahc::AsyncBody) -> std::io::Result<Vec<u8>> {
        let cap = body.len().unwrap_or(1024);
        let mut bytes = Vec::with_capacity(cap as usize);
        super::io::copy(body, &mut bytes).await?;
        Ok(bytes)
    }

    /// Read the body and try to convert to UTF-8.
    ///
    /// Consider using [`Response::text`], it uses the header encoding information if available.
    pub async fn text_utf8(&mut self) -> Result<String, Box<dyn std::error::Error>> {
        let bytes = self.bytes().await?;
        let r = String::from_utf8(bytes)?;
        Ok(r)
    }

    /// Read at most `limit` bytes from the body.
    pub async fn bytes_limited(&mut self, limit: ByteLength) -> std::io::Result<Vec<u8>> {
        Self::bytes_limited_impl(&mut self.0, limit).await
    }

    async fn bytes_limited_impl(body: &mut isahc::AsyncBody, limit: ByteLength) -> std::io::Result<Vec<u8>> {
        if let Some(len) = body.len() {
            let cap = len.min(limit.0 as u64);
            let mut bytes = Vec::with_capacity(cap as usize);
            super::io::copy(body, &mut bytes).await?;
            Ok(bytes)
        } else {
            let mut bytes = vec![];
            body.take(limit.0 as u64).read_to_end(&mut bytes).await?;
            Ok(bytes)
        }
    }

    /// Read some bytes from the body, returns how many bytes where read.
    pub async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        BufReader::new(&mut self.0).read(buf).await
    }

    /// Read the from the body to exactly fill the buffer.
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        BufReader::new(&mut self.0).read_exact(buf).await
    }
}
impl From<Body> for isahc::AsyncBody {
    fn from(r: Body) -> Self {
        r.0
    }
}
impl From<isahc::AsyncBody> for Body {
    fn from(r: isahc::AsyncBody) -> Self {
        Body(r)
    }
}
impl From<()> for Body {
    fn from(body: ()) -> Self {
        Body(body.into())
    }
}
impl From<String> for Body {
    fn from(body: String) -> Self {
        Body(body.into())
    }
}
impl From<Vec<u8>> for Body {
    fn from(body: Vec<u8>) -> Self {
        Body(body.into())
    }
}
impl From<&'_ [u8]> for Body {
    fn from(body: &[u8]) -> Self {
        body.to_vec().into()
    }
}
impl From<&'_ str> for Body {
    fn from(body: &str) -> Self {
        body.as_bytes().into()
    }
}
impl<T: Into<Self>> From<Option<T>> for Body {
    fn from(body: Option<T>) -> Self {
        match body {
            Some(body) => body.into(),
            None => Self::empty(),
        }
    }
}
impl AsyncRead for Body {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Pin::new(&mut self.get_mut().0).poll_read(cx, buf)
    }
}

/// Send a GET request to the `uri`.
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn get(uri: impl TryUri) -> Result<Response, Error> {
    default_client().get(uri).await
}

/// Send a GET request to the `uri` and read the response as a string.
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn get_text(uri: impl TryUri) -> Result<String, Error> {
    default_client().get_text(uri).await
}

/// Send a GET request to the `uri` and read the response as raw bytes.
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn get_bytes(uri: impl TryUri) -> Result<Vec<u8>, Error> {
    default_client().get_bytes(uri).await
}

/// Send a GET request to the `uri` and de-serializes the response.
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn get_json<O>(uri: impl TryUri) -> Result<O, Box<dyn std::error::Error>>
where
    O: serde::de::DeserializeOwned + std::marker::Unpin,
{
    default_client().get_json(uri).await
}

/// Send a HEAD request to the `uri`.
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn head(uri: impl TryUri) -> Result<Response, Error> {
    default_client().head(uri).await
}

/// Send a PUT request to the `uri` with a given request body.
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn put(uri: impl TryUri, body: impl TryBody) -> Result<Response, Error> {
    default_client().put(uri, body).await
}

/// Send a POST request to the `uri` with a given request body.
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn post(uri: impl TryUri, body: impl TryBody) -> Result<Response, Error> {
    default_client().post(uri, body).await
}

/// Send a DELETE request to the `uri`.
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn delete(uri: impl TryUri) -> Result<Response, Error> {
    default_client().delete(uri).await
}

/// Send a custom [`Request`].
///
/// The [`default_client`]  is used to send the request.
#[inline]
pub async fn send(request: Request) -> Result<Response, Error> {
    default_client().send(request).await
}

/// The [`Client`] used by the functions in this module and Zero-Ui.
///
/// You can replace the default client at the start of the process using [`set_default_client_init`].
///
/// # Defaults
///
/// The default client is created using [`Client::new`].
///
/// [`isahc`]: https://docs.rs/isahc
pub fn default_client() -> &'static Client {
    use once_cell::sync::Lazy;

    static SHARED: Lazy<Client> = Lazy::new(|| {
        let ci = mem::replace(&mut *CLIENT_INIT.lock(), ClientInit::Inited);
        if let ClientInit::Set(init) = ci {
            init()
        } else {
            // browser defaults
            Client::new()
        }
    });
    &SHARED
}

static CLIENT_INIT: Mutex<ClientInit> = const_mutex(ClientInit::None);

enum ClientInit {
    None,
    Set(Box<dyn FnOnce() -> Client + Send>),
    Inited,
}

/// Set a custom initialization function for the [`default_client`].
///
/// The [`default_client`] is used by all Zero-Ui functions and is initialized on the first usage,
/// you can use this function before any HTTP operation to replace the [`isahc`] client
/// used by Zero-Ui.
///
/// Returns an error if the [`default_client`] was already initialized.
///
/// [`isahc`]: https://docs.rs/isahc
pub fn set_default_client_init<I>(init: I) -> Result<(), DefaultAlreadyInitedError>
where
    I: FnOnce() -> Client + Send + 'static,
{
    let mut ci = CLIENT_INIT.lock();
    if let ClientInit::Inited = &*ci {
        Err(DefaultAlreadyInitedError)
    } else {
        *ci = ClientInit::Set(Box::new(init));
        Ok(())
    }
}

/// Error returned by [`set_default_client_init`] if the default was already initialized.
#[derive(Debug, Clone, Copy)]
pub struct DefaultAlreadyInitedError;
impl fmt::Display for DefaultAlreadyInitedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "default client already initialized, can only set before first use")
    }
}
impl std::error::Error for DefaultAlreadyInitedError {}

/// Information about the state of an HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// including name lookup, connect, pretransfer and transfer before final transaction was started.
    pub redirect_time: Duration,
}
impl Metrics {
    /// Init from [`isahc::Metrics`].
    pub fn from_isahc(m: &isahc::Metrics) -> Self {
        Self {
            upload_progress: {
                let (c, t) = m.upload_progress();
                ((c as usize).bytes(), (t as usize).bytes())
            },
            upload_speed: (m.upload_speed().round() as usize).bytes(),
            download_progress: {
                let (c, t) = m.download_progress();
                ((c as usize).bytes(), (t as usize).bytes())
            },
            download_speed: (m.download_speed().round() as usize).bytes(),
            name_lookup_time: m.name_lookup_time(),
            connect_time: m.connect_time(),
            secure_connect_time: m.secure_connect_time(),
            transfer_start_time: m.transfer_start_time(),
            transfer_time: m.transfer_time(),
            total_time: m.total_time(),
            redirect_time: m.redirect_time(),
        }
    }

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
impl From<isahc::Metrics> for Metrics {
    fn from(m: isahc::Metrics) -> Self {
        Metrics::from_isahc(&m)
    }
}
impl fmt::Display for Metrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut ws = false; // written something

        if self.upload_progress.0 != self.upload_progress.1 {
            write!(
                f,
                "upload: {} of {}, {}/s",
                self.upload_progress.0, self.upload_progress.1, self.upload_speed
            )?;
            ws = true;
        }
        if self.download_progress.0 != self.download_progress.1 {
            write!(
                f,
                "{}download: {} of {}, {}/s",
                if ws { "\n" } else { "" },
                self.download_progress.0,
                self.download_progress.1,
                self.download_speed
            )?;
            ws = true;
        }

        if !ws {
            if self.upload_progress.1.bytes() > 0 {
                write!(f, "uploaded: {}", self.upload_progress.1)?;
                ws = true;
            }
            if self.download_progress.1.bytes() > 0 {
                write!(f, "{}downloaded: {}", if ws { "\n" } else { "" }, self.download_progress.1)?;
                ws = true;
            }

            if ws {
                write!(f, "\ntotal time: {:?}", self.total_time)?;
            }
        }

        Ok(())
    }
}

/// HTTP client.
///
/// An HTTP client acts as a session for executing one of more HTTP requests.
pub struct Client {
    client: isahc::HttpClient,
    cache: Option<Box<dyn CacheDb>>,
    cache_mode: Arc<dyn Fn(&Request) -> CacheMode + Send + Sync>,
}
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
impl Clone for Client {
    fn clone(&self) -> Self {
        Client {
            client: self.client.clone(),
            cache: self.cache.as_ref().map(|b| b.clone_boxed()),
            cache_mode: self.cache_mode.clone(),
        }
    }
}
impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}
impl Client {
    /// New client with default config.
    ///
    /// This enables cookies, sets the `redirect_policy` with a limit of up-to 20 redirects and `auto_referer`, sets
    /// the `connect_timeout` to 90 seconds and enables `metrics`.
    pub fn new() -> Self {
        Client::builder()
            .cookies()
            .redirect_policy(RedirectPolicy::Limit(20))
            .connect_timeout(90.secs())
            .metrics(true)
            .build()
    }

    /// Start a new [`ClientBuilder`] for creating a custom client.
    #[inline]
    pub fn builder() -> ClientBuilder {
        ClientBuilder {
            builder: isahc::HttpClient::builder(),
            cache: None,
            cache_mode: None,
        }
    }

    /// Gets the configured cookie-jar for this client, if cookies are enabled.
    pub fn cookie_jar(&self) -> Option<&CookieJar> {
        self.client.cookie_jar()
    }

    ///  Send a GET request to the `uri`.
    #[inline]
    pub async fn get(&self, uri: impl TryUri) -> Result<Response, Error> {
        self.send(Request::get(uri)?.build()).await
    }

    /// Send a GET request to the `uri` and read the response as a string.
    pub async fn get_text(&self, uri: impl TryUri) -> Result<String, Error> {
        let mut r = self.get(uri).await?;
        let r = r.text().await?;
        Ok(r)
    }

    /// Send a GET request to the `uri` and read the response as raw bytes.
    pub async fn get_bytes(&self, uri: impl TryUri) -> Result<Vec<u8>, Error> {
        let mut r = self.get(uri).await?;
        let r = r.bytes().await?;
        Ok(r)
    }

    /// Send a GET request to the `uri` and de-serializes the response.
    pub async fn get_json<O>(&self, uri: impl TryUri) -> Result<O, Box<dyn std::error::Error>>
    where
        O: serde::de::DeserializeOwned + std::marker::Unpin,
    {
        let mut r = self.get(uri).await?;
        let r = r.json::<O>().await?;
        Ok(r)
    }

    /// Send a HEAD request to the `uri`.
    #[inline]
    pub async fn head(&self, uri: impl TryUri) -> Result<Response, Error> {
        self.send(Request::head(uri)?.build()).await
    }
    /// Send a PUT request to the `uri` with a given request body.
    #[inline]
    pub async fn put(&self, uri: impl TryUri, body: impl TryBody) -> Result<Response, Error> {
        self.send(Request::put(uri)?.body(body)?).await
    }

    /// Send a POST request to the `uri` with a given request body.
    #[inline]
    pub async fn post(&self, uri: impl TryUri, body: impl TryBody) -> Result<Response, Error> {
        self.send(Request::post(uri)?.body(body)?).await
    }

    /// Send a DELETE request to the `uri`.
    #[inline]
    pub async fn delete(&self, uri: impl TryUri) -> Result<Response, Error> {
        self.send(Request::delete(uri)?.build()).await
    }

    /// Send a custom [`Request`].
    ///
    /// # Cache
    ///
    /// If the client has a [`cache`] and the request uses the `GET` method the result will be cached
    /// according with the [`cache_mode`] selected for the request.
    ///
    /// [`cache`]: Self::cache
    /// [`cache_mode`]: Self::cache_mode
    /// [`get_cached`]: Self::get_cached
    #[inline]
    pub async fn send(&self, request: Request) -> Result<Response, Error> {
        if let Some(db) = &self.cache {
            match self.cache_mode(&request) {
                CacheMode::NoCache => self.client.send_async(request.0).await.map(Response),
                CacheMode::Default => self.send_cache_default(&**db, request, 0).await,
                CacheMode::Permanent => todo!(),
                CacheMode::Error(e) => Err(e),
            }
        } else {
            self.client.send_async(request.0).await.map(Response)
        }
    }

    #[async_recursion::async_recursion]
    async fn send_cache_default(&self, db: &dyn CacheDb, request: Request, retry_count: u8) -> Result<Response, Error> {
        if retry_count == 3 {
            tracing::error!("retried cache 3 times, skipping cache");
            return self.client.send_async(request.0).await.map(Response);
        }

        let key = CacheKey::new(&request.0);
        if let Some(policy) = db.policy(&key).await {
            match policy.before_request(&request.0) {
                BeforeRequest::Fresh(parts) => {
                    if let Some(body) = db.body(&key).await {
                        let response = isahc::Response::from_parts(parts, body.0);
                        Ok(Response(response))
                    } else {
                        tracing::error!("cache returned policy but not body");
                        db.remove(&key).await;
                        self.send_cache_default(db, request, retry_count + 1).await
                    }
                }
                BeforeRequest::Stale { request: parts, matches } => {
                    if matches {
                        let (_, body) = request.0.into_parts();
                        let request = Request(isahc::Request::from_parts(parts, body));
                        let policy_request = request.clone_with(()).unwrap().0;
                        let no_req_body = request.0.body().len().map(|l| l == 0).unwrap_or(false);

                        let response = self.client.send_async(request.0).await?;

                        match policy.after_response(&policy_request, &response) {
                            AfterResponse::NotModified(policy, parts) => {
                                if let Some(body) = db.body(&key).await {
                                    let response = isahc::Response::from_parts(parts, body.0);

                                    db.set_policy(&key, CachePolicy(policy)).await;

                                    Ok(Response(response))
                                } else {
                                    tracing::error!("cache returned policy but not body");
                                    db.remove(&key).await;

                                    if no_req_body {
                                        self.send_cache_default(db, Request(policy_request), retry_count + 1).await
                                    } else {
                                        Err(std::io::Error::new(
                                            std::io::ErrorKind::NotFound,
                                            "cache returned policy but not body, cannot auto-retry",
                                        )
                                        .into())
                                    }
                                }
                            }
                            AfterResponse::Modified(policy, parts) => {
                                let (_, body) = response.into_parts();

                                if let Some(body) = db.set(&key, CachePolicy(policy), Body(body)).await {
                                    let response = isahc::Response::from_parts(parts, body.0);
                                    Ok(Response(response))
                                } else {
                                    tracing::error!("cache db failed to store body");
                                    db.remove(&key).await;

                                    if no_req_body {
                                        self.send_cache_default(db, Request(policy_request), retry_count + 1).await
                                    } else {
                                        Err(std::io::Error::new(
                                            std::io::ErrorKind::NotFound,
                                            "cache db failed to store body, cannot auto-retry",
                                        )
                                        .into())
                                    }
                                }
                            }
                        }
                    } else {
                        tracing::error!("cache policy did not match request, {:?}", request);
                        db.remove(&key).await;
                        self.client.send_async(request.0).await.map(Response)
                    }
                }
            }
        } else {
            let no_req_body = request.0.body().len().map(|l| l == 0).unwrap_or(false);
            let policy_request = request.clone_with(()).unwrap().0;

            let response = self.client.send_async(request.0).await?;

            let policy = CachePolicy::new(&policy_request, &response).0;

            if policy.is_storable() {
                let (parts, body) = response.into_parts();

                if let Some(body) = db.set(&key, CachePolicy(policy), Body(body)).await {
                    let response = isahc::Response::from_parts(parts, body.0);

                    Ok(Response(response))
                } else {
                    tracing::error!("cache db failed to store body");
                    db.remove(&key).await;

                    if no_req_body {
                        self.send_cache_default(db, Request(policy_request), retry_count + 1).await
                    } else {
                        Err(std::io::Error::new(std::io::ErrorKind::NotFound, "cache db failed to store body, cannot auto-retry").into())
                    }
                }
            } else {
                Ok(Response(response))
            }
        }
    }

    /// Reference the cache used in this client.
    #[inline]
    pub fn cache(&self) -> Option<&dyn CacheDb> {
        self.cache.as_deref()
    }

    /// Returns the [`CacheMode`] that is used in this client if the request is made.
    pub fn cache_mode(&self, request: &Request) -> CacheMode {
        if self.cache.is_none() || request.method() != Method::GET {
            CacheMode::NoCache
        } else {
            (&self.cache_mode)(request)
        }
    }
}
impl From<Client> for isahc::HttpClient {
    fn from(c: Client) -> Self {
        c.client
    }
}
impl From<isahc::HttpClient> for Client {
    fn from(client: isahc::HttpClient) -> Self {
        Self {
            client,
            cache: None,
            cache_mode: Arc::new(|_| CacheMode::default()),
        }
    }
}

/// Builder that can be used to create a [`Client`].
///
/// Use [`Client::builder`] to start building.
///
/// # Example
///
/// ```
/// use zero_ui_core::task::http::*;
///
/// let client = Client::builder().metrics(true).build();
/// ```
pub struct ClientBuilder {
    builder: isahc::HttpClientBuilder,
    cache: Option<Box<dyn CacheDb>>,
    cache_mode: Option<Arc<dyn Fn(&Request) -> CacheMode + Send + Sync>>,
}
impl Default for ClientBuilder {
    fn default() -> Self {
        Client::builder()
    }
}
impl ClientBuilder {
    /// New default builder.
    #[inline]
    pub fn new() -> Self {
        Client::builder()
    }

    /// Build the [`Client`] using the configured options.
    #[inline]
    pub fn build(self) -> Client {
        Client {
            client: self.builder.build().unwrap(),
            cache: self.cache,
            cache_mode: self.cache_mode.unwrap_or_else(|| Arc::new(|_| CacheMode::default())),
        }
    }

    /// Build the client with more custom build calls in the [inner builder].
    ///
    /// [inner builder]: isahc::HttpClientBuilder
    pub fn build_custom<F>(self, custom: F) -> Result<Client, Error>
    where
        F: FnOnce(isahc::HttpClientBuilder) -> Result<isahc::HttpClient, Error>,
    {
        custom(self.builder).map(|c| Client {
            client: c,
            cache: self.cache,
            cache_mode: self.cache_mode.unwrap_or_else(|| Arc::new(|_| CacheMode::default())),
        })
    }

    /// Add a default header to be passed with every request.
    #[inline]
    pub fn default_header(self, key: impl TryHeaderName, value: impl TryHeaderValue) -> Result<Self, Error> {
        Ok(Self {
            builder: self.builder.default_header(key.try_header_name()?, value.try_header_value()?),
            cache: self.cache,
            cache_mode: self.cache_mode,
        })
    }

    /// Enable persistent cookie handling for all requests using this client using a shared cookie jar.
    #[inline]
    pub fn cookies(self) -> Self {
        Self {
            builder: self.builder.cookies(),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Set a cookie jar to use to accept, store, and supply cookies for incoming responses and outgoing requests.
    ///
    /// Note that the [`default_client`] already has a cookie jar.
    pub fn cookie_jar(self, cookie_jar: CookieJar) -> Self {
        Self {
            builder: self.builder.cookie_jar(cookie_jar),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Specify a maximum amount of time that a complete request/response cycle is allowed to
    /// take before being aborted. This includes DNS resolution, connecting to the server,
    /// writing the request, and reading the response.
    ///
    /// Note that this includes the response read operation, so if you get a response but don't
    /// read-it within this timeout you will get a [`TimedOut`] IO error.
    ///
    /// By default no timeout is used.
    ///
    /// [`TimedOut`]: https://doc.rust-lang.org/nightly/std/io/enum.ErrorKind.html#variant.TimedOut
    pub fn timeout(self, timeout: Duration) -> Self {
        Self {
            builder: self.builder.timeout(timeout),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Set a timeout for establishing connections to a host.
    ///
    /// If not set, the [`default_client`] default of 90 seconds will be used.
    pub fn connect_timeout(self, timeout: Duration) -> Self {
        Self {
            builder: self.builder.connect_timeout(timeout),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Specify a maximum amount of time where transfer rate can go below a minimum speed limit.
    ///
    /// The `low_speed` limit is in bytes/s. No low-speed limit is configured by default.
    pub fn low_speed_timeout(self, low_speed: u32, timeout: Duration) -> Self {
        Self {
            builder: self.builder.low_speed_timeout(low_speed, timeout),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Set a policy for automatically following server redirects.
    ///
    /// If enabled the "Referer" header will be set automatically too.
    pub fn redirect_policy(self, policy: RedirectPolicy) -> Self {
        if !matches!(policy, RedirectPolicy::None) {
            Self {
                builder: self.builder.redirect_policy(policy).auto_referer(),
                cache: self.cache,
                cache_mode: self.cache_mode,
            }
        } else {
            Self {
                builder: self.builder.redirect_policy(policy),
                cache: self.cache,
                cache_mode: self.cache_mode,
            }
        }
    }

    /// Enable or disable automatic decompression of the response body.
    ///
    /// If enabled the "Accept-Encoding" will also be set automatically, if it was not set using [`default_header`].
    ///
    /// This is enabled by default.
    ///
    /// [`default_header`]: Self::default_header
    pub fn auto_decompress(self, enabled: bool) -> Self {
        Self {
            builder: self.builder.automatic_decompression(enabled),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Set a maximum upload speed for the request body, in bytes per second.
    pub fn max_upload_speed(self, max: u64) -> Self {
        Self {
            builder: self.builder.max_upload_speed(max),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Set a maximum download speed for the response body, in bytes per second.
    pub fn max_download_speed(self, max: u64) -> Self {
        Self {
            builder: self.builder.max_download_speed(max),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Enable or disable metrics collecting.
    ///
    /// When enabled you can get the information using the [`Response::metrics`] method.
    ///
    /// This is enabled by default.
    pub fn metrics(self, enable: bool) -> Self {
        Self {
            builder: self.builder.metrics(enable),
            cache: self.cache,
            cache_mode: self.cache_mode,
        }
    }

    /// Sets the [`CacheDb`] to use.
    ///
    /// Caching is only enabled if there is a DB, no caching is done by default.
    pub fn cache(self, cache: impl CacheDb) -> Self {
        Self {
            builder: self.builder,
            cache: Some(Box::new(cache)),
            cache_mode: self.cache_mode,
        }
    }

    /// Sets the [`CacheMode`] selector.
    ///
    /// The `selector` closure is called for every cacheable request before it is made, it
    /// must return a [`CacheMode`] value that configures how the [`cache`] is used.
    ///
    /// Note that the closure is only called if a [`cache`] is set.
    ///
    /// [`cache`]: Self::cache
    pub fn cache_mode(self, selector: impl Fn(&Request) -> CacheMode + Send + Sync + 'static) -> Self {
        Self {
            builder: self.builder,
            cache: self.cache,
            cache_mode: Some(Arc::new(selector)),
        }
    }
}
