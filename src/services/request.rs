use common::*;

use serde::de::DeserializeOwned;
use serde_json::de;

use hyper::Error as HyperError;
use hyper::{header, Client, Method, Request, Uri};
use hyper_tls::HttpsConnector;

use std::str::FromStr;

const USER_AGENT: &'static str = "Mozilla/5.0 (Macintosh; Intel Mac OS X \
                                  10_12_1) AppleWebKit/537.36 (KHTML, like \
                                  Gecko) Chrome/54.0.2840.98 Safari/537.36";

error_chain! {
  foreign_links {
    Hyper(HyperError);
  }

  errors {
    RequestError(uri: Uri) {
        description("request error")
        display("{} responded with a non-200 code", uri)
    }
  }
}

pub fn request<T: DeserializeOwned + 'static>(
  handle: &reactor::Handle,
  uri: &str,
) -> impl Future<Item = T, Error = Error> {
  let uri = Uri::from_str(uri).unwrap();
  let mut req = Request::new(Method::Get, uri.clone());
  req.headers_mut().set(header::UserAgent::new(USER_AGENT));

  let request = match uri.scheme().unwrap_or("http") {
    "http" => Client::new(handle).request(req).from_err(),
    "https" => Client::configure()
      .connector(HttpsConnector::new(4, handle).unwrap())
      .build(handle)
      .request(req)
      .from_err(),
    _ => panic!("Invalid url scheme"),
  };

  request
    .and_then(move |response| {
      if !response.status().is_success() {
        err(ErrorKind::RequestError(uri).into())
      } else {
        ok(response)
      }
    })
    .and_then(|response| response.body().concat2().from_err())
    .map(|chunk| {
      let v = chunk.to_vec();
      String::from_utf8_lossy(&v).to_string()
    })
    .and_then(|text| de::from_str(text.as_str()).map_err(From::from))
    .from_err()
}
