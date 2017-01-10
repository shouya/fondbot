use common::*;
use ext_stack::ExtensionStack;

use std::cell::RefCell;
use std::fs::File;
use std::path::Path;

pub struct Context {
    pub bot: tg::Api,
    pub exts: RefCell<ExtensionStack>,
    pub save_to: String,
}

impl Context {
    pub fn new(bot: tg::Api, exts: ExtensionStack, loc: String) -> Context {
        Context {
            bot: bot,
            exts: RefCell::new(exts),
            save_to: loc,
        }
    }

    pub fn save_state(&self) {
        let exts_val = self.exts.borrow().save();
        let mut file = File::create(Path::new(&self.save_to)).expect("Invalid state filename");
        if serde_json::ser::to_writer_pretty(&mut file, &exts_val).is_err() {
            warn!("error writing context state to file");
        }
    }

    pub fn load_state(&mut self) {
        File::open(Path::new(&self.save_to))
            .map_err(|e| e.to_string())
            .and_then(|f| serde_json::de::from_reader(f).map_err(|e| e.to_string()))
            .map(|json| {
                self.exts.borrow_mut().load(json);
                info!("State loaded from json");
            })
            .map_err(|e| info!("Invalid state json: {}", e))
            .ok();
    }
}
