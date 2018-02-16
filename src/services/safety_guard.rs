use common::*;

#[derive(Serialize, Deserialize, Default)]
pub struct SafetyGuard {
  pub safe_chats: HashSet<tg::ChatId>,
}

impl SafetyGuard {
  pub fn is_safe(&self, msg: &tg::Message) -> bool {
    self.safe_chats.contains(&msg.chat.id())
  }

  pub fn initialize_from_env(&mut self) {
    env::var("SAFE_CHATS")
      .unwrap_or("".into())
      .split(",")
      .map(|x| tg::ChatId::from(x.parse::<tg::Integer>().unwrap()))
      .for_each(|x| self.add_safe_chat(&x))
  }

  pub fn add_safe_chat(&mut self, id: &tg::ChatId) {
    self.safe_chats.insert(id.clone());
  }

  pub fn remove_safe_chat(&mut self, id: &tg::ChatId) {
    self.safe_chats.remove(id);
  }
}
