use common::*;
use ext_stack::ExtensionStack;

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;

pub struct Context {
  pub bot: Bot,
  pub exts: RefCell<ExtensionStack>,
  pub save_to: String
}

impl Context {
  pub fn new(bot: Bot, exts: ExtensionStack, loc: String) -> Context {
    Context {
      bot: bot,
      exts: RefCell::new(exts),
      save_to: loc
    }
  }

  pub fn save_state(&self) {
    let exts_val = self.exts.borrow().save();
    let mut file = File::create(Path::new(&self.save_to))
                        .expect("Invalid state filename");
    if let Err(_) = serde_json::ser::to_writer_pretty(&mut file, &exts_val) {
      warn!("error writing context state to file");
    }
  }

  pub fn load_state(&mut self) {
    // TODO: enhance bPath::new(&file))oilerplate
    match File::open(Path::new(&self.save_to)) {
      Ok(f) => match serde_json::de::from_reader(f) {
        Ok(json) => {
          self.exts.borrow_mut().load(json);
          info!("State loaded from json");
        },
        Err(_) => { info!("Invalid state json"); }
      },
      Err(_) =>   { info!("Invalid state json"); }
    }
  }
}
