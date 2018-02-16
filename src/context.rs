use common::*;
use services::safety_guard::SafetyGuard;

pub struct Context {
  pub bot: tg::Api,
  pub handle: reactor::Handle,
  pub bypass: Cell<bool>,
  pub extension_stack: Vec<Box<BotExtension>>,
  pub logger: Logger,
  pub state: RefCell<ContextState>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ContextState {
  pub guard: SafetyGuard,
}

impl Context {
  pub fn new(bot: tg::Api, handle: reactor::Handle, logger: Logger) -> Context {
    let mut state: ContextState = Default::default();
    state.guard.initialize_from_env();

    Context {
      bot: bot,
      handle: handle,
      bypass: Cell::new(false),
      extension_stack: vec![],
      logger: logger,
      state: RefCell::new(state),
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
    if !self.state.borrow().guard.is_safe(message) {
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
}
