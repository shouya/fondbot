use common::*;

pub type Integer = i64;
pub type Bot = tg::Api;

pub trait TgApiExt {
    fn to_api(&self) -> &tg::Api;
    fn from_token(token: &str) -> Self where Self: Sized;
    fn from_default_env() -> Self where Self: Sized {
        let token = env::var("TELEGRAM_BOT_TOKEN").expect(
            "telegram bot token undefined!",
        );
        Self::from_token(&token)
    }
    fn consume_updates(&self) -> usize {
        use tg::requests::GetUpdates;
        use futures::Future;

        let mut count = 0;
        let mut last = 0;

        loop {
            let updates = self.to_api().send(GetUpdates::new().offset(last))
                .wait()
                .unwrap_or_default();
            if updates.is_empty() {
                break;
            }
            count += updates.len();
            last = updates.last().unwrap().id + 1;
        }
        count
    }
    fn reply_to<'s, T: Into<Cow<'s, str>>>(&self, whom: &tg::Message, what: T) {
        self.to_api().spawn(
            tg::SendMessage::new(&whom.chat, what)
                .reply_to(whom),
        );
    }
    fn reply_md_to<'s, T: Into<Cow<'s, str>>>(
        &self,
        whom: &tg::Message,
        what: T,
    ) {
        self.to_api().spawn(
            tg::SendMessage::new(&whom.chat, what)
                .reply_to(whom)
                .parse_mode(tg::ParseMode::Markdown),
        );
    }
    fn say<'s, W: ToChatRef, T: Into<Cow<'s, str>>>(&self, whom: W, what: T) {
        self.to_api().spawn(tg::SendMessage::new(whom, what))
    }
    fn get_me(&self) -> tg::User {
        use futures::Future;
        let api = self.to_api();
        // println!("api: {:?}", api);
        let req = api.send(tg::GetMe);
        // println!("req: {:?}", req);
        let res = req.wait();
        println!("res: {:?}", res);
        // self.to_api().send(tg::GetMe).wait().expect("Failed getting bot itself")
        res.unwrap()
    }
    fn send_typing(&self, msg: &tg::Message) {
        self.to_api().spawn(tg::SendChatAction::new(
            &msg.chat,
            tg::ChatAction::Typing,
        ))
    }
    //    fn send_typing<T>(&self, chat: T) where T: Chattable;
}

pub trait TgMessageExt {
    fn to_msg(&self) -> &tg::Message;
    fn to_msg_mut(&mut self) -> &mut tg::Message;

    fn text(&self) -> Option<String> {
        if let tg::MessageKind::Text { ref data, .. } = self.to_msg().kind {
            Some(data.to_string())
        } else {
            None
        }
    }

    fn is_cmd(&self, prefix: &str) -> bool {
        if let Some(txt) = self.text() {
            txt.eq(&format!("/{}", prefix)) ||
                txt.starts_with(&format!("/{} ", prefix))
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
        if let Some(txt) = self.text() {
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
    //   if let Some(txt) = text(msg) {
    //     txt.as_str().split_whitespace().nth(1).map(String::from)
    //   } else {
    //     None
    //   }
    // }

    fn cmd_arg(&self, prefix: &str) -> Option<String> {
        if !self.is_cmd(prefix) {
            None
        } else {
            let txt = self.text().unwrap();
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

    fn is_processable(&self) -> bool {
        self.to_msg().from.is_some()
    }

    fn clean_cmd(&mut self) {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^(?P<cmd>/\w+)@\w+bot").unwrap();
        }
        let text = self.text();
        if text.is_none() {
            return;
        }
        self.to_msg_mut().kind = tg::MessageKind::Text {
            data: RE.replace(&text.unwrap(), "$cmd").into_owned(),
            entities: Vec::new(),
        };
    }

    fn from_user_name(&self) -> String {
        self.to_msg()
            .from
            .as_ref()
            .map(TgUserExt::user_name)
            .unwrap_or("".into())
    }

    fn id_i64(&self) -> i64 {
        self.to_msg().id.into()
    }
    fn chat_id_i64(&self) -> i64 {
        self.to_msg().chat.id().into()
    }
    fn from_id_i64(&self) -> Option<i64> {
        self.to_msg().from.as_ref().map(|x| x.id.into())
    }
    fn is_to_group(&self) -> bool {
        match self.to_msg().chat {
            tg::Chat::Supergroup(_) => true,
            tg::Chat::Group(_) => true,
            _ => false
        }
    }
}

pub trait TgUserExt {
    fn user_name(&self) -> String;
}

pub fn bot() -> Bot {
    Bot::from_default_env()
}

/// ///////////////// implementing the extensions  ////////////////////


impl TgApiExt for tg::Api {
    fn to_api<'a>(&'a self) -> &'a tg::Api {
        self
    }

    fn from_token(token: &str) -> Self {
        use tokio_core;
        let core = tokio_core::reactor::Core::new().unwrap();
        Self::configure(token).build(core.handle())
    }
}

impl TgMessageExt for tg::Message {
    fn to_msg(&self) -> &tg::Message {
        self
    }
    fn to_msg_mut(&mut self) -> &mut tg::Message {
        self
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
