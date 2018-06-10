use diesel;
use diesel::expression::sql_literal::sql;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use diesel::types::{Bool, Text};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
use std;

const DB_FILE: &'static str = "data.db";

pub struct Db {
  conn: SqliteConnection,
}

pub mod schema {
  table! {
      messages (id) {
          id -> Nullable<Integer>,
          msg_id -> BigInt,
          user_id -> BigInt,
          user_name -> Nullable<Text>,
          chat_id -> BigInt,
          chat_name -> Nullable<Text>,
          is_group -> Bool,
          reply_to_msg_id -> Nullable<BigInt>,
          text -> Nullable<Text>,
          created_at -> Nullable<BigInt>,
      }
  }
}

use self::schema::*;

#[derive(Insertable, Queryable, Serialize, Deserialize, Clone, Debug)]
#[table_name = "messages"]
pub struct DbMessage {
  pub id: Option<i32>,
  pub msg_id: i64,
  pub user_id: i64,
  pub user_name: Option<String>,
  pub chat_id: i64,
  pub chat_name: Option<String>,
  pub is_group: bool,
  pub reply_to_msg_id: Option<i64>,
  pub text: Option<String>,
  pub created_at: Option<i64>,
}

pub fn quote_str(s: &str) -> String {
  s.replace("'", "''")
}

pub const SEARCH_PER: usize = 10;

impl Db {
  pub fn init() -> Self {
    // check file
    let conn = SqliteConnection::establish(DB_FILE).unwrap();
    let db = Db { conn };
    db.init_table_config();
    db.init_table_messages();
    db
  }

  pub fn init_table_config(&self) {
    self.execute_sql(
      "CREATE TABLE IF NOT EXISTS config (
                id INTEGER PRIMARY KEY ASC,
                key TEXT UNIQUE,
                value TEXT
             );",
    );
  }

  pub fn init_table_messages(&self) {
    self.execute_sql(
      "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY ASC,
                msg_id BIGINT NOT NULL,
                user_id BIGINT NOT NULL,
                user_name TEXT NOT NULL,
                chat_id BIGINT NOT NULL,
                chat_name TEXT NOT NULL,
                is_group INT NOT NULL,
                reply_to_msg_id BIGINT,
                text TEXT,
                created_at BIGINT,
                UNIQUE(msg_id, chat_id) ON CONFLICT IGNORE
             );",
    );
  }

  pub fn save_conf<T>(&self, key: &str, value: T)
  where
    T: Serialize,
  {
    let value_str = serde_json::to_string_pretty(&value).unwrap();
    self.execute_sql(&format!(
      "INSERT OR REPLACE INTO config (id, key, value)
      VALUES (
          (SELECT id FROM config WHERE key = '{key}'),
          '{key}',
          '{val}'
      )",
      key=quote_str(&key),
      val=quote_str(&value_str)
    ));
  }

  pub fn load_conf<T>(&self, key: &str) -> Option<T>
  where
    T: DeserializeOwned,
  {
    sql::<Text>(&format!(
      "SELECT value FROM config WHERE key = '{}'",
      quote_str(key)
    )).get_result::<String>(&self.conn)
      .ok()
      .and_then(|val_str| serde_json::from_str(&val_str).ok())
  }

  #[allow(dead_code)]
  pub fn list_conf(&self) -> Vec<(String, String)> {
    sql::<(Text, Text)>("SELECT key, value FROM config")
      .get_results(&self.conn)
      .unwrap_or_default()
  }

  pub fn save_msg(&self, msg: &DbMessage) {
    diesel::insert(msg)
      .into(messages::table)
      .execute(&self.conn)
      .ok();
  }

  pub fn search_msg(
    &self,
    page: usize,
    pattern: &str,
    users: &[i64],
  ) -> (usize, Vec<DbMessage>) {
    if pattern.is_empty() {
      return Default::default();
    }
    let msg_filter_sql = format!("lower(text) LIKE lower('%{}%')", pattern);
    let user_filter_sql = users
      .iter()
      .chain(std::iter::once(&-1))
      .map(|i| format!("user_id = {}", i))
      .collect::<Vec<String>>()
      .join(" OR ");
    let query = messages::table
      .filter(sql(&msg_filter_sql))
      .filter(sql(&format!("({})", user_filter_sql)))
      .order(messages::created_at.desc());
    let count: i64 = query
      .clone()
      .count()
      .get_result(&self.conn)
      .unwrap_or_default();
    print_sql! {query.clone()};
    let result = query
      .offset(((page - 1) * SEARCH_PER) as i64)
      .limit(SEARCH_PER as i64)
      .load(&self.conn)
      .unwrap_or_default();
    (count as usize, result)
  }

  pub fn execute_sql(&self, s: &str) -> bool {
    sql::<Bool>(s).execute(&self.conn).is_ok()
  }
}
