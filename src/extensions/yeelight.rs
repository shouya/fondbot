use common::*;

use std::sync::Arc;
use std::sync::Mutex;

use serde_json;
use serde_json::Value as JsonValue;
use std::net::SocketAddr;
use tokio_core::net::TcpStream;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum Power {
  On,
  Off,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
  pub modes: Vec<(String, Request)>,
  #[serde(skip_serializing, skip_deserializing)]
  pub current_state: Arc<Mutex<Option<State>>>,
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

  #[fail(display = "Cannot find mode {}", _0)]
  Mode(String),
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

  fn refresh_state(
    &self,
    handle: &reactor::Handle,
  ) -> Box<Future<Item = (), Error = Error>> {
    let state_ref = self.current_state.clone();

    box self.query_current_state(&handle).and_then(move |new_state| {
      *state_ref.lock().unwrap() = Some(new_state);
      ok(())
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
        TcpStream::connect2(&addr).map_err(|_| Error::Network)
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

  fn switch_to_mode(
    &self,
    name: &str,
    handle: &reactor::Handle,
  ) -> Box<Future<Item = (), Error = Error>> {
    let req = self
      .modes
      .iter()
      .find(|(mode_name, _)| name == mode_name)
      .map(|(_, req)| req);

    if req.is_none() {
      return box err(Error::Mode(name.into()));
    }

    box self.request(req.unwrap().as_slice(), handle).map(|_| ())
  }

  fn switch_power(
    &self,
    power: Power,
    handle: &reactor::Handle,
  ) -> Box<Future<Item = (), Error = Error>> {
    let req = vec![Query {
      method: "set_power".into(),
      params: json!([power.to_string(), "smooth", 500]),
    }];
    box self.request(&req, handle).map(|_| ())
  }

  fn show_panel(&self, msg: &tg::Message) -> tg::SendMessage {
    let mut markup = tg::InlineKeyboardMarkup::new();
    let functional_row = vec![
      self.callback_button("Update state", "update"),
      self.callback_button("Turn on", "on"),
      self.callback_button("Turn off", "off"),
    ];
    markup.add_row(functional_row);
    for row in self.render_modes() {
      markup.add_row(row);
    }

    let mut req = msg.text_reply(self.report());
    req.reply_markup(markup);
    req
  }

  fn render_modes(&self) -> Vec<Vec<tg::InlineKeyboardButton>> {
    let mut rows = Vec::new();
    for chunk in self.modes.chunks(3) {
      let mut row = Vec::new();
      for (name, _) in chunk {
        let key = format!("mode.{}", name);
        row.push(self.callback_button(name, &key));
      }
      rows.push(row);
    }
    rows
  }

  fn current_state(&self) -> Option<State> {
    let rc = self.current_state.clone();
    let locked = rc.lock().unwrap();
    (*locked).clone()
  }
}

impl BotExtension for Yeelight {
  fn init(ctx: &Context) -> Self {
    let mut o: Yeelight = ctx.db.load_conf("yeelight").unwrap_or(Yeelight {
      addr: Some(
        env::var("YEELIGHT_ADDR")
          .expect("yeelight address not specified.")
          .parse()
          .unwrap(),
      ),
      ..Default::default()
    });
    let refresh_fut = o.refresh_state(&ctx.handle.clone());
    ctx.handle.spawn(refresh_fut.map_err(|_| ()));
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

  fn process_callback(&mut self, query: &tg::CallbackQuery, ctx: &Context) {
    if query.key().is_none() {
      return;
    }

    let fut = match query.key().unwrap() {
      "update" => self.refresh_state(&ctx.handle),
      "on" => self.switch_power(Power::On, &ctx.handle),
      "off" => self.switch_power(Power::Off, &ctx.handle),
      k => self.switch_to_mode(k, &ctx.handle),
    };

    unimplemented!()
  }

  fn report(&self) -> String {
    let state = self.current_state();
    if let None = state {
      return "Unable to get current state.".into();
    }
    let state: State = state.unwrap();

    let power = match state.power {
      Power::On => "on (💚)",
      Power::Off => "off",
    };
    format!(
      "💡 [{name}] is currently powered {power}.\n \
       - color: {color}\n \
       - brightness: {brightness}\n",
      name = &state.name,
      power = &power,
      color = &state.color,
      brightness = &state.brightness
    )
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

impl ToString for Power {
  fn to_string(&self) -> String {
    match self {
      Power::On => "on".into(),
      Power::Off => "off".into(),
    }
  }
}
