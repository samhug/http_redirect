//! Redirect requests using the 'http' scheme to the equivalent 'https' uri.
//!
//! # Example
//!
//! ```
//! use http_redirect::{RedirectLayer, HttpsAndHostRedirect};
//! use hyper::{Request, Response, Body, Error};
//! use http::StatusCode;
//! use tower::{Service, ServiceExt, ServiceBuilder, service_fn};
//!
//! async fn handle(request: Request<Body>) -> Result<Response<Body>, Error> {
//!     Ok(Response::new(Body::empty()))
//! }
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut service = ServiceBuilder::new()
//!     .layer(RedirectLayer::new(HttpsAndHostRedirect::new("localhost")))
//!     .service_fn(handle);
//!
//! // Requests using a uri with an https scheme are passed to the inner service
//! let request = Request::builder()
//!     .uri("https://localhost/")
//!     .body(Body::empty())
//!     .unwrap();
//!
//! let response = service
//!     .ready()
//!     .await?
//!     .call(request)
//!     .await?;
//!
//! assert_eq!(StatusCode::OK, response.status());
//!
//! // Requests using a uri with an http scheme get a `301 Moved Permanently` response
//! let request = Request::builder()
//!     .body(Body::empty())
//!     .unwrap();
//!
//! let response = service
//!     .ready()
//!     .await?
//!     .call(request)
//!     .await?;
//!
//! assert_eq!(StatusCode::MOVED_PERMANENTLY, response.status());
//! # Ok(())
//! # }
//! ```

pub mod layer;
mod redirect;
pub mod service;

use http::{Request, Response};
pub use layer::RedirectLayer;
pub use redirect::HttpsAndHostRedirect;
pub use service::Redirect;

/// Trait for redirecting requests.
pub trait Redirector<B> {
    /// The body type used for responses to redirected requests.
    type ResponseBody;

    /// redirect the request.
    ///
    /// If `None` is returned then the request is not redirected
    fn redirect(&mut self, request: &mut Request<B>) -> Result<(), Response<Self::ResponseBody>>;
}

impl<B, F, ResBody> Redirector<B> for F
where
    F: FnMut(&mut Request<B>) -> Result<(), Response<ResBody>>,
{
    type ResponseBody = ResBody;

    fn redirect(&mut self, request: &mut Request<B>) -> Result<(), Response<Self::ResponseBody>> {
        self(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{header, Request, Response, StatusCode};
    use tower::{BoxError, ServiceBuilder, ServiceExt};
    use tower_service::Service;

    #[tokio::test]
    async fn https_request() {
        let mut service = ServiceBuilder::new()
            .layer(RedirectLayer::new(HttpsAndHostRedirect::new("localhost")))
            .service_fn(echo);

        let request = Request::get("https://localhost/")
            .header("host", "localhost")
            .header("x-forwarded-proto", "https")
            .body(hyper::Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn http_request() {
        let mut service = ServiceBuilder::new()
            .layer(RedirectLayer::new(HttpsAndHostRedirect::new("localhost")))
            .service_fn(echo);

        let request = Request::get("http://localhost/")
            .header("host", "localhost")
            .header("x-forwarded-proto", "http")
            .body(hyper::Body::empty())
            .unwrap();

        let res = service.ready().await.unwrap().call(request).await.unwrap();

        assert_eq!(res.status(), StatusCode::MOVED_PERMANENTLY);

        let redirect_target = res.headers().get(header::LOCATION).unwrap();
        assert_eq!(redirect_target, "https://localhost/");
    }

    async fn echo(req: Request<hyper::Body>) -> Result<Response<hyper::Body>, BoxError> {
        Ok(Response::new(req.into_body()))
    }
}
