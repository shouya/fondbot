use common::*;

#[derive(Serialize, Deserialize, Default)]
pub struct NameMap {
  pub names: HashMap<tg::UserId, String>,
}

impl ContextExtension for NameMap {
  fn name() -> &'static str {
    "name-map"
  }
  fn new_from_env() -> Option<Self> {
    let mut ret: Self = Default::default();
    let env_value = env::var("NAME_MAP").unwrap_or("".into());
    let assocs = env_value.split(",");
    for pair in assocs {
      let mut pair = pair.split("->");
      let user_id: tg::UserId =
        pair.nth(0)?.parse::<tg::Integer>().ok()?.into();
      let user_name = pair.nth(1).expect("Invalid name map");
      ret.add_name_map(user_id, user_name);
    }
    Some(ret)
  }
}

pub trait ToIdOrUser {
  fn to_user_id(&self) -> &tg::UserId;
  fn to_user(&self) -> Option<&tg::User> {
    None
  }
}

impl ToIdOrUser for tg::UserId {
  fn to_user_id(&self) -> &tg::UserId {
    self
  }
}

impl ToIdOrUser for tg::User {
  fn to_user_id(&self) -> &tg::UserId {
    &self.id
  }
  fn to_user(&self) -> Option<&tg::User> {
    Some(self)
  }
}

impl NameMap {
  pub fn get<T: ToIdOrUser>(&self, u: &T) -> String {
    let id = u.to_user_id();
    if let Some(name) = self.names.get(id) {
      return name.clone();
    }

    let user = u.to_user();
    if let Some(user) = user {
      return user.first_name.clone();
    }

    "".into()
  }

  pub fn add_name_map(&mut self, id: tg::UserId, name: &str) {
    self.names.insert(id, name.into());
  }
}
