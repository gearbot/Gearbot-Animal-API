#[macro_use]
extern crate prometheus;

use actix_web::{HttpServer, App, web};
use serde::{Deserialize, Serialize};
use prometheus::{IntCounterVec, Registry, TextEncoder, Encoder};

use std::fs;
use std::env;

mod animal_facts;
mod admin;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub struct Perms {
    add: bool,
    delete: bool
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct AuthKeys {
    name: String,
    key: String
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Admin {
    auth: AuthKeys,
    permissions: Perms,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ModifyRequest {
    fact_id: Option<u32>,
    content: Option<String>,
    target: String,
    auth: AuthKeys
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ModifyAction {
    Add,
    Delete
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum AnimalChoice {
    Cat,
    Dog
}


#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Fact {
    id: u32,
    fact: String
}

pub struct APIState {
    cat_fact_list: Vec<Fact>,
    dog_fact_list: Vec<Fact>,
    stat_register: Registry,
    req_counter: IntCounterVec,
    admins: Vec<Admin>
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

    println!("----------------------");
    println!("Loading API Admins...");
    let admins: Vec<Admin> = match fs::read_to_string("admin_keys.json") {
        Ok(key_file) => {
            // This will never panic if you don't mess up the JSON
            let admins: Vec<Admin> = serde_json::from_str(&key_file).unwrap();
            println!("{} admins registered", admins.len());
            if admins.is_empty() { println!("No admins will be able to interact with the API via REST!") }
            admins
        }
        Err(_) => {
            println!("Warning: No keyfile found, no one will be able to interact with the API via REST!");
            vec![] // This will just be ignored later
        }
    };
    println!("----------------------");

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
                req_counter: req_count.to_owned(),
                admins: admins.to_owned()
            })
            .service(web::resource("/cat/fact")
                .route(web::get().to(animal_facts::get_cat_fact)))
            
            .service(web::resource("/dog/fact")
                .route(web::get().to(animal_facts::get_dog_fact)))

            .service(web::resource("/").to(index))
            .service(web::resource("/metrics").to(prom_stats))
            .service(web::resource("/geargrinding/revenge").to(sneaky_secret))

            .service(web::resource("/admin/add")
                .route(web::post().to(admin::admin_modify_fact)))
            .service(web::resource("/admin/delete")
                .route(web::post().to(admin::admin_modify_fact)))
    )
    .bind(local_ip).expect("Failed to bind to a port or IP!")
    .run().unwrap();
}
