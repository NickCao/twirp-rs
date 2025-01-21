//! Support for serving Twirp APIs.
//!
//! There is not much to see in the documentation here. This API is meant to be used with
//! `twirp-build`. See <https://github.com/github/twirp-rs#usage> for details and an example.

use std::fmt::Debug;
use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::response::IntoResponse;
use futures::Future;
use http::Extensions;
use http_body_util::BodyExt;
use hyper::{header, Request, Response};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::time::{Duration, Instant};

use crate::headers::{CONTENT_TYPE_JSON, CONTENT_TYPE_PROTOBUF};
use crate::{error, serialize_proto_message, Context, GenericError, IntoTwirpResponse};

// TODO: Properly implement JsonPb (de)serialization as it is slightly different
// than standard JSON.
#[derive(Debug, Clone, Copy, Default)]
enum BodyFormat {
    #[default]
    JsonPb,
    Pb,
}

impl BodyFormat {
    fn from_content_type(req: &Request<Body>) -> BodyFormat {
        match req
            .headers()
            .get(header::CONTENT_TYPE)
            .map(|x| x.as_bytes())
        {
            Some(CONTENT_TYPE_PROTOBUF) => BodyFormat::Pb,
            _ => BodyFormat::JsonPb,
        }
    }
}

/// Entry point used in code generated by `twirp-build`.
pub(crate) async fn handle_request<S, F, Fut, Req, Resp, Err>(
    service: S,
    req: Request<Body>,
    f: F,
) -> Response<Body>
where
    F: FnOnce(S, Context, Req) -> Fut + Clone + Sync + Send + 'static,
    Fut: Future<Output = Result<Resp, Err>> + Send,
    Req: prost::Message + Default + serde::de::DeserializeOwned,
    Resp: prost::Message + serde::Serialize,
    Err: IntoTwirpResponse,
{
    let mut timings = req
        .extensions()
        .get::<Timings>()
        .copied()
        .unwrap_or_else(|| Timings::new(Instant::now()));

    let (req, exts, resp_fmt) = match parse_request(req, &mut timings).await {
        Ok(pair) => pair,
        Err(err) => {
            // TODO: Capture original error in the response extensions. E.g.:
            // resp_exts
            //     .lock()
            //     .expect("mutex poisoned")
            //     .insert(RequestError(err));
            let mut twirp_err = error::malformed("bad request");
            twirp_err.insert_meta("error".to_string(), err.to_string());
            return twirp_err.into_response();
        }
    };

    let resp_exts = Arc::new(Mutex::new(Extensions::new()));
    let ctx = Context::new(exts, resp_exts.clone());
    let res = f(service, ctx, req).await;
    timings.set_response_handled();

    let mut resp = match write_response(res, resp_fmt) {
        Ok(resp) => resp,
        Err(err) => {
            // TODO: Capture original error in the response extensions.
            let mut twirp_err = error::unknown("error serializing response");
            twirp_err.insert_meta("error".to_string(), err.to_string());
            return twirp_err.into_response();
        }
    };
    timings.set_response_written();

    resp.extensions_mut()
        .extend(resp_exts.lock().expect("mutex poisoned").clone());
    resp.extensions_mut().insert(timings);
    resp
}

async fn parse_request<T>(
    req: Request<Body>,
    timings: &mut Timings,
) -> Result<(T, Extensions, BodyFormat), GenericError>
where
    T: prost::Message + Default + DeserializeOwned,
{
    let format = BodyFormat::from_content_type(&req);
    let (parts, body) = req.into_parts();
    let bytes = body.collect().await?.to_bytes();
    timings.set_received();
    let request = match format {
        BodyFormat::Pb => T::decode(&bytes[..])?,
        BodyFormat::JsonPb => serde_json::from_slice(&bytes)?,
    };
    timings.set_parsed();
    Ok((request, parts.extensions, format))
}

fn write_response<T, Err>(
    response: Result<T, Err>,
    response_format: BodyFormat,
) -> Result<Response<Body>, GenericError>
where
    T: prost::Message + Serialize,
    Err: IntoTwirpResponse,
{
    let res = match response {
        Ok(response) => match response_format {
            BodyFormat::Pb => Response::builder()
                .header(header::CONTENT_TYPE, CONTENT_TYPE_PROTOBUF)
                .body(Body::from(serialize_proto_message(response)))?,
            BodyFormat::JsonPb => {
                let data = serde_json::to_string(&response)?;
                Response::builder()
                    .header(header::CONTENT_TYPE, CONTENT_TYPE_JSON)
                    .body(Body::from(data))?
            }
        },
        Err(err) => err.into_twirp_response().map(|err| err.into_axum_body()),
    };
    Ok(res)
}

/// Axum handler function that returns 404 Not Found with a Twirp JSON payload.
///
/// `axum::Router`'s default fallback handler returns a 404 Not Found with no body content.
/// Use this fallback instead for full Twirp compliance.
///
/// # Usage
///
/// ```
/// use axum::Router;
///
/// # fn build_app(twirp_routes: Router) -> Router {
/// let app = Router::new()
///     .nest("/twirp", twirp_routes)
///     .fallback(twirp::server::not_found_handler);
/// # app }
/// ```
pub async fn not_found_handler() -> Response<Body> {
    error::bad_route("not found").into_response()
}

