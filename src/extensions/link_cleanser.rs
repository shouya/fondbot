use crate::common::*;

use std::str::FromStr;
use url::Url;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LinkCleanser;

impl LinkCleanser {
  fn parse_url(txt: &str) -> Option<Url> {
    lazy_static! {
      static ref URL_REGEX: Regex = Regex::new(r"(http|https)://[\w-]+(\.[\w-]+)+([\w.,@?^=%&amp;:/~+#-]*[\w@?^=%&/~+#-])?").unwrap();
    }

    URL_REGEX
      .find(txt)
      .and_then(|m| Url::from_str(m.as_str()).ok())
  }

  fn primitive_filter(url: Url) -> Option<Url> {
    if url.scheme() != "http" && url.scheme() != "https" {
      return None;
    }

    if url.host_str().is_none() {
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

    if Self::domain_suffix_in(&url, &["intl.taobao.com", "intl.m.taobao.com"])
      && url.path() == "/detail/detail.html"
    {
      url.set_host(Some("item.taobao.com")).ok()?;
      url.set_path("/item.htm");
      return Self::whitelist_query(url, &["id"], true);
    }

    if Self::domain_suffix_in(&url, &["item.jd.com", "item.m.jd.com"]) {
      url.set_host(Some("item.jd.com")).ok()?;
      url.set_query(None);
      let path = url.path().replace("/product/", "/");
      url.set_path(&path);
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

  fn text_pipeline(text: &str) -> Option<String> {
    Self::parse_url(text)
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
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn test_pipeline() {
    assert_cleanse(
      "https://item.taobao.com/item.htm?spm=b130k.1.12.81.991c63ccnNGlO9&id=574189924259&ns=1&abbucket=8#detail",
      "https://item.taobao.com/item.htm?id=574189924259#detail"
    );

    assert_cleanse(
      "https://item.m.jd.com/product/4385461.html?&utm_source=iosapp&utm_medium=appshare&utm_campaign=t_3139j9774&utm_term=CopyURL",
      "https://item.jd.com/4385461.html"
    );

    assert_cleanse(
      "https://item.m.jd.com/product/4385461.html",
      "https://item.jd.com/4385461.html",
    );

    assert_cleanse(
      "https://m.intl.taobao.com/detail/detail.html?id=3933748539",
      "https://item.taobao.com/item.htm?id=3933748539",
    );

    assert_cleanse(
      "aaa bbb https://m.intl.taobao.com/detail/detail.html?id=3933748539\nccc",
      "https://item.taobao.com/item.htm?id=3933748539",
    );
  }

  fn assert_cleanse(input: &str, output: &str) {
    assert_eq!(LinkCleanser::text_pipeline(input), Some(output.into()))
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

    Self::text_pipeline(&text)
      .map(|clean_url| ctx.bot.reply_to(msg, clean_url));
  }

  fn name(&self) -> &str {
    "link_cleanser"
  }
}
