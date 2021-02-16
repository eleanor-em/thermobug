use std::env;
use std::iter::Iterator;
use futures::stream::StreamExt;
use std::process::exit;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;

use thermobug::app::{DataResponse, Response, TempState};
use thermobug::util::Measurement;

#[get("/update/{name}/{temp}")]
async fn update(state: web::Data<TempState>, web::Path((name, temp)): web::Path<(String, u16)>) -> impl Responder {
    if state.update(&name, temp).await {
        HttpResponse::Ok().json(Response::ok())
    } else {
        HttpResponse::BadRequest().json(Response::err("unrecognised data source"))
    }
}

#[get("/recent/{name}")]
async fn get_recent_data(state: web::Data<TempState>, web::Path(name): web::Path<String>) -> impl Responder {
    if let Some(data) = state.get(&name) {
        HttpResponse::Ok().json(DataResponse::new(data))
    } else {
        HttpResponse::BadRequest().json(Response::err("unrecognised data source"))
    }
}

#[get("/all/{name}/since/{timestamp}")]
async fn get_data_since(state: web::Data<TempState>, web::Path((name, timestamp)): web::Path<(String, u64)>) -> impl Responder {
    if let Some(mut cursor) = state.get_since(&name, timestamp).await {
        let mut data = Vec::new();
        while let Some(Ok(doc)) = cursor.next().await {
            let deg_c = doc.get_i64("deg_c").unwrap() as f32 / 10.0;
            let timestamp = doc.get_i64("timestamp").unwrap() as u64;
            data.push(Measurement {
                deg_c, timestamp
            });
        }
        HttpResponse::Ok().json(DataResponse::new(data))
    } else {
        HttpResponse::BadRequest().json(Response::err("unrecognised data source"))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Load configuration
    if let Err(_) = dotenv() {
        println!("WARN: failed to load .env file");
    }

    let mut keys = Vec::new();
    let mut bind_addr = "".to_string();
    let mut max_records = 60 * 60 * 24;
    let mut workers = num_cpus::get();
    let mut db_addr = "mongodb://localhost:27017".to_string();
    let mut db_name = "thermobug".to_string();
    let mut persist_interval: u64 = 60 * 60;

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
        } else if key == "THERMOBUG_WORKER_THREADS" {
            workers = value.parse()
                .expect("Incorrectly formatted THERMOBUG_WORKER_THREADS argument.");
        } else if key == "THERMOBUG_DB_ADDR" {
            db_addr = value;
        } else if key == "THERMOBUG_DB_NAME" {
            db_name = value;
        } else if key == "THERMOBUG_PERSIST_INTERVAL" {
            persist_interval = value.parse()
                .expect("Incorrectly formatted THERMOBUG_PERSIST_INTERVAL argument.");
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

    println!("Keys:\t\t\t{:?}", keys);
    println!("Bind address:\t\t{}", bind_addr);
    println!("Max records:\t\t{}", max_records);
    println!("Worker threads:\t\t{}", workers);
    println!("Database address:\t{}", db_addr);
    println!("Database name:\t\t{}", db_name);
    println!("Persist interval:\t{} seconds", persist_interval);
    println!("Thermobug starting...");

    let state = TempState::new(keys.clone(),
                               max_records,
                               db_addr,
                               db_name,
                               persist_interval)
        .await
        .expect("Failed to initialise web app");
    let state = web::Data::new(state);

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .service(update)
            .service(get_recent_data)
            .service(get_data_since)
            .default_service(web::to(|| HttpResponse::NotFound().json(Response::err("path not found"))))
    })
        .workers(workers)
        .bind(&bind_addr)
        .expect("Failed to start server.")
        .run()
        .await
}
