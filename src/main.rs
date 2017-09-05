#![feature(custom_attribute)]
#![feature(iterator_for_each)]
#[macro_use]
extern crate serde_derive;
#[macro_use(o, slog_log, slog_trace, slog_debug, slog_info, slog_warn, slog_error)]
extern crate slog;
#[macro_use] extern crate slog_scope;
extern crate slog_term;
extern crate dotenv;
#[macro_use] extern crate lazy_static;

extern crate serde;
pub extern crate serde_json;

mod common;
mod extensions;
mod ext_stack;
mod context;
mod bot;
mod services;
mod tg_logger;
mod db;

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

    let mut ctx = Context::new(bot);
    ctx.plug_ext::<afk::Afk>();
    ctx.plug_ext::<tracker::Tracker>();
    ctx.plug_ext::<weather::Weather>();
    ctx.plug_ext::<manager::Manager>();

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
