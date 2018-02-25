use common::*;

pub fn ellipsis(s: &str, trunc_len: usize) -> String {
  let str_len = s.chars().count();
  if str_len <= trunc_len {
    return s.into();
  } else {
    format!("{}...", &s.chars().take(trunc_len - 3).collect::<String>())
  }
}

pub fn escape_markdown(s: &str) -> String {
  s.replace("_", r"\_")
    .replace("*", r"\*")
    .replace("`", r"\`")
    .into()
}

#[allow(unused_must_use)]
pub fn format_duration(d: &Duration) -> String {
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
  if d.num_seconds() >= 0 {
    str.push(format!("{} secs", d.num_seconds()));
  }
  if d.num_seconds() < 0 {
    str.push("<0 sec".into());
  }

  str.join(" ")
}

pub fn format_time<Tz>(t: &DateTime<Tz>) -> String
where
  Tz: TimeZone,
  Tz::Offset: Display,
{
  let fmt = t.format("%Y-%m-%d %H:%M:%S %z");
  format!("{}", fmt)
}

pub fn format_human_time<Tz>(t: &DateTime<Tz>) -> String
where
  Tz: TimeZone,
  Tz::Offset: Display,
{
  let fmt = t.format("%a %h %e %k:%M:%S");
  format!("{}", fmt)
}
