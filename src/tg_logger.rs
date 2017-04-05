extern crate slog_extra;

use telegram_bot;
use telegram_bot::Api;
use slog::{Drain, Record, OwnedKeyValueList};
use self::slog_extra::Async;

pub struct TgDrain {
    api: Api,
    chat_id: i64,
}

impl Drain for TgDrain {
    type Error = ();
    fn log(&self,
           info: &Record,
           options: &OwnedKeyValueList)
           -> Result<(), ()> {
        let text = format_log(info, options).clone();
        self.send_message(text);
        Ok(())
    }
}


impl TgDrain {
    pub fn new(api_token: &str, chat_id: i64) -> Async {
        let api = telegram_bot::Api::from_token(api_token.into()).unwrap();
        Async::new(TgDrain {
            api: api,
            chat_id: chat_id,
        })
    }

    pub fn send_message(&self, text: String) {
        let chat_id = self.chat_id;
        self.api
            .send_message(chat_id,
                          text,
                          Some(telegram_bot::ParseMode::Html),
                          Some(true),
                          None,
                          None)
            .ok();
    }
}

fn format_log(info: &Record, _: &OwnedKeyValueList) -> String {
    format!("<code>[{}]{}{}:</code> {}",
            info.level().as_short_str(),
            info.module(),
            info.function(),
            info.msg())
}
