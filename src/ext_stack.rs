use common::*;

pub struct ExtensionStack {
  extensions: Vec<Box<BotExtension>>
}

impl ExtensionStack {
  pub fn new() -> Self {
    ExtensionStack {
      extensions: Vec::new()
    }
  }
  pub fn plug<T>(&mut self, ext: T) where T: BotExtension + 'static {
    self.extensions.push(Box::new(ext));
  }

  pub fn process(&mut self, msg: &tg::Message, ctx: &Context) {
    for ext in &mut self.extensions {
      println!("Checking with plugin: {}", ext.name());
      if ext.should_process(msg, ctx) {
        println!("Processing with plugin: {}", ext.name());
        ext.process(msg, ctx);
      }
    }
  }

  // pub fn report(&self) {
  //   for ext in &self.extensions {
  //     println!("==== Report for {} ====\n{}\n",
  //              ext.name(),
  //              ext.report())
  //   }
  // }

  #[allow(dead_code)]
  pub fn save(&self) -> JsonValue {
    let mut obj = serde_json::Map::new();
    for ext in &self.extensions {
      obj.insert(
        ext.name().into(),
        ext.save()
      );
    }
    JsonValue::Object(obj)
  }

  pub fn load(&mut self, json: JsonValue) {
    if let JsonValue::Object(obj) = json {
      for ext in &mut self.extensions {
        if let Some(ext_json) = obj.get(ext.name().into()) {
          info!("Loading config to extension: {}", ext.name());
          ext.load(ext_json.clone());
        } else {
          info!("Config to extension {} is not available", ext.name());
        }
      }
    }
  }
}
