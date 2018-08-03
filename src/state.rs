use postgres::error::{Error, UNIQUE_VIOLATION};
use postgres::{Connection, TlsMode};
use std::env;

pub struct State {
    db: Connection,
}

impl State {
    pub fn new() -> Result<State, String> {
        let url = env::var("DATABASE_URL").expect("No DATABASE_URL environment variable set.");
        let db = match Connection::connect(url.as_str(), TlsMode::None) {
            Ok(db) => db,
            Err(error) => return Err(format!("Can't connect to Postgres at {}: {:?}", url, error)),
        };
        match db.execute(
            "CREATE TABLE IF NOT EXISTS state (id VARCHAR PRIMARY KEY)",
            &[],
        ) {
            Ok(_) => (),
            Err(error) => return Err(format!("Can't create Postgres table: {:?}", error)),
        }
        trace!("Connected to database: {}", url);
        Ok(State { db })
    }

    /// Adds a key to the state set, returning true if it was added or false if
    /// it was already present.
    pub fn insert(&mut self, key: u64) -> Result<bool, String> {
        match self
            .db
            .execute("INSERT INTO state (id) VALUES ($1)", &[&format!("{}", key)])
        {
            Ok(_) => Ok(true),
            Err(ref error) if is_dupe(error) => Ok(false),
            Err(error) => Err(format!("Can't insert into Postgres table: {:?}", error)),
        }
    }
}

fn is_dupe(error: &Error) -> bool {
    match error.code() {
        Some(code) if *code == UNIQUE_VIOLATION => true,
        _ => false,
    }
}
