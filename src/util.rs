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

pub fn url_escape(s: &str) -> String {
    s.replace("%", "%25").replace("?", "%3F").replace("&", "%26")
}
