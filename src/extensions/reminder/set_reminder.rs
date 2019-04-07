use super::*;
use crate::common::*;

#[derive(Clone, Debug)]
pub struct SetReminder {
  chat_id: tg::ChatId,
  message_id: tg::MessageId,

  content: Option<String>,
  remind_at: Option<DateTime<Local>>,
  remind_at_type: Option<String>,

  stage: &'static str,
}

impl InteractiveBuilder for SetReminder {
  type Target = Reminder;
  type Prompt = &'static str;

  fn build(&self) -> Option<Self::Target> {
    if self.remind_at.is_none() || self.content.is_none() {
      return None;
    }

    Some(Reminder {
      remind_at: self.remind_at.unwrap(),
      set_at: Local::now(),
      content: self.content.clone().unwrap(),
      chat_id: self.chat_id,
      message_id: self.message_id,
      deleted: false,
    })
  }

  fn on_message(&mut self, msg: &tg::Message, ctx: &Context) {
    if msg.is_cmd("remind_me") {
      if self.stage == "content" {
        self.prompt("set_content", None, ctx);
      } else {
        self.prompt("set_time", None, ctx);
      }
      return;
    }
    if !msg.is_reply_to_bot() {
      return;
    }

    if self.stage != "content" {
      return;
    }

    msg
      .text_content()
      .map(|t| {
        let reply_msg = msg.reply_to_message.as_ref().unwrap().deref();
        let text = format!("Notification content set to {}", t);
        let req = reply_msg
          .edit_text(text)
          .reply_markup(tg::ReplyKeyboardRemove::new())
          .clone();

        ctx.bot.spawn(req);
        self.prompt("set_time", None, ctx);
        self.content = Some(t);
        self.stage = "time";
      })
      .or_else(|| {
        self.prompt("invalid_content", Some(msg), ctx);
        None
      });
  }

  fn on_callback(&mut self, query: &tg::CallbackQuery, ctx: &Context) {
    if self.stage != "time" {
      return;
    }

    if query.key() == Some("commit_time") {
      if self.duration_too_short() {
        self.prompt("invalid_time", Some(&query.message), ctx);
        return;
      }
      self.stage = "finish";
      self.prompt("finish", Some(&query.message), ctx);
    } else {
      let new = {
        let current = self.remind_at.as_ref().unwrap();
        self.key_to_time(current, query.key().unwrap_or("unknown"))
      };
      self.remind_at = Some(new);
      self.prompt("set_time", Some(&query.message), ctx);
      ctx.bot.spawn(query.acknowledge());
    }
  }

  fn prompt(
    &self,
    prompt: Self::Prompt,
    msg: Option<&tg::Message>,
    ctx: &Context,
  ) {
    match prompt {
      "set_time" => self.prompt_reminder_time(msg, ctx),
      "set_content" => self.prompt_set_content(ctx),
      "invalid_content" => {
        let alert = "Invalid content, please try again";
        ctx.bot.spawn(msg.unwrap().chat.text(alert));
        self.prompt_set_content(ctx);
      }
      "finish" => self.prompt_finish(msg.unwrap(), ctx),
      "invalid_time" => {
        let alert = "Duration too short or negative, please try another one";
        ctx.bot.spawn(msg.unwrap().chat.text(alert))
      }
      _ => {}
    }
  }

  fn ready(&self) -> bool {
    self.stage == "finish"
  }
}

impl SetReminder {
  pub fn init(message: &tg::Message, _ctx: &Context) -> SetReminder {
    let content = message.cmd_arg();
    let stage = if content.is_none() { "content" } else { "time" };

    SetReminder {
      message_id: message.id,
      chat_id: message.chat.id(),
      remind_at: Some(Local::now()),
      remind_at_type: None,
      content,
      stage,
    }
  }

