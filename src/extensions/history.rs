use chrono::{DateTime, Local, TimeZone};
use common::*;
use db::DbMessage;
use db::SEARCH_PER;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Saver {
  search_chats: HashSet<tg::ChatId>,
  search_users: HashSet<tg::UserId>,
}

#[derive(Debug)]
struct SearchQuery {
  pattern: String,
  page: usize,
  total: usize,
  items: Vec<DbMessage>,
}

#[derive(Debug, Default)]
pub struct Searcher {
  search: Option<SearchQuery>,
}

const EMPTY_PATTERN_PROMPT: &str = "Please enter pattern";

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
    reply_to_msg_id: msg
      .reply_to_message
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
    if msg.is_cmd("enable_search_for_chat") {
      self.search_chats.insert(msg.chat.id());
      ctx.db.save_conf("history.search_chats", &self.search_chats);
      ctx
        .bot
        .reply_to(msg, format!("Chat {} added to search group", msg.chat.id()));
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
  fn beginning_search(&mut self, query_msg: &tg::Message, ctx: &Context) {
    let pattern = if query_msg.is_cmd("search") {
      // as /search command arg
      query_msg.cmd_arg()
    } else {
      // as force reply content
      query_msg.text_content()
    };

    if pattern.is_none() {
      let req = query_msg
        .text_reply(EMPTY_PATTERN_PROMPT)
        .reply_markup(tg::ForceReply::new().selective().clone())
        .clone();
      ctx.bot.spawn(req);
      return;
    }

    self.search = Some(SearchQuery {
      pattern: pattern.unwrap(),
      page: 1,
      total: 0,
      items: Vec::new(),
    });

    let (reply, pagination_buttons) = self.search_content(&ctx.db);

    let mut keyboard = tg::InlineKeyboardMarkup::new();
    if !pagination_buttons.is_empty() {
      keyboard.add_row(pagination_buttons);
    }

    ctx
      .bot
      .spawn(query_msg.text_reply(reply).reply_markup(keyboard).clone());
  }

  fn flip_page(&mut self, action: &str, edit_msg: &tg::Message, ctx: &Context) {
    if self.search.is_none() {
      return;
    }

    {
      let search = self.search.as_mut().unwrap();

      match action {
        "prev" => search.page -= 1,
        "next" => search.page += 1,
        _ => {
          error!(ctx.logger, "invalid flip page action: {}", action);
          panic!("invalid flip page action");
        }
      }
      if search.page <= 0 {
        search.page = 1;
      }
    }

    let (reply, pagination_buttons) = self.search_content(&ctx.db);

    let mut keyboard = tg::InlineKeyboardMarkup::new();
    if !pagination_buttons.is_empty() {
      keyboard.add_row(pagination_buttons);
    };

    let req = ctx
      .bot
      .send(edit_msg.edit_text(reply).reply_markup(keyboard))
      .then(|_| ok(()));
    ctx.handle.spawn(req);
  }

  fn search_content(
    &mut self,
    db: &Db,
  ) -> (String, Vec<tg::InlineKeyboardButton>) {
    {
      let users = db
        .load_conf::<Vec<i64>>("history.search_users")
        .unwrap_or_default();
      let search = self.search.as_mut().unwrap();
      let pattern = &search.pattern;
      let page = search.page;

      let db_pat: String = pattern.replace("*", "%").replace("'", "''");
      let (count, result) = db.search_msg(page, &db_pat, &users);

      search.total = count;
      search.items = result.clone();
    }

    (self.format_reply(), self.pagination())
  }

  fn format_reply(&self) -> String {
    let search = self.search.as_ref().unwrap();

    let mut reply_buf = String::new();
    writeln!(&mut reply_buf, "Searching for: {}", search.pattern).ok();

    if search.total == 0 {
      return "No matching result found.".into();
    }

    let start = (search.page - 1) * SEARCH_PER + 1;
    writeln!(
      &mut reply_buf,
      "Showing {}-{} of {} search results",
      start,
      start + search.items.len() - 1,
      search.total
    )
    .ok();
    writeln!(&mut reply_buf).ok();

    for (i, message) in search.items.iter().enumerate() {
      let user = ellipsis(
        &message
          .user_name
          .as_ref()
          .map(Clone::clone)
          .unwrap_or("someone".into()),
        10,
      );

      let group = ellipsis(
        &message
          .chat_name
          .as_ref()
          .map(Clone::clone)
          .unwrap_or("some chat".into()),
        11,
      );
      let extract = escape_markdown(&message.text.clone().unwrap_or_default());

      writeln!(
        &mut reply_buf,
        // "/ref_{} ({}) {} at {}:\n\u{27A4} {}",
        "\u{27A4} {}, {} at {}:\n{} (\u{261E} /ref_{})",
        format_time(message.created_at),
        user,
        group,
        extract,
        i + 1,
      )
      .ok();
    }

    reply_buf
  }

  fn pagination(&self) -> Vec<tg::InlineKeyboardButton> {
    let search = self.search.as_ref().unwrap();
    let page = search.page;

    let mut pagination = Vec::new();

    if page > 1 {
      pagination.push(self.callback_button("«", "prev_page"));
    }
    let count_so_far = search.items.len() + (page - 1) * SEARCH_PER;
    if count_so_far < search.total {
      pagination.push(self.callback_button("»", "next_page"));
    };

    pagination
  }

  fn try_refer_result(
    &self,
    nth_result: i32,
    msg: &tg::Message,
    ctx: &Context,
  ) {
    let ref_msg = match self.search {
      Some(ref search) => search.items.iter().nth(nth_result as usize).clone(),
      None => None,
    };

    if ref_msg.is_none() {
      ctx.bot.reply_to(msg, "Unable to find message");
      return;
    }

    let ref_msg = ref_msg.unwrap();
    let mut req =
      tg::SendMessage::new(tg::ChatId::from(ref_msg.chat_id), "Here you go");
    req.reply_to(tg::MessageId::from(ref_msg.msg_id));
    let bot = ctx.bot.clone();
    let msg = msg.clone();
    let future = ctx.bot.send(req).then(move |e| {
      match e {
        Ok(..) => {}
        Err(e) => {
          bot.reply_to(msg, format!("Unable to refer to message: {:}", e))
        }
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
      self.beginning_search(msg, ctx);
      return;
    }

    if msg.cmd_name().map(|x| x.starts_with("ref")) == Some(true) {
      let text = msg.text_content().unwrap_or_default();
      let match_reference = RE.captures(&text);
      if let Some(caps) = match_reference {
        let n = caps.get(1).unwrap().as_str().parse::<i32>().unwrap();
        self.try_refer_result(n - 1, msg, ctx);
      }
      return;
    }

    if msg.is_force_reply(EMPTY_PATTERN_PROMPT) {
      self.beginning_search(msg, ctx);
    }
  }

  fn process_callback(&mut self, callback: &tg::CallbackQuery, ctx: &Context) {
    let edit_msg = &callback.message;
    match callback.key() {
      Some("prev_page") => self.flip_page("prev", edit_msg, ctx),
      Some("next_page") => self.flip_page("next", edit_msg, ctx),
      Some(_) => {}
      None => {}
    }
  }

  fn name(&self) -> &str {
    "history_searcher"
  }
}
