error_chain! {
    foreign_links {
        Telegram(::telegram_bot::Error);
    }

    errors {
        Unknown(desc: String) {
            description("unknown error")
            display("Unknown error occured: {}", desc)
        }
    }
}

