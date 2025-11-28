use std::{fmt, time::Duration};

use crate::{
    http::{Error, HttpClient, Metrics, Request, Response},
    io::{BufReader, ReadLimited},
};
use futures_lite::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _};
use http::Uri;
use once_cell::sync::Lazy;
use zng_unit::{ByteLength, ByteUnits as _};
use zng_var::{Var, const_var, var};

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

    let mut curl = crate::process::Command::new(&*CURL);

    curl.stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    curl.arg("--include"); // print header

    curl.arg("-X").arg(request.method.as_str());

    #[cfg(feature = "http-compression")]
    if request.auto_decompress && !request.headers.contains_key(http::header::ACCEPT_ENCODING) {
        curl.arg("-H").arg("accept-encoding").arg("zstd, br, gzip");
    }
    for (name, value) in request.headers {
        if let Some(name) = name
            && let Ok(value) = value.to_str()
        {
            curl.arg("-H").arg(format!("{name}: {value}"));
        }
    }

    let connect_timeout = request.timeout.min(request.connect_timeout);
    if connect_timeout < Duration::MAX {
        curl.arg("--connect-timeout").arg(request.connect_timeout.as_secs().to_string());
    }
    if request.timeout < Duration::MAX {
        curl.arg("--max-time").arg(request.timeout.as_secs().to_string());
    }
    if request.low_speed_timeout.0 < Duration::MAX && request.low_speed_timeout.1 > 0.bytes() {
        curl.arg("-y")
            .arg(request.low_speed_timeout.0.as_secs().to_string())
            .arg("-Y")
            .arg(request.low_speed_timeout.1.bytes().to_string());
    }

    if request.redirect_limit > 0 {
        curl.arg("-L").arg("--max-redirs").arg(request.redirect_limit.to_string());
    }
    let rate_limit = request.max_upload_speed.min(request.max_download_speed);
    if rate_limit < ByteLength::MAX {
        curl.arg("--limit-rate").arg(format!("{}K", rate_limit.kibis()));
    }

    if !request.body.is_empty() {
        curl.arg("--data-binary").arg("@-");
    }

    curl.arg(request.uri.to_string());

    let mut curl = curl.spawn()?;

    let mut stdin = curl.stdin.take().unwrap();
    let mut stdout = BufReader::new(curl.stdout.take().unwrap());
    let stderr = curl.stderr.take().unwrap();

    if !request.body.is_empty() {
        stdin.write_all(&request.body[..]).await?;
    }

    let metrics = if request.metrics {
        let m = var(Metrics::zero());
        read_metrics(m.clone(), stderr);
        m.read_only()
    } else {
        const_var(Metrics::zero())
    };

    let mut response_bytes = Vec::with_capacity(1024);
    let mut effective_uri = request.uri;

    loop {
        let len = stdout.read_until(b'\r', &mut response_bytes).await?;
        if len == 0 {
            let mut response_headers = [httparse::EMPTY_HEADER; 64];
            let mut response = httparse::Response::new(&mut response_headers);
            response.parse(&response_bytes)?;
            return run_response(
                response,
                effective_uri,
                #[cfg(feature = "http-compression")]
                request.auto_decompress,
                request.require_length,
                request.max_length,
                metrics,
                stdout,
            );
        }

        let mut b = [0u8; 1];
        stdout.read_exact(&mut b).await?;
        if b[0] == b'\n' {
            response_bytes.push(b'\n');
            let mut b = [0u8; 2];
            stdout.read_exact(&mut b).await?;
            if b == [b'\r', b'\n'] {
                let mut response_headers = [httparse::EMPTY_HEADER; 64];
                let mut response = httparse::Response::new(&mut response_headers);
                response.parse(&response_bytes)?;
                let code = http::StatusCode::from_u16(response.code.unwrap_or(0))?;
                if code.is_redirection()
                    && let Some(l) = response.headers.iter().find(|h| h.name.eq_ignore_ascii_case("Location"))
                    && let Ok(l) = str::from_utf8(l.value)
                    && let Ok(l) = l.parse::<Uri>()
                {
                    effective_uri = l;
                    response_bytes.clear();
                    continue; // to next header
                } else {
                    return run_response(
                        response,
                        effective_uri,
                        #[cfg(feature = "http-compression")]
                        request.auto_decompress,
                        request.require_length,
                        request.max_length,
                        metrics,
                        stdout,
                    );
                }
            } else {
                response_bytes.push(b[0]);
                response_bytes.push(b[1]);
            }
        }
    }
}
fn read_metrics(metrics: Var<Metrics>, stderr: crate::process::ChildStderr) {
    let mut stderr = BufReader::new(stderr);
    let mut progress_bytes = Vec::with_capacity(92);
    let mut run = async move || -> std::io::Result<()> {
        loop {
            progress_bytes.clear();
            let len = stderr.read_until(b'\r', &mut progress_bytes).await?;
            if len == 0 {
                break;
            }

            let progress = str::from_utf8(&progress_bytes).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            if !progress.trim_start().chars().next().unwrap_or('\0').is_ascii_digit() {
                continue;
            }
            // https://everything.curl.dev/cmdline/progressmeter.html#progress-meter-legend
            let mut iter = progress.split_whitespace();
            let _pct = iter.next();
            let _total = iter.next();
            let pct_down: u8 = iter.next().unwrap_or("100").parse().unwrap_or(100);
            let down = parse_curl_bytes(iter.next().unwrap_or("0"));
            let response_total = (down.0 as f64 * 100.0 / pct_down as f64).bytes();
            let pct_up: u8 = iter.next().unwrap_or("100").parse().unwrap_or(100);
            let up = parse_curl_bytes(iter.next().unwrap_or("0"));
            let request_total = (up.0 as f64 * 100.0 / pct_up as f64).bytes();
            let down_speed = parse_curl_bytes(iter.next().unwrap_or("0"));
            let up_speed = parse_curl_bytes(iter.next().unwrap_or("0"));
            let _total_time = iter.next();
            let time_current = parse_curl_duration(iter.next().unwrap_or("HH:MM:SS"));

            metrics.set(Metrics {
                read_progress: (down, response_total),
                read_speed: down_speed,
                write_progress: (up, request_total),
                write_speed: up_speed,
                total_time: time_current,
            });
        }

        Ok(())
    };
    crate::spawn(async move {
        let _ = run().await;
    });
}
fn parse_curl_bytes(s: &str) -> ByteLength {
    // https://everything.curl.dev/cmdline/progressmeter.html#units
    let (s, scale) = if let Some(s) = s.strip_suffix("K") {
        (s, 2usize.pow(10))
    } else if let Some(s) = s.strip_suffix("M") {
        (s, 2usize.pow(20))
    } else if let Some(s) = s.strip_prefix("G") {
        (s, 2usize.pow(30))
    } else if let Some(s) = s.strip_prefix("T") {
        (s, 2usize.pow(40))
    } else if let Some(s) = s.strip_prefix("P") {
        (s, 2usize.pow(50))
    } else {
        (s, 1)
    };
    let l: usize = s.parse().unwrap_or(0);
    ByteLength::from_byte(l * scale)
}
fn parse_curl_duration(s: &str) -> Duration {
    // HH:MM:SS
    let mut iter = s.split(':');
    let h: usize = iter.next().unwrap_or("0").parse().unwrap_or(0);
    let m: u8 = iter.next().unwrap_or("0").parse().unwrap_or(0);
    let s: u8 = iter.next().unwrap_or("0").parse().unwrap_or(0);
    Duration::from_hours(h as _) + Duration::from_mins(m as _) + Duration::from_secs(s as _)
}

