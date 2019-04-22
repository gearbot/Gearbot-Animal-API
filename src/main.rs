#[macro_use]
extern crate prometheus;

use actix_web::{HttpServer, App, web, web::Json, Result};
use serde::{Deserialize, Serialize};
use rand::prelude::*;
use prometheus::{IntCounter, Registry, TextEncoder, Encoder};

use std::fs;
use std::env;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct CatFact {
    id: i32,
    fact: String
}

struct CatAPIState {
    fact_list: Vec<CatFact>,
    stat_register: Registry,
    req_counter: IntCounter
}

fn get_fact(app_data: web::Data<CatAPIState>) -> Result<Json<CatFact>> {
    let mut rng = thread_rng();
    let fact_list: &Vec<CatFact> = &app_data.fact_list;
    let rand_index = rng.gen_range(0, fact_list.len()); // This should never panic as we will always be in range
    let rand_pick: CatFact = fact_list.get(rand_index).unwrap().to_owned();

    app_data.req_counter.inc();

    Ok(Json(rand_pick))
}

fn prom_stats(app_data: web::Data<CatAPIState>) -> String {
    let register = &app_data.stat_register;

    let mut buffer: Vec<u8> = Vec::with_capacity(1000);
    let encoder = TextEncoder::new();

    let metrics = register.gather();
    encoder.encode(&metrics, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

fn index() -> &'static str {
    "Hello There! This is Gearbot's cat fact API. Head over to /catfact to try it out!"
}

fn main() {
    let cat_fact_count = register_int_counter!("cat_fact_count", "How many cat facts are currently loaded").unwrap();
    let req_count = register_int_counter!("api_request_count", "How many requests we have served").unwrap();

    let reg = Registry::new();
    reg.register(Box::new(cat_fact_count.clone())).unwrap();
    reg.register(Box::new(req_count.clone())).unwrap();

    println!("Loading cat facts!");

    let cat_fact_list: Vec<CatFact> = serde_json::from_str(&fs::read_to_string("cat_facts.json")
        .expect("Cat facts file not found!")).unwrap();

    cat_fact_count.inc_by(cat_fact_list.len() as i64);

    let local_ip = match env::var("LOCAL_IP") { // Easy way to know what interface to bind on
        Ok(ip) => ip,
        Err(_) => "127.0.0.1:8081".to_string()
    };
    println!("Starting API server on: {}", local_ip);

    HttpServer::new(move ||
        App::new().data(CatAPIState {
                fact_list: cat_fact_list.to_owned(),
                stat_register: reg.to_owned(),
                req_counter: req_count.to_owned()
            })
            .service(web::resource("/catfact")
                .route(web::get().to(get_fact)))

            .service(web::resource("/").to(index))
            .service(web::resource("/promstats").to(prom_stats))
    )
    .bind(local_ip).expect("Failed to bind to a port or IP!")
    .run().unwrap();
}
