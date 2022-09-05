use http::{Request, Response};
use pin_project_lite::pin_project;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower_service::Service;

use crate::Redirector;

/// Middleware that redirects all http requests to https.
#[derive(Clone, Debug)]
pub struct Redirect<S, R> {
    inner: S,
    redirect: R,
}

impl<S, R> Redirect<S, R> {
    pub(crate) fn new(inner: S, redirect: R) -> Self {
        Self { inner, redirect }
    }
}

impl<ReqBody, ResBody, S, R> Service<Request<ReqBody>> for Redirect<S, R>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    R: Redirector<ReqBody, ResponseBody = ResBody>,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future, ResBody>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        match self.redirect.redirect(&mut req) {
            Ok(_) => ResponseFuture::future(self.inner.call(req)),
            Err(res) => ResponseFuture::redirect(res),
        }
    }
}

pin_project! {
    /// Response future for [`Redirect`].
    pub struct ResponseFuture<F, B> {
        #[pin]
        kind: Kind<F, B>,
    }
}

impl<F, B> ResponseFuture<F, B> {
    fn future(future: F) -> Self {
        Self {
            kind: Kind::Future { future },
        }
    }

    fn redirect(res: Response<B>) -> Self {
        Self {
            kind: Kind::Redirect {
                response: Some(res),
            },
        }
    }
}

pin_project! {
    #[project = KindProj]
    enum Kind<F, B> {
        Future {
            #[pin]
            future: F,
        },
        Redirect {
            response: Option<Response<B>>,
        },
    }
}

impl<F, B, E> Future for ResponseFuture<F, B>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().kind.project() {
            KindProj::Future { future } => future.poll(cx),
            KindProj::Redirect { response } => {
                let response = response.take().unwrap();
                Poll::Ready(Ok(response))
            }
        }
    }
}
