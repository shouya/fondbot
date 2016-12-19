use common::*;

pub type Integer = i64;

pub struct Bot {
    pub api: tg::Api,
}

impl Bot {
    pub fn from_env() -> Self {
        Bot { api: tg::Api::from_env("TELEGRAM_BOT_TOKEN").unwrap() }
    }

    pub fn send_raw<T: Into<String>>(&self,
                                     chat_id: Integer,
                                     reply_to_msg_id: Option<Integer>,
                                     txt: T,
                                     parse_mode: Option<tg::ParseMode>) {
        let mut retry_count = 3;
        let txt = txt.into();
        while let Err(err) = self.api.send_message(chat_id, // chat id
                                                   txt.clone(), // txt
                                                   parse_mode, // parse mode
                                                   None, // disable web preview
                                                   reply_to_msg_id, // reply to msg id
                                                   None) {
            // reply markup (kbd)
            warn!("send message failed {}, retrying {}", err, retry_count);
            retry_count -= 1;
            if retry_count == 0 {
                break;
            }
        }
    }
    pub fn reply_raw<T: Into<String>>(&self,
                                      chat_id: Integer,
                                      msg_id: Integer,
                                      txt: T,
                                      parse_mode: Option<tg::ParseMode>) {
        self.send_raw(chat_id, Some(msg_id), txt, parse_mode);
    }

    pub fn reply_to<T: Into<String>>(&self, msg: &tg::Message, txt: T) {
        self.reply_raw(msg.chat.id(), msg.message_id, txt, None);
    }

    pub fn reply_markdown_to<T: Into<String>>(&self, msg: &tg::Message, md_txt: T) {
        let markdown = Some(tg::ParseMode::Markdown);
        self.reply_raw(msg.chat.id(), msg.message_id, md_txt, markdown);
    }

    pub fn consume_updates(&self) -> usize {
        let mut count = 0;
        let mut last = 0;
        while let Ok(updates) = self.api.get_updates(Some(last), None, None) {
            if updates.is_empty() {
                break;
            }
            for u in updates {
                count += 1;
                last = u.update_id + 1;
            }
        }
        count
    }
}


pub fn msg_txt(msg: &tg::Message) -> Option<String> {
    if let tg::MessageType::Text(ref txt) = msg.msg {
        Some(txt.clone().into())
    } else {
        None
    }
}

pub fn is_cmd(msg: &tg::Message, prefix: &str) -> bool {
    if let Some(txt) = msg_txt(msg) {
        txt.eq(&format!("/{}", prefix)) || txt.starts_with(&format!("/{} ", prefix))
    } else {
        false
    }
}

// retrun true if any of prefixes matches, prefixes are splitted by whitespaces
pub fn is_cmds(msg: &tg::Message, prefixes: &str) -> bool {
    for prefix in prefixes.split_whitespace() {
        if is_cmd(msg, prefix) {
            return true;
        }
    }
    false
}

pub fn cmd_cmd(msg: &tg::Message) -> Option<String> {
    if let Some(txt) = msg_txt(msg) {
        if txt.len() <= 1 {
            return None;
        }
        if txt.chars().nth(0).unwrap() != '/' {
            return None;
        }
        if let Some(cmd) = txt[1..].split_whitespace().next() {
            return Some(cmd.into());
        }
    }
    None
}

// pub fn cmd_arg_nocheck(msg: &tg::Message) -> Option<String> {
//   if let Some(txt) = msg_txt(msg) {
//     txt.as_str().split_whitespace().nth(1).map(String::from)
//   } else {
//     None
//   }
// }

pub fn cmd_arg(msg: &tg::Message, prefix: &str) -> Option<String> {
    if !is_cmd(msg, prefix) {
        None
    } else {
        let txt = msg_txt(msg).unwrap();
        if prefix.len() + 2 >= txt.len() {
            return None;
        }

        let (_, b) = txt.split_at(prefix.len() + 2);
        Some(b.to_string())
    }
}

pub fn user_name(user: &tg::User) -> String {
    let user = user.clone();
    let add_space = |x: String| " ".to_string() + &x;
    let last_name = user.last_name.map_or("".into(), add_space);
    let formal_name = user.first_name + &last_name;

    user.username.unwrap_or(formal_name)
}
