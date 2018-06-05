use common::*;

use serde_json;
use serde_json::Value as JsonValue;
use std::net::SocketAddr;
use tokio_core::net::TcpStream;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct State {
  name: String,
  brightness: i32,
  color: u32,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Mode {
  name: String,
  commands: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Yeelight {
  addr: Option<SocketAddr>,
  modes: Vec<Mode>,
  current_state: Option<State>,
}

#[derive(Deserialize, Debug)]
pub struct Response {
  id: Option<u32>,
  result: Vec<String>,
}

#[derive(Fail, Debug)]
pub enum Error {
  #[fail(display = "Device IP not present")]
  NotReady,

  #[fail(display = "Network error")]
  Network,

  #[fail(display = "Failed to decode response")]
  Decode(#[cause] serde_json::Error),

  #[fail(display = "Invalid response")]
  Response(String),
}

impl Yeelight {
  fn query_current_state(
    &self,
    handle: &reactor::Handle,
  ) -> impl Future<Item = State, Error = Error> {
    self
      .request(
        "get_prop",
        json!(["name", "brit", "color"]),
        handle,
      )
      .and_then(|resp| {
        if resp.result.len() != 3 {
          return err(Error::Response(format!("{:?}", &resp)));
        }

        let mut vals = resp.result;
        ok(State {
          name: vals.remove(0),
          brightness: vals[0].parse::<i32>().unwrap(),
          color: vals[1].parse::<u32>().unwrap(),
        })
      })
  }

  fn request(
    &self,
    method: &str,
    params: JsonValue,
    handle: &reactor::Handle,
  ) -> impl Future<Item = Response, Error = Error> {
    use std::io::{Read, Write};
    let handle = handle.clone();
    let req_str = serde_json::to_string(&json!({
      "id": 1,
      "method": method,
      "params": params
    })).unwrap();

    self
      .addr
      .ok_or(Error::NotReady)
      .into_future()
      .and_then(move |addr| {
        TcpStream::connect(&addr, &handle).map_err(|_| Error::Network)
      })
      .map(move |mut stream| {
        write!(stream, "{}\r\n", &req_str).ok();
        stream.flush().ok();
        stream
      })
      .and_then(|mut stream| {
        let mut buf = String::new();
        stream.read_to_string(&mut buf).ok();
        drop(stream);
        serde_json::from_str::<Response>(&buf)
          .map_err(|e| Error::Decode(e))
          .into_future()
      })
  }
}

impl BotExtension for Yeelight {
  fn init(ctx: &Context) -> Self {
    let mut core = reactor::Core::new().unwrap();
    let handle = core.handle();
    let mut o: Yeelight = ctx
      .db
      .load_conf("yeelight")
      .unwrap_or(Yeelight {
        addr: Some(
          env::var("YEELIGHT_ADDR")
            .expect("yeelight address not specified.")
            .parse()
            .unwrap(),
        ),
        ..Default::default()
      });

    if let Ok(s) = core.run(o.query_current_state(&handle)) {
      o.current_state = Some(s);
    }

    o
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

impl Response {
  fn is_ok(&self) -> bool {
    true
  }

  fn value(&self) -> Option<&String> {
    self.result.iter().nth(0)
  }
}
