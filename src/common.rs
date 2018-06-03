pub use std::borrow::{Borrow, BorrowMut};
pub use std::cell::{Cell, RefCell};
pub use std::collections::{HashMap, HashSet};
pub use std::env;
pub use std::fmt::Write;
pub use std::fmt::{Debug, Display};
pub use std::ops::{Deref, DerefMut, Drop};

pub use telegram_bot as tg;
pub use tg::prelude::*;
pub use tg::ParseMode::{Html, Markdown};

pub use tokio_core::reactor;

pub use futures;
pub use futures::future::{err, ok, FutureResult};
pub use futures::Future;
pub use futures::Stream;

pub use serde::de::{Deserialize, DeserializeOwned};
pub use serde::ser::Serialize;

pub use slog;
pub use slog::Logger;

pub use regex::Regex;

pub use bot::{reply, TgApiExt, TgCallbackQueryExt, TgMessageExt};

pub use context::Context;
pub use context_extensions::ContextExtension;
pub use db::Db;
pub use extensions::{BotExtension, ExtensionError, InteractiveBuilder};
pub use services::request::request;
pub use services::request::RequestError;

pub use url::Url;

pub use context_extensions::name_map::NameMap;
pub use context_extensions::safety_guard::SafetyGuard;

pub use util::{
  ellipsis, escape_markdown, format_duration, format_human_time, format_time,
};

pub use chrono;
pub use chrono::{Date, DateTime, Duration, Local, TimeZone};

pub use errors::{FondbotError, Result};
pub use failure::SyncFailure;

// lazy_static! {
//   pub static ref GLOBAL_TIMEZONE: chrono::FixedOffset = chrono::FixedOffset::east(28800);
// }
