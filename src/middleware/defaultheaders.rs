//! Default response headers
use std::rc::Rc;

use actix_http::http::header::{HeaderName, HeaderValue, CONTENT_TYPE};
use actix_http::http::{HeaderMap, HttpTryFrom};
use actix_http::Request;
use actix_service::{IntoNewTransform, Service, Transform};
use futures::{Async, Future, Poll};

use crate::middleware::MiddlewareFactory;
use crate::service::ServiceResponse;

/// `Middleware` for setting default response headers.
///
/// This middleware does not set header if response headers already contains it.
///
/// ```rust
/// # extern crate actix_web;
/// use actix_web::{http, middleware, App, HttpResponse};
///
/// fn main() {
///     let app = App::new()
///         .middleware(middleware::DefaultHeaders::new().header("X-Version", "0.2"))
///         .resource("/test", |r| {
///             r.method(http::Method::GET).f(|_| HttpResponse::Ok());
///             r.method(http::Method::HEAD)
///                 .f(|_| HttpResponse::MethodNotAllowed());
///         });
/// }
/// ```
#[derive(Clone)]
pub struct DefaultHeaders {
    inner: Rc<Inner>,
}

struct Inner {
    ct: bool,
    headers: HeaderMap,
}

impl Default for DefaultHeaders {
    fn default() -> Self {
        DefaultHeaders {
            inner: Rc::new(Inner {
                ct: false,
                headers: HeaderMap::new(),
            }),
        }
    }
}

impl DefaultHeaders {
    /// Construct `DefaultHeaders` middleware.
    pub fn new() -> DefaultHeaders {
        DefaultHeaders::default()
    }

    /// Set a header.
    #[inline]
    #[cfg_attr(feature = "cargo-clippy", allow(match_wild_err_arm))]
    pub fn header<K, V>(mut self, key: K, value: V) -> Self
    where
        HeaderName: HttpTryFrom<K>,
        HeaderValue: HttpTryFrom<V>,
    {
        match HeaderName::try_from(key) {
            Ok(key) => match HeaderValue::try_from(value) {
                Ok(value) => {
                    Rc::get_mut(&mut self.inner)
                        .expect("Multiple copies exist")
                        .headers
                        .append(key, value);
                }
                Err(_) => panic!("Can not create header value"),
            },
            Err(_) => panic!("Can not create header name"),
        }
        self
    }

    /// Set *CONTENT-TYPE* header if response does not contain this header.
    pub fn content_type(mut self) -> Self {
        Rc::get_mut(&mut self.inner)
            .expect("Multiple copies exist")
            .ct = true;
        self
    }
}

impl<S> IntoNewTransform<MiddlewareFactory<DefaultHeaders, S>, S> for DefaultHeaders
where
    S: Service<Request = Request, Response = ServiceResponse>,
    S::Future: 'static,
{
    fn into_new_transform(self) -> MiddlewareFactory<DefaultHeaders, S> {
        MiddlewareFactory::new(self)
    }
}

impl<S> Transform<S> for DefaultHeaders
where
    S: Service<Request = Request, Response = ServiceResponse>,
    S::Future: 'static,
{
    type Request = S::Request;
    type Response = S::Response;
    type Error = S::Error;
    type Future = Box<Future<Item = Self::Response, Error = Self::Error>>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(Async::Ready(()))
    }

    fn call(&mut self, req: Request, srv: &mut S) -> Self::Future {
        let inner = self.inner.clone();

        Box::new(srv.call(req).map(move |mut res| {
            match res {
                ServiceResponse::Response(ref mut res) => {
                    // set response headers
                    for (key, value) in inner.headers.iter() {
                        if !res.headers().contains_key(key) {
                            res.headers_mut().insert(key, value.clone());
                        }
                    }
                    // default content-type
                    if inner.ct && !res.headers().contains_key(CONTENT_TYPE) {
                        res.headers_mut().insert(
                            CONTENT_TYPE,
                            HeaderValue::from_static("application/octet-stream"),
                        );
                    }
                }
                _ => (),
            }
            res
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::header::CONTENT_TYPE;
    use test::TestRequest;

    #[test]
    fn test_default_headers() {
        let mw = DefaultHeaders::new().header(CONTENT_TYPE, "0001");

        let req = TestRequest::default().finish();

        let resp = HttpResponse::Ok().finish();
        let resp = match mw.response(&req, resp) {
            Ok(Response::Done(resp)) => resp,
            _ => panic!(),
        };
        assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "0001");

        let resp = HttpResponse::Ok().header(CONTENT_TYPE, "0002").finish();
        let resp = match mw.response(&req, resp) {
            Ok(Response::Done(resp)) => resp,
            _ => panic!(),
        };
        assert_eq!(resp.headers().get(CONTENT_TYPE).unwrap(), "0002");
    }
}
