use std::collections::HashMap;
use std::sync::{Mutex, RwLock};
use std::time::Instant;

use mongodb::{Client, Database, Cursor};
use serde::Serialize;

use mongodb::bson::doc;
use mongodb::options::{ClientOptions, InsertManyOptions};

use crate::util::Measurement;

const TIMESTAMP_INDEX: &str = "timestamp_index";

pub struct TempState {
    data: HashMap<String, Mutex<Vec<Measurement>>>,
    db: Database,
    last_persist: RwLock<Instant>,
    persist_interval: u64,
}

impl TempState {
    pub async fn new(keys: Vec<String>, max_records: usize, db_addr: String, db_name: String, persist_interval: u64)
            -> mongodb::error::Result<Self> {
        let mut db_opts = ClientOptions::parse(&db_addr).await?;
        db_opts.app_name = Some("Thermobug".to_string());
        let client = Client::with_options(db_opts)?;
        let db = client.database(&db_name);

        let mut data = HashMap::new();
        for key in keys {
            db.run_command(doc! {
                "createIndexes": &key,
                "indexes": [{
                    "key": {
                        "timestamp": 1,
                    },
                    "name": TIMESTAMP_INDEX,
                    "unique": true
                }]
            }, None).await?;
            data.insert(key, Mutex::new(Vec::with_capacity(max_records)));
        }

        Ok(Self { data, db, last_persist: RwLock::new(Instant::now()), persist_interval })
    }

    pub async fn update(&self, name: &str, value: u16) -> bool {
        let now = Instant::now();
        if (now - *self.last_persist.read().unwrap()).as_secs() > self.persist_interval {
            let mut last_persist = self.last_persist.write().unwrap();
            // Check if two threads captured it at once
            if (now - *last_persist).as_secs() > self.persist_interval {
                *last_persist = now;
                self.persist().await;
            }
        }

        if let Some(inner) = self.data.get(name) {
            let mut data = inner.lock().unwrap();
            data.push(Measurement::new(value));
            true
        } else {
            false
        }
    }

    pub fn get(&self, name: &str) -> Option<Vec<Measurement>> {
        if let Some(inner) = self.data.get(name) {
            Some(inner.lock().unwrap().clone())
        } else {
            None
        }
    }

    pub async fn get_since(&self, name: &str, timestamp: u64) -> Option<Cursor> {
        if self.data.contains_key(name) {
            let db = self.db.clone();
            let collection = db.collection(name);
            match collection.find(doc! {
                "timestamp": {
                    "$gte": timestamp
                }
            }, None).await {
                Err(e) => {
                    println!("ERROR fetching documents: {:?}", e);
                    None
                },
                Ok(cursor) => Some(cursor)
            }
        } else {
            None
        }
    }

    pub async fn persist(&self) {
        println!("Persisting state to database.");
        let data: HashMap<_, _> = self.data.iter()
            .map(|(key, value)| {
                let mut value = value.lock().unwrap();
                let ret = (key.clone(), value.clone());
                if !ret.1.is_empty() {
                    value.clear();
                    value.push(*ret.1.last().unwrap());
                }
                ret
            })
            .filter(|(_, value)| !value.is_empty())
            .collect();

        let db = self.db.clone();
        // Write to database
        for (key, value) in data.into_iter() {
            let value = value.into_iter()
                .map(|v| doc! { "deg_c": (v.deg_c * 10.0) as u64, "timestamp": v.timestamp })
                .collect::<Vec<_>>();

            let collection = db.collection(&key);
            let opts = InsertManyOptions::builder()
                .ordered(false)
                .build();

            if let Err(e) = collection.insert_many(value, Some(opts)).await {
                let string = format!("{:?}", e);
                if !string.contains("duplicate key error") {
                    println!("ERROR persisting to database: {}", string);
                }
            }
        }
    }
}

#[derive(Serialize)]
pub struct DataResponse {
    success: bool,
    data: Vec<Measurement>
}

impl DataResponse {
    pub fn new(data: Vec<Measurement>) -> Self {
        Self { success: true, data }
    }
}

#[derive(Serialize)]
pub struct Response {
    success: bool,
    message: Option<String>,
}

impl Response {
    pub fn ok() -> Self {
        Response {
            success: true,
            message: None
        }
    }

   pub fn err(msg: &str) -> Self {
        Response {
            success: false,
            message: Some(msg.to_string())
        }
    }
}
