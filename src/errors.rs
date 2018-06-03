use common::*;
use std::result;
use extensions as ext;

#[derive(Fail, Debug)]
pub enum FondbotError {
    #[fail(display = "Telegram error: {}", _0)]
    Telegram(#[cause] SyncFailure<tg::Error>),

    #[fail(display = "Request error: {}", _0)]
    Request(#[cause] RequestError),

    #[fail(display = "Extension error: {}", _0)]
    Extension(#[cause] ext::ExtensionError),

    #[fail(display = "{}", _0)]
    Message(String)
}

pub type Result<T> = result::Result<T, FondbotError>;


impl From<RequestError> for FondbotError {
    fn from(e: RequestError) -> FondbotError {
        FondbotError::Request(e)
    }
}

impl From<tg::Error> for FondbotError {
    fn from(e: tg::Error) -> FondbotError {
        FondbotError::Telegram(SyncFailure::new(e))
    }
}

impl From<String> for FondbotError {
    fn from(e: String) -> FondbotError {
        FondbotError::Message(e)
    }
}
impl<'a> From<&'a str> for FondbotError {
    fn from(e: &'a str) -> FondbotError {
        FondbotError::Message(e.into())
    }
}

impl From<ext::ExtensionError> for FondbotError {
    fn from(e: ext::ExtensionError) -> FondbotError {
        FondbotError::Extension(e)
    }
}
