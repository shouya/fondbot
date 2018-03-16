// #![feature(custom_attribute)]
#![feature(iterator_for_each)]
#![feature(conservative_impl_trait)]
#![feature(box_patterns)]
#![feature(associated_type_defaults)]
#![feature(box_syntax)]
#![feature(option_filter)]
#![feature(proc_macro, conservative_impl_trait, generators)]

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

extern crate dotenv;
extern crate slog_async;
extern crate slog_term;

pub extern crate chrono;

pub extern crate futures_await as futures;
// pub extern crate futures;
pub extern crate regex;
pub extern crate serde;
pub extern crate serde_json;
pub extern crate telegram_bot;
pub extern crate tokio_core;

pub extern crate hyper;
pub extern crate hyper_tls;

pub extern crate url;

mod common;
mod context;
mod bot;
mod db;
mod context_extensions;
mod extensions;
mod util;
mod services;

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

  use extensions::*;
  ctx.plug_ext::<history::Saver>();
  ctx.plug_ext::<afk::Afk>();
  ctx.plug_ext::<weather::Weather>();
  ctx.plug_ext::<history::Searcher>();
  ctx.plug_ext::<reminder::ReminderPool>();
  ctx.plug_ext::<music::Music>();

  let serve = futures::lazy(|| {
    info!(logger, "Started serving");
    ok(())
  }).and_then(|_| ctx.serve());

  let future = consume_updates.then(|_| serve);

  core.run(future).ok();
}

#[allow(dead_code)]
fn debug() {
  println!("got: {:?}", 1);
}
