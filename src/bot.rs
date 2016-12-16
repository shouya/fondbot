use common::*;

pub struct Bot {
  pub api: tg::Api
}


impl Bot {
  pub fn new(api: tg::Api) -> Self {
    Bot { api: api }
  }

  pub fn reply_to<T: Into<String>>(&self, msg: &tg::Message, txt: T) {
    let txt = txt.into();
    while let Err(err) =
      self.api.send_message(msg.chat.id(),          // chat id
                            txt.clone(),            // txt
                            None,                   // parse mode
                            None,                   // disable web preview
                            Some(msg.message_id),   // reply to msg id
                            None) {                 // reply markup (kbd)

      warn!("reply_to failed {}, retrying", err);
    }
  }

  pub fn reply_markdown_to<T>(&self, msg: &tg::Message, md_txt: T)
    where T: Into<String>
  {
    let md_txt = md_txt.into();
    let as_markdown = Some(tg::ParseMode::Markdown);
    while let Err(err) =
      self.api.send_message(msg.chat.id(),        // chat id
                            md_txt.clone(),       // txt
                            as_markdown,          // parse mode
                            None,                 // disable web preview
                            Some(msg.message_id), // reply to msg id
                            None) {               // reply markup (kbd)

      warn!("reply_markdown_to failed {}, retrying", err);
    }
  }
}
