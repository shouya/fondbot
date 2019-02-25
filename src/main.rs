#[macro_use]
pub extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
#[macro_use]
extern crate lazy_static;
#[macro_use]
pub extern crate serde_derive;
#[macro_use]
pub extern crate slog;

pub extern crate failure;
pub extern crate failure_derive;

extern crate dotenv;
extern crate slog_async;
extern crate slog_term;

pub extern crate chrono;
pub extern crate futures;
pub extern crate regex;
pub extern crate serde;
#[macro_use]
pub extern crate serde_json;
pub extern crate telegram_bot;
pub extern crate tokio_core;

pub extern crate curl;
pub extern crate hyper;
pub extern crate hyper_tls;
pub extern crate url;

mod bot;
mod common;
mod context;
mod context_extensions;
mod db;
mod errors;
mod extensions;
mod services;
mod util;

use common::*;
use context::Context;

const DEBUG: bool = false;

const TELEGRAM_DEFAULT_BIND: &'static str = "127.0.0.1:6407";

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
    use slog::*;
    use slog_async::*;
    use slog_term::*;

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
    info!(
      logger,
      "Consumed previous {} updates",
      updates.len()
    );
    ok(())
  });

  info!(logger, "Initializing bot context");
  let mut ctx = Context::new(bot.clone(), core.handle(), logger.clone());

  use extensions::*;
  ctx.plug_ext::<history::Saver>();
  ctx.plug_ext::<afk::Afk>();
  ctx.plug_ext::<weather::Weather>();
  ctx.plug_ext::<history::Searcher>();
  ctx.plug_ext::<reminder::ReminderPool>();
  ctx.plug_ext::<music::Music>();
  ctx.plug_ext::<yeelight::Yeelight>();
  ctx.plug_ext::<link_cleanser::LinkCleanser>();

  let serve = {
    let webhook_callback = env::var("TELEGRAM_WEBHOOK_CALLBACK");
    let bind =
      env::var("TELEGRAM_WEBHOOK_BIND").unwrap_or(TELEGRAM_DEFAULT_BIND.into());

    if let Ok(callback_url) = webhook_callback {
      info!(
        logger,
        "Started serving with webhook at {}, bind on {}", callback_url, bind
      );
      ctx.serve_webhook(&callback_url, &bind)
    } else {
      info!(logger, "Started serving with long polling");
      ctx.serve_poll()
    }
  };

  let future = {
    match env::var("CONSUME_UPDATES") {
      Ok(ref x) if x == "1" => Box::new(consume_updates.then(|_| serve)),
      _ => serve,
    }
  };

  core.run(future).unwrap();
}

#[allow(dead_code)]
fn debug() {
  let mut yee = extensions::yeelight::Yeelight {
    addr: Some("10.144.233.101:55443".parse().unwrap()),
    ..Default::default()
  };
  // let mut core = reactor::Core::new().unwrap();
  let res = yee.add_mode(r#"Test-[{"method": "a","params":["a","b"]}]"#);

  println!("{:?}\n{:?}", res, yee);
}
