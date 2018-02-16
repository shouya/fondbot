pub mod safety_guard;
pub mod name_map;

use common::*;

pub trait ContextExtension
where
  Self: Default + Serialize + DeserializeOwned,
{
  fn name() -> &'static str;
  fn new_from_env() -> Option<Self> {
    None
  }

  fn new_from_db(db: &Db) -> Option<Self> {
    let key = format!("exts.{}", Self::name());
    db.load_conf(&key)
  }

  fn save(&self, db: &Db) {
    let key = format!("exts.{}", Self::name());
    db.save_conf(&key, self)
  }

  fn new(db: &Db) -> Self {
    Self::new_from_db(db)
      .or_else(|| Self::new_from_env())
      .unwrap_or_default()
  }
}
