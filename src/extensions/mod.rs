pub mod afk;
// pub mod tracker;
// pub mod weather;
// pub mod manager;
pub mod history;

use common::*;

pub trait BotExtension {
  fn init(ctx: &Context) -> Self
  where
    Self: Sized;

  fn process(&mut self, _message: &tg::Message, _ctx: &Context) {}
  fn process_callback(&mut self, _query: &tg::CallbackQuery, _ctx: &Context) {}
  fn name(&self) -> &str;

  /// Report current status
  fn report(&self) -> String {
    self.name().into()
  }
}

