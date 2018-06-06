use common::*;

use serde_json;
use serde_json::Value as JsonValue;
use std::net::SocketAddr;
use tokio_core::net::TcpStream;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum Power {
  On,
  Off,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct State {
  name: String,
  brightness: i32,
  color: u32,
  power: Power,
}

type Request = Vec<Query>;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Yeelight {
  pub addr: Option<SocketAddr>,
  pub modes: HashMap<String, Request>,
  pub current_state: Option<State>,
}

#[derive(Deserialize, Debug)]
pub struct Response {
  result: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
pub struct Query {
  method: String,
  params: JsonValue,
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
  pub fn query_current_state(
    &self,
    handle: &reactor::Handle,
  ) -> impl Future<Item = State, Error = Error> {
    self
      .request1(
        &Query {
          method: "get_prop".into(),
          params: json!(["name", "bright", "rgb", "power"]),
        },
        handle,
      )
      .and_then(|resp| {
        if resp.result.len() != 4 {
          return err(Error::Response(format!(
            "Response with incorrect length: {:?}",
            &resp
          )));
        }

        let mut vals = resp.result;

        let name = vals.remove(0);
        let brightness = vals.remove(0).parse::<i32>().unwrap();
        let color = vals.remove(0).parse::<u32>().unwrap();
        let power = match vals.remove(0).as_str() {
          "on" => Power::On,
          _ => Power::Off,
        };

        ok(State {
          name,
          brightness,
          power,
          color,
        })
      })
  }

  fn request(
    &self,
    req: &[Query],
    handle: &reactor::Handle,
  ) -> impl Future<Item = Vec<Response>, Error = Error> {
    use std::io::{Read, Write};
    let handle = handle.clone();

    let mut conn: Box<
      Future<Item = (TcpStream, Vec<Response>), Error = Error>,
    > = box self
      .addr
      .ok_or(Error::NotReady)
      .into_future()
      .and_then(move |addr| {
        TcpStream::connect(&addr, &handle).map_err(|_| Error::Network)
      })
      .map(|stream| (stream, Vec::new()));

    for q in req {
      let req_str = q.to_string();

      conn = box conn.and_then(move |(mut stream, mut carry)| {
        write!(stream, "{}\r\n", &req_str).ok();
        stream.flush().ok();
        while stream.poll_read().is_not_ready() {}

        let mut buf = String::new();
        stream.read_to_string(&mut buf).ok();
        match serde_json::from_str::<Response>(&buf) {
          Ok(res) => {
            carry.push(res);
            ok((stream, carry))
          }
          Err(e) => err(Error::Decode(e)),
        }
      });
    }

    conn.map(|(_, carry)| carry)
  }

  fn request1(
    &self,
    q: &Query,
    handle: &reactor::Handle,
  ) -> impl Future<Item = Response, Error = Error> {
    self
      .request(vec![q.clone()].as_slice(), handle)
      .and_then(|mut res| {
        if res.is_empty() {
          err(Error::Response("No response received".into()))
        } else {
          ok(res.remove(0))
        }
      })
  }

  fn switch_to_mode(&self, name: &str, ctx: &Context) {}

  fn show_panel(&self, msg: &tg::Message, ctx: &Context) {}
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
    if msg.is_cmd("yeelight") {
      // self.control_panel(msg, ctx);
    } else if msg.is_cmd("set_yeelight_mode") {
      // self.set_mode(msg, ctx);
    } else if msg.is_cmd("del_yeelight_mode") {
      // self.del_mode(msg, ctx);
    }
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

impl ToString for Query {
  fn to_string(&self) -> String {
    serde_json::to_string(&json!({
      "id": 1,
      "method": self.method,
      "params": self.params
    })).unwrap()
  }
}
