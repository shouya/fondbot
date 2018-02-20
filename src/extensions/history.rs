use common::*;
use db::SEARCH_PER;
use db::DbMessage;
use chrono::{DateTime, Local, TimeZone};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Saver {
    search_chats: HashSet<tg::ChatId>,
    search_users: HashSet<tg::UserId>,
}

#[derive(Debug, Clone, Default)]
pub struct Searcher;

fn chat_name(chat: &tg::MessageChat) -> String {
    use tg::MessageChat::*;

    match chat {
        &Private(..) => "private".into(),
        &Group(ref g) => g.title.clone(),
        &Supergroup(ref g) => g.title.clone(),
        _ => "Unknown".into(),
    }
}

fn is_group(chat: &tg::MessageChat) -> bool {
    use tg::MessageChat::*;

    match chat {
        &Private(..) => false,
        &Group(..) => true,
        &Supergroup(..) => true,
        _ => false,
    }
}

fn to_db_message(msg: &tg::Message, ctx: &Context) -> DbMessage {
    use tg::ToMessageId;

    DbMessage {
        id: None,
        msg_id: msg.id.into(),
        user_id: msg.from.id.into(),
        user_name: Some(ctx.names.get(&msg.from)),
        chat_id: msg.chat.id().into(),
        chat_name: Some(chat_name(&msg.chat)),
        is_group: is_group(&msg.chat),
        reply_to_msg_id: msg.reply_to_message
            .as_ref()
            .map(|x| x.to_message_id().into()),
        text: msg.text_content(),
        created_at: Some(msg.date),
    }
}

fn format_time(time: Option<i64>) -> String {
    let time: DateTime<Local> = Local.timestamp(time.unwrap_or(0), 0);
    time.format("%Y-%m-%d").to_string()
}

impl BotExtension for Saver {
    fn init(ctx: &Context) -> Self {
        ctx.db.load_conf("history.search_chats").unwrap_or_default()
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        if msg.is_cmd("enable_search_for_group") {
            self.search_chats.insert(msg.chat.id());
            ctx.db.save_conf("history.search_chats", &self.search_chats);
            ctx.bot.reply_to(
                msg,
                format!("Chat {} added to search group", msg.chat.id()),
            );
            return;
        }
        if msg.is_cmd("enable_search_for_me") {
            self.search_users.insert(msg.from.id);
            ctx.db.save_conf("history.search_users", &self.search_users);
            ctx.bot.reply_to(
                msg,
                format!(
                    "You ({}) have been added to search users",
                    ctx.names.get(&msg.from)
                ),
            );
            return;
        }

        if !self.search_chats.contains(&msg.chat.id()) {
            trace!(ctx.logger, "history: Message not saved: not in group");
            return;
        }

        if msg.text_content().is_none() {
            // we only want to search text messages
            trace!(ctx.logger, "history: Message not saved: not text");
            return;
        }

        let msg_text = msg.text_content().unwrap();

        if msg_text.starts_with("/") {
            trace!(ctx.logger, "history: Message not saved: bot command")
        }

        if msg_text.chars().count() >= 400 {
            // we don't like message too long
            trace!(ctx.logger, "history: Message not saved: too long");
            return;
        }

        ctx.db.save_msg(&to_db_message(msg, ctx));
        trace!(ctx.logger, "history: Message saved");
    }

    fn name(&self) -> &str {
        "history_saver"
    }
}

