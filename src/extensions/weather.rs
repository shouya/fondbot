use common::*;

#[derive(Serialize, Deserialize, Default)]
pub struct Weather {
  weather_loc: HashMap<String, String>,
}

pub trait WeatherProvider: Display {
  fn from_query(
    city: &str,
    extra: Option<&str>,
  ) -> Box<Future<Item = Self, Error = FondbotError>>
  where
    Self: Sized;
}

impl BotExtension for Weather {
  fn init(ctx: &Context) -> Self
  where
    Self: Sized,
  {
    ctx.db.load_conf("weather").unwrap_or_default()
  }

  fn process(&mut self, msg: &tg::Message, ctx: &Context) {
    if msg.cmd_name().is_none() {
      return;
    }
    match msg.cmd_name().unwrap().as_ref() {
      "weather" => self.send_weather_report(msg, ctx),
      "add_loc" => {
        let args = msg.cmd_args();
        let (city, long_lat) = (args.get(0), args.get(1));
        if city.is_none() || long_lat.is_none() {
          ctx.bot.reply_to(msg, "Usage /add_loc <city> <long,lat>");
          return;
        }

        let (city, long_lat) = (city.unwrap(), long_lat.unwrap());

        self.weather_loc.insert(city.clone(), long_lat.clone());
        ctx.db.save_conf("weather", self);
        ctx
          .bot
          .reply_to(msg, format!("Location {} ({}) added.", city, long_lat));
      }
      _ => {}
    }
  }

  fn name(&self) -> &str {
    "weather"
  }
}

impl Weather {
  fn send_weather_report(&self, msg: &tg::Message, ctx: &Context) {
    trace!(ctx.logger, "User requests for weather report");
    trace!(ctx.logger, "Available locs: {:?}", &self.weather_loc);
    use std::fmt::Write;
    for (ref city, ref long_lat) in self.weather_loc.iter() {
      trace!(ctx.logger, "Querying weather for {}", city);
      let waiting = msg.from.chat_action(tg::ChatAction::Typing);
      let mut out = format!("*Weather Report for {}*\n", city);
      let msg = msg.clone();
      let bot = ctx.bot.clone();
      let future =
        Caiyun::from_query(&city, Some(&long_lat)).then(move |result| {
          match result {
            Ok(w) => writeln!(out, "{}", w).ok(),
            Err(e) => writeln!(out, "Error: {}", e).ok(),
          };
          bot.spawn(msg.chat.text(out).parse_mode(Markdown));
          ok(())
        });
      ctx.handle.spawn(
        ctx
          .bot
          .send(waiting)
          .map_err(|_| ())
          .join(future)
          .map(|_| ()),
      );
    }
  }
}

#[derive(Deserialize, PartialEq, PartialOrd)]
struct _CaiyunResultValue<T> {
  value: T,
}

#[derive(Deserialize)]
struct _CaiyunResultHourly {
  skycon: Vec<_CaiyunResultValue<String>>,
  humidity: Vec<_CaiyunResultValue<f32>>,
  aqi: Vec<_CaiyunResultValue<f32>>,
  temperature: Vec<_CaiyunResultValue<f32>>,
}

#[derive(Deserialize)]
struct _CaiyunResult {
  hourly: _CaiyunResultHourly,
}
#[derive(Deserialize)]
struct Caiyun {
  result: _CaiyunResult,
}

const CAIYUN_API_BASE: &str = "https://api.caiyunapp.com/v2";
use std::fmt;

impl Display for Caiyun {
  #[allow(unused_attributes)]
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let data = &self.result.hourly;
    let (temp_lo, temp_hi, temp_curr) = lo_hi_curr(&data.temperature).unwrap();
    let (hmd_lo, hmd_hi, _) = lo_hi_curr(&data.humidity).unwrap();
    let skycon = self.compress_skycon();

    writeln!(f, "*Conditions*: {}", skycon).ok();
    writeln!(
      f,
      "*Temperature*: {:.0}℃ ({:.0}-{:.0}℃)",
      temp_curr, temp_lo, temp_hi
    )
    .ok();
    writeln!(
      f,
      "*Humidity*: {:.0}-{:.0}%",
      hmd_lo * 100.0,
      hmd_hi * 100.0
    )
    .ok();
    write!(f, "*AQI*: {}", self.fmt_aqi()).ok();

