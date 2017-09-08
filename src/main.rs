#![feature(custom_attribute)]
#![feature(iterator_for_each)]
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate slog;
#[macro_use] extern crate slog_scope;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate diesel_codegen;
#[macro_use] extern crate diesel;

extern crate slog_term;
extern crate dotenv;

pub extern crate serde;
pub extern crate serde_json;
pub extern crate chrono;

mod db;
mod common;
mod extensions;
mod ext_stack;
mod context;
mod bot;
mod services;
mod tg_logger;
mod util;

use common::*;
use extensions::*;

fn setup_logger() {
    use slog::{DrainExt, Level, LevelFilter};
    let api_token = std::env::var("TELEGRAM_BOT_TOKEN").unwrap();
    let log_channel =
        std::env::var("TELEGRAM_LOG_CHANNEL").unwrap().parse::<i64>().unwrap();
    let tg_drain = tg_logger::TgDrain::new(&api_token, log_channel);
    let tg_drain_filtered = LevelFilter::new(tg_drain, Level::Warning);

    let term_drain = slog_term::streamer().build().fuse();
    let dup_drain = slog::Duplicate::new(tg_drain_filtered, term_drain);
    let root_logger = slog::Logger::root(dup_drain.ignore_err(), o![]);
    slog_scope::set_global_logger(root_logger);
}

const DEBUG: bool = false;

fn main() {
    if DEBUG {
        debug();
        return
    }
    // DEBUG
    dotenv::dotenv().ok();

    setup_logger();

    let bot = Bot::from_default_env();
    info!("Running as {:?}", bot.get_me());

    info!("Eating up all previous messages!");
    info!("Consumed {} messages", bot.consume_updates());

    info!("Initializing bot context");
    let mut ctx = Context::new(bot);
    info!("Initializing plugin stack");
    ctx.plug_ext::<history::Saver>();
    ctx.plug_ext::<afk::Afk>();
    ctx.plug_ext::<history::Searcher>();
    ctx.plug_ext::<tracker::Tracker>();
    ctx.plug_ext::<weather::Weather>();
    ctx.plug_ext::<manager::Manager>();

    info!("Loading safe chats");
    ctx.load_safe_chats_from_env();

    info!("Started serving");
    ctx.serve();
}

#[allow(dead_code)]
fn debug() {
    let db = db::Db::init();
    db.save_conf("a", "\\dd'!@$!@#%#$&$%$)^&)&^*^!#$");
    let v = db.load_conf::<String>("a");
    println!("got: {:?}", v);
}
