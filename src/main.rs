#![feature(proc_macro)]
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate dotenv;

mod common;
mod extensions;
mod ext_stack;
mod context;
mod bot;
mod services;

use common::*;
use extensions::*;
use ext_stack::ExtensionStack;

fn process_message(ctx: &Context, msg: &tg::Message) {
    debug!("Got msg: {:?}", msg);
    let mut exts = ctx.exts.borrow_mut();
    exts.process(msg, ctx);
}

fn serve(ctx: &mut Context) {
    let mut listener = {
        ctx.bot.listener(tg::ListeningMethod::LongPoll(None))
    };

    listener.listen(move |u| {
            if let Some(msg) = u.message {
                process_message(ctx, &msg);
            }
            info!("saving state");
            ctx.save_state();
            Ok(tg::ListeningAction::Continue)
        })
        .unwrap();
}

fn main() {
    // DEBUG
    env_logger::init().unwrap();
    let _ = dotenv::dotenv(); // ignore the result

    let bot = Bot::from_default_env();
    info!("Running as {:?}", bot.get_me());

    info!("Eating up all previous messages!");
    info!("Consumed {} messages", bot.consume_updates());

    let mut ctx = {
        let mut exts = ExtensionStack::new();

        exts.plug(afk::Afk::new());
        exts.plug(tracker::Tracker::new());

        Context::new(bot, exts, "state.json".into())
    };

    info!("Loading state");
    ctx.load_state();

    info!("Started serving");
    serve(&mut ctx);
    // exts.process(ctx);
}
