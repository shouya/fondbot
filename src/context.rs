use common::*;
use ext_stack::ExtensionStack;

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;

pub struct ContextState {
    pub safe_chats: Vec<Integer>,
}

pub struct Context {
    pub bot: tg::Api,
    pub exts: RefCell<ExtensionStack>,
    pub state: RefCell<ContextState>,
    pub save_to: String,
}

impl Default for ContextState {
    fn default() -> Self {
        ContextState { safe_chats: vec![] }
    }
}

impl ContextState {
    pub fn save_state(&self) -> Dict<JsonValue> {
        let json: JsonValue = json!({
            "safe_chats": self.safe_chats.clone()
        });
        json.as_object().unwrap().iter().map(|(a, b)| (a.clone(), b.clone())).collect()
    }

    pub fn load_state(&mut self, map: &Dict<JsonValue>) {
        self.safe_chats = serde_json::from_value(map["safe_chats"].clone())
            .expect("error reading safe_chats");
    }
}

impl Context {
    pub fn new(bot: tg::Api, exts: ExtensionStack, loc: String) -> Context {
        Context {
            bot: bot,
            exts: RefCell::new(exts),
            state: RefCell::new(Default::default()),
            save_to: loc,
        }
    }

    pub fn save_state(&self) {
        let mut exts_val = self.exts.borrow().save();
        exts_val.append(&mut self.state.borrow().save_state());
        let mut file = File::create(Path::new(&self.save_to)).expect("Invalid state filename");
        if serde_json::ser::to_writer_pretty(&mut file, &exts_val).is_err() {
            warn!("error writing context state to file");
        }
    }

    pub fn safety_guard(&self, msg: &tg::Message) -> bool {
        let chat_id = msg.chat.id();
        self.state.borrow().safe_chats.contains(&chat_id)
    }

    pub fn load_state(&mut self) {
        File::open(Path::new(&self.save_to))
            .map_err(|e| e.to_string())
            .and_then(|f| serde_json::de::from_reader(f).map_err(|e| e.to_string()))
            .map(|map| {
                self.exts.borrow_mut().load(&map);
                self.state.borrow_mut().load_state(&map);
                info!("State loaded from json");
            })
            .map_err(|e| info!("Invalid state json: {}", e))
            .ok();
    }

    pub fn process_message(&self, msg: &tg::Message) {
        let mut exts = self.exts.borrow_mut();
        if self.safety_guard(msg) {
            exts.process(msg, self);
        } else {
            self.bot.reply_to(msg,
                              "Unauthorized access. This incidence will be reported to \
                               administrator.");
        }
    }

    pub fn serve(&mut self) {
        let mut listener = {
            self.bot.listener(tg::ListeningMethod::LongPoll(None))
        };

        listener.listen(move |u| {
                info!("Got msg: {:?}", u);
                if let Some(mut msg) = u.message {
                    msg.clean_cmd();
                    self.process_message(&msg);
                }
                info!("saving state");
                self.save_state();
                Ok(tg::ListeningAction::Continue)
            })
            .unwrap();
    }
}
