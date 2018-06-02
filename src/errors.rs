use services::request;

error_chain! {
    foreign_links {
        Telegram(::telegram_bot::Error);
        Json(::serde_json::Error);
    }

    links {
        Request(request::Error, request::ErrorKind);
    }

    errors {
        Unknown(desc: String) {
            description("unknown error")
            display("Unknown error occured: {}", desc)
        }
    }
}

