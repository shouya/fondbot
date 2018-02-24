use common::*;
use super::*;

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
      remind_at: self.remind_at.clone().unwrap(),
      set_at: Local::now(),
      content: self.content.clone().unwrap(),
      chat_id: self.chat_id,
      message_id: self.message_id,
      deleted: false
    })
  }

  fn on_message(&mut self, msg: &tg::Message, ctx: &Context) {
    if !msg.is_reply_to_bot() {
      return;
    }

    if self.stage != "content" {
      return;
    }

    msg
      .text_content()
      .map(|t| {
        self.prompt("set_time", Some(msg), ctx);
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

    let time = self.key_to_time(query.key().unwrap_or("unknown"));
    self.remind_at = Some(time);
  }

  fn prompt(
    &self,
    prompt: Self::Prompt,
    msg: Option<&tg::Message>,
    ctx: &Context,
  ) {
    match prompt {
      "set_time" => self.prompt_reminder_time(msg.unwrap(), ctx),
      "set_content" => self.prompt_set_content(msg.unwrap(), ctx),
      _ => {}
    }
  }

  fn ready(&self) -> bool {
    self.build().is_some()
  }
}

impl SetReminder {
  pub fn init(message: &tg::Message, ctx: &Context) -> SetReminder {
    let content = message.cmd_arg();
    let stage = if content.is_none() { "content" } else { "time" };

    SetReminder {
      message_id: message.id,
      chat_id: message.chat.id(),
      remind_at: None,
      remind_at_type: None,
      content,
      stage,
    }
  }

  fn prompt_reminder_time(&self, message: &tg::Message, ctx: &Context) {
    let callback_button = |text: &str, key: &str| -> tg::InlineKeyboardButton {
      let callback = format!("{}.{}", "reminder", key);
      tg::InlineKeyboardButton::callback(text, callback)
    };
    let keyboard = {
      let mut keyboard = tg::InlineKeyboardMarkup::new();
      let rows = vec![
        vec![
          callback_button("1 min", "1min"),
          callback_button("5 min", "5min"),
          callback_button("10 min", "10min"),
        ],
        vec![
          callback_button("1 hour", "1hr"),
          callback_button("2 hours", "2hr"),
          callback_button("4 hours", "4hr"),
        ],
        vec![
          callback_button("12pm", "12pm"),
          callback_button("5pm", "5pm"),
          callback_button("8pm", "8pm"),
          callback_button("10pm", "10pm"),
        ],
        vec![
          callback_button("tmr 8am", "8am+"),
          callback_button("tmr 10am", "10am+"),
          callback_button("tmr 12pm", "12pm+"),
          callback_button("tmr 5pm", "5pm+"),
          callback_button("tmr 8pm", "8pm+"),
          callback_button("tmr 10pm", "10pm+"),
        ],
      ];
      for row in rows.into_iter() {
        keyboard.add_row(row);
      }
      keyboard
    };

    let mut req = message.text_reply(
      "Please choose when to remind, or reply with the time you want",
    );
    req.reply_markup(keyboard);
    ctx.bot.spawn(req);
  }

  fn prompt_set_content(&self, msg: &tg::Message, ctx: &Context) {
    let req = msg
      .text_reply("What do you want to be reminded about?")
      .reply_markup(tg::ForceReply::new().selective().clone())
      .clone();
    ctx.bot.spawn(req);
  }

  fn key_to_time(&self, key: &str) -> DateTime<Local> {
    let now = Local::now();
    let today = Local::today();
    let tmr = today.succ();

    match key {
      "1min" => now + Duration::minutes(1),
      "5min" => now + Duration::minutes(5),
      "10min" => now + Duration::minutes(10),

      "1hr" => now + Duration::hours(1),
      "2hr" => now + Duration::hours(2),
      "4hr" => now + Duration::hours(4),

      "12pm" => today.and_hms(12, 0, 0),
      "5pm" => today.and_hms(17, 0, 0),
      "8pm" => today.and_hms(20, 0, 0),
      "10pm" => today.and_hms(22, 0, 0),

      "8am+" => tmr.and_hms(8, 0, 0),
      "10am+" => tmr.and_hms(10, 0, 0),
      "12pm+" => tmr.and_hms(12, 0, 0),
      "5pm+" => tmr.and_hms(17, 0, 0),
      "8pm+" => tmr.and_hms(20, 0, 0),
      "10pm+" => tmr.and_hms(22, 0, 0),

      _ => now,
    };

    now
  }
}
