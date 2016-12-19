use common::*;

pub type Integer = i64;
pub type Bot = tg::Api;

pub trait TgApiExt {
    fn from_default_env() -> Self;
    fn send_raw<T: Into<String>>(&self,
                                 chat_id: Integer,
                                 reply_to_msg_id: Option<Integer>,
                                 txt: T,
                                 parse_mode: Option<tg::ParseMode>)
                                 -> Result<tg::Message>;
    fn consume_updates(&self) -> usize;

    fn reply_and_get_msg<R, T>(&self, msg: R, txt: T) -> Result<tg::Message>
        where R: Repliable,
              T: Into<String>
    {
        self.send_raw(msg.chat_id(), msg.message_id(), txt, None)
    }
    fn reply_md_and_get_msg<R, T>(&self, msg: R, md_txt: T) -> Result<tg::Message>
        where R: Repliable,
              T: Into<String>
    {
        let markdown = Some(tg::ParseMode::Markdown);
        self.send_raw(msg.chat_id(), msg.message_id(), md_txt, markdown)
    }
    fn reply_to<R, T>(&self, msg: R, txt: T)
        where R: Repliable,
              T: Into<String>
    {
        self.send_raw(msg.chat_id(), msg.message_id(), txt, None).omit();
    }
    fn reply_md_to<R, T>(&self, msg: R, md_txt: T)
        where R: Repliable,
              T: Into<String>
    {
        let markdown = Some(tg::ParseMode::Markdown);
        self.send_raw(msg.chat_id(), msg.message_id(), md_txt, markdown).omit();
    }
}

pub trait TgMessageExt {
    fn msg_txt(&self) -> Option<String>;
    fn is_cmd(&self, prefix: &str) -> bool;
    fn is_cmds(&self, prefixes: &str) -> bool;
    fn cmd_cmd(&self) -> Option<String>;
    fn cmd_arg(&self, prefix: &str) -> Option<String>;
}

pub trait TgUserExt {
    fn user_name(&self) -> String;
}

pub trait Repliable {
    fn chat_id(&self) -> Integer;
    fn message_id(&self) -> Option<Integer>;
}

/// ///////////////// implementing the extensions  ////////////////////


impl TgApiExt for tg::Api {
    fn from_default_env() -> Self {
        Self::from_env("TELEGRAM_BOT_TOKEN").unwrap()
    }

    fn send_raw<T: Into<String>>(&self,
                                 chat_id: Integer,
                                 reply_to_msg_id: Option<Integer>,
                                 txt: T,
                                 parse_mode: Option<tg::ParseMode>)
                                 -> Result<tg::Message> {
        let mut retry_count = 0;
        let txt = txt.into();
        loop {
            let res = self.send_message(chat_id, // chat id
                                        txt.clone(), // txt
                                        parse_mode, // parse mode
                                        None, // disable web preview
                                        reply_to_msg_id, // reply to msg id
                                        None);
            match res {
                Err(err) => {
                    retry_count += 1;
                    warn!("send message failed {}, retrying {}", err, retry_count);
                    if retry_count > 3 {
                        return Err("Eventually failed to send message".into());
                    }
                }
                Ok(msg) => return Ok(msg),
            }
        }
    }

    fn consume_updates(&self) -> usize {
        let mut count = 0;
        let mut last = 0;
        while let Ok(updates) = self.get_updates(Some(last), None, None) {
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

impl<'a> TgMessageExt for &'a tg::Message {
    fn msg_txt(&self) -> Option<String> {
        if let tg::MessageType::Text(ref txt) = self.msg {
            Some(txt.clone().into())
        } else {
            None
        }
    }

    fn is_cmd(&self, prefix: &str) -> bool {
        if let Some(txt) = self.msg_txt() {
            txt.eq(&format!("/{}", prefix)) || txt.starts_with(&format!("/{} ", prefix))
        } else {
            false
        }
    }

    // retrun true if any of prefixes matches, prefixes are splitted by whitespaces
    fn is_cmds(&self, prefixes: &str) -> bool {
        for prefix in prefixes.split_whitespace() {
            if self.is_cmd(prefix) {
                return true;
            }
        }
        false
    }

    fn cmd_cmd(&self) -> Option<String> {
        if let Some(txt) = self.msg_txt() {
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

    // pub fn cmd_arg_nocheck(&self, -> Option<String> {
    //   if let Some(txt) = msg_txt(msg) {
    //     txt.as_str().split_whitespace().nth(1).map(String::from)
    //   } else {
    //     None
    //   }
    // }

    fn cmd_arg(&self, prefix: &str) -> Option<String> {
        if !self.is_cmd(prefix) {
            None
        } else {
            let txt = self.msg_txt().unwrap();
            if prefix.len() + 2 >= txt.len() {
                return None;
            }

            let (_, b) = txt.split_at(prefix.len() + 2);
            Some(b.to_string())
        }
    }
}

impl TgUserExt for tg::User {
    fn user_name(&self) -> String {
        let user = self.clone();
        let add_space = |x: String| " ".to_string() + &x;
        let last_name = user.last_name.map_or("".into(), add_space);
        let formal_name = user.first_name + &last_name;

        user.username.unwrap_or(formal_name)
    }
}

impl<'a> Repliable for &'a tg::Message {
    fn chat_id(&self) -> Integer {
        self.chat.id()
    }
    fn message_id(&self) -> Option<Integer> {
        Some(self.message_id)
    }
}

impl Repliable for (Integer, Integer) {
    fn chat_id(&self) -> Integer {
        self.0
    }
    fn message_id(&self) -> Option<Integer> {
        Some(self.1)
    }
}

impl Repliable for (Integer, Option<Integer>) {
    fn chat_id(&self) -> Integer {
        self.0
    }
    fn message_id(&self) -> Option<Integer> {
        self.1
    }
}
