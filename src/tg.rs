pub extern crate telegram_bot;

pub use self::telegram_bot::*;
pub use self::telegram_bot::prelude::*;
pub use self::telegram_bot::types::*;

pub mod req {
    pub use super::telegram_bot::types::requests::*;
}

pub mod traits {
    pub use super::telegram_bot::types::{ToChatRef, ToMessageId, ToSourceChat};
}
