pub extern crate telegram_bot;
pub extern crate serde;
pub extern crate serde_json;
pub extern crate erased_serde;

pub use serde_json::Value as JsonValue;
pub use serde::de::Deserialize;
pub use serde::ser::Serialize;

pub use bot::Bot;
pub use context::Context;
pub use self::telegram_bot as tg;
pub use self::tg::Listener;

pub trait BotExtension {
  fn new() -> Self where Self: Sized;

  fn should_process(&self,     msg: &tg::Message, ctx: &Context) -> bool;
  fn process       (&mut self, msg: &tg::Message, ctx: &Context);
  /// Report current status
  fn report(&self) -> String;
  fn name(&self)   -> &str;

  fn save(&self) -> JsonValue;
  fn load(&mut self, JsonValue);
}

pub fn msg_txt(msg: &tg::Message) -> Option<String> {
  if let tg::MessageType::Text(ref txt) = msg.msg {
    Some(txt.clone().into())
  } else {
    None
  }
}

pub fn is_cmd(msg: &tg::Message, prefix: &str) -> bool {
  if let Some(txt) = msg_txt(msg) {
    txt.eq(&format!("/{}", prefix)) || txt.starts_with(&format!("/{} ", prefix))
  } else {
    false
  }
}

pub fn cmd_arg(msg: &tg::Message, prefix: &str) -> Option<String> {
  if !is_cmd(msg, prefix) {
    None
  } else {
    let txt = msg_txt(msg).unwrap();
    if prefix.len() + 2 >= txt.len() { return None }

    let (_,b) = txt.split_at(prefix.len() + 2);
    Some(b.to_string())
  }
}

pub fn user_name(user: &tg::User) -> String {
  let user = user.clone();
  let add_space = |x: String| " ".to_string() + &x;
  let last_name = user.last_name.map_or("".into(), add_space);
  let formal_name = user.first_name + &last_name;

  user.username.unwrap_or(formal_name)
}

pub fn eat_updates(bot: &Bot) {
  while let Ok(vec) = bot.api.get_updates(None, None, None) {
    if vec.len() == 0 { return }
  }
}
