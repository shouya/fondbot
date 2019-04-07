pub use std::borrow::{Borrow, BorrowMut};
pub use std::cell::{Cell, RefCell};
pub use std::collections::{HashMap, HashSet};
pub use std::env;
pub use std::fmt::Write;
pub use std::fmt::{Debug, Display};
pub use std::ops::{Deref, DerefMut, Drop};

pub use telegram_bot as tg;
pub use crate::tg::prelude::*;
pub use crate::tg::ParseMode::{Html, Markdown};

pub use tokio_core::reactor;

pub use futures;
pub use futures::future;
pub use futures::future::{err, ok, FutureResult};
pub use futures::future::{Future, IntoFuture};
pub use futures::stream::Stream;

pub use serde::de::{Deserialize, DeserializeOwned};
pub use serde::ser::Serialize;

pub use slog;
pub use slog::Logger;

pub use regex::{Regex, RegexSet};

pub use crate::bot::{reply, TgApiExt, TgCallbackQueryExt, TgMessageExt};

pub use crate::context::Context;
pub use crate::context_extensions::ContextExtension;
pub use crate::db::Db;
pub use crate::extensions::{BotExtension, ExtensionError, InteractiveBuilder};
pub use crate::services::request::request;
pub use crate::services::request::RequestError;

pub use url::Url;

pub use crate::context_extensions::name_map::NameMap;
pub use crate::context_extensions::safety_guard::SafetyGuard;

pub use crate::util::{
  ellipsis, escape_markdown, format_duration, format_human_time, format_time,
};

pub use chrono;
pub use chrono::{Date, DateTime, Duration, Local, TimeZone};

pub use crate::errors::{FondbotError, Result};
pub use failure::{Fail, SyncFailure};

// lazy_static! {
//   pub static ref GLOBAL_TIMEZONE: chrono::FixedOffset = chrono::FixedOffset::east(28800);
// }
