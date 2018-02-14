use common::*;

pub type Integer = i64;
pub type Bot = tg::Api;

pub trait TgApiExt {
    fn from_default_env() -> Self;
    fn send_short_raw(
        &self,
        chat_id: Integer,
        reply_to_msg_id: Option<Integer>,
        txt: &str,
        parse_mode: Option<tg::ParseMode>,
    ) -> Result<tg::Message>;
    fn consume_updates(&self) -> usize;
    fn send_typing<T>(&self, chat: T)
    where
        T: Chattable;

    fn send_raw(
        &self,
        chat_id: Integer,
        reply_to_msg_id: Option<Integer>,
        txt: &str,
        parse_mode: Option<tg::ParseMode>,
    ) -> Result<tg::Message> {
        let txt = url_escape(txt);
        if txt.chars().count() < 4096 {
            self.send_short_raw(chat_id, reply_to_msg_id, &txt, parse_mode)
        } else if txt.lines().count() <= 1 {
            let init = txt.chars().take(4090).collect::<String>();
            let rest = txt.chars().skip(4090).collect::<String>();
            self.send_short_raw(chat_id, reply_to_msg_id, &init, parse_mode)
                .ok();
            self.send_short_raw(chat_id, reply_to_msg_id, &rest, parse_mode)
        } else {
            let mut acc_len = 0;
            let mut buf = String::new();
            for line in txt.lines() {
                acc_len += line.chars().count() + 1;
                if acc_len >= 4096 {
                    break;
                }
                buf.push_str(&format!("{}\n", line));
            }
            let rest = &txt[buf.len()..];
            if buf.is_empty() {
                // first line very long
                let mut lines = txt.lines();
                let init = lines.next().unwrap();
                let rest = lines.collect::<Vec<&str>>().concat();
                self.send_raw(chat_id, reply_to_msg_id, init, parse_mode)
                    .ok();
                self.send_raw(chat_id, reply_to_msg_id, &rest, parse_mode)
            } else {
                self.send_short_raw(chat_id, reply_to_msg_id, &buf, parse_mode)
                    .ok();
                self.send_raw(chat_id, reply_to_msg_id, &rest, parse_mode)
            }
        }
    }

    fn reply_and_get_msg<R>(&self, msg: R, txt: &str) -> Result<tg::Message>
    where
        R: Repliable,
    {
        self.send_raw(msg.chat_id(), msg.message_id(), txt, None)
    }
    fn reply_md_and_get_msg<R>(
        &self,
        msg: R,
        md_txt: &str,
    ) -> Result<tg::Message>
    where
        R: Repliable,
    {
        let markdown = Some(tg::ParseMode::Markdown);
        self.send_raw(msg.chat_id(), msg.message_id(), md_txt, markdown)
    }
    fn reply_to<R>(&self, msg: R, txt: &str)
    where
        R: Repliable,
    {
        self.send_raw(msg.chat_id(), msg.message_id(), txt, None)
            .ok();
    }
    fn reply_md_to<R>(&self, msg: R, md_txt: &str)
    where
        R: Repliable,
    {
        let markdown = Some(tg::ParseMode::Markdown);
        self.send_raw(msg.chat_id(), msg.message_id(), md_txt, markdown)
            .ok();
    }
}

pub trait TgMessageExt {
    fn msg_txt(&self) -> Option<String>;
    fn is_cmd(&self, prefix: &str) -> bool;
    fn is_cmds(&self, prefixes: &str) -> bool;
    fn cmd_cmd(&self) -> Option<String>;
    fn cmd_arg(&self, prefix: &str) -> Option<String>;
    fn cmd_args(&self, prefix: &str) -> Vec<String>;
    fn clean_cmd(&mut self);
}

pub trait TgUserExt {
    fn user_name(&self) -> String;
    fn formal_name(&self) -> String;
}

pub trait Chattable {
    fn chat_id(&self) -> Integer;
}

pub trait Repliable: Chattable {
    fn message_id(&self) -> Option<Integer>;
}

pub fn bot() -> Bot {
    Bot::from_default_env()
}

/// ///////////////// implementing the extensions  ////////////////////

fn url_escape(s: &str) -> String {
    s.replace("%", "%25")
        .replace("?", "%3F")
        .replace("&", "%26")
}

impl TgApiExt for tg::Api {
    fn from_default_env() -> Self {
        Self::from_env("TELEGRAM_BOT_TOKEN").unwrap()
    }

    fn send_short_raw(
        &self,
        chat_id: Integer,
        reply_to_msg_id: Option<Integer>,
        txt: &str,
        parse_mode: Option<tg::ParseMode>,
    ) -> Result<tg::Message> {
        self.send_message(
            chat_id,         // chat id
            txt.into(),     // txt
            parse_mode,      // parse mode
            None,            // disable web preview
            reply_to_msg_id, // reply to msg id
            None,
        ).map_err(|err| format!("{:?}", err))
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

    fn send_typing<T>(&self, chat: T)
    where
        T: Chattable,
    {
        self.send_chat_action(chat.chat_id(), tg::ChatAction::Typing)
            .ok();
    }
}

impl<'a> TgMessageExt for tg::Message {
    fn msg_txt(&self) -> Option<String> {
        if let tg::MessageType::Text(ref txt) = self.msg {
            Some(txt.clone().into())
        } else {
            None
        }
    }

    fn is_cmd(&self, prefix: &str) -> bool {
        if let Some(txt) = self.msg_txt() {
            txt.eq(&format!("/{}", prefix))
                || txt.starts_with(&format!("/{} ", prefix))
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

    fn cmd_args(&self, prefix: &str) -> Vec<String> {
        if let Some(arg_str) = self.cmd_arg(prefix) {
            arg_str
                .as_str()
                .split_whitespace()
                .map(String::from)
                .collect()
        } else {
            Vec::new()
        }
    }

    fn clean_cmd(&mut self) {
        let msg = &mut self.msg;
        if let &mut tg::MessageType::Text(ref mut txt) = msg {
            lazy_static! {
                static ref RE: Regex = Regex::new(r"^(?P<cmd>/\w+)@\w+bot").unwrap();
            }

            let old_txt = txt.clone();
            *txt = RE.replace(&old_txt, "$cmd").into_owned()
        }
    }
}

impl TgUserExt for tg::User {
    fn user_name(&self) -> String {
        self.username.clone().unwrap_or(self.formal_name())
    }

    fn formal_name(&self) -> String {
        let mut name = self.first_name.clone();
        if self.last_name.is_some() {
            name += " ";
            name += &self.first_name;
        }
        name
    }
}

impl<'a> Chattable for &'a tg::Message {
    fn chat_id(&self) -> Integer {
        self.chat.id()
    }
}

impl<'a> Repliable for &'a tg::Message {
    fn message_id(&self) -> Option<Integer> {
        Some(self.message_id)
    }
}

impl Chattable for (Integer, Integer) {
    fn chat_id(&self) -> Integer {
        self.0
    }
}

impl Repliable for (Integer, Integer) {
    fn message_id(&self) -> Option<Integer> {
        Some(self.1)
    }
}

impl Chattable for (Integer, Option<Integer>) {
    fn chat_id(&self) -> Integer {
        self.0
    }
}

impl Repliable for (Integer, Option<Integer>) {
    fn message_id(&self) -> Option<Integer> {
        self.1
    }
}

impl Chattable for Integer {
    fn chat_id(&self) -> Integer {
        *self
    }
}
