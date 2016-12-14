use common::*;
use ext_stack::ExtensionStack;

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;
use std::io;

pub struct Context {
  pub bot: Bot,
  pub exts: RefCell<ExtensionStack>
}

impl Context {
  pub fn new(bot: Bot, exts: ExtensionStack) -> Context {
    Context {
      bot: bot,
      exts: RefCell::new(exts)
    }
  }

  pub fn save_state(&self, file: String) {
    let exts_val = self.exts.borrow().save();
    let mut file = File::create(Path::new(&file))
                        .expect("Invalid state filename");
    serde_json::ser::to_writer_pretty(&mut file, &exts_val);
  }

  pub fn load_state(&mut self, file: String) {
    // TODO: enhance boilerplate
    match File::open(Path::new(&file)) {
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
