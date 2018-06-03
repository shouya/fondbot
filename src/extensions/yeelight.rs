use common::*;


#[derive(Serialize, Deserialize, Default)]
pub struct State {
  name: String,
  brightness: i32,
  color: (u8, u8, u8)
}

#[derive(Serialize, Deserialize, Default)]
pub struct Mode {
  name: String,
  commands: Vec<String>
}

#[derive(Serialize, Deserialize, Default)]
pub struct Yeelight {
  device_ip: Option<String>,
  modes: Vec<Mode>,
  current_state: Option<State>
}

#[derive(Serialize, Deserialize, Default)]
pub struct YeelightRequest {
  id: Option<u32>, // defaults to 1
  method: String,
  params: Vec<Box<Display>>
}

#[derive(Fail, Debug)]
pub enum Error {
  #[fail(display = "Device IP not present")]
  NotReady
}

impl Yeelight {
  fn query_current_state(&self) -> impl Future<Item=State, Error=Error> {
    let ip = self.device_ip.ok_or(Error::NotReady).into_future();
    let resp = YeelightRequest {
      method: "get_prop",
      params: vec![box "name", box "brit", box "color"]
    };

    unimplemented!()
  }
}

impl BotExtension for Yeelight {
    fn init(ctx: &Context) -> Self {
        ctx.db.load_conf("yeelight").unwrap_or_default()
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        ctx.db.save_conf("yeelight", &self);
    }

    fn report(&self) -> String {
        "this is yeelight stuff!".to_string()
    }
    fn name(&self) -> &str {
        "yeelight"
    }
}
