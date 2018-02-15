use common::*;

pub struct Context {
  pub bot: tg::Api,
  pub handle: reactor::Handle,
  pub bypass: Cell<bool>,
  pub extension_stack: Vec<Box<BotExtension>>,
  pub logger: Logger,
}

impl Context {
  pub fn new(bot: tg::Api, handle: reactor::Handle, logger: Logger) -> Context {
    Context {
      bot: bot,
      handle: handle,
      bypass: Cell::new(false),
      extension_stack: vec![],
      logger: logger,
    }
  }

  pub fn plug_ext<T: BotExtension + 'static>(&mut self) {
    let plugin = T::init(&self);
    info!(self.logger, "Loading plugin {}", plugin.name());
    self.extension_stack.push(Box::new(plugin));
  }

  pub fn serve<'a>(&'a mut self) -> impl Future<Item = (), Error = ()> + 'a {
    self
      .bot
      .stream()
      .for_each(move |update| {
        match update.kind {
          tg::UpdateKind::Message(message) => {
            self.process_message(&message);
          }
          _ => {}
        };
        Ok(())
      })
      .map_err(|_| ())
  }

  pub fn process_message(&mut self, message: &tg::Message) {
    info!(self.logger, "Got message {:?}", message);
  }
}
