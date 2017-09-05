/// Request Library

extern crate hyper;

use common::*;

use serde::de::DeserializeOwned;
use serde_json::de::from_reader;

use self::hyper::Client;
use self::hyper::client::IntoUrl;
use self::hyper::header::UserAgent;

const USER_AGENT: &'static str = "Mozilla/5.0 (Macintosh; Intel Mac OS X \
                                  10_12_1) AppleWebKit/537.36 (KHTML, like \
                                  Gecko) Chrome/54.0.2840.98 Safari/537.36";

pub fn request<URL: IntoUrl, T: DeserializeOwned>(url: URL) -> Result<T> {
    let url = url.into_url().unwrap();
    let resp = try_strerr!(Client::new()
        .get(url.clone())
        .header(UserAgent(USER_AGENT.into()))
        .send());

    if !resp.status.is_success() {
        return Err(format!("Failed requesting url: {}", url.as_str()));
    }

    Ok(try_strerr!(from_reader(resp)))
}
