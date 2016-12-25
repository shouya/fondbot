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

#[derive(Deserialize)]
struct _CaiyunResult {
    temperature: f32,
    skycon: String,
    aqi: i32,
    humidity: f32,
}
#[derive(Deserialize)]
struct Caiyun {
    result: _CaiyunResult,
}

const CAIYUN_API_BASE: &'static str = "https://api.caiyunapp.com/v2";

impl fmt::Display for Caiyun {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let temp = self.result.temperature.round();
        write!(f, "*Weather*: {}℃, ️{}\n", temp, self.fmt_skycon()).omit();
        write!(f, "*Humidity*: {:.2}%\n", self.result.humidity * 100.0).omit();
        write!(f, "*AQI*: {}, {}\n", self.result.aqi, self.aqi_level()).omit();
        Ok(())
    }
}
impl WeatherProvider for Caiyun {
    fn from_query(_: &str, long_lat: Option<&str>) -> Result<Self> {
        let long_lat = long_lat.unwrap();
        let api_key = std::env::var("CAIYUN_API_KEY").unwrap();
        let url = format!("{}/{}/{}/realtime.json", CAIYUN_API_BASE, api_key, long_lat);

        Ok(try!(request(&url)))
    }
}
impl Caiyun {
    fn fmt_skycon(&self) -> String {
        let skycon = match self.result.skycon.as_str() {
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
            _ => self.result.skycon.as_str(),
        };
        skycon.into()
    }

    fn aqi_level(&self) -> &'static str {
        let aqi = self.result.aqi;
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
}
