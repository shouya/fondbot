pub fn ellipsis(s: &str, trunc_len: usize) -> String {
  let str_len = s.chars().count();
  if str_len < trunc_len {
    return s.into();
  } else {
    format!("{}...", &s.chars().take(trunc_len - 3).collect::<String>())
  }
}
