use common::*;

pub struct ExtensionStack {
    pub extensions: Vec<Box<BotExtension>>,
}

impl ExtensionStack {
    pub fn new() -> Self {
        ExtensionStack { extensions: Vec::new() }
    }
    pub fn plug<T>(&mut self, ext: T)
        where T: BotExtension + 'static
    {
        self.extensions.push(Box::new(ext));
    }

    // pub fn report(&self) {
    //   for ext in &self.extensions {
    //     println!("==== Report for {} ====\n{}\n",
    //              ext.name(),
    //              ext.report())
    //   }
    // }
}
