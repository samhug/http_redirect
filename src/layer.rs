use crate::service::Redirect;

use tower_layer::Layer;

/// Layer that applies [`HttpsRedirect`] which redirects all http requests to https
#[derive(Debug, Clone, Default)]
pub struct RedirectLayer<R> {
    redirect: R,
}

impl<R> RedirectLayer<R> {
    pub fn new(redirect: R) -> Self {
        Self { redirect }
    }
}

impl<S, R> Layer<S> for RedirectLayer<R>
where
    R: Clone,
{
    type Service = Redirect<S, R>;

    fn layer(&self, inner: S) -> Self::Service {
        Redirect::new(inner, self.redirect.clone())
    }
}
