#![feature(custom_attribute)]
#![feature(iterator_for_each)]
#![feature(conservative_impl_trait)]

#[macro_use]
pub extern crate slog;

#[macro_use]
extern crate lazy_static;

extern crate dotenv;
extern crate slog_async;
extern crate slog_term;

pub extern crate chrono;
pub extern crate futures;
pub extern crate serde;
pub extern crate serde_json;
pub extern crate telegram_bot;
pub extern crate tokio_core;

mod common;
mod context;
mod bot;
mod bot_extension;

use common::*;
use context::Context;

const DEBUG: bool = false;

fn main() {
    let mut core = reactor::Core::new().unwrap();

    // DEBUG
    if DEBUG {
        debug();
        return;
    }

    // load env
    dotenv::dotenv().ok();

    // make sure the logger lives long enough
    let logger = {
        use slog_term::*;
        use slog_async::*;
        use slog::*;

        let drain = term_full().fuse();
        let drain = Async::new(drain).build().fuse();

        slog::Logger::root(drain, o!())
    };

    info!(logger, "Initializing bot API");
    let bot = {
        let token = env::var("TELEGRAM_BOT_TOKEN")
            .expect("TELEGRAM_BOT_TOKEN env var not defined");
        tg::Api::configure(token)
            .build(core.handle())
            .expect("Failed building bot API")
    };

    let consume_updates = bot.consume_updates().and_then(|updates| {
        info!(logger, "Consumed previous {} updates", updates.len());
        ok(())
    });

    info!(logger, "Initializing bot context");
    let mut ctx = Context::new(bot.clone(), core.handle(), logger.clone());

    let serve = futures::lazy(|| {
        info!(logger, "Started serving");
        ok(())
    }).and_then(|_| ctx.serve());

    let future = consume_updates.then(|_| serve);

    core.run(future).unwrap();
}

#[allow(dead_code)]
fn debug() {
    println!("got: {:?}", 1);
}
