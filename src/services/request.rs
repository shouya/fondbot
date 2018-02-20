extern crate hyper;
extern crate hyper_tls;

use common::*;

use serde::de::DeserializeOwned;
use serde_json::de;

use self::hyper::{header, Client, Method, Request, Uri};
use self::hyper_tls::HttpsConnector;

use std::str::FromStr;
use std::error::Error;
use std::fmt;

const USER_AGENT: &'static str = "Mozilla/5.0 (Macintosh; Intel Mac OS X \
                                  10_12_1) AppleWebKit/537.36 (KHTML, like \
                                  Gecko) Chrome/54.0.2840.98 Safari/537.36";

#[derive(Debug)]
enum RequestError {
  FailedRequest { uri: Uri },
  InvalidResponse(Box<Error>),
  HyperError(hyper::Error),
}

impl From<hyper::Error> for RequestError {
  fn from(e: hyper::Error) -> RequestError {
    RequestError::HyperError(e)
  }
}

impl Display for RequestError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      &RequestError::FailedRequest { ref uri } => {
        write!(f, "Failed requesting for {}", uri)
      }
      &RequestError::InvalidResponse(ref e) => fmt::Display::fmt(e, f),
      &RequestError::HyperError(ref e) => fmt::Display::fmt(e, f),
    }
  }
}

impl Error for RequestError {
  fn description(&self) -> &str {
    match self {
      &RequestError::FailedRequest { .. } => "Failed request",
      &RequestError::InvalidResponse(ref e) => e.description(),
      &RequestError::HyperError(ref e) => e.description(),
    }
  }
  fn cause(&self) -> Option<&Error> {
    match self {
      &RequestError::FailedRequest { .. } => None,
      &RequestError::InvalidResponse(ref e) => Some(e.as_ref()),
      &RequestError::HyperError(ref e) => Some(e),
    }
  }
}

pub fn request<T: DeserializeOwned + 'static>(
  handle: &reactor::Handle,
  uri: &str,
) -> Box<Future<Item = T, Error = Box<Error>>> {
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

  let response = request
    .and_then(move |response| {
      if !response.status().is_success() {
        err(RequestError::FailedRequest { uri })
      } else {
        ok(response)
      }
    })
    .and_then(|response| response.body().concat2().from_err())
    .map(|chunk| {
      let v = chunk.to_vec();
      String::from_utf8_lossy(&v).to_string()
    })
    .and_then(|text| {
      de::from_str(text.as_str())
        .map_err(|x| RequestError::InvalidResponse(Box::new(x)))
    })
    .from_err();

  Box::new(response)
}
