pub use std::env;
pub use std::cell::{Cell, RefCell};
pub use std::error::Error;
pub use std::collections::{HashMap, HashSet};
pub use std::fmt::{Debug, Display};
pub use std::fmt::Write;

pub use telegram_bot as tg;
pub use tg::ParseMode::{Html, Markdown};
pub use tg::{CanEditMessageReplyMarkup, CanEditMessageText,
             CanReplySendMessage, CanSendChatAction, CanSendMessage};

pub use tokio_core::reactor;

pub use futures;
pub use futures::future::{err, ok, FutureResult};
pub use futures::Future;
pub use futures::Stream;

pub use serde::ser::Serialize;
pub use serde::de::{Deserialize, DeserializeOwned};

pub use slog;
pub use slog::Logger;

pub use regex::Regex;

pub use bot::{reply, TgApiExt, TgCallbackQueryExt, TgMessageExt};

pub use context::Context;
pub use extensions::BotExtension;
pub use context_extensions::ContextExtension;
pub use db::Db;
pub use services::request::request;

pub use context_extensions::name_map::NameMap;
pub use context_extensions::safety_guard::SafetyGuard;

pub use util::{ellipsis, escape_markdown};

// pub use chrono;

// lazy_static! {
//   pub static ref GLOBAL_TIMEZONE: chrono::FixedOffset = chrono::FixedOffset::east(28800);
// }
