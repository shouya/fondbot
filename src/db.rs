use diesel;
use diesel::prelude::*;
use diesel::expression::sql_literal::sql;
use diesel::sqlite::SqliteConnection;
use diesel::types::{Bool, Text};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

use std::cell::{RefCell, Ref};

const DB_FILE: &'static str = "data.db";

pub struct Db {
    conn: RefCell<SqliteConnection>
}

pub mod schema {
    table! {
        messages (id) {
            id -> Nullable<Integer>,
            msg_id -> BigInt,
            user_id -> BigInt,
            chat_id -> BigInt,
            reply_to_msg_id -> Nullable<BigInt>,
            text -> Nullable<Text>,
            created_at -> Nullable<BigInt>,
        }
    }
}

use self::schema::*;

#[derive(Insertable, Queryable)]
#[table_name="messages"]
pub struct DbMessage {
    pub msg_id: i64,
    pub user_id: i64,
    pub chat_id: i64,
    pub reply_to_msg_id: Option<i64>,
    pub text: Option<String>,
    pub created_at: Option<i64>
}

pub fn quote_str(s: &str) -> String {
    s.to_string()
     .replace("'", "''")
     .into()
}

impl Db {
    pub fn init() -> Self {
        // check file
        let conn = SqliteConnection::establish(DB_FILE)
                      .unwrap();
        let db = Db { conn: RefCell::new(conn) };
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
             );");
    }

    pub fn init_table_messages(&self) {
        self.execute_sql(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY ASC,
                msg_id BIGINT NOT NULL UNIQUE,
                user_id BIGINT NOT NULL,
                chat_id BIGINT NOT NULL,
                reply_to_msg_id BIGINT,
                text TEXT,
                created_at BIGINT
             );");
    }

    pub fn save_conf<T>(&self, key: &str, value: T) where T: Serialize {
        let value_str = serde_json::to_string_pretty(&value).unwrap();
        self.execute_sql(
            &format!("INSERT INTO config (key,value) VALUES('{}', '{}')",
                     quote_str(&key), quote_str(&value_str)))
        || self.execute_sql(
            &format!("UPDATE config SET value = '{}' WHERE key = '{}'",
                     quote_str(&value_str), quote_str(key)));
    }

    pub fn load_conf<T>(&self, key: &str) -> Option<T> where T: DeserializeOwned {
        sql::<Text>(&format!("SELECT value FROM config WHERE key = '{}'",
                             quote_str(key)))
            .get_result::<String>(&*self.conn_ref())
            .ok()
            .and_then(|val_str| serde_json::from_str(&val_str).ok())
    }

    pub fn list_conf(&self) -> Vec<(String, String)> {
        sql::<(Text, Text)>("SELECT key, value FROM config")
            .get_results(&*self.conn_ref())
            .unwrap_or_default()
    }

    pub fn save_msg(&self, msg: &DbMessage) {
        diesel::insert(msg)
            .into(messages::table)
            .execute(&*self.conn_ref());
    }

    pub fn conn_ref(&self) -> Ref<SqliteConnection> {
        self.conn.borrow()
    }

    pub fn execute_sql(&self, s: &str) -> bool {
        sql::<Bool>(s).execute(&*self.conn_ref()).is_ok()
    }
}
