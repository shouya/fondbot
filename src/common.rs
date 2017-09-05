#![macro_use]

pub extern crate telegram_bot;
pub extern crate erased_serde;
pub extern crate regex;

pub use serde_json;
pub use serde_json::Value as JsonValue;
pub use serde::de::Deserialize;
pub use serde::ser::Serialize;
pub use regex::Regex;

use std;
pub use std::fmt;
pub use std::sync::mpsc::{Sender, Receiver};
pub use std::cell::{Cell, RefCell};

pub type Dict<T> = std::collections::BTreeMap<String, T>;

pub use bot::*;
pub use context::Context;
pub use self::telegram_bot as tg;
pub use self::tg::Listener;
pub use services::*;

#[allow(unused_variables)]
pub trait BotExtension {
    fn init(ctx: &Context) -> Self where Self: Sized;

    fn should_process(&self, msg: &tg::Message, ctx: &Context) -> bool {
        false
    }
    fn process(&mut self, msg: &tg::Message, ctx: &Context);
    fn name(&self) -> &str;

    /// Report current status
    fn report(&self) -> String {
        self.name().into()
    }
}

// convert Result<T, E: Debug> to Result<T, String>
pub type Result<T> = std::result::Result<T, String>;
#[macro_export]
macro_rules! try_strerr {
  [ $maybe:expr ] => {
    try!($maybe.map_err(|e| format!("{:?}", e)))
  }
}
