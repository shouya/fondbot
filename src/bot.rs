use common::*;
use std::borrow::Cow;

pub trait TgApiExt {
  // This method blocks the main thread
  fn consume_updates<'a>(
    &'a self,
  ) -> Box<Future<Item = Vec<tg::Update>, Error = Box<Error>> + 'a>;

  fn reply_to<'s, R, T>(&self, to: R, text: T)
  where
    R: tg::ToMessageId + tg::ToSourceChat,
    T: Into<Cow<'s, str>>;
  fn reply_md_to<'s, R, T>(&self, to: R, md_text: T)
  where
    R: tg::ToMessageId + tg::ToSourceChat,
    T: Into<Cow<'s, str>>;
}

impl TgApiExt for tg::Api {
  fn consume_updates<'a>(
    &'a self,
  ) -> Box<Future<Item = Vec<tg::Update>, Error = Box<Error>> + 'a> {
    use futures::future::{loop_fn, Loop};
    let init_state: (tg::Integer, Vec<tg::Update>) = (0, Vec::new());

    Box::new(
      loop_fn(init_state, move |(last, consumed)| {
        let req = tg::GetUpdates::new().offset(last).clone();
        self.send(req).and_then(|batch| {
          if batch.is_empty() {
            Ok(Loop::Break(consumed))
          } else {
            let mut new_consumed = consumed.clone();
            let new_last = batch.last().unwrap().id + 1;
            new_consumed.extend(batch.into_iter());
            Ok(Loop::Continue((new_last, new_consumed)))
          }
        })
      }).from_err(),
    )
  }

  fn reply_to<'s, R, T>(&self, to: R, text: T)
  where
    R: tg::ToMessageId + tg::ToSourceChat,
    T: Into<Cow<'s, str>>,
  {
    self.spawn(reply(to, text));
  }
  fn reply_md_to<'s, R, T>(&self, to: R, md_text: T)
  where
    R: tg::ToMessageId + tg::ToSourceChat,
    T: Into<Cow<'s, str>>,
  {
    self.spawn(reply(to, md_text).parse_mode(Markdown));
  }
}

pub trait TgMessageExt {
  fn clean_cmd(&mut self);
  fn is_cmd(&self, pat: &str) -> bool {
    let name = self.cmd_name();
    name.is_some() && name.unwrap().as_str() == pat
  }
  fn cmd_name(&self) -> Option<String>;
  fn cmd_arg(&self) -> Option<String>;
}

impl TgMessageExt for tg::Message {
  fn clean_cmd(&mut self) {
    lazy_static! {
      static ref RE: Regex = Regex::new(r"^(?P<cmd>/\w+)@\w+bot").unwrap();
    }
    if let tg::MessageKind::Text { ref mut data, .. } = self.kind {
      if !data.starts_with("/") {
        return;
      }

      *data = RE.replace(&data, "$cmd").into_owned();
    }
  }

  fn cmd_name(&self) -> Option<String> {
    lazy_static! {
      static ref RE: Regex = Regex::new(r"^/(?P<cmd>\w+)(@\w+bot)?").unwrap();
    }
    if let tg::MessageKind::Text { ref data, .. } = self.kind {
      RE.captures(data)
        .and_then(|cap| cap.name("cmd"))
        .map(|x| x.as_str().into())
    } else {
      None
    }
  }
  fn cmd_arg(&self) -> Option<String> {
    lazy_static! {
      static ref RE: Regex = Regex::new(r"^/(?P<cmd>\w+)(@\w+bot)?\s+?(?P<arg>.*)$").unwrap();
    }
    if let tg::MessageKind::Text { ref data, .. } = self.kind {
      RE.captures(data)
        .and_then(|cap| cap.name("arg"))
        .map(|x| x.as_str().into())
    } else {
      None
    }
  }
}

// Requests
pub fn reply<'s, R, T>(to: R, text: T) -> tg::SendMessage<'s>
where
  R: tg::ToMessageId + tg::ToSourceChat,
  T: Into<Cow<'s, str>>,
{
  tg::SendMessage::new(to.to_source_chat(), text)
    .reply_to(to)
    .clone()
}
