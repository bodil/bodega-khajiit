use futures::future::{Executor, Future};
use hyper::service::service_fn_ok;
use hyper::{Body, Request, Response, Server};
use std::env;
use tokio_core::reactor::Core;

const INDEX_HTML: &[u8] = include_bytes!("../index.html");

#[allow(unknown_lints)]
#[allow(needless_pass_by_value)]
fn web_responder(_req: Request<Body>) -> Response<Body> {
    Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(
            ::std::str::from_utf8(INDEX_HTML).expect("index.html has invalid UTF-8!"),
        )).unwrap()
}

pub fn run_server(core: &mut Core) -> Result<(), String> {
    let port = env::var("PORT")
        .expect("No PORT environment variable set.")
        .parse()
        .expect("Unable to parse value of PORT environment variable.");
    let addr = ([0, 0, 0, 0], port).into();
    let responder = || service_fn_ok(web_responder);
    let server = Server::bind(&addr)
        .serve(responder)
        .map_err(|e| error!("Web server error: {}", e));
    core.execute(server)
        .map_err(|error| format!("Failed to start web server: {:?}", error))
}
