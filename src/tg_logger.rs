
use telegram_bot;
use slog::{Drain, Record, OwnedKVList};
use slog::Never;

pub struct TgDrain {
    token: String,
    chat_id: i64,
}

impl Drain for TgDrain {
    type Ok = ();
    type Err = Never;
    fn log(&self,
           info: &Record,
           options: &OwnedKVList)
           -> Result<Self::Ok, Self::Err> {
        let text = format_log(info, options).clone();
        self.send_message(text);
        Ok(())
    }
}


impl TgDrain {
    pub fn new(api_token: &str, chat_id: i64) -> Self {
        TgDrain {
            token: api_token.into(),
            chat_id: chat_id,
        }
    }

    pub fn send_message(&self, text: String) {
        let chat_id = self.chat_id;
        let api = telegram_bot::Api::from_token(&self.token).unwrap();
        api.send_message(chat_id,
                          text,
                          Some(telegram_bot::ParseMode::Html),
                          Some(true),
                          None,
                          None)
            .ok();
    }
}

fn format_log(info: &Record, _: &OwnedKVList) -> String {
    format!("<code>[{}]{}{}:</code> {}",
            info.level().as_short_str(),
            info.module(),
            info.function(),
            info.msg())
}