impl Searcher {
    fn search(&self, msg: &tg::Message, ctx: &Context) {
        let pattern = ctx.db
            .load_conf::<String>("history.last_search_pattern")
            .unwrap_or_default();
        let page = ctx.db
            .load_conf::<usize>("history.last_search_page")
            .unwrap_or(1);
        let users = ctx.db
            .load_conf::<Vec<i64>>("history.search_users")
            .unwrap_or_default();

        if pattern.is_empty() {
            ctx.bot.reply_md_to(
                msg,
                "Usage: `/search <pattern>`\n\
                 Wildcard '*' in <pattern> matches any string.\n",
            );
            return;
        }

        let db_pat: String = pattern.replace("*", "%").replace("'", "''");
        let (count, result) = ctx.db.search_msg(page, &db_pat, &users);
        let result_count = result.len();
        let mut reply_buf = String::new();
        let start = (page - 1) * SEARCH_PER + 1;
        writeln!(&mut reply_buf, "Searching for: {}", pattern).ok();

        if count == 0 {
            writeln!(&mut reply_buf, "No matching result found").ok();

            if !pattern.starts_with("*") && !pattern.ends_with("*") {
                writeln!(
                    &mut reply_buf,
                    "Hint: try search for '*{}*'",
                    pattern
                ).ok();
            }

            ctx.bot.reply_to(msg, reply_buf);
            return;
        }

        writeln!(
            &mut reply_buf,
            "Showing {}-{} of {} search results",
            start,
            start + result_count - 1,
            count
        ).ok();
        writeln!(&mut reply_buf, "---------------").ok();

        for (i, message) in result.iter().enumerate() {
            writeln!(
                &mut reply_buf,
                "/ref_{} ({}) {}@{}:\n\u{1F539} {}",
                i + 1,
                format_time(message.created_at),
                ellipsis(
                    &message
                        .user_name
                        .as_ref()
                        .map(Clone::clone,)
                        .unwrap_or("someone".into(),),
                    10,
                ),
                ellipsis(
                    &message
                        .chat_name
                        .as_ref()
                        .map(Clone::clone,)
                        .unwrap_or("some chat".into(),),
                    10,
                ),
                message.text.clone().unwrap_or_default()
            ).ok();
        }
        writeln!(&mut reply_buf, "---------------").ok();

        if page > 1 {
            writeln!(&mut reply_buf, "/search_prev_page").ok();
        }
        let count_so_far = result_count + (page - 1) * SEARCH_PER;
        if count_so_far < count {
            writeln!(&mut reply_buf, "/search_next_page").ok();
        }

        ctx.db.save_conf("history.last_search_result", result);

        ctx.bot.reply_to(msg, reply_buf);
    }

    fn try_refer_result(
        &self,
        nth_result: i32,
        msg: &tg::Message,
        ctx: &Context,
    ) {
        let last_results = ctx.db
            .load_conf::<Vec<DbMessage>>("history.last_search_result")
            .unwrap_or_default();
        let result = last_results.iter().nth(nth_result as usize);
        if result.is_none() {
            ctx.bot.reply_to(msg, "Unable to find message");
            return;
        }

        let dbmsg = result.unwrap();
        let mut req = tg::SendMessage::new(
            tg::ChatId::from(dbmsg.chat_id),
            "Here you go",
        );
        req.reply_to(tg::MessageId::from(dbmsg.msg_id));
        let bot = ctx.bot.clone();
        let msg = msg.clone();
        let future = ctx.bot.send(req).then(move |e| {
            match e {
                Ok(..) => {}
                Err(e) => bot.reply_to(
                    msg,
                    format!("Unable to refer to message: {:}", e),
                ),
            }
            Ok(())
        });
        ctx.handle.spawn(future);
    }
}

impl BotExtension for Searcher {
    fn init(_: &Context) -> Self {
        Default::default()
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^/ref_(\d+)(@\w+bot)?$").unwrap();
        };
        if msg.is_cmd("search") {
            let search_pattern = msg.cmd_arg().unwrap_or("".into());
            ctx.db
                .save_conf("history.last_search_pattern", search_pattern);
            ctx.db.save_conf("history.last_search_page", 1);
            self.search(msg, ctx);
            return;
        } else if msg.is_cmd("search_next_page") {
            let page = ctx.db
                .load_conf::<usize>("history.last_search_page")
                .unwrap_or(1);
            ctx.db.save_conf("history.last_search_page", page + 1);
            self.search(msg, ctx);
            return;
        } else if msg.is_cmd("search_prev_page") {
            let page = ctx.db
                .load_conf::<usize>("history.last_search_page")
                .unwrap_or(2);
            ctx.db.save_conf("history.last_search_page", page - 1);
            self.search(msg, ctx);
            return;
        }
        let text = msg.text_content().unwrap_or_default();
        let match_reference = RE.captures(&text);
        if let Some(caps) = match_reference {
            let n = caps.get(1).unwrap().as_str().parse::<i32>().unwrap();
            self.try_refer_result(n - 1, msg, ctx);
        }
    }

    fn name(&self) -> &str {
        "history_searcher"
    }
}
