use common::*;

use chrono::{Date, DateTime, Duration, Local};
use std::cell::RefCell;
use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, Hash)]
struct Reminder {
  remind_at: DateTime<Local>,
  set_at: DateTime<Local>,
  content: String,
  chat_id: tg::ChatId,
  message_id: tg::MessageId,
}

#[derive(Clone, Debug)]
struct SetReminder {
  chat_id: tg::ChatId,
  message_id: tg::MessageId,

  content: Option<String>,
  remind_at: Option<DateTime<Local>>,
  remind_at_type: Option<String>,
}

struct ReminderPool {
  reminders: Arc<RefCell<Vec<Reminder>>>,
  set_reminder: Option<SetReminder>,
}

impl BotExtension for ReminderPool {
  fn init(ctx: &Context) -> Self
  where
    Self: Sized,
  {
    let reminders = ctx
      .db
      .load_conf::<Vec<Reminder>>("reminders")
      .unwrap_or(Vec::new());

    ReminderPool {
      reminders: Arc::new(RefCell::new(reminders)),
      set_reminder: None,
    }
  }

  fn process(&mut self, message: &tg::Message, ctx: &Context) {
    if self.set_reminder_message(message) {
      self.set_reminder(Some(message), None, ctx);
    }
  }
  fn process_callback(&mut self, _query: &tg::CallbackQuery, _ctx: &Context) {}
  fn name(&self) -> &str {
    "reminder"
  }
}

impl ReminderPool {
  fn set_reminder_message(&self, message: &tg::Message) -> bool {
    message.is_cmd("remind_me")
      || (message.is_reply_to_bot() && self.set_reminder.is_some())
  }

  fn set_reminder(
    &mut self,
    message: Option<&tg::Message>,
    callback: Option<&tg::CallbackQuery>,
    ctx: &Context,
  ) {
    // initiate set reminder
    if self.set_reminder.is_none() {
      let message = message.unwrap();
      self.set_reminder = Some(SetReminder {
        message_id: message.id,
        chat_id: message.chat.id(),
        content: message.cmd_arg(),
        remind_at: None,
        remind_at_type: None,
      });
    }

    if self.set_reminder.unwrap().remind_at.is_none() {
      self.prompt_reminder_time(&self, message, ctx);
    } else if self.set_reminder.unwrap().content.is_none() {
      let req = message.text_reply("What do you want to be reminded about");
      ctx.bot.spawn(req);
    }
  }

  fn prompt_reminder_time(&self, message: &tg::Message, ctx: &Context) {
    let keyboard = {
      let mut keyboard = tg::InlineKeyboardMarkup::new();
      let rows = vec![
        vec![
          self.callback_button("1 min", "1min"),
          self.callback_button("5 min", "5min"),
          self.callback_button("10 min", "10min"),
        ],
        vec![
          self.callback_button("1 hour", "1hr"),
          self.callback_button("2 hours", "2hr"),
          self.callback_button("4 hours", "4hr"),
        ],
        vec![
          self.callback_button("today 12pm", "12pm"),
          self.callback_button("today 5pm", "5pm"),
          self.callback_button("today 8pm", "8pm"),
          self.callback_button("today 10pm", "10pm"),
        ],
        vec![
          self.callback_button("tmr 8am", "8am+"),
          self.callback_button("tmr 10am", "10am+"),
          self.callback_button("tmr 12pm", "12pm+"),
          self.callback_button("tmr 5pm", "5pm+"),
          self.callback_button("tmr 8pm", "8pm+"),
          self.callback_button("tmr 10pm", "10pm+"),
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

  fn key_to_time(key: &str) -> DateTime<Local> {
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



user: /remind me
sys: set_reminder on
sys: please enter time
user: enter invalid time
sys: invalid time, please enter time
user: enter correct time
