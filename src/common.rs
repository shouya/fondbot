#![macro_use]

pub extern crate telegram_bot;
pub extern crate erased_serde;
pub extern crate regex;

pub use serde_json;
pub use serde_json::Value as JsonValue;
pub use serde::de::Deserialize;
pub use serde::ser::Serialize;
pub use regex::Regex;
pub use chrono;

use std;
pub use std::fmt;
pub use std::fmt::Write;
pub use std::sync::mpsc::{Sender, Receiver};
pub use std::cell::{Cell, RefCell};
pub use std::collections::HashMap;

pub type Dict<T> = std::collections::BTreeMap<String, T>;

pub use bot::*;
pub use context::Context;
pub use self::telegram_bot as tg;
pub use self::tg::Listener;
pub use services::*;

lazy_static! {
    pub static ref GLOBAL_TIMEZONE: chrono::FixedOffset = chrono::FixedOffset::east(28800);
}

#[allow(unused_variables)]
pub trait BotExtension {
    fn init(ctx: &Context) -> Self where Self: Sized;

    fn process(&mut self, msg: &tg::Message, ctx: &Context);
    fn name(&self) -> &str;

    /// Report current status
    fn report(&self) -> String {
        self.name().into()
    }
}

pub fn escape_md(s: &str) -> String {
    s.replace("_", "\\_")
     .replace("[", "\\[")
     .replace("*", "\\*")
     .replace("]", "\\]")
     .replace("(", "\\)")
     .replace(")", "\\)")
}

// convert Result<T, E: Debug> to Result<T, String>
pub type Result<T> = std::result::Result<T, String>;
#[macro_export]
macro_rules! try_strerr {
  [ $maybe:expr ] => {
    try!($maybe.map_err(|e| format!("{:?}", e)))
  }
}
