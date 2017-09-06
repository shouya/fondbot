use common::*;
use db::DbMessage;

#[derive(Debug, Clone, Default)]
pub struct Saver {}


fn to_db_message(msg: &tg::Message) -> DbMessage {
    DbMessage {
        msg_id: msg.message_id,
        user_id: msg.from.id,
        chat_id: msg.chat.id(),
        reply_to_msg_id: msg.reply.as_ref().map(|ref x| x.message_id),
        text: msg.msg_txt(),
        created_at: Some(msg.date)
    }
}

impl BotExtension for Saver {
    fn init(_: &Context) -> Self {
        Default::default()
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        ctx.db.save_msg(&to_db_message(msg));
    }

    fn name(&self) -> &str {
        "history.saver"
    }
}
