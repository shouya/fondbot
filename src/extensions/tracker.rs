use std;
use common::*;

use std::time::Duration;

type TrackerHandle = Sender<Signal>;

// Signals to control trackers in bg
#[derive(Clone)]
enum Signal {
    Tick, // used by the periodic timer
    Ping, // used to test if the worker is invalidated
    Quit, // used to tell the worker to halt
    Save(Sender<TrackerState>), // used to get a copy of current worker state
    Report,
}

pub struct Tracker {
    trackers: Dict<TrackerHandle>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProgressItem {
    time: String,
    context: String, // info
    location: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Progress {
    nu: String, // tracking no.
    com: String, // express provider
    ischeck: String, // delivered? "0"/"1": no/yes
    data: Vec<ProgressItem>,
}

#[derive(Debug, Clone)]
struct ProgressTracker {
    progress: RefCell<Progress>,
    last_msg_id: Cell<Option<i64>>, // message id of last progress
    ack_len: Cell<usize>, // no. of progress item acknowledged
    chat_id: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TrackerState {
    tracking_no: String,
    last_msg_id: Option<i64>,
    ack_len: usize,
    chat_id: i64,
}

impl BotExtension for Tracker {
    fn new() -> Self {
        Tracker { trackers: Dict::new() }
    }

    fn should_process(&self, msg: &tg::Message, _: &Context) -> bool {
        msg.is_cmds("track untrack list query cleanup")
    }
    fn process(&mut self, msg: &tg::Message, ctx: &Context) {
        match msg.cmd_cmd().unwrap().as_ref() {
            "track" => {
                if let Some(tracking_no) = msg.cmd_arg("track") {
                    self.track(tracking_no, msg);
                } else {
                    ctx.bot.reply_to(msg, "Usage: /track <tracking_no>");
                }
            }
            "untrack" => {
                if let Some(tracking_no) = msg.cmd_arg("untrack") {
                    self.untrack(&tracking_no);
                    let body = format!("Tracker {} killed", &tracking_no);
                    ctx.bot.reply_to(msg, body);
                } else {
                    ctx.bot.reply_to(msg, "Usage: /untrack <tracking_no>");
                }
            }
            "list" => {
                ctx.bot.reply_md_to(msg, self.list());
            }
            "query" => {
                if let Some(tracking_no) = msg.cmd_arg("query") {
                    self.query(tracking_no);
                } else {
                    ctx.bot.reply_to(msg, "Usage: /query <tracking_no>");
                }
            }
            "cleanup" => {
                let reply = format!("{}---\n{} entries removed.", self.list(), self.cleanup());
                ctx.bot.reply_md_to(msg, reply);
            }
            _ => {
                ctx.bot.reply_to(msg, "Invalid usage of tracker plugin");
            }
        }
    }

    fn report(&self) -> String {
        self.name().into()
    }
    fn name(&self) -> &str {
        "tracker"
    }

    fn save(&self) -> JsonValue {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut trackers = Vec::new();
        for (_, handle) in &self.trackers {
            if let Ok(_) = handle.send(Signal::Save(tx.clone())) {
                if let Ok(state) = rx.recv_timeout(Duration::from_millis(300)) {
                    trackers.push(serde_json::to_value(state).unwrap())
                }
            }
        }
        JsonValue::Array(trackers)
    }
    fn load(&mut self, val: JsonValue) {
        if let JsonValue::Array(arr) = val {
            for json in arr {
                let state = serde_json::from_value::<TrackerState>(json).unwrap();
                let no = state.tracking_no.clone();
                let tracker = state.into_tracker().unwrap();
                self.trackers.insert(no, tracker.schedule());
            }
        }
    }
}

const BASE_URL: &'static str = "https://www.kuaidi100.com";


impl Tracker {
    fn track(&mut self, tracking_no: String, msg: &tg::Message) {
        let pt = match ProgressTracker::from_tracking_no(&tracking_no, msg) {
            Ok(pt) => pt,
            Err(msg) => {
                warn!("Failed creating tracking record: {}", msg);
                return;
            }
        };
        let handle = pt.schedule();
        self.trackers.insert(tracking_no, handle);
    }
    fn untrack(&mut self, tracking_no: &String) {
        match self.trackers.get(tracking_no) {
            Some(handle) => handle.send(Signal::Quit).ok(),
            None => {
                warn!("Tracker is not active");
                return;
            }
        };
        self.trackers.remove(tracking_no);
    }
    fn query(&self, tracking_no: String) {
        match self.trackers.get(&tracking_no) {
            Some(handle) => handle.send(Signal::Report).ok(),
            None => {
                warn!("Trying to query invalid tracker");
                return;
            }
        };
    }
    fn cleanup(&mut self) -> usize {
        let before_len = self.trackers.len();
        let dead_trackers: Vec<String> = {
            let iter = self.trackers.iter();
            let deads = iter.filter(|&(_, v)| !Self::is_alive(v));
            deads.map(|(k, _)| k.clone()).collect()
        };

        for no in dead_trackers {
            self.trackers.remove(&no);
        }
        let after_len = self.trackers.len();
        before_len - after_len
    }

    fn list(&mut self) -> String {
        let mut out = format!(">>> Tracking {} items <<<\n", self.trackers.len());
        for (k, v) in &self.trackers {
            let state = if Self::is_alive(v) {
                "`[alive]`"
            } else {
                "`[dead ]`"
            };
            out.push_str(&format!("{}\t{}\n", state, k));
        }
        out
    }

    fn is_alive(th: &TrackerHandle) -> bool {
        th.send(Signal::Ping).is_ok()
    }

    #[allow(non_snake_case)]
    fn query_express_provider(tracking_no: &str) -> Result<String> {
        #[derive(Deserialize, Debug)]
        struct _Auto {
            comCode: String,
        }
        #[derive(Deserialize, Debug)]
        struct _AutoComNum {
            auto: Vec<_Auto>,
        }

        let url = format!("{}/autonumber/autoComNum?text={}", BASE_URL, tracking_no);
        let result: _AutoComNum = try_strerr!(request(&url));
        result.auto
            .first()
            .and_then(|x| Some(x.comCode.clone()))
            .ok_or("Express provider not found".into())
    }

    fn query_express_progress(tracking_no: &str, provider: &str) -> Result<Progress> {
        let url = format!("{}/query?type={}&postid={}",
                          BASE_URL,
                          provider,
                          tracking_no);
        Ok(try_strerr!(request(&url)))
    }
}


// background worker
impl ProgressTracker {
    fn from_tracking_no<T>(no: &str, reply: T) -> Result<ProgressTracker>
        where T: Repliable
    {
        let provider = try!(Tracker::query_express_provider(no));
        let progress = try!(Tracker::query_express_progress(no, &provider));

        Ok(ProgressTracker {
            last_msg_id: Cell::new(reply.message_id()),
            ack_len: Cell::new(0),
            progress: RefCell::new(progress),
            chat_id: reply.chat_id(),
        })
    }

    fn schedule(self) -> TrackerHandle {
        let (tx, rx) = std::sync::mpsc::channel();
        let check_intvl = 5 * 60 * 1000; // 5 min
        Timer::<Signal>::new(check_intvl, tx.clone(), Signal::Tick).tick_forever();

        let owntx = tx.clone();
        std::thread::spawn(move || {
            for signal in rx {
                match signal {
                    Signal::Tick => {
                        self.update_progress();
                        self.report_progress(false);

                        if self.done() {
                            self.report_done();
                            owntx.send(Signal::Quit).unwrap();
                        }
                    }
                    Signal::Quit => break,
                    Signal::Ping => {} // Just Ping this
                    Signal::Save(tx) => {
                        tx.send(TrackerState::from_tracker(&self)).unwrap();
                    }
                    Signal::Report => self.report_progress(true),
                }
            }
        });

        tx
    }

    fn done(&self) -> bool {
        self.progress.borrow().ischeck == "1"
    }

    fn bot(&self) -> Bot {
        Bot::from_default_env()
    }

    fn update_progress(&self) {
        let query = self::Tracker::query_express_progress;
        let mut progress = self.progress.borrow_mut();
        match query(&progress.nu, &progress.com) {
            Ok(new) => {
                *progress = new;
            }
            Err(err) => {
                warn!("failed fetching progress: {:?}", err);
            }
        }
    }

    fn progress_text(&self) -> String {
        let mut text = String::new();
        let ack_len = self.ack_len.get();
        let data = self.progress.borrow().data.clone();
        let (new, old) = data.split_at(data.len() - ack_len);

        for item in old.iter().rev() {
            text.push_str(&format!("`{}` - {}\n", item.time, item.context));
        }
        if new.len() == 0 {
            return text;
        }

        text.push_str("-- _new progress below_ --\n");
        for item in new.iter().rev() {
            text.push_str(&format!("`{}` - {}\n", item.time, item.context));
        }
        text
    }

    fn report_progress(&self, force: bool) {
        if !self.is_updated() && !force {
            return;
        }

        let reply = (self.chat_id, self.last_msg_id.get());
        let body = self.progress_text();

        if let Ok(msg) = self.bot().reply_md_and_get_msg(reply, body) {
            self.last_msg_id.set(Some(msg.message_id));
        }

        self.update_ack_len();
    }

    fn is_updated(&self) -> bool {
        let curr_len = self.progress.borrow().data.len();
        curr_len != self.ack_len.get()
    }

    fn update_ack_len(&self) {
        let curr_len = self.progress.borrow().data.len();
        self.ack_len.set(curr_len);
    }

    fn report_done(&self) {
        let msg = "此快遞已送達，追蹤終止 (使用 /cleanup 指令清除失效的追蹤)";
        let reply = (self.chat_id, self.last_msg_id.get());
        self.bot().reply_to(reply, msg);
    }
}

impl TrackerState {
    fn from_tracker(tracker: &ProgressTracker) -> Self {
        TrackerState {
            last_msg_id: tracker.last_msg_id.get().clone(),
            ack_len: tracker.ack_len.get(),
            chat_id: tracker.chat_id,
            tracking_no: tracker.progress.borrow().nu.clone(),
        }
    }

    fn into_tracker(self) -> Result<ProgressTracker> {
        let builder = ProgressTracker::from_tracking_no;
        let tracker = try!(builder(&self.tracking_no, (self.chat_id, self.last_msg_id)));
        tracker.ack_len.set(self.ack_len);
        Ok(tracker)
    }
}
