use common::*;
extern crate chrono;

use self::chrono::{DateTime, Duration, UTC, TimeZone, FixedOffset};

#[inline]
fn notify_interval() -> Duration {
    Duration::seconds(60)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Afk {
    who: Option<String>,
    afk_at: Option<DateTime<UTC>>,
    reason: Option<String>,
    last_notify: Option<DateTime<UTC>>,
}

impl Afk {
    fn set_afk(&mut self, msg: &tg::Message) {
        self.who = Some(user_name(&msg.from));
        self.afk_at = Some(Self::now());
        self.reason = cmd_arg(msg, "afk");
        self.last_notify = Some(Self::now() - notify_interval());
    }

    fn unset_afk(&mut self) {
        self.afk_at = None;
    }

    fn notification_expired(&self) -> bool {
        assert!(self.is_afk());
        Self::now() - self.last_notify.unwrap() >= notify_interval()
    }

    fn report_afk(&mut self, msg: &tg::Message, bot: &Bot) {
        if !self.is_afk() {
            println!("not afk now");
            return;
        }
        if !self.notification_expired() {
            println!("notify not expired");
            return;
        }

        let who = self.who.clone().unwrap_or("Somebody".into());
        let afk_at = Self::format_time(&self.afk_at.unwrap());
        let duration = Self::format_duration(&(Self::now() - self.afk_at.unwrap()));
        let reason = self.reason.clone().unwrap_or("[not given]".into());

        let txt = format!("{} is *AFK* now.\nAFK set time: _{}, {} ago_\n*Reason*: {}",
                          who,
                          afk_at,
                          duration,
                          reason);

        bot.reply_markdown_to(msg, txt);
    }

    #[allow(unused_must_use)]
    fn format_duration(d: &Duration) -> String {
        use std::fmt::Write;
        let mut str = String::new();
        if d.num_days() >= 1 {
            write!(&mut str, "{} days", d.num_days());
        }
        if d.num_hours() >= 1 {
            write!(&mut str, "{} hours", d.num_hours());
        }
        if d.num_minutes() >= 1 {
            write!(&mut str, "{} mins", d.num_minutes());
        }
        if d.num_seconds() >= 1 {
            write!(&mut str, "{} secs", d.num_seconds());
        }
        str
    }

    fn format_time<TZ: TimeZone>(t: &DateTime<TZ>) -> String {
        let tz = FixedOffset::east(8 * 60 * 60);
        let fmt = t.with_timezone(&tz).format("%Y-%m-%d %H:%M:%S");
        format!("{}", fmt)
    }

    fn now() -> DateTime<UTC> {
        chrono::UTC::now()
    }

    fn is_afk(&self) -> bool {
        self.afk_at.is_some()
    }
}


impl BotExtension for Afk {
    fn new() -> Self {
        Afk {
            who: None,
            afk_at: None,
            reason: None,
            last_notify: None,
        }
    }

    fn should_process(&self, msg: &tg::Message, _: &Context) -> bool {
        if self.is_afk() {
            return true;
        }
        is_cmd(msg, "afk")
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        if is_cmd(msg, "afk") {
            self.set_afk(msg);
            ctx.bot.reply_to(msg, "Afk set");
            return;
        }

        if is_cmd(msg, "noafk") {
            self.unset_afk();
            ctx.bot.reply_to(msg, "Afk unset");
            return;
        }

        self.report_afk(msg, &ctx.bot);
    }

    fn report(&self) -> String {
        "this is afk!".to_string()
    }
    fn name(&self) -> &str {
        "afk"
    }

    fn save(&self) -> JsonValue {
        serde_json::to_value(self)
    }
    fn load(&mut self, val: JsonValue) {
        *self = serde_json::from_value(val).unwrap()
    }
}
