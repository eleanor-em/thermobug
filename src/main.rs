use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::Serialize;
use dotenv::dotenv;
use std::env;
use std::process::exit;

struct CycleVec<T: Clone> {
    index: usize,
    capacity: usize,
    vec: Vec<T>,
}

impl<T: Clone> CycleVec<T> {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            index: 0,
            capacity,
            vec: Vec::with_capacity(capacity)
        }
    }

    fn push(&mut self, val: T) {
        if self.vec.len() == self.capacity {
            self.vec[self.index] = val;
        } else {
            self.vec.push(val);
        }
        self.index += 1;
        if self.index == self.capacity {
            self.index = 0;
        }
    }

    fn as_inner(&self) -> Vec<T> {
        self.vec.clone()
    }
}

#[derive(Copy, Clone, Serialize)]
struct Measurement {
    deg_c: f32,
    time: u64
}

impl Measurement {
    fn new(deg_c: u16) -> Self {
        Self {
            deg_c: deg_c as f32 / 10.0,
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        }
    }
}

struct TempState {
    data: HashMap<String, Mutex<CycleVec<Measurement>>>,
}

impl TempState {
    fn new(max_records: usize, keys: Vec<String>) -> Self {
        let mut data = HashMap::new();
        for key in keys {
            data.insert(key, Mutex::new(CycleVec::with_capacity(max_records)));
        }
        Self { data }
    }

    fn update(&self, name: &str, value: u16) -> bool {
        if let Some(inner) = self.data.get(name) {
            let mut data = inner.lock().unwrap();
            data.push(Measurement::new(value));
            true
        } else {
            false
        }
    }

    fn get(&self, name: &str) -> Option<Vec<Measurement>> {
        if let Some(inner) = self.data.get(name) {
            Some(inner.lock().unwrap().as_inner())
        } else {
            None
        }
    }
}

#[derive(Serialize)]
struct DataResponse {
    status_code: bool,
    data: Vec<Measurement>
}

impl DataResponse {
    fn new(data: Vec<Measurement>) -> Self {
        Self { status_code: true, data }
    }
}

#[derive(Serialize)]
struct Response {
    status_code: bool,
    status: Option<String>,
}

impl Response {
    fn ok() -> Self {
        Response {
            status_code: true,
            status: None
        }
    }

    fn err(msg: &str) -> Self {
        Response {
            status_code: false,
            status: Some(msg.to_string())
        }
    }
}

#[get("/update/{name}/{temp}")]
async fn update(state: web::Data<TempState>, web::Path((name, temp)): web::Path<(String, u16)>) -> impl Responder {
    if state.update(&name, temp) {
        HttpResponse::Ok().json(Response::ok())
    } else {
        HttpResponse::BadRequest().json(Response::err("Unrecognised data source"))
    }
}

#[get("/data/{name}")]
async fn get_data(state: web::Data<TempState>, web::Path(name): web::Path<String>) -> impl Responder {
    if let Some(data) = state.get(&name) {
        HttpResponse::Ok().json(DataResponse::new(data))
    } else {
        HttpResponse::BadRequest().json(Response::err("Unrecognised data source"))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().expect("Failed to load .env file");

    let mut keys = Vec::new();
    let mut bind_addr = "".to_string();
    let mut max_records = 60 * 60 * 24;

    for (key, value) in env::vars() {
        if key == "THERMOBUG_KEYS" {
            keys = value.split(',')
                .map(|str_ref| str_ref.to_string())
                .collect();
        } else if key == "THERMOBUG_BIND_ADDR" {
            bind_addr = value;
        } else if key == "THERMOBUG_MAX_RECORDS" {
            max_records = value.parse()
                .expect("Incorrectly formatted THERMOBUG_MAX_RECORDS argument.");
        }
    }
    if keys.is_empty() {
        println!("Empty THERMOBUG_KEYS argument.");
        exit(1);
    }
    if bind_addr.is_empty() || !bind_addr.contains(':') {
        println!("Invalid THERMOBUG_BIND_ADDR argument.");
        exit(1);
    }

    println!("Keys:\t\t{:?}", keys);
    println!("Bind address:\t{}", bind_addr);
    println!("Max records:\t{}", max_records);
    println!("Thermobug starting...");

    let state = web::Data::new(TempState::new(max_records, keys.clone()));
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(update)
            .service(get_data)
    })
        .bind(&bind_addr)?
        .run()
        .await
}
