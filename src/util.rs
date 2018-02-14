use std;

pub fn ellipsis(s: &str, trunc_len: usize) -> String {
    let str_len = s.chars().count();
    if str_len < trunc_len {
        return s.into();
    } else {
        format!("{}...", &s.chars().take(trunc_len - 3).collect::<String>())
    }
}

pub fn escape_md(s: &str) -> String {
    s.replace("_", "\\_")
        .replace("[", "\\[")
        .replace("*", "\\*")
        .replace("]", "\\]")
        .replace("(", "\\)")
        .replace(")", "\\)")
}

pub fn auto_retry<F, T, E>(
    expr: F,
    count: Option<u32>,
    interval: Option<std::time::Duration>,
) -> Result<T, E>
    where F: Fn() -> Result<T, E>
{
    use std::u32;
    let mut curr_count = 0;
    let mut result = expr();
    while result.is_err() && curr_count < count.unwrap_or(u32::MAX) {
        std::thread::sleep(interval.unwrap_or(std::time::Duration::from_secs(0)));
        curr_count += 1;
        result = expr();
    }
    result
}