fn run_response(
    response: httparse::Response<'_, '_>,
    effective_uri: Uri,
    #[cfg(feature = "http-compression")] auto_decompress: bool,
    require_length: bool,
    max_length: ByteLength,
    metrics: Var<Metrics>,
    reader: BufReader<crate::process::ChildStdout>,
) -> Result<Response, Error> {
    let code = http::StatusCode::from_u16(response.code.unwrap())?;

    let mut header = http::header::HeaderMap::new();
    for r in response.headers {
        if r.name.is_empty() {
            continue;
        }
        header.append(
            http::HeaderName::from_bytes(r.name.as_bytes())?,
            http::HeaderValue::from_bytes(r.value)?,
        );
    }
    if require_length {
        if let Some(l) = header.get(http::header::CONTENT_LENGTH)
            && let Ok(l) = l.to_str()
            && let Ok(l) = l.parse::<usize>()
        {
            if l < max_length.bytes() {
                return Err(Box::new(ContentLengthExceedsMaxError));
            }
        } else {
            return Err(Box::new(ContentLengthRequiredError));
        }
    }

    let reader = ReadLimited::new_default_err(reader, max_length);

    macro_rules! respond {
        ($read:expr) => {
            return Ok(Response::from_read(code, header, effective_uri, metrics, Box::new($read)))
        };
    }

    #[cfg(feature = "http-compression")]
    if auto_decompress && let Some(enc) = header.get(http::header::CONTENT_ENCODING) {
        if enc == "zstd" {
            respond!(async_compression::futures::bufread::ZstdDecoder::new(reader))
        } else if enc == "br" {
            respond!(async_compression::futures::bufread::BrotliDecoder::new(reader))
        } else if enc == "gzip" {
            respond!(async_compression::futures::bufread::GzipDecoder::new(reader))
        }
    }
    respond!(reader)
}

static CURL: Lazy<String> = Lazy::new(|| std::env::var("ZNG_CURL").unwrap_or_else(|_| "curl".to_owned()));

#[derive(Debug)]
struct NotHttpUriError;
impl fmt::Display for NotHttpUriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "uri is not HTTP or HTTPS")
    }
}
impl std::error::Error for NotHttpUriError {}

#[derive(Debug)]
struct ContentLengthRequiredError;
impl fmt::Display for ContentLengthRequiredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "response content length is required")
    }
}
impl std::error::Error for ContentLengthRequiredError {}

#[derive(Debug)]
struct ContentLengthExceedsMaxError;
impl fmt::Display for ContentLengthExceedsMaxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "response content length is exceeds maximum")
    }
}
impl std::error::Error for ContentLengthExceedsMaxError {}
