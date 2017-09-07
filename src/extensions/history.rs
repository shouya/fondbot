use common::*;
use db::DbMessage;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Saver {
    search_groups: Vec<Integer>
}


fn to_db_message(msg: &tg::Message) -> DbMessage {
    DbMessage {
        msg_id: msg.message_id,
        user_id: msg.from.id,
        chat_id: msg.chat.id(),
        is_group: msg.chat.is_group() || msg.chat.is_supergroup(),
        reply_to_msg_id: msg.reply.as_ref().map(|ref x| x.message_id),
        text: msg.msg_txt(),
        created_at: Some(msg.date)
    }
}

impl BotExtension for Saver {
    fn init(ctx: &Context) -> Self {
        ctx.db.load_conf("history.search_groups").unwrap_or_default()
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        if msg.is_cmd("add_to_search_group") {
            self.search_groups.push(msg.chat.id());
            ctx.db.save_conf("history.search_groups", self);
            ctx.bot.reply_to(msg, format!("Chat {} added to search group",
                                          msg.chat.id()));
            return;
        }

        if !self.search_groups.contains(&msg.chat.id()) {
            return;
        }

        if msg.msg_txt().is_none() {
            // we only want to search text messages
            return;
        }

        ctx.db.save_msg(&to_db_message(msg));
    }

    fn name(&self) -> &str {
        "history_saver"
    }
}
