use futures::future::{self, Executor, Future};
use hyper::header::{HeaderValue, HOST, LOCATION};
use hyper::service::service_fn;
use hyper::{Body, Request, Response, Server};
use std::env;
use std::io::{Error, ErrorKind};
use tokio_core::reactor::Core;

const INDEX_HTML: &[u8] = include_bytes!("../index.html");

#[cfg(production)]
fn web_responder(req: Request<Body>) -> impl Future<Item = Response<Body>, Error = Error> {
    check_https_redirect(&req).or_else(|_| serve_static())
}

#[cfg(not(production))]
fn web_responder(_req: Request<Body>) -> impl Future<Item = Response<Body>, Error = Error> {
    serve_static()
}

fn serve_static() -> impl Future<Item = Response<Body>, Error = Error> {
    future::ok(
        Response::builder()
            .status(200)
            .header("Content-Type", "text/html")
            .body(Body::from(
                ::std::str::from_utf8(INDEX_HTML).expect("index.html has invalid UTF-8!"),
            ))
            .unwrap(),
    )
}

#[allow(dead_code)]
fn check_https_redirect(req: &Request<Body>) -> impl Future<Item = Response<Body>, Error = Error> {
    if req.headers().get("x-forwarded-proto") != Some(&HeaderValue::from_static("https")) {
        let redirect = format!(
            "https://{}{}",
            req.uri().host().unwrap_or_else(|| req
                .headers()
                .get(HOST)
                .and_then(|host| host.to_str().ok())
                .unwrap_or_default()),
            req.uri().path()
        );
        trace!("Redirecting: {:?} => {:?}", req.uri(), redirect);
        future::ok(
            Response::builder()
                .status(301)
                .header(LOCATION, redirect.as_str())
                .body(Body::empty())
                .unwrap(),
        )
    } else {
        future::err(Error::new(ErrorKind::Other, "no redirect"))
    }
}

pub fn run_server(core: &mut Core) -> Result<(), String> {
    let port = env::var("PORT")
        .expect("No PORT environment variable set.")
        .parse()
        .expect("Unable to parse value of PORT environment variable.");
    let addr = ([0, 0, 0, 0], port).into();
    let responder = || service_fn(web_responder);
    let server = Server::bind(&addr)
        .serve(responder)
        .map_err(|e| error!("Web server error: {}", e));
    core.execute(server)
        .map_err(|error| format!("Failed to start web server: {:?}", error))
}
