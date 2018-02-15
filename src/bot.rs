use common::*;

pub trait TgApiExt {
  // This method blocks the main thread
  fn consume_updates<'a>(
    &'a self,
  ) -> Box<Future<Item = Vec<tg::Update>, Error = Box<Error>> + 'a>;
}

impl TgApiExt for tg::Api {
  fn consume_updates<'a>(
    &'a self,
  ) -> Box<Future<Item = Vec<tg::Update>, Error = Box<Error>> + 'a> {
    use futures::future::{loop_fn, Loop};
    let init_state: (tg::Integer, Vec<tg::Update>) = (0, Vec::new());

    Box::new(
      loop_fn(init_state, move |(last, consumed)| {
        let req = tg::GetUpdates::new().offset(last).clone();
        self.send(req).and_then(|batch| {
          if batch.is_empty() {
            Ok(Loop::Break(consumed))
          } else {
            let mut new_consumed = consumed.clone();
            let new_last = batch.last().unwrap().id + 1;
            new_consumed.extend(batch.into_iter());
            Ok(Loop::Continue((new_last, new_consumed)))
          }
        })
      }).from_err()
    )
  }
}
