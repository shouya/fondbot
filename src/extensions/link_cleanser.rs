use common::*;

use std::str::FromStr;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LinkCleanser;

impl LinkCleanser {
  fn parse_url(txt: &str) -> Option<Url> {
    Url::from_str(txt).ok()
  }

  fn primitive_filter(url: Url) -> Option<Url> {
    if url.scheme() != "http" && url.scheme() != "https" {
      return None;
    }

    if url.query().is_none() || url.host_str().is_none() {
      return None;
    }

    Some(url)
  }

  fn cleanse_url<'a>(mut url: Url) -> Option<Url> {
    if Self::domain_suffix_in(&url, &["tmall.com", "taobao.com"])
      && url.path() == "/item.htm"
    {
      return Self::whitelist_query(url, &["id"], true);
    }

    if Self::domain_suffix_in(&url, &["item.jd.com"]) {
      url.set_query(None);
      return Some(url);
    }

    if Self::domain_suffix_in(&url, &["youtube.com"]) && url.path() == "/watch"
    {
      return Self::whitelist_query(url, &["v"], true);
    }

    None
  }

  fn domain_suffix_in(url: &Url, suff: &[&str]) -> bool {
    let host = url.host_str().unwrap();
    suff.iter().any(|s| host.ends_with(s))
  }

  fn get_query(url: &Url, key: &str) -> Option<String> {
    for (k, v) in url.query_pairs().into_iter() {
      if k == key {
        return Some(v.into());
      }
    }

    None
  }

  fn whitelist_query<'a, 'b, 'c>(
    mut url: Url,
    keys: &'b [&'c str],
    req_all: bool,
  ) -> Option<Url> {
    let kv: Vec<(String, Option<String>)> = keys
      .iter()
      .map(|k| ((*k).into(), Self::get_query(&url, k)))
      .collect();
    if req_all && kv.iter().any(|(_, v)| v.is_none()) {
      return None;
    }

    {
      let mut query_pairs = url.query_pairs_mut();
      query_pairs.clear();

      for (k, v) in kv {
        if v.is_some() {
          query_pairs.append_pair(&k, v.unwrap().as_ref());
        }
      }
    }

    Some(url)
  }
}

impl BotExtension for LinkCleanser {
  fn init(_ctx: &Context) -> Self {
    Self
  }

  fn process(&mut self, msg: &tg::Message, ctx: &Context) {
    let text = match msg.text_content() {
      None => return,
      Some(t) => t,
    };

    Self::parse_url(&text)
      .and_then(Self::primitive_filter)
      .and_then(Self::cleanse_url)
      .map(|url| url.into_string())
      .and_then(|clean_url| {
        if clean_url == text {
          None
        } else {
          Some(clean_url)
        }
      })
      .map(|clean_url| ctx.bot.reply_to(msg, clean_url));
  }

  fn name(&self) -> &str {
    "link_cleanser"
  }
}
