use common::*;

use std;
use std::sync::Arc;
use std::sync::Mutex;

use serde_json;
use serde_json::Value as JsonValue;
use std::net::SocketAddr;
use tokio_core::net::TcpStream;

use tg::ToChatRef;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum Power {
  On,
  Off,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
  name: String,
  brightness: u32,
  color: u32,
  color_temperature: u32,
  power: Power,
  updated_at: DateTime<Local>,
}

type Request = Vec<Query>;

#[derive(Serialize, Deserialize, Debug)]
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

  #[fail(display = "Bot API error: {}", _0)]
  Telegram(#[cause] SyncFailure<tg::Error>),

  #[fail(display = "Invalid mode format")]
  ModeFormat,

  #[fail(display = "Mode already exist: {}", _0)]
  ModeAlreadyExist(String),
}

type Result<T> = std::result::Result<T, Error>;

impl Yeelight {
  pub fn query_current_state(
    &self,
  ) -> impl Future<Item = State, Error = Error> {
    self
      .request1(&Query {
        method: "get_prop".into(),
        params: json!(["name", "bright", "rgb", "power", "ct"]),
      })
      .and_then(|resp| {
        if resp.result.len() != 5 {
          return err(Error::Response(format!(
            "Response with incorrect length: {:?}",
            &resp
          )));
        }

        let mut vals = resp.result;

        let name = vals.remove(0);
        let brightness = vals.remove(0).parse::<u32>().unwrap();
        let color = vals.remove(0).parse::<u32>().unwrap();
        let power = match vals.remove(0).as_str() {
          "on" => Power::On,
          _ => Power::Off,
        };
        let color_temperature = vals.remove(0).parse::<u32>().unwrap();
        let updated_at = Local::now();

        ok(State {
          name,
          brightness,
          power,
          color,
          color_temperature,
          updated_at,
        })
      })
  }

  fn refresh_state(&self) -> Box<Future<Item = State, Error = Error>> {
    let state_ref = self.current_state.clone();

    box self.query_current_state().and_then(move |new_state| {
      *state_ref.lock().unwrap() = Some(new_state.clone());
      ok(new_state)
    })
  }

