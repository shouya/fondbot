extern crate slog_extra;

use tg;
use slog::{Drain, Record, OwnedKeyValueList};
use self::slog_extra::Async;

pub struct TgDrain {
    api_token: String,
    chat_id: tg::ChatId,
}

impl Drain for TgDrain {
    type Error = ();
    fn log(
        &self,
        info: &Record,
        options: &OwnedKeyValueList,
    ) -> Result<(), ()> {
        let text = format_log(info, options).clone();
        self.send_message(text);
        Ok(())
    }
}


impl TgDrain {
    pub fn new(api_token: &str, chat_id: i64) -> Async {
        Async::new(TgDrain {
            api_token: api_token.into(),
            chat_id: tg::ChatId::new(chat_id),
        })
    }

    pub fn send_message(&self, text: String) {
        let chat_id = self.chat_id;
        use bot::TgApiExt;
        let api = tg::Api::from_token(&self.api_token);
        api.spawn(
            tg::SendMessage::new(chat_id, text)
                .parse_mode(tg::ParseMode::Html)
                .disable_preview(),
        );
    }
}

fn format_log(info: &Record, _: &OwnedKeyValueList) -> String {
    format!(
        "<code>[{}]{}{}:</code> {}",
        info.level().as_short_str(),
        info.module(),
        info.function(),
        info.msg()
    )
}
