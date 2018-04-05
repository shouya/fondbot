use common::*;

pub struct Context {
  pub bot: tg::Api,
  pub handle: reactor::Handle,
  pub bypass: Cell<bool>,
  pub exts: RefCell<Vec<Box<BotExtension>>>,
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
      bypass: Cell::new(false),
      exts: RefCell::new(vec![]),
      bot,
      handle,
      logger,
      db,
      guard,
      names,
    }
  }

  pub fn plug_ext<T: BotExtension + 'static>(&mut self) {
    let plugin = T::init(&self);
    info!(self.logger, "Loading plugin {}", plugin.name());
    self.exts.borrow_mut().push(Box::new(plugin));
  }

  pub fn serve_poll<'a>(
    &'a mut self,
  ) -> Box<Future<Item = (), Error = ()> + 'a> {
    Box::new(
      self
        .bot
        .stream()
        .for_each(move |update| {
          self.process_update(update);
          ok(())
        })
        .map_err(|_| ()),
    )
  }

  pub fn serve_webhook<'a>(
    &'a mut self,
    callback_url: &str,
    bind: &str,
  ) -> Box<Future<Item = (), Error = ()> + 'a> {
    let mut webhook = self.bot.webhook();
    webhook.register(callback_url);
    webhook.serve_at(
      bind
        .parse()
        .expect(&format!("invalid bind format {}", bind)),
    );
    Box::new(
      webhook
        .for_each(move |update| {
          self.process_update(update);
          ok(())
        })
        .map_err(|_| ()),
    )
  }

  pub fn process_callback(&mut self, query: &tg::CallbackQuery) {
    if !self.guard.is_safe(&query.message) {
      return;
    }

    info!(self.logger, "Got callback {:?}", query);
    if query.ext().is_none() {
      warn!(self.logger, "Unknown callback from nowhere");
      return;
    }

    let mut exts = self.exts.borrow_mut();
    let ext = exts
      .iter_mut()
      .find(|ext| query.ext().unwrap() == ext.name());

    if ext.is_none() {
      warn!(self.logger, "Cannot find ext: {}", query.ext().unwrap());
      return;
    }

    let ext = ext.unwrap();
    ext.process_callback(query, self);
  }

  pub fn process_message(&mut self, message: &tg::Message) {
    if !self.guard.is_safe(message) {
      self.prohibit_access(message);
      return;
    }

    info!(self.logger, "Got message {:?}", message);
    self.exts_process_message(message);
  }

  pub fn process_update(&mut self, update: tg::Update) {
    match update.kind {
      tg::UpdateKind::Message(message) => {
        self.process_message(&message);
      }
      tg::UpdateKind::CallbackQuery(query) => {
        self.process_callback(&query);
      }
      _ => {}
    }
  }

  fn exts_process_message(&self, msg: &tg::Message) {
    let mut exts = self.exts.borrow_mut();
    for ext in exts.iter_mut() {
      if self.bypass.get() {
        trace!(
          self.logger,
          "Not processing with plugin: {} (bypassed)",
          ext.name()
        );
      } else {
        trace!(self.logger, "Processing with plugin: {}", ext.name());
        ext.process(msg, self);
      }
    }
    self.bypass.set(false);
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
