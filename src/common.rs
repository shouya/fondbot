#![macro_use]

pub extern crate telegram_bot;
pub extern crate serde;
pub extern crate serde_json;
pub extern crate erased_serde;

pub use serde_json::Value as JsonValue;
pub use serde::de::Deserialize;
pub use serde::ser::Serialize;

use std;
pub use std::sync::mpsc::{Sender, Receiver};
pub use std::cell::{Cell, RefCell};

pub type Dict<T> = std::collections::BTreeMap<String, T>;

pub use bot::*;
pub use context::Context;
pub use self::telegram_bot as tg;
pub use self::tg::Listener;
pub use services::*;
pub use utils::*;

#[allow(unused_variables)]
pub trait BotExtension {
    fn new() -> Self where Self: Sized;

    fn should_process(&self, msg: &tg::Message, ctx: &Context) -> bool {
        false
    }
    fn process(&mut self, msg: &tg::Message, ctx: &Context);
    /// Report current status
    fn name(&self) -> &str;
    fn report(&self) -> String {
        self.name().into()
    }

    fn save(&self) -> JsonValue {
        JsonValue::Null
    }
    fn load(&mut self, JsonValue) {}
}

// convert Result<T, E: Debug> to Result<T, String>
pub type Result<T> = std::result::Result<T, String>;
#[macro_export]
macro_rules! try_strerr {
  [ $maybe:expr ] => {
    try!($maybe.map_err(|e| format!("{:?}", e)))
  }
}
