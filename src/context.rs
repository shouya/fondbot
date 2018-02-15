use common::*;

pub struct Context {
  pub bot: tg::Api,
  pub handle: reactor::Handle,
  pub bypass: Cell<bool>,
  pub logger: Logger
}

impl Context {
  pub fn new(
    bot: tg::Api,
    handle: reactor::Handle,
    logger: Logger
  ) -> Context {
    Context {
      bot: bot,
      handle: handle,
      bypass: Cell::new(false),
      logger: logger
    }
  }

  pub fn serve<'a>(&'a mut self) -> impl Future<Item = (), Error = ()> + 'a {
    self.bot.stream().for_each(move |update| {
      match update.kind {
        tg::UpdateKind::Message(message) => {
          self.process_message(&message);
        },
        _ => {}
      };
      Ok(())
    }).map_err(|_| ())
  }

  pub fn process_message(&mut self, message: &tg::Message) {
    info!(self.logger, "Got message {:?}", message);
  }
}
