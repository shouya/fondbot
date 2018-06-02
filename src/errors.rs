use services::request;
use extensions as ext;

error_chain! {
    foreign_links {
        Telegram(::telegram_bot::Error);
    }

    links {
        Request(request::err::Error, request::err::ErrorKind);
        Music(ext::music::err::Error, ext::music::err::ErrorKind);
    }

    errors {
        Unknown(desc: String) {
            description("unknown error")
            display("Unknown error occured: {}", desc)
        }
    }
}

