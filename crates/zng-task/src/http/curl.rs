use std::fmt;

use crate::http::{Error, HttpClient, Request, Response};

use super::uri::Scheme;

/// Basic [`HttpClient`] implementation that uses the `curl` command line utility.
#[derive(Default)]
pub struct CurlProcessClient {}
impl HttpClient for CurlProcessClient {
    fn send(&'static self, request: Request) -> std::pin::Pin<Box<dyn Future<Output = Result<Response, Error>> + Send>> {
        Box::pin(run(request))
    }
}

async fn run(request: Request) -> Result<Response, Error> {
    let not_http = match request.uri.scheme() {
        Some(s) => s != &Scheme::HTTP && s != &Scheme::HTTPS,
        None => true,
    };
    if not_http {
        return Err(Box::new(NotHttpUriError));
    }

    // !!: TODO

    todo!()
}

#[derive(Debug)]
struct NotHttpUriError;
impl fmt::Display for NotHttpUriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "uri is not HTTP or HTTPS")
    }
}
impl std::error::Error for NotHttpUriError {}