  fn prompt_reminder_time(
    &self,
    message_to_update: Option<&tg::Message>,
    ctx: &Context,
  ) {
    let callback_button = |text: &str, key: &str| -> tg::InlineKeyboardButton {
      let callback = format!("{}.{}", "reminder", key);
      tg::InlineKeyboardButton::callback(text, callback)
    };
    let mut keyboard = {
      let rows = vec![
        vec![
          callback_button("+1 min", "+/1/min"),
          callback_button("+5 min", "+/5/min"),
          callback_button("+30 min", "+/30/min"),
        ],
        vec![
          callback_button("-1 min", "-/1/min"),
          callback_button("-5 min", "-/5/min"),
          callback_button("-30 min", "-/30/min"),
        ],
        vec![
          callback_button("+1 hour", "+/1/hr"),
          callback_button("+2 hours", "+/2/hr"),
          callback_button("+4 hours", "+/4/hr"),
        ],
        vec![
          callback_button("-1 hour", "-/1/hr"),
          callback_button("-2 hours", "-/2/hr"),
          callback_button("-4 hours", "-/4/hr"),
        ],
        vec![
          callback_button("+1 day", "+/1/day"),
          callback_button("+5 day", "+/5/day"),
          callback_button("+1 month", "+/30/day"),
        ],
        vec![
          callback_button("-1 day", "-/1/day"),
          callback_button("-5 day", "-/5/day"),
          callback_button("-1 month", "-/30/day"),
        ],
      ];
      let mut keyboard = tg::InlineKeyboardMarkup::new();
      for row in rows.into_iter() {
        keyboard.add_row(row);
      }
      keyboard
    };

    let current_time = self.remind_at.unwrap();
    let duration = current_time.signed_duration_since(Local::now());
    let mut text = format!(
      "*Reminder time:*\n\
       {}\n\
       ({} from now)\n\n\
       Please adjust according to your need\n",
      format_human_time(&current_time),
      format_duration(duration)
    );

    if self.duration_too_short() {
      text.push_str("_Unable to save this reminder_");
    } else {
      keyboard.add_row(vec![callback_button("Done", "commit_time")]);
    }

    if let Some(msg) = message_to_update {
      // update time
      let req = msg.edit_text(text).reply_markup(keyboard).clone();
      ctx.bot.spawn(req);
    } else {
      // initiate
      let req = tg::SendMessage::new(self.chat_id, text)
        .reply_markup(keyboard)
        .clone();
      ctx.bot.spawn(req);
    }
  }

  fn prompt_set_content(&self, ctx: &Context) {
    let req = tg::SendMessage::new(
      self.chat_id,
      "What do you want to be reminded about?",
    )
    .reply_markup(tg::ForceReply::new().selective().clone())
    .reply_to(self.message_id)
    .clone();
    ctx.bot.spawn(req);
  }

  fn prompt_finish(&self, msg: &tg::Message, ctx: &Context) {
    let remind_at = self.remind_at.as_ref().unwrap();
    let text = format!(
      "Done, I'll remind you at {} ({} from now)",
      format_human_time(remind_at),
      format_duration(remind_at.signed_duration_since(Local::now()))
    );
    let req = msg
      .edit_text(text)
      .reply_markup(tg::ReplyKeyboardRemove::new())
      .clone();
    ctx.bot.spawn(req);
  }

  fn key_to_time<Tz: TimeZone>(
    &self,
    current: &DateTime<Tz>,
    key: &str,
  ) -> DateTime<Tz> {
    let current = current.clone();
    let parts: Vec<&str> = key.splitn(3, '/').collect();
    let (sign, num, unit) =
      (parts[0], parts[1].parse::<i64>().unwrap(), parts[2]);

    let mut dur = match unit {
      "min" => Duration::minutes(num),
      "hr" => Duration::hours(num),
      "day" => Duration::days(num),
      _ => Duration::minutes(1),
    };

    if sign == "-" {
      dur = -dur;
    }

    current + dur
  }

  fn duration_too_short(&self) -> bool {
    let remind_at = self.remind_at.as_ref().unwrap();
    let duration = remind_at.signed_duration_since(Local::now());
    duration <= Duration::seconds(10)
  }
}
