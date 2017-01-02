#![feature(proc_macro)]
#![feature(custom_attribute)]
#[macro_use]
extern crate serde_derive;
#[macro_use(o, slog_log, slog_trace, slog_debug, slog_info, slog_warn, slog_error)]
extern crate slog;
#[macro_use]
extern crate slog_scope;
extern crate slog_term;
extern crate dotenv;
#[macro_use]
extern crate lazy_static;

mod common;
mod extensions;
mod ext_stack;
mod context;
mod bot;
mod services;
mod utils;

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
            if let Some(mut msg) = u.message {
                msg.clean_cmd();
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
    use slog::DrainExt;
    let drain = slog_term::streamer().build().fuse();
    let root_logger = slog::Logger::root(drain, o![]);
    slog_scope::set_global_logger(root_logger);
    let _ = dotenv::dotenv(); // ignore the result

    let bot = Bot::from_default_env();
    info!("Running as {:?}", bot.get_me());

    info!("Eating up all previous messages!");
    info!("Consumed {} messages", bot.consume_updates());

    let mut ctx = {
        let mut exts = ExtensionStack::new();

        exts.plug(afk::Afk::new());
        exts.plug(tracker::Tracker::new());
        exts.plug(weather::Weather::new());

        Context::new(bot, exts, "state.json".into())
    };

    info!("Loading state");
    ctx.load_state();

    info!("Started serving");
    serve(&mut ctx);
    // exts.process(ctx);
}
