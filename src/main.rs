#![feature(proc_macro)]
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;

mod common;
mod extensions;
mod ext_stack;
mod context;
mod bot;

use common::*;
use extensions::*;
use ext_stack::ExtensionStack;

fn process_message(ctx: &Context, msg: &tg::Message) {
  println!("Got msg: {:?}", msg);
  let mut exts = ctx.exts.borrow_mut();
  exts.process(msg, ctx);
}

fn serve(ctx: &mut Context) {
  let mut listener = {
    let api = &ctx.bot.api;
    api.listener(tg::ListeningMethod::LongPoll(None))
  };

  listener.listen(move |u| {
    if let Some(msg) = u.message {
      process_message(ctx, &msg);
    }
    info!("saving state");
    ctx.save_state();
    Ok(tg::ListeningAction::Continue)
  }).unwrap();
}

fn main() {
  // DEBUG
  std::env::set_var("TELEGRAM_BOT_TOKEN",
                    "167818725:AAHoBuwE2GGU63yrApdk4q-8xYqR8ng0v7w");

  let api = tg::Api::from_env("TELEGRAM_BOT_TOKEN").unwrap();
  println!("Running as {:?}", api.get_me());
  let bot = Bot::new(api);

  eat_updates(&bot);

  let mut ctx = {
    let mut exts = ExtensionStack::new();

    exts.plug(afk::Afk::new());

    Context::new(bot, exts, "state.json".into())
  };

  info!("loading state");
  ctx.load_state();
  serve(&mut ctx);
  // exts.process(ctx);
}

