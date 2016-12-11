use common::*;

pub struct Bot {
  pub api: tg::Api
}


impl Bot {
  pub fn new(api: tg::Api) -> Self {
    Bot { api: api }
  }

  pub fn reply_to<T: Into<String>>(&self, msg: &tg::Message, txt: T) {
    self.api.send_message(msg.chat.id(),          // chat id
                          txt.into(),             // txt
                          None,                   // parse mode
                          None,                   // disable web preview
                          Some(msg.message_id),   // reply to msg id
                          None);                  // reply markup (kbd)
  }

  pub fn reply_markdown_to<T>(&self, msg: &tg::Message, md_txt: T)
    where T: Into<String>
  {
    let as_markdown = Some(tg::ParseMode::Markdown);
    self.api.send_message(msg.chat.id(),        // chat id
                          md_txt.into(),        // txt
                          as_markdown,          // parse mode
                          None,                 // disable web preview
                          Some(msg.message_id), // reply to msg id
                          None);                // reply markup (kbd)
  }
}