    Ok(())
  }
}

impl WeatherProvider for Caiyun {
  fn from_query(
    _: &str,
    long_lat: Option<&str>,
  ) -> Box<Future<Item = Self, Error = FondbotError>> {
    let long_lat = long_lat.unwrap();
    let api_key = env::var("CAIYUN_API_KEY").unwrap();
    let url =
      format!("{}/{}/{}/forecast.json", CAIYUN_API_BASE, api_key, long_lat);

    Box::new(request(&url).from_err().map(|mut weather_data: Self| {
      weather_data.truncate_result();
      weather_data
    }))
  }
}

impl Caiyun {
  // Only keep data for 1 day
  fn truncate_result(&mut self) {
    let hourly = &mut self.result.hourly;
    hourly.aqi.truncate(24);
    hourly.humidity.truncate(24);
    hourly.skycon.truncate(24);
    hourly.temperature.truncate(24);
  }

  fn compress_skycon(&self) -> String {
    let skycon = &self.result.hourly.skycon;
    let mut last = String::new();
    let mut compressed = Vec::new();

    for curr in skycon.iter() {
      let val = &curr.value;
      if last != *val {
        compressed.push(fmt_skycon(&val));
        last = val.clone();
      }
    }

    compressed.join("→")
  }

  fn fmt_aqi(&self) -> String {
    let data = &self.result.hourly;
    let (aqi_lo, aqi_hi, aqi_curr) = lo_hi_curr(&data.aqi).unwrap();
    if aqi_lo == 10.0 && aqi_hi == 10.0 && aqi_curr == 10.0 {
      return "<not available>".into();
    }
    format!(
      "{:.0}, {} ({:.0}-{:.0})\n",
      aqi_curr,
      aqi_level(aqi_curr as i32),
      aqi_lo,
      aqi_hi
    )
  }
}

fn aqi_level(aqi: i32) -> &'static str {
  if 0 >= aqi && aqi < 50 {
    "Good"
  } else if 51 >= aqi && aqi < 100 {
    "Moderate"
  } else if 101 >= aqi && aqi < 150 {
    "Unhealthy for sensitive groups"
  } else if 151 >= aqi && aqi < 200 {
    "Unhealthy"
  } else if 201 >= aqi && aqi < 300 {
    "Very unhealthy"
  } else if 300 >= aqi && aqi < 1000 {
    "Hazardous"
  } else {
    "Meter exploded"
  }
}

fn lo_hi_curr<T>(vec: &Vec<_CaiyunResultValue<T>>) -> Result<(T, T, T)>
where
  _CaiyunResultValue<T>: Ord,
  T: Copy,
{
  if vec.is_empty() {
    return Err(FondbotError::Message("lo_hi_curr got empty vector".into()));
  }

  let curr = vec.first().unwrap().value;
  let lo = vec.iter().min().unwrap().value;
  let hi = vec.iter().max().unwrap().value;

  Ok((lo, hi, curr))
}

fn fmt_skycon(skycon: &String) -> String {
  let skycon_fmted = match skycon.as_str() {
    "CLEAR_DAY" => "☀️",
    "CLEAR_NIGHT" => "☀️🌙",
    "PARTLY_CLOUDY_DAY" => "⛅️",
    "PARTLY_CLOUDY_NIGHT" => "⛅️🌙",
    "CLOUDY" => "☁️",
    "RAIN" => "☔️",
    "SNOW" => "☃",
    "WIND" => "🌬",
    "FOG" => "🌫",
    "HAZE" => "🌫☠",
    "SLEET" => "🌧❄️",
    _ => skycon.as_str(),
  };
  skycon_fmted.into()
}

// Why not derive? because f32 does not implement Ord
use std::cmp::{Eq, Ord, Ordering, PartialOrd};
impl<T> Eq for _CaiyunResultValue<T> where T: PartialEq {}
impl<T> Ord for _CaiyunResultValue<T>
where
  _CaiyunResultValue<T>: PartialOrd + Eq,
{
  fn cmp(&self, other: &Self) -> Ordering {
    self.partial_cmp(other).unwrap_or(Ordering::Equal)
  }
}
