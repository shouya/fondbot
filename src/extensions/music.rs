use common::*;

use serde_json::Value;

#[derive(Serialize, Deserialize, Default)]
pub struct Music {
  auto_parse: bool,
}

#[derive(Debug)]
struct AudioDetail {
  id: u64,
  title: String,
  composer: Option<String>,
  album: Option<String>,
}

fn parse_song_id(url: &str) -> Option<u64> {
  if !url.starts_with("http") {
    return None;
  }
  Url::parse(url)
    .ok()
    .and_then(|url| {
      if url.host_str() != Some("music.163.com") {
        return None;
      }
      url.fragment().map(|x| x.to_string()).clone()
    })
    .and_then(|fragment| {
      let prefix = "/song?id=";
      if !fragment.starts_with(prefix) {
        return None;
      }

      fragment[prefix.len()..].parse::<u64>().ok()
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
    let api_url = format!("http://music.163.com/api/song/detail/?ids=[{}]", id);
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
      let composer = song["artists"][0]["name"].as_str().map(|x| x.into());
      let album = song["album"][0]["name"].as_str().map(|x| x.into());

      ok(Self {
        id,
        title,
        composer,
        album,
      })
    });
    box object
  }

  fn get_audio_url(&self) -> Box<Future<Item = String, Error = ()>> {
    box ok(format!("https://lain.li/netease_music/{}.mp3", self.id))
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
    msg: tg::Message,
    ctx: Context,
  ) {
    let id = url.map(|x| parse_song_id(&x));
    if id.is_none() {
      if !self.auto_parse {
        ctx.bot.spawn(reply(msg, "Invalid netease url"));
      }
      return;
    }

    let id = id.unwrap();

    let audio_fut = AudioDetail::from_id(id, ctx.handle.clone());
    audio_fut.and_then(|audio| audio)
  }
}
