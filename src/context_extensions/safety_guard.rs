use crate::common::*;

#[derive(Serialize, Deserialize, Default)]
pub struct SafetyGuard {
  pub safe_chats: HashSet<tg::ChatId>,
}

impl ContextExtension for SafetyGuard {
  fn name() -> &'static str {
    "safety-guard"
  }
  fn new_from_env() -> Option<Self> {
    let mut ret: Self = Default::default();

    env::var("SAFE_CHATS").ok().map(|x| {
      let xs = x.split(',');
      xs.filter_map(|v| v.parse::<tg::Integer>().ok())
        .map(tg::ChatId::from)
        .for_each(|chat_id| ret.add_safe_chat(chat_id));
      ret
    })
  }
}

impl SafetyGuard {
  pub fn is_safe(&self, msg: &tg::Message) -> bool {
    self.safe_chats.contains(&msg.chat.id())
  }

  pub fn add_safe_chat(&mut self, id: tg::ChatId) {
    self.safe_chats.insert(id);
  }
}
