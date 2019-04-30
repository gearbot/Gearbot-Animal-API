#[macro_use]
extern crate prometheus;

use actix_web::{HttpServer, App, web, web::Json, Result};
use serde::{Deserialize, Serialize};
use rand::prelude::*;
use prometheus::{IntCounterVec, Registry, TextEncoder, Encoder};

use std::fs;
use std::env;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Fact {
    id: i32,
    fact: String
}

struct APIState {
    cat_fact_list: Vec<Fact>,
    dog_fact_list: Vec<Fact>,
    stat_register: Registry,
    req_counter: IntCounterVec
}

fn get_cat_fact(app_data: web::Data<APIState>) -> Result<Json<Fact>> {
    let mut rng = thread_rng();
    let fact_list: &Vec<Fact> = &app_data.cat_fact_list;
    let rand_index = rng.gen_range(0, fact_list.len()); // This should never panic as we will always be in range
    let rand_pick: Fact = fact_list.get(rand_index).unwrap().to_owned();

    app_data.req_counter.with_label_values(&["cat"]).inc();

    Ok(Json(rand_pick))
}

fn get_dog_fact(app_data: web::Data<APIState>) -> Result<Json<Fact>> {
    let mut rng = thread_rng();
    let fact_list: &Vec<Fact> = &app_data.dog_fact_list;
    let rand_index = rng.gen_range(0, fact_list.len());
    let rand_pick: Fact = fact_list.get(rand_index).unwrap().to_owned();

    app_data.req_counter.with_label_values(&["dog"]).inc();

    Ok(Json(rand_pick))
}

fn prom_stats(app_data: web::Data<APIState>) -> String {
    let register = &app_data.stat_register;

    let mut buffer: Vec<u8> = Vec::with_capacity(1000);
    let encoder = TextEncoder::new();

    let metrics = register.gather();
    encoder.encode(&metrics, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

fn index() -> &'static str {
    "Hello There! This is Gearbot's animal fact API. Head over to /cat/fact or /dog/fact to try it out!"
}

fn sneaky_secret() -> &'static str {
    "Those silly people grinding my gears! One day... I will strike back..."
}

fn main() {
    let fact_count: IntCounterVec = register_int_counter_vec!("fact_count", "How many animal facts are currently loaded", &["animal"]).unwrap();
    let req_count: IntCounterVec = register_int_counter_vec!("api_request_count", "How many requests we have served", &["animal"]).unwrap();

    let reg = Registry::new();
    reg.register(Box::new(fact_count.clone())).unwrap();
    reg.register(Box::new(req_count.clone())).unwrap();

    println!("Loading animal facts...");

    let cat_fact_list: Vec<Fact> = {
        let facts: Vec<Fact> = serde_json::from_str(&fs::read_to_string("cat_facts.json")
            .expect("Cat facts file not found!")).unwrap();
        
        fact_count.with_label_values(&["cat"]).inc_by(facts.len() as i64);
        println!("Cat facts loaded!");
        facts
    };
        
    let dog_fact_list: Vec<Fact> = {
        let facts: Vec<Fact> = serde_json::from_str(&fs::read_to_string("dog_facts.json")
            .expect("Dog facts file not found!")).unwrap();

        fact_count.with_label_values(&["dog"]).inc_by(facts.len() as i64);
        println!("Dog facts loaded!");
        facts
    };


    let local_ip = match env::var("API_LOCAL_IP") { // Easy way to know what interface to bind on
        Ok(ip) => ip,
        Err(_) => "127.0.0.1:8081".to_string()
    };
    println!("Starting API server on: {}", local_ip);

    HttpServer::new(move ||
        App::new().data(APIState {
                cat_fact_list: cat_fact_list.to_owned(),
                dog_fact_list: dog_fact_list.to_owned(),
                stat_register: reg.to_owned(),
                req_counter: req_count.to_owned()
            })
            .service(web::resource("/cat/fact")
                .route(web::get().to(get_cat_fact)))
            
            .service(web::resource("/dog/fact")
                .route(web::get().to(get_dog_fact)))

            .service(web::resource("/").to(index))
            .service(web::resource("/metrics").to(prom_stats))
            .service(web::resource("/geargrinding/revenge").to(sneaky_secret))
    )
    .bind(local_ip).expect("Failed to bind to a port or IP!")
    .run().unwrap();
}
