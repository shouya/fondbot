use common::*;

use chrono;
use chrono::Duration;
use chrono::prelude::*;

lazy_static! {
    static ref NOTIFY_INTERVAL: Duration = Duration::seconds(60);
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Afk {
    state: Option<AfkState>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AfkState {
    afk_at: DateTime<Local>,
    reason: Option<String>,
    last_notify: DateTime<Local>,
    user_id: tg::UserId,
    user_name: String,
}

impl Afk {
    fn set_afk(&mut self, msg: &tg::Message, ctx: &Context) {
        let state = AfkState {
            afk_at: Local::now(),
            reason: msg.cmd_arg(),
            last_notify: Local::now() - *NOTIFY_INTERVAL,
            user_id: msg.from.id,
            user_name: ctx.names.get(&msg.from),
        };
        self.state = Some(state);
    }

    fn unset_afk(&mut self) {
        self.state = None;
    }

    fn notification_expired(&self) -> bool {
        assert!(self.is_afk());
        let last_notify = self.state.as_ref().unwrap().last_notify;
        Utc::now().signed_duration_since(last_notify) >= *NOTIFY_INTERVAL
    }

    fn report_afk(&mut self, msg: &tg::Message, ctx: &Context) {
        if !self.is_afk() {
            println!("not afk now");
            return;
        }
        if !self.notification_expired() {
            println!("notify not expired");
            return;
        }

        let state = self.state.as_ref().unwrap();
        let name = &state.user_name;
        let afk_at = Self::format_time(&state.afk_at);
        let duration = Self::now().signed_duration_since(state.afk_at);
        let duration = Self::format_duration(&duration);
        let reason = state.reason.clone().unwrap_or("[not given]".into());

        let txt = format!(
            "{} is *AFK* now.\n\
             AFK set time: _{}, {} ago_\n\
             *Reason*: {}",
            name, afk_at, duration, reason
        );

        ctx.bot.reply_md_to(msg, txt);
    }

    #[allow(unused_must_use)]
    fn format_duration(d: &Duration) -> String {
        let mut str = Vec::new();
        let mut d = d.clone();
        if d.num_days() >= 1 {
            str.push(format!("{} days", d.num_days()));
            d = d - Duration::days(d.num_days());
        }
        if d.num_hours() >= 1 {
            str.push(format!("{} hours", d.num_hours()));
            d = d - Duration::hours(d.num_hours());
        }
        if d.num_minutes() >= 1 {
            str.push(format!("{} mins", d.num_minutes()));
            d = d - Duration::minutes(d.num_minutes());
        }
        if d.num_seconds() >= 1 {
            str.push(format!("{} secs", d.num_seconds()));
        }

        str.join(" ")
    }

    fn format_time<TZ>(t: &DateTime<TZ>) -> String
    where
        TZ: TimeZone,
        TZ::Offset: Display,
    {
        let fmt = t.format("%Y-%m-%d %H:%M:%S");
        format!("{}", fmt)
    }

    fn now() -> DateTime<Utc> {
        chrono::Utc::now()
    }

    fn is_afk(&self) -> bool {
        self.state.is_some()
    }
}

impl BotExtension for Afk {
    fn init(ctx: &Context) -> Self {
        ctx.db.load_conf("afk").unwrap_or_default()
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        if !self.is_afk() && !msg.is_cmd("afk") {
            return;
        }

        if msg.is_cmd("afk") {
            self.set_afk(msg, ctx);
            ctx.bot.reply_to(msg, "Afk set");
            ctx.db.save_conf("afk", &self);
            return;
        }

        if msg.is_cmd("noafk") {
            self.unset_afk();
            ctx.bot.reply_to(msg, "Afk unset");
            ctx.db.save_conf("afk", &self);
            return;
        }

        self.report_afk(msg, ctx);
        ctx.db.save_conf("afk", &self);
        ctx.set_bypass();
    }

    fn report(&self) -> String {
        "this is afk!".to_string()
    }
    fn name(&self) -> &str {
        "afk"
    }
}
