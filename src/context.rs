use common::*;
use ext_stack::ExtensionStack;
use db::Db;
use tokio_core;

use std::collections::HashSet;
use std::cell::{Cell, RefCell};
use std::env;

#[derive(Serialize, Deserialize, Default)]
pub struct ContextState {
    pub safe_chats: HashSet<Integer>,
}

pub struct Context {
    pub bot: tg::Api,
    pub exts: RefCell<ExtensionStack>,
    pub db: Db,
    pub bypass: Cell<bool>,
    pub tokio_core: tokio_core::reactor::Core,
    pub bot_user: tg::User,
}

impl Context {
    pub fn context_state(&self) -> ContextState {
        match self.db.load_conf("context-state") {
            None => Default::default(),
            Some(x) => x,
        }
    }

    pub fn add_safe_chat(&self, chat_id: Integer) {
        let mut state = self.context_state();
        state.safe_chats.insert(chat_id);
        self.db.save_conf("context-state", state);
    }

    pub fn plug_ext<T>(&mut self)
    where
        T: BotExtension + 'static,
    {
        let plugin = T::init(&self);
        info!("Loading plugin {}", plugin.name());
        self.exts.borrow_mut().plug(plugin);
    }

    pub fn set_bypass(&self) {
        self.bypass.set(true)
    }

    pub fn new(bot: tg::Api) -> Context {
        // let bot_user = bot.get_me();
        let core = tokio_core::reactor::Core::new().unwrap();
        Context {
            bot: bot,
            exts: RefCell::new(ExtensionStack::new()),
            db: Db::init(),
            bypass: Cell::new(false),
            tokio_core: core,
            bot_user: tg::User { id: tg::UserId::new(0), first_name: "".into(), last_name: None, username: None } //bot_user,,
        }
    }

    pub fn load_safe_chats_from_env(&self) {
        env::var("SAFE_CHATS")
            .unwrap_or("".into())
            .split(",")
            .flat_map(|x| x.parse::<Integer>())
            .for_each(|x| self.add_safe_chat(x))
    }

    pub fn safety_guard(&self, msg: &tg::Message) -> bool {
        let chat_id = msg.to_source_chat().into();
        self.context_state().safe_chats.contains(&chat_id)
    }

    pub fn process_message(&self, msg: &tg::Message) {
        if self.safety_guard(msg) {
            self.exts_process_message(msg);
        } else {
            self.bot.reply_to(
                &msg,
                "Unauthorized access. This incidence will be \
                               reported to administrator.",
            );
            // TODO: Report event
        }
    }

    pub fn exts_process_message(&self, msg: &tg::Message) {
        let mut exts = self.exts.borrow_mut();
        for ext in &mut exts.extensions {
            if self.bypass.get() {
                trace!("Not processing with plugin: {} (bypassed)", ext.name());
            } else {
                trace!("Processing with plugin: {}", ext.name());
                ext.process(msg, self);
            }
        }
        self.bypass.set(false);
    }

    pub fn serve(&mut self) {
        use futures::Stream;
        use tg::UpdateKind::Message;
        use tokio_core;
        let mut core = tokio_core::reactor::Core::new().unwrap();

        let future = self.bot.stream().for_each(|update| {
            debug!("Got update: {:?}", update);
            match update.kind {
                Message(msg) => {
                    let mut msg = msg.clone();
                    msg.clean_cmd();
                    if msg.is_processable() {
                        self.process_message(&msg);
                    }
                }
                _ => {}
            };
            Ok(())
        });

        core.run(future).unwrap();
    }
}