/// Contains timing information associated with a request.
/// To access the timings in a given request, use the [extensions](Request::extensions)
/// method and specialize to `Timings` appropriately.
#[derive(Debug, Clone, Copy)]
pub struct Timings {
    // When the request started.
    start: Instant,
    // When the request was received (headers and body).
    request_received: Option<Instant>,
    // When the request body was parsed.
    request_parsed: Option<Instant>,
    // When the response handler returned.
    response_handled: Option<Instant>,
    // When the response was written.
    response_written: Option<Instant>,
}

impl Timings {
    #[allow(clippy::new_without_default)]
    pub fn new(start: Instant) -> Self {
        Self {
            start,
            request_received: None,
            request_parsed: None,
            response_handled: None,
            response_written: None,
        }
    }

    fn set_received(&mut self) {
        self.request_received = Some(Instant::now());
    }

    fn set_parsed(&mut self) {
        self.request_parsed = Some(Instant::now());
    }

    fn set_response_handled(&mut self) {
        self.response_handled = Some(Instant::now());
    }

    fn set_response_written(&mut self) {
        self.response_written = Some(Instant::now());
    }

    pub fn received(&self) -> Option<Duration> {
        self.request_received.map(|x| x - self.start)
    }

    pub fn parsed(&self) -> Option<Duration> {
        match (self.request_parsed, self.request_received) {
            (Some(parsed), Some(received)) => Some(parsed - received),
            _ => None,
        }
    }

    pub fn response_handled(&self) -> Option<Duration> {
        match (self.response_handled, self.request_parsed) {
            (Some(handled), Some(parsed)) => Some(handled - parsed),
            _ => None,
        }
    }

    pub fn response_written(&self) -> Option<Duration> {
        match (self.response_written, self.response_handled) {
            (Some(written), Some(handled)) => Some(written - handled),
            (Some(written), None) => {
                if let Some(parsed) = self.request_parsed {
                    Some(written - parsed)
                } else {
                    self.request_received.map(|received| written - received)
                }
            }
            _ => None,
        }
    }

    /// The total duration since the request started.
    pub fn total_duration(&self) -> Duration {
        self.start.elapsed()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::test::*;

    use axum::middleware::{self, Next};
    use tower::Service;

    fn timings() -> Timings {
        Timings::new(Instant::now())
    }

    #[tokio::test]
    async fn test_bad_route() {
        let mut router = test_api_router();
        let req = Request::get("/nothing")
            .extension(timings())
            .body(Body::empty())
            .unwrap();

        let resp = router.call(req).await.unwrap();
        let data = read_err_body(resp.into_body()).await;
        assert_eq!(data, error::bad_route("not found"));
    }

    #[tokio::test]
    async fn test_ping_success() {
        let mut router = test_api_router();
        let resp = router.call(gen_ping_request("hi")).await.unwrap();
        assert!(resp.status().is_success(), "{:?}", resp);
        let data: PingResponse = read_json_body(resp.into_body()).await;
        assert_eq!(&data.name, "hi");
    }

    #[tokio::test]
    async fn test_ping_invalid_request() {
        let mut router = test_api_router();
        let req = Request::post("/twirp/test.TestAPI/Ping")
            .extension(timings())
            .body(Body::empty()) // not a valid request
            .unwrap();
        let resp = router.call(req).await.unwrap();
        assert!(resp.status().is_client_error(), "{:?}", resp);
        let data = read_err_body(resp.into_body()).await;

        // TODO: I think malformed should return some info about what was wrong
        // with the request, but we don't want to leak server errors that have
        // other details.
        let mut expected = error::malformed("bad request");
        expected.insert_meta(
            "error".to_string(),
            "EOF while parsing a value at line 1 column 0".to_string(),
        );
        assert_eq!(data, expected);
    }

    #[tokio::test]
    async fn test_boom() {
        let mut router = test_api_router();
        let req = serde_json::to_string(&PingRequest {
            name: "hi".to_string(),
        })
        .unwrap();
        let req = Request::post("/twirp/test.TestAPI/Boom")
            .extension(timings())
            .body(Body::from(req))
            .unwrap();
        let resp = router.call(req).await.unwrap();
        assert!(resp.status().is_server_error(), "{:?}", resp);
        let data = read_err_body(resp.into_body()).await;
        assert_eq!(data, error::internal("boom!"));
    }

    #[tokio::test]
    async fn test_middleware() {
        let mut router = test_api_router().layer(middleware::from_fn(request_id_middleware));

        // no request-id header
        let resp = router.call(gen_ping_request("hi")).await.unwrap();
        assert!(resp.status().is_success(), "{:?}", resp);
        let data: PingResponse = read_json_body(resp.into_body()).await;
        assert_eq!(&data.name, "hi");

        // now pass a header with x-request-id
        let req = Request::post("/twirp/test.TestAPI/Ping")
            .header("x-request-id", "abcd")
            .body(Body::from(
                serde_json::to_string(&PingRequest {
                    name: "hello".to_string(),
                })
                .expect("will always be valid json"),
            ))
            .expect("always a valid twirp request");
        let resp = router.call(req).await.unwrap();
        assert!(resp.status().is_success(), "{:?}", resp);
        let data: PingResponse = read_json_body(resp.into_body()).await;
        assert_eq!(&data.name, "hello-abcd");
    }

    async fn request_id_middleware(
        mut request: http::Request<Body>,
        next: Next,
    ) -> http::Response<Body> {
        let rid = request
            .headers()
            .get("x-request-id")
            .and_then(|v| v.to_str().ok())
            .map(|x| RequestId(x.to_string()));
        if let Some(rid) = rid {
            request.extensions_mut().insert(rid);
        }

        next.run(request).await
    }
}
