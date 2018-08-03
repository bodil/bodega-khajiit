use futures::future::Future;
use hyper_rustls;
use tokio_core::reactor::Core;

pub fn load_url(core: &mut Core, url: &str) -> Result<Vec<u8>, String> {
    use hyper::{rt::Stream, Body, Client, Uri};

    trace!("Loading image from URL: {}", url);

    let uri = match url.parse::<Uri>() {
        Ok(uri) => uri,
        Err(error) => return Err(format!("Failed to parse URL {:?}: {:?}", url, error)),
    };

    let https = hyper_rustls::HttpsConnector::new(4);
    let client: Client<_, Body> = Client::builder().build(https);
    match core.run(client.get(uri)) {
        Ok(res) => {
            if !res.status().is_success() {
                return Err(format!("URL {:?} gave status code {:?}", url, res.status()));
            }
            let body = res.into_body();
            match body.concat2().wait() {
                Ok(data) => Ok(data.to_vec()),
                Err(error) => Err(format!("Failed to read body of URL {:?}: {:?}", url, error)),
            }
        }
        Err(error) => Err(format!("Failed to read URL {:?}: {:?}", url, error)),
    }
}
