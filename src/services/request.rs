use common::*;

use serde::de::DeserializeOwned;
use serde_json;
use serde_json::de;

use hyper;
use hyper::{Body, Client, Request, Uri};
use hyper_rustls::HttpsConnector;

use std::str::FromStr;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X \
                          10_12_1) AppleWebKit/537.36 (KHTML, like \
                          Gecko) Chrome/54.0.2840.98 Safari/537.36";

#[derive(Fail, Debug)]
pub enum RequestError {
  #[fail(display = "Hyper error: {}", _0)]
  Hyper(hyper::Error),
  #[fail(display = "Failed decoding json: {}", _0)]
  Json(serde_json::Error),
  #[fail(display = "Failed requesting resource: {}", _0)]
  Failed(hyper::Uri),
}

impl From<hyper::Error> for RequestError {
  fn from(e: hyper::Error) -> RequestError {
    RequestError::Hyper(e)
  }
}

pub fn request<T: DeserializeOwned + 'static>(
  uri: &str,
) -> impl Future<Item = T, Error = RequestError> {
  let uri = Uri::from_str(uri).unwrap();
  let req = Request::get(uri.clone())
    .header("User-Agent", USER_AGENT)
    .body(Body::empty())
    .unwrap();

  let request = match uri.scheme_str().unwrap_or("http") {
    "http" => Client::new().request(req).from_err(),
    "https" => Client::builder()
      .build(HttpsConnector::new(4))
      .request(req)
      .from_err(),
    _ => panic!("Invalid url scheme"),
  };

  request
    .and_then(move |response| {
      if !response.status().is_success() {
        err(RequestError::Failed(uri))
      } else {
        ok(response)
      }
    })
    .and_then(|response| response.into_body().concat2().from_err())
    .map(|chunk| {
      let v = chunk.to_vec();
      String::from_utf8_lossy(&v).to_string()
    })
    .and_then(|text| {
      de::from_str(text.as_str()).map_err(|e| RequestError::Json(e))
    })
}
