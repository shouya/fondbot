extern crate diesel;

use self::diesel::prelude::*;
use self::diesel::expression::sql_literal::sql;
use self::diesel::sqlite::SqliteConnection;
use self::diesel::types::{Bool, Text};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

use std::cell::{RefCell, Ref};


pub struct Db {
    conn: RefCell<SqliteConnection>
}

pub fn quote_str(s: &str) -> String {
    s.to_string()
     .replace("'", "''")
     .into()
}

impl Db {
    pub fn init() -> Self {
        // check file
        let conn = SqliteConnection::establish("data.db")
                      .unwrap();
        let db = Db { conn: RefCell::new(conn) };
        db.init_table_config();
        db
    }

    pub fn init_table_config(&self) {
        self.execute_sql(
            "CREATE TABLE IF NOT EXISTS config (
                id INTEGER PRIMARY_KEY ASC,
                key TEXT UNIQUE,
                value TEXT
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

    fn conn_ref(&self) -> Ref<SqliteConnection> {
        self.conn.borrow()
    }

    pub fn execute_sql(&self, s: &str) -> bool {
        sql::<Bool>(s).execute(&*self.conn_ref()).is_ok()
    }
}