  fn request(
    &self,
    req: &[Query],
  ) -> impl Future<Item = Vec<Response>, Error = Error> {
    use std::io::{Read, Write};

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

  fn request1(&self, q: &Query) -> impl Future<Item = Response, Error = Error> {
    self
      .request(vec![q.clone()].as_slice())
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
  ) -> Box<Future<Item = (), Error = Error>> {
    let req = self
      .modes
      .iter()
      .find(|(mode_name, _)| name == mode_name)
      .map(|(_, req)| req);

    if req.is_none() {
      return box err(Error::Mode(name.into()));
    }

    box self.request(req.unwrap().as_slice()).map(|_| ())
  }

  fn switch_power(
    &self,
    power: Power,
  ) -> Box<Future<Item = (), Error = Error>> {
    let req = vec![Query {
      method: "set_power".into(),
      params: json!([power.to_string(), "smooth", 500]),
    }];
    box self.request(&req).map(|_| ())
  }

  fn show_panel(
    &self,
    chat: tg::ChatRef,
    edit: Option<&tg::Message>,
    bot: &tg::Api,
  ) -> Box<Future<Item = (), Error = Error>> {
    // to be moved into futures
    let bot = bot.clone();
    let state = self.current_state.clone();

    // generate keyboard markup
    let mut markup = tg::InlineKeyboardMarkup::new();
    let functional_row = vec![
      self.callback_button("Refresh", "update"),
      self.callback_button("Turn on", "on"),
      self.callback_button("Turn off", "off"),
    ];
    markup.add_row(functional_row);
    for row in self.render_modes() {
      markup.add_row(row);
    }

    // get current state
    let get_curr_state_fut = future::lazy(move || {
      let state = state.lock().unwrap();
      ok(
        (*state)
          .clone()
          .map(|x| x.report())
          .unwrap_or("Unable to get current state".into()),
      )
    });

    match edit {
      Some(msg) => {
        let msg = msg.clone();
        box (get_curr_state_fut
          .and_then(move |curr_state_str| {
            let mut req = msg.edit_text(curr_state_str);
            req.reply_markup(markup).parse_mode(tg::ParseMode::Markdown);
            bot.send(req)
          })
          .map(|_| ())
          .map_err(SyncFailure::new)
          .map_err(Error::Telegram))
      }
      None => {
        box (get_curr_state_fut
          .and_then(move |curr_state_str| {
            let mut req = chat.text(curr_state_str);
            req.reply_markup(markup).parse_mode(tg::ParseMode::Markdown);
            bot.send(req)
          })
          .map(|_| ())
          .map_err(SyncFailure::new)
          .map_err(Error::Telegram))
      }
    }
  }

  fn render_modes(&self) -> Vec<Vec<tg::InlineKeyboardButton>> {
    let mut rows = Vec::new();
    for chunk in self.modes.chunks(4) {
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

  pub fn add_mode(&mut self, input: &str) -> Result<String> {
    let n = input.find("-").ok_or(Error::ModeFormat)?;
    let (name, mode) = input.split_at(n);
    let name = name.trim();
    let mode = mode.trim_left_matches("-");
    let mode = serde_json::from_str(mode).map_err(Error::Decode)?;
    if self.modes.iter().find(|(n, _)| n == name).is_some() {
      return Err(Error::ModeAlreadyExist(name.into()));
    }
    self.modes.push((name.into(), mode));
    Ok(name.into())
  }

  pub fn default_modes() -> Vec<(String, Request)> {
    let set_prop = |method: &str, value| {
      vec![Query {
        method: method.into(),
        params: json!([value, "smooth", 500]),
      }]
    };
    let inc_prop = |prop| {
      vec![Query {
        method: "set_adjust".into(),
        params: json!(["increase", prop]),
      }]
    };
    let dec_prop = |prop| {
      vec![Query {
        method: "set_adjust".into(),
        params: json!(["decrease", prop]),
      }]
    };

    vec![
      ("Brightest", set_prop("set_bright", 100)),
      ("Brighter", inc_prop("bright")),
      ("Darker", dec_prop("bright")),
      ("Darkest", set_prop("set_bright", 0)),
      ("Coldest", set_prop("set_ct_abx", 1700)),
      ("Colder", dec_prop("ct")),
      ("Warmer", inc_prop("ct")),
      ("Warmest", set_prop("set_ct_abx", 6500)),
    ].into_iter()
      .map(|(a, b)| (a.into(), b))
      .collect()
  }
}

impl BotExtension for Yeelight {
  fn init(ctx: &Context) -> Self {
    let o: Yeelight = ctx.db.load_conf("yeelight").unwrap_or_default();
    ctx.handle.spawn(o.refresh_state().then(|_| ok(())));
    o
  }

  fn process(&mut self, msg: &tg::Message, ctx: &Context) {
    if msg.is_cmd("yeelight") {
      let fut = self.show_panel(msg.chat.to_chat_ref(), None, &ctx.bot);
      ctx.handle.spawn(fut.map_err(|_| ()));
    } else if msg.is_cmd("add_yeelight_mode") {
      msg
        .cmd_arg()
        .ok_or(Error::ModeFormat)
        .and_then(|arg| self.add_mode(&arg))
        .map(|m| {
          ctx.bot.spawn(
            msg.text_reply(format!("Successfully added mode for: {}", m)),
          )
        })
        .map_err(|e| {
          let notice =
            "Usage: /add_yeelight_mode <mode_name> - [<req>, <req>, ...]\n\
             <req>: {\"method\": <method>, \"params\": [<param>, <param>, ...]";
          ctx.bot.spawn(
            msg.text_reply(format!("Failed to add mode: {}\n\n{}", e, notice)),
          )
        })
        .ok();
      ctx.db.save_conf("yeelight", &self);
    } else if msg.is_cmd("del_yeelight_mode") {
      let mode_name = msg.cmd_arg();
      if mode_name.is_none() {
        ctx
          .bot
          .reply_to(msg, "Usage: /del_yeelight_mode <mode_name>");
      } else {
        let mode_name = mode_name.unwrap();
        self
          .modes
          .iter()
          .position(|(name, _)| name.as_str() == mode_name)
          .map(|n| {
            self.modes.remove(n);
            ctx
              .bot
              .reply_to(msg, format!("Successfully removed {}", mode_name))
          })
          .or_else(|| {
            ctx.bot.reply_to(msg, format!("Cannot find {}", mode_name));
            None
          });
      }
      ctx.db.save_conf("yeelight", &self);
    }
  }

  fn process_callback(&mut self, query: &tg::CallbackQuery, ctx: &Context) {
    if query.key().is_none() {
      return;
    }

    let control_fut = match query.key().unwrap() {
      "update" => box ok(()),
      "on" => self.switch_power(Power::On),
      "off" => self.switch_power(Power::Off),
      k => self.switch_to_mode(k.trim_left_matches("mode.")),
    };

    let msg = query.message.clone();
    let chat = msg.chat.to_chat_ref();
    let show_panel_fut = self.show_panel(chat, Some(&msg), &ctx.bot);
    let refresh_fut = self.refresh_state().and_then(|_| show_panel_fut);

    let api1 = ctx.bot.clone();
    let notify_fut = api1.send(query.acknowledge()).then(|_| ok(()));

    let api2 = ctx.bot.clone();
    let query = query.clone();
    let notify_err_fut = move |e: Error| {
      let req = query.answer(format!("Failed: {}", e));
      api2.send(req).then(|_| ok(()))
    };

    let fut = control_fut
      .and_then(|_| notify_fut)
      .and_then(|_| refresh_fut)
      .or_else(notify_err_fut)
      .map(|_| ());

    ctx.handle.spawn(fut);
  }

  fn report(&self) -> String {
    let state = self.current_state();
    if let None = state {
      return "Unable to get current state.".into();
    }
    state.as_ref().unwrap().report()
  }
  fn name(&self) -> &str {
    "yeelight"
  }
}

// impl Response {
//   fn is_ok(&self) -> bool {
//     true
//   }

//   fn value(&self) -> Option<&String> {
//     self.result.iter().nth(0)
//   }
// }

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

impl State {
  fn color_hex(&self) -> String {
    let c = self.color;
    let r = (c >> 16) & 0xff;
    let g = (c >> 8) & 0xff;
    let b = (c >> 0) & 0xff;
    format!("{:02X}{:02X}{:02X}", r, g, b)
  }

  fn report(&self) -> String {
    let power = match self.power {
      Power::On => "on 💡",
      Power::Off => "off 🔌",
    };
    let dur = Local::now().signed_duration_since(self.updated_at);

    format!(
      "{name} is currently powered {power}.\n\
       Color: {color}\n\
       Color Temperature: {color_temperature}K\n\
       Brightness: {brightness}\n\
       Last update: {updated_at} ({dur} ago)",
      name = &self.name,
      power = &power,
      color = &self.color_hex(),
      color_temperature = &self.color_temperature,
      brightness = &self.brightness,
      updated_at = format_time(&self.updated_at),
      dur = format_duration(&dur)
    )
  }
}

impl Default for Yeelight {
  fn default() -> Self {
    Yeelight {
      addr: env::var("YEELIGHT_ADDR").ok().and_then(|x| x.parse().ok()),
      modes: Yeelight::default_modes(),
      current_state: Arc::new(Mutex::new(None)),
    }
  }
}
