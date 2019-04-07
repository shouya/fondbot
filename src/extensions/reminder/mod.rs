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
pub struct ReminderPool {
  reminders: RemindersType,
  set_reminder: Option<SetReminder>,
  deletion: Option<Vec<Reminder>>,
  listing_message: Arc<RefCell<Option<tg::Message>>>,
}

impl BotExtension for ReminderPool {
  fn init(ctx: &Context) -> Self
  where
    Self: Sized,
  {
    let now = Local::now();
    let reminders: RemindersType = ctx
      .db
      .load_conf::<Vec<Reminder>>("reminders")
      .unwrap_or(Vec::new())
      .into_iter()
      .filter(|x| x.remind_at >= now && !x.deleted)
      .map(|rem| Arc::new(RefCell::new(rem)))
      .collect();

    for rem in reminders.iter() {
      Reminder::settle(rem.clone(), ctx);
    }

    ReminderPool {
      set_reminder: None,
      deletion: None,
      listing_message: Arc::new(RefCell::new(None)),
      reminders,
    }
  }

  fn process(&mut self, message: &tg::Message, ctx: &Context) {
    if message.is_cmd("remind_me") {
      self.set_reminder = Some(SetReminder::init(message, ctx));
      self.set_reminder.as_mut().unwrap().on_message(message, ctx);
      self.settle_new_reminder(Some(message), None, ctx);
    } else if message.is_reply_to_bot() && self.set_reminder.is_some() {
      self.set_reminder.as_mut().unwrap().on_message(message, ctx);
      self.settle_new_reminder(Some(message), None, ctx);
    } else if message.is_cmd("list_reminders") {
      self.list_reminders(message, false, ctx);
    } else if self.deletion.is_some() && message.is_cmd_prefix("del_") {
      self.delete_reminder(message, ctx);
    }
  }
  fn process_callback(&mut self, query: &tg::CallbackQuery, ctx: &Context) {
    if self.set_reminder.is_some() {
      self.set_reminder.as_mut().unwrap().on_callback(query, ctx);
      self.settle_new_reminder(None, Some(query), ctx);
    }
  }
  fn name(&self) -> &str {
    "reminder"
  }
}

impl ReminderPool {
  fn list(&self) -> Vec<Reminder> {
    self
      .reminders
      .iter()
      .map(|x| x.deref().borrow().clone())
      .filter(|x| !x.deleted)
      .collect::<Vec<_>>()
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
    self.save(ctx);
  }

  fn list_reminders(
    &mut self,
    msg: &tg::Message,
    edit_mode: bool,
    ctx: &Context,
  ) {
    let reminders = self.list();
    let mut text = String::new();

    writeln!(text, "Reminders ({})\n----------", reminders.len()).ok();
    for (i, rem) in reminders.iter().enumerate() {
      writeln!(
        text,
        "{}: {} (/del_{})",
        format_time(&rem.remind_at),
        rem.content,
        i
      )
      .ok();
    }
    if reminders.is_empty() {
      writeln!(text, "no reminders").ok();
    }

    let slot = self.listing_message.clone();
    self.deletion = Some(reminders);

    if edit_mode {
      ctx.bot.spawn(msg.edit_text(text))
    } else {
      let future = ctx.bot.send(msg.chat.text(text));
      let future = future
        .map(move |listing_msg| {
          (*slot.as_ref().borrow_mut()) = Some(listing_msg);
        })
        .map_err(|_| ());

      ctx.handle.spawn(future);
    }
  }

  fn delete_reminder(&mut self, msg: &tg::Message, ctx: &Context) {
    {
      let n: usize = msg.cmd_suffix("del_").unwrap().parse().unwrap();
      let reminder = if let &Some(rem) =
        &self.deletion.as_ref().unwrap().into_iter().nth(n)
      {
        rem
      } else {
        let req = msg.chat.text("Invalid index, please try another one");
        ctx.bot.spawn(req);
        return;
      };

      let pred =
        |x: &Arc<RefCell<Reminder>>| x.deref().borrow().deref() == reminder;

      if let Some(loc) = self.reminders.iter().position(pred) {
        self.reminders.remove(loc);
      }
    }

    self.save(ctx);
    let listing_msg = self.listing_message.deref().borrow().clone();
    listing_msg.map(|msg| {
      self.list_reminders(&msg, true, ctx);
    });
  }

  fn save(&self, ctx: &Context) {
    let reminders = self
      .reminders
      .iter()
      .map(|x| x.deref().borrow().clone())
      .filter(|x| !x.deleted)
      .collect::<Vec<_>>();
    ctx.db.save_conf("reminders", reminders);
  }
}

impl Reminder {
  fn settle(this: Arc<RefCell<Reminder>>, ctx: &Context) {
    let that = this.clone();
    let this = this.deref().borrow();

    let duration = this
      .remind_at
      .signed_duration_since(Local::now())
      .to_std()
      .unwrap();

    let timeout = reactor::Timeout::new(duration, &ctx.handle).unwrap();
    let bot = ctx.bot.clone();

    let future = timeout.then(move |_| {
      let mut this = that.deref().borrow_mut();
      this.deref_mut().send_alert(&bot);
      ok(())
    });

    ctx.handle.spawn(future);
  }

  fn describe(&self) -> String {
    let now = Local::now();
    let mut output = Vec::new();

    output.push(format!("It's time for {}", self.content));
    output.push(format!(
      "Set at: {} ({} ago)",
      format_time(&self.set_at),
      format_duration(&now.signed_duration_since(self.set_at))
    ));
    output.push(format!("Alert at: {}", format_time(&self.remind_at)));

    output.join("\n")
  }

  fn send_alert(&mut self, bot: &tg::Api) {
    let req = tg::SendMessage::new(self.chat_id, self.describe())
      .reply_to(self.message_id)
      .clone();

    if !self.deleted {
      bot.spawn(req);
      self.deleted = true;
    }
  }
}
