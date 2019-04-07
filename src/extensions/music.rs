use common::*;

use curl::easy::Easy;
use serde_json::Value;

#[derive(Serialize, Deserialize, Default)]
pub struct Music {
  auto_parse: bool,
}

#[derive(Debug)]
struct AudioDetail {
  id: u64,
  title: String,
  performer: Option<String>,
  album: Option<String>,
}

#[derive(Fail, Debug)]
pub enum MusicError {
  #[fail(display = "Failed requesting info: {}", _0)]
  Request(RequestError),

  #[fail(display = "Invalid song detail for id={}", id)]
  InvalidSongDetail { id: u64 },
}

fn parse_song_id(url: &str) -> Option<u64> {
  lazy_static! {
    static ref PATTERNS: Vec<Regex> = vec![
      r"http(s?)://music\.163\.com/#/song\?id=(?P<id>\d+).*",
      r"http(s?)://music\.163\.com/song\?id=(?P<id>\d+).*",
      r"http(s?)://music\.163\.com/#/m/song\?id=(?P<id>\d+).*",
      r"http(s?)://music\.163\.com/song/(?P<id>\d+).*/?.*",
    ]
    .into_iter()
    .map(Regex::new)
    .map(|x| x.unwrap())
    .collect();
  }

  for re in PATTERNS.iter() {
    let parsed_id = re
      .captures(url)
      .and_then(|cap| cap["id"].parse::<u64>().ok());
    if let Some(id) = parsed_id {
      return Some(id);
    }
  }

  None
}

impl AudioDetail {
  #[allow(dead_code)]
  fn url(&self) -> String {
    format!("https://music.163.com/#/song?id={}", self.id)
  }

  fn from_id(id: u64) -> impl Future<Item = Self, Error = MusicError> {
    let api_url =
      format!("https://music.163.com/api/song/detail/?ids=[{}]", id);
    let song = request(&api_url)
      .map_err(|e| MusicError::Request(e))
      .and_then(move |value: Value| {
        let song = &value["songs"][0];
        if song.is_object() {
          ok(song.clone())
        } else {
          err(MusicError::InvalidSongDetail { id })
        }
      });

    song.and_then(move |song: Value| {
      let title = song["name"].as_str().unwrap().into();
      let performer = song["artists"][0]["name"].as_str().map(|x| x.into());
      let album = song["album"][0]["name"].as_str().map(|x| x.into());

      ok(Self {
        id,
        title,
        performer,
        album,
      })
    })
  }
}

impl BotExtension for Music {
  fn init(ctx: &Context) -> Self
  where
    Self: Sized,
  {
    ctx
      .db
      .load_conf("music")
      .unwrap_or(Music { auto_parse: true })
  }

  fn process(&mut self, msg: &tg::Message, ctx: &Context) {
    if msg.is_cmd("music") && !self.auto_parse {
      self.handle_message(msg.cmd_arg(), msg, ctx)
    } else {
      self.handle_message(msg.text_content(), msg, ctx)
    }
  }
  fn name(&self) -> &str {
    "music"
  }
}

impl Music {
  fn handle_message(
    &self,
    url: Option<String>,
    msg: &tg::Message,
    ctx: &Context,
  ) {
    let id = url.and_then(|x| parse_song_id(&x));
    if id.is_none() {
      if !self.auto_parse {
        ctx.bot.spawn(reply(msg, "Invalid netease url"));
      }
      return;
    }
    let id = id.unwrap();

    info!(ctx.logger, "Found music query: {}", id);

    let detail_fut = AudioDetail::from_id(id)
      .map_err(ExtensionError::Music)
      .map_err(FondbotError::Extension);
    let file_fut = Self::download(id)
      .map_err(ExtensionError::Music)
      .map_err(FondbotError::Extension);
    let bot = ctx.bot.clone();
    let msg = msg.clone();

    let download_action = bot
      .send(msg.chat.chat_action(tg::ChatAction::RecordAudio))
      .from_err();
    let upload_action = bot
      .send(msg.chat.chat_action(tg::ChatAction::UploadAudio))
      .from_err();

    let download_fut = download_action
      .join3(detail_fut, file_fut)
      .and_then(move |(_, detail, file)| ok((detail, file)));

    let upload_fut = upload_action
      .join(download_fut)
      .and_then(move |(_, (detail, audio_data))| {
        let mut req =
          msg.audio_file_content_reply(audio_data, Some("music.mp3"));
        req.title(detail.title);
        if detail.performer.is_some() {
          req.performer(detail.performer.unwrap());
        }
        bot.send(req).from_err()
      })
      .map_err(|_| ());

    ctx.handle.spawn(upload_fut);
  }

  fn download(id: u64) -> impl Future<Item = Vec<u8>, Error = MusicError> {
    use curl::easy::List;
    let mut curl = Easy::new();
    let mut headers = List::new();

    let url =
      format!("http://music.163.com/song/media/outer/url?id={}.mp3", id);
    curl.url(&url).unwrap();

    headers.append("X-Real-IP: 221.192.199.49").unwrap();

    curl.http_headers(headers).unwrap();
    curl.follow_location(true).unwrap();
    // FnMut(&[u8]) -> Result<usize, WriteError> + Send + 'static
    future::lazy(move || {
      let mut buf = Vec::new();
      {
        let mut transfer = curl.transfer();
        transfer
          .write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
          })
          .unwrap();
        transfer.perform().unwrap();
      }
      ok(buf)
    })
  }

  #[allow(dead_code)]
  fn send_audio<'a>(
    bot_token: String,
    audio: Vec<u8>,
    options: HashMap<&'static str, String>,
  ) {
    use curl::easy::Form;
    let mut form = Form::new();
    let mut curl = Easy::new();

    {
      let mut part = form.part("audio");
      part.buffer("music.mp3", audio);
      part.add().unwrap();
    }

    for (k, v) in options.iter() {
      let mut part = form.part(k);
      part.contents(v.as_bytes());
      part.add().unwrap();
    }

    let url = format!("https://api.telegram.org/bot{}/sendAudio", bot_token);
    curl.url(&url).unwrap();
    curl.httppost(form).unwrap();
    curl.perform().expect("Failed sending audio");
  }
}
