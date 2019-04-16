use actix_web::{HttpServer, App, web, web::Json, Result};
use serde::{Deserialize, Serialize};
use rand::prelude::*;

use std::fs;
use std::env;

#[derive(Clone, Serialize, Deserialize, Debug)]
struct CatFact {
    id: i32,
    fact: String
}

struct CatAPIState {
    fact_list: Vec<CatFact>
}

fn get_fact(app_data: web::Data<CatAPIState>) -> Result<Json<CatFact>> {
    let mut rng = thread_rng();
    let fact_list: &Vec<CatFact> = &app_data.fact_list;
    let rand_index = rng.gen_range(0, fact_list.len()); // This should never panic as we will always be in range
    let rand_pick: CatFact = fact_list.get(rand_index).unwrap().to_owned();
    Ok(Json(rand_pick))
}

fn index() -> &'static str {
    "Hello There! This is Gearbot's cat fact API. Head over to /catfact to try it out!"
}

fn main() {
    println!("Loading cat facts!");

    let cat_fact_list: Vec<CatFact> = serde_json::from_str(&fs::read_to_string("cat_facts.json")
        .expect("Cat facts file not found!")).unwrap();

    let local_ip = match env::var("LOCAL_IP") { // Easy way to know what interface to bind on
        Ok(ip) => ip,
        Err(_) => "127.0.0.1:8081".to_string()
    };
    println!("Starting API server on: {}", local_ip);

    HttpServer::new(move ||
        App::new().data(CatAPIState {fact_list: cat_fact_list.to_owned()})
            .service(web::resource("/catfact")
                .route(web::get().to(get_fact)))

            .service(web::resource("/").to(index))
    )
    .bind(local_ip).unwrap()
    .run().unwrap();
}
