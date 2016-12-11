use common::*;
use ext_stack::ExtensionStack;

use std::cell::RefCell;

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
}
