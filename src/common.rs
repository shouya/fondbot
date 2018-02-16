pub use std::env;
pub use std::cell::Cell;
pub use std::error::Error;

pub use telegram_bot as tg;

pub use tokio_core::reactor;

pub use futures;
pub use futures::future::{ok, err, FutureResult};
pub use futures::Future;
pub use futures::Stream;

pub use slog;
pub use slog::Logger;

pub use bot::TgApiExt;

pub use context::Context;
pub use bot_extension::BotExtension;
pub use db::Db;

// pub use chrono;

// lazy_static! {
//   pub static ref GLOBAL_TIMEZONE: chrono::FixedOffset = chrono::FixedOffset::east(28800);
// }

