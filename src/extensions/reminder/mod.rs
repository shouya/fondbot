mod set_reminder;

use self::set_reminder::*;

use common::*;

use std::sync::Arc;

#[derive(Clone, Debug, Serialize, Deserialize, Hash, PartialEq)]
pub struct Reminder {
  remind_at: DateTime<Local>,
  set_at: DateTime<Local>,
  content: String,
  chat_id: tg::ChatId,
  message_id: tg::MessageId,
  deleted: bool,
}

type RemindersType = Vec<Arc<RefCell<Reminder>>>;
struct ReminderPool {
  reminders: RemindersType,
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
      .unwrap_or(Vec::new())
      .into_iter()
      .map(|rem| Arc::new(RefCell::new(rem)))
      .collect();

    ReminderPool {
      set_reminder: None,
      reminders,
    }
  }

  fn process(&mut self, message: &tg::Message, ctx: &Context) {
    if message.is_cmd("remind_me") {
      self.set_reminder = Some(SetReminder::init(message, ctx));
    } else if message.is_reply_to_bot() && self.set_reminder.is_some() {
      self.set_reminder.as_mut().unwrap().on_message(message, ctx);
    }
    self.settle_new_reminder(Some(message), None, ctx);
  }
  fn process_callback(&mut self, query: &tg::CallbackQuery, ctx: &Context) {
    if self.set_reminder.is_some() {
      self.set_reminder.as_mut().unwrap().on_callback(query, ctx);
    }
    self.settle_new_reminder(None, Some(query), ctx);
  }
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
      // self.set_reminder = Some(SetReminder::init());
    }

    // if self.set_reminder.unwrap().remind_at.is_none() {
    //   self.prompt_reminder_time(&self, message, ctx);
    // } else if self.set_reminder.unwrap().content.is_none() {
    //   let req = message.text_reply("What do you want to be reminded about");
    //   ctx.bot.spawn(req);
    // }
  }

  fn settle_new_reminder(
    &mut self,
    msg: Option<&tg::Message>,
    query: Option<&tg::CallbackQuery>,
    ctx: &Context,
  ) {
    if self.set_reminder.is_none() {
      return;
    }

    {
      let set_reminder = self.set_reminder.as_ref().unwrap();
      if !set_reminder.ready() {
        return;
      }

      let reminder = set_reminder.build().unwrap();
      let reminder = Arc::new(RefCell::new(reminder));
      Reminder::settle(reminder.clone(), ctx);

      self.reminders.push(reminder);

      msg.map(|msg| ctx.bot.spawn(msg.text_reply("Reminder set")));
      query.map(|query| {
        ctx.bot.spawn(query.answer("Reminder set"));
      });
    }

    self.set_reminder.take();
  }

  fn delete_reminder(&mut self, reminder: &Reminder) {
    use std::ops::Deref;
    let pred =
      |x: &Arc<RefCell<Reminder>>| x.deref().borrow().deref() == reminder;

    if let Some(loc) = self.reminders.iter().position(pred) {
      self.reminders.remove(loc);
    }
  }
}

impl Reminder {
  fn settle(this: Arc<RefCell<Reminder>>, ctx: &Context) {
    let this = (*this).borrow();

    let duration = Local::now()
      .signed_duration_since(this.remind_at)
      .to_std()
      .unwrap();

    let timeout = reactor::Timeout::new(duration, &ctx.handle).unwrap();
    let bot = ctx.bot.clone();
    let chat_id = this.chat_id.clone();
    let text = format!("It's time for {}", this.content);
    let that = this.clone();

    let future = timeout.then(move |_| {
      let req = tg::SendMessage::new(chat_id, text);
      if !that.borrow().deleted {
        bot.spawn(req);
      }
      ok(())
    });

    ctx.handle.spawn(future);
  }
}
