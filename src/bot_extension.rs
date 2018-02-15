use common::*;

pub trait BotExtension {
  fn init(ctx: &Context) -> Self
  where
    Self: Sized;

  fn process(&mut self, msg: &tg::Message, ctx: &Context) {}
  fn process_callback(&mut self, callback: &tg::CallbackQuery, ctx: &Context) {}
  fn name(&self) -> &str;

  /// Report current status
  fn report(&self) -> String {
    self.name().into()
  }
}
