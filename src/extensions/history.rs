use common::*;
use db::SEARCH_PER;
use db::DbMessage;
use chrono::{DateTime, NaiveDateTime};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Saver {
    search_groups: HashSet<Integer>,
    search_users: HashSet<Integer>,
}

#[derive(Debug, Clone, Default)]
pub struct Searcher;

fn chat_name(chat: &tg::Chat) -> String {
    match *chat {
        tg::Chat::Private(_) => "private chat".into(),
        tg::Chat::Group(ref group) => group.title.clone(),
        tg::Chat::Supergroup(ref group) => group.title.clone(),
        tg::Chat::Channel(ref chan) => chan.title.clone(),
        _ => "".into(),
    }
}

fn to_db_message(msg: &tg::Message) -> DbMessage {
    DbMessage {
        id: None,
        msg_id: msg.id_i64(),
        user_id: msg.from.as_ref().map(|x| x.id.into()).unwrap_or(-1i64),
        user_name: Some(msg.from_user_name()),
        chat_id: msg.chat_id_i64(),
        chat_name: Some(chat_name(&msg.chat)),
        is_group: msg.is_to_group(),
        reply_to_msg_id: msg.reply_to_message.as_ref().map(|ref x| x.id_i64()),
        text: msg.text(),
        created_at: Some(msg.date),
    }
}

fn format_time(time: Option<i64>) -> String {
    let naive_time = NaiveDateTime::from_timestamp(time.unwrap_or_default(), 0);
    let time: DateTime<chrono::FixedOffset> =
        DateTime::from_utc(naive_time, *GLOBAL_TIMEZONE);
    time.format("%Y-%m-%d").to_string()
}

impl BotExtension for Saver {
    fn init(ctx: &Context) -> Self {
        ctx.db
            .load_conf("history.search_groups")
            .unwrap_or_default()
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        if msg.is_cmd("enable_search_for_group") {
            self.search_groups.insert(msg.chat_id_i64());
            ctx.db.save_conf(
                "history.search_groups",
                &self.search_groups,
            );
            ctx.bot.reply_to(
                msg,
                format!("Chat {} added to search group", msg.chat.id()),
            );
            return;
        }
        if msg.is_cmd("enable_search_for_me") {
            self.search_users.insert(msg.from_id_i64().unwrap_or(0));
            ctx.db.save_conf("history.search_users", &self.search_users);
            ctx.bot.reply_to(
                msg,
                format!(
                    "You ({}) have been added to search users",
                    msg.from_user_name()
                ),
            );
            return;
        }

        if !self.search_groups.contains(&msg.chat_id_i64()) {
            return;
        }

        if msg.text().is_none() {
            // we only want to search text messages
            return;
        }

        ctx.db.save_msg(&to_db_message(msg));
    }

    fn name(&self) -> &str {
        "history_saver"
    }
}

impl Searcher {
    fn search(&self, msg: &tg::Message, ctx: &Context) {
        let args = ctx.db
            .load_conf::<Vec<String>>("history.last_search_args")
            .unwrap_or_default();
        let page = ctx.db
            .load_conf::<usize>("history.last_search_page")
            .unwrap_or(1);
        let users = ctx.db
            .load_conf::<Vec<i64>>("history.search_users")
            .unwrap_or_default();

        if args.is_empty() {
            ctx.bot.reply_md_to(
                msg,
                "Usage: search <pattern> [pattern...]\n\
                                      Patterns:\n    \
                                      *: matches any string",
            );
            return;
        }

        let patterns = args.iter()
            .map(|x| x.replace("*", "%"))
            .collect::<Vec<String>>();
        let (count, result) = ctx.db.search_msg(page, &patterns, &users);
        let result_count = result.len();
        let mut reply_buf = String::new();
        let start = (page - 1) * SEARCH_PER + 1;
        writeln!(&mut reply_buf, "Searching for: {}", args.join(" ")).ok();
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
                    &message.user_name.as_ref().map(Clone::clone).unwrap_or(
                        "someone".into(),
                    ),
                    10,
                ),
                ellipsis(
                    &message.chat_name.as_ref().map(Clone::clone).unwrap_or(
                        "some chat"
                            .into(),
                    ),
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

    fn refer_result(&self, nth_result: i32, msg: &tg::Message, ctx: &Context) {
        let last_results = ctx.db
            .load_conf::<Vec<DbMessage>>("history.last_search_result")
            .unwrap_or_default();
        let result = last_results.iter().nth(nth_result as usize);
        if let Some(dbmsg) = result {
            ctx.bot.spawn(&tg::SendMessage::new(
                tg::ChatId::new(dbmsg.chat_id),
                "Here you go",
            ).reply_to(tg::MessageId::new(dbmsg.msg_id)));
        } else {
            ctx.bot.reply_to(msg, "Unable to find message");
        }
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
        let text = msg.text();
        if text.is_none() {
            return;
        }
        if msg.is_cmd("search") {
            ctx.db.save_conf(
                "history.last_search_args",
                msg.cmd_args("search"),
            );
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
        let text_bang = text.unwrap();
        let match_reference = RE.captures(&text_bang);
        if let Some(caps) = match_reference {
            let n = caps.get(1).unwrap().as_str().parse::<i32>().unwrap();
            self.refer_result(n - 1, msg, ctx);
        }
    }

    fn name(&self) -> &str {
        "history_searcher"
    }
}
