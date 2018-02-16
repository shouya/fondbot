use common::*;

pub struct Context {
  pub bot: tg::Api,
  pub handle: reactor::Handle,
  pub bypass: Cell<bool>,
  pub extension_stack: Vec<Box<BotExtension>>,
  pub logger: Logger,
  pub guard: SafetyGuard,
  pub names: NameMap,
  pub db: Db,
}

impl Context {
  pub fn new(bot: tg::Api, handle: reactor::Handle, logger: Logger) -> Context {
    use ContextExtension;
    let db = Db::init();

    let guard = SafetyGuard::new(&db);
    let names = NameMap::new(&db);

    Context {
      bot: bot,
      handle: handle,
      bypass: Cell::new(false),
      extension_stack: vec![],
      logger: logger,
      db: db,
      guard: guard,
      names: names
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
    if !self.guard.is_safe(message) {
      self.prohibit_access(message);
      return;
    }
    info!(self.logger, "Got message {:?}", message);
  }

  pub fn prohibit_access(&self, msg: &tg::Message) {
    let warning =
      "You're not permitted to use this bot. This indicidence will be reported";
    let req = tg::SendMessage::new(&msg.chat, warning)
      .reply_to(&msg)
      .clone();
    self.bot.spawn(req);

    warn!(self.logger, "Prohibited access: {:?}", msg)
  }

  pub fn set_bypass(&self) {
    self.bypass.replace(true);
  }
}
