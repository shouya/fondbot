pub mod afk;
// pub mod tracker;
pub mod weather;
// pub mod manager;
pub mod history;
pub mod music;
pub mod reminder;
pub mod yeelight;

use common::*;

pub trait BotExtension {
  fn init(ctx: &Context) -> Self
  where
    Self: Sized;

  fn process(&mut self, _message: &tg::Message, _ctx: &Context) {}
  fn process_callback(&mut self, _query: &tg::CallbackQuery, _ctx: &Context) {}
  fn name(&self) -> &str;

  fn callback_button(&self, text: &str, key: &str) -> tg::InlineKeyboardButton {
    let callback = format!("{}.{}", self.name(), key);
    tg::InlineKeyboardButton::callback(text, callback)
  }

  /// Report current status
  fn report(&self) -> String {
    self.name().into()
  }
}

pub trait InteractiveBuilder {
  type Target;
  type Prompt = &'static str;

  fn build(&self) -> Option<Self::Target>;
  fn prompt(
    &self,
    prompt: Self::Prompt,
    msg: Option<&tg::Message>,
    ctx: &Context,
  );

  fn on_message(&mut self, _msg: &tg::Message, _ctx: &Context) {}
  fn on_callback(&mut self, _query: &tg::CallbackQuery, _ctx: &Context) {}

  fn ready(&self) -> bool {
    self.build().is_some()
  }
}

#[derive(Fail, Debug)]
pub enum ExtensionError {
  #[fail(display = "Error in `music`: {}", _0)]
  Music(#[cause] music::MusicError),
}
