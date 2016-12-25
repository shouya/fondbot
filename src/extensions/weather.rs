use common::*;

use std;

#[derive(Serialize, Deserialize)]
pub struct Weather {
    weather_loc: Vec<(String, String)>,
}

pub trait WeatherProvider: fmt::Display {
    fn from_query(city: &str, extra: Option<&str>) -> Result<Self> where Self: Sized;
}

impl BotExtension for Weather {
    fn new() -> Self
        where Self: Sized
    {
        Weather { weather_loc: Vec::new() }
    }

    fn should_process(&self, msg: &tg::Message, _: &Context) -> bool {
        msg.is_cmds("weather add_loc del_loc")
    }

    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        match msg.cmd_cmd().unwrap().as_ref() {
            "weather" => self.send_weather_report(msg),
            "add_loc" => {
                let args = msg.cmd_args("add_loc");
                let (city, long_lat) = (args.get(0), args.get(1));
                if city.is_none() || long_lat.is_none() {
                    ctx.bot.reply_to(msg, "Usage /add_loc <city> <long,lat>");
                    return;
                }

                self.weather_loc.push((city.unwrap().clone(), long_lat.unwrap().clone()));
            }
            _ => ctx.bot.reply_to(msg, "Command not recognized"),
        }
    }

    fn name(&self) -> &str {
        "weather"
    }

    fn save(&self) -> JsonValue {
        serde_json::to_value(self)
    }
    fn load(&mut self, val: JsonValue) {
        match serde_json::from_value(val) {
            Ok(val) => *self = val,
            Err(e) => warn!("Failed to restore state for {}: {}", self.name(), e),
        }
    }
}

impl Weather {
    fn send_weather_report(&self, msg: &tg::Message) {
        use std::fmt::Write;
        for &(ref city, ref long_lat) in &self.weather_loc {
            bot().send_typing(msg);
            let mut out = format!("=== Weather Report for {} ===\n", city);
            match Caiyun::from_query(&city, Some(&long_lat)) {
                Ok(w) => write!(out, "{}\n", w).omit(),
                Err(e) => write!(out, "Error: {}\n", e).omit(),
            };
            bot().reply_md_to(msg, out);
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
    aqi: Vec<_CaiyunResultValue<i32>>,
    temperature: Vec<_CaiyunResultValue<i32>>,
}

#[derive(Deserialize)]
struct _CaiyunResult {
    hourly: _CaiyunResultHourly,
}
#[derive(Deserialize)]
struct Caiyun {
    result: _CaiyunResult,
}

const CAIYUN_API_BASE: &'static str = "https://api.caiyunapp.com/v2";

impl fmt::Display for Caiyun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let data = &self.result.hourly;
        let (temp_lo, temp_hi, temp_curr) = lo_hi_curr(&data.temperature).unwrap();
        let (hmd_lo, hmd_hi, _) = lo_hi_curr(&data.humidity).unwrap();
        let skycon = self.compress_skycon();

        write!(f, "*Conditions*: {}\n", skycon).omit();

        write!(f,
               "*Weather*: {}℃ ({}-{}℃)\n",
               temp_curr,
               temp_lo,
               temp_hi)
            .omit();

        write!(f,
               "*Humidity*: {:.2}-{:.2}%\n",
               hmd_lo * 100.0,
               hmd_hi * 100.0)
            .omit();
        write!(f, "*AQI*: {}", self.fmt_aqi()).omit();

        Ok(())
    }
}

impl WeatherProvider for Caiyun {
    fn from_query(_: &str, long_lat: Option<&str>) -> Result<Self> {
        let long_lat = long_lat.unwrap();
        let api_key = std::env::var("CAIYUN_API_KEY").unwrap();
        let url = format!("{}/{}/{}/forecast.json", CAIYUN_API_BASE, api_key, long_lat);
        let mut weather_data: Caiyun = try!(request(&url));

        weather_data.truncate_result();

        Ok(weather_data)
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
        if aqi_lo == 10 && aqi_hi == 10 && aqi_curr == 10 {
            return "<not available>".into();
        }
        format!("*AQI*: {}, {} ({}-{})\n",
                aqi_curr,
                aqi_level(aqi_curr),
                aqi_lo,
                aqi_hi)
    }
}

fn aqi_level(aqi: i32) -> &'static str {
    if 0 <= aqi && aqi < 50 {
        "Good"
    } else if 51 <= aqi && aqi < 100 {
        "Moderate"
    } else if 101 <= aqi && aqi < 150 {
        "Unhealthy for sensitive groups"
    } else if 151 <= aqi && aqi < 200 {
        "Unhealthy"
    } else if 201 <= aqi && aqi < 300 {
        "Very unhealthy"
    } else if 300 <= aqi && aqi < 1000 {
        "Hazardous"
    } else {
        "Meter exploded"
    }
}

fn lo_hi_curr<T>(vec: &Vec<_CaiyunResultValue<T>>) -> Result<(T, T, T)>
    where _CaiyunResultValue<T>: Ord,
          T: Copy
{
    if vec.is_empty() {
        return Err("lo_hi_curr got empty vector".into());
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
use std::cmp::{Ord, PartialOrd, Eq, Ordering};
impl<T> Eq for _CaiyunResultValue<T> where T: PartialEq {}
impl<T> Ord for _CaiyunResultValue<T>
    where _CaiyunResultValue<T>: PartialOrd + Eq
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}
