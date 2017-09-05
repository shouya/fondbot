use common::*;
use ext_stack::ExtensionStack;
use db::Db;

use std::collections::HashSet;
use std::cell::RefCell;
use std::env;

#[derive(Serialize, Deserialize)]
pub struct ContextState {
    pub safe_chats: HashSet<Integer>,
}

pub struct Context {
    pub bot: tg::Api,
    pub exts: RefCell<ExtensionStack>,
    pub db: Db
}

impl Default for ContextState {
    fn default() -> Self {
        ContextState { safe_chats: vec![] }
    }
}

impl Context {
    pub fn context_state(&self) -> ContextState {
        match self.db.load_conf("context-state") {
            None => Default::default(),
            Some(x) => x
        }
    }

    pub fn add_safe_chat(&self, chat_id: Integer) {
        let mut state = self.context_state();
        state.safe_chats.insert(chat_id);
        self.db.save_conf("context-state", state);
    }

    pub fn plug_ext<T>(&mut self)
        where T: BotExtension + 'static
    {
        self.exts.borrow_mut().plug(T::init(&self));
    }

    pub fn new(bot: tg::Api) -> Context {
        Context {
            bot: bot,
            exts: RefCell::new(ExtensionStack::new()),
            db: Db::init()
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
        let chat_id = msg.chat.id();
        self.context_state().safe_chats.contains(&chat_id)
    }

    pub fn process_message(&self, msg: &tg::Message) {
        let mut exts = self.exts.borrow_mut();
        if self.safety_guard(msg) {
            exts.process(msg, self);
        } else {
            self.bot.reply_to(msg,
                              "Unauthorized access. This incidence will be \
                               reported to administrator.");
            // TODO: Report event
        }
    }

    pub fn serve(&mut self) {
        let mut listener = {
            self.bot.listener(tg::ListeningMethod::LongPoll(None))
        };

        listener.listen(move |u| {
                debug!("Got msg: {:?}", u);
                if let Some(mut msg) = u.message {
                    msg.clean_cmd();
                    self.process_message(&msg);
                }
                Ok(tg::ListeningAction::Continue)
            })
            .unwrap();
    }
}
