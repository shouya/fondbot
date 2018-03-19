use common::*;

use futures::prelude::*;

use serde_json::Value;
use curl::easy::Easy;

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

fn parse_song_id(url: &str) -> Option<u64> {
  lazy_static! {
    static ref RE: Regex = Regex::new(r"http(s?)://music\.163\.com/.*?/song\?id=(\d+)").unwrap();
  }
  RE.captures(url).and_then(|cap| {
    cap[2].parse::<u64>().ok()
  })
}

impl AudioDetail {
  fn url(&self) -> String {
    format!("https://music.163.com/#/song?id={}", self.id)
  }

  fn from_id(
    id: u64,
    handle: &reactor::Handle,
  ) -> Box<Future<Item = Self, Error = ()>> {
    let api_url =
      format!("https://music.163.com/api/song/detail/?ids=[{}]", id);
    let song = request(handle, &api_url).map_err(|_| ()).and_then(
      move |value: Value| {
        let song = &value["songs"][0];
        if song.is_object() {
          ok(song.clone())
        } else {
          err(())
        }
      },
    );
    let object = song.and_then(move |song: Value| {
      let title = song["name"].as_str().unwrap().into();
      let performer = song["artists"][0]["name"].as_str().map(|x| x.into());
      let album = song["album"][0]["name"].as_str().map(|x| x.into());

      ok(Self {
        id,
        title,
        performer,
        album,
      })
    });
    box object
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

    let detail_fut = AudioDetail::from_id(id, &ctx.handle);
    let file_fut = futures::lazy(move || Self::download(id));
    let bot = ctx.bot.clone();
    let msg = msg.clone();

    let download_action = bot
      .send(msg.chat.chat_action(tg::ChatAction::RecordAudio))
      .map_err(|_| ());
    let upload_action = bot
      .send(msg.chat.chat_action(tg::ChatAction::UploadAudio))
      .map_err(|_| ());

    let download_fut = download_action
      .join3(detail_fut, file_fut)
      .and_then(move |(_, detail, file)| ok((detail, file)));

    let upload_fut = upload_action.join(download_fut).and_then(
      move |(_, (detail, audio_data))| {
        let mut options: HashMap<&'static str, String> = HashMap::new();
        options.insert("chat_id", format!("{}", msg.chat.id()));
        options.insert("title", detail.title);
        options.insert("reply_to_message_id", format!("{}", msg.id));
        if detail.performer.is_some() {
          options.insert("performer", detail.performer.unwrap());
        }

        // currently unable to get token out of bot yet.
        let token = env::var("TELEGRAM_BOT_TOKEN")
          .expect("TELEGRAM_BOT_TOKEN env var not defined");

        Self::send_audio(token, audio_data, options);
        ok(())
      },
    );

    ctx.handle.spawn(upload_fut);
  }

  #[async]
  fn download(id: u64) -> Result<Vec<u8>, ()> {
    use curl::easy::List;
    let mut curl = Easy::new();
    let mut buf = Vec::new();
    let mut headers = List::new();

    let url =
      format!("http://music.163.com/song/media/outer/url?id={}.mp3", id);
    curl.url(&url).unwrap();

    headers.append("X-Real-IP: 36.110.107.162").unwrap();

    curl.http_headers(headers).unwrap();
    curl.follow_location(true).unwrap();
    // FnMut(&[u8]) -> Result<usize, WriteError> + Send + 'static
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
    Ok(buf)
  }

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