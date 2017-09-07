use common::*;

#[derive(Debug, Clone, Default)]
pub struct Manager {}

mod config {
    pub fn format_config_item(key: &str, value: &str) -> String {
        format!("Key: [{}]\nValue:\n{}\n", key, value)
    }
}

impl BotExtension for Manager {
    fn init(_: &Context) -> Self {
        Default::default()
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        use std::fmt::Write;
        if msg.is_cmd("list_conf") {
            let confs = ctx.db.list_conf();
            let mut buf = String::new();
            writeln!(&mut buf, "Listing {} config items\n---", confs.len()).ok();
            writeln!(&mut buf, "{}",
                confs.into_iter()
                     .map(|(k,v)| config::format_config_item(&k, &v))
                     .collect::<Vec<String>>()
                     .join("---\n")).ok();
            ctx.bot.reply_to(msg, &buf);
        }
    }

    fn name(&self) -> &str {
        "manager"
    }
}
