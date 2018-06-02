use hyper;

error_chain! {
    foreign_links {
        Telegram(::telegram_bot::Error);
        Hyper(hyper::Error);
        Json(::serde_json::Error);
    }

    errors {
        Unknown(desc: String) {
            description("unknown error")
            display("Unknown error occured: {}", desc)
        }
        RequestError(uri: hyper::Uri) {
            description("request error")
            display("{} responded with a non-200 code", uri)
        }
    }
}

