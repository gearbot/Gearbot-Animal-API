#[macro_use]
extern crate prometheus;

use actix_web::{HttpRequest, HttpServer, App, web, web::Json, Result};
use actix_web::HttpResponse;

use serde::{Deserialize, Serialize};
use rand::prelude::*;
use prometheus::{IntCounterVec, Registry, TextEncoder, Encoder};

use std::fs;
use std::env;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
struct Perms {
    add: bool,
    delete: bool
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
struct AuthKeys {
    name: String,
    key: String
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
struct Admin {
    auth: AuthKeys,
    permissions: Perms,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct ModifyRequest {
    fact_id: Option<u32>,
    content: Option<String>,
    target: String,
    auth: AuthKeys
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
enum ModifyAction {
    Add,
    Delete
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
enum AnimalChoice {
    Cat,
    Dog
}


#[derive(Clone, Serialize, Deserialize, Debug)]
struct Fact {
    id: u32,
    fact: String
}

struct APIState {
    cat_fact_list: Vec<Fact>,
    dog_fact_list: Vec<Fact>,
    stat_register: Registry,
    req_counter: IntCounterVec,
    admins: Vec<Admin>
}

fn check_admin_perms(unchecked_auth: AuthKeys, admin_list: Vec<Admin>) -> (bool, Option<Perms>) {
    match admin_list.iter().find(|admin| admin.auth == unchecked_auth) {
        Some(admin) => {
            if admin.permissions.add || admin.permissions.delete { // If they have no perms, say it
                (true, Some(
                    Perms {
                        add: admin.permissions.add,
                        delete: admin.permissions.delete
                    }
                ))
            } else {
                (true, None)
            }
        }
        None => (false, None)
    }
}

fn admin_modify_fact(req: HttpRequest, body: Json<ModifyRequest>) -> HttpResponse {
    let state: web::Data<APIState> = req.to_owned().app_data().unwrap();

    fn modify_fact(request: ModifyRequest, mut fact_list: Vec<Fact>, action: ModifyAction, animal: AnimalChoice, perms: Perms) -> HttpResponse {
        if action == ModifyAction::Add && perms.add {
            let last_id = match fact_list.last() {
                Some(fact) => fact.id,
                None => 0 // It could be a brand new JSON file.. somehow...
            };

            match request.content {
                Some(content) => {
                    fact_list.push(
                        Fact {
                            id: last_id + 1,
                            fact: content
                        }
                    );
                }

                None => return HttpResponse::BadRequest().body("NO \"content\" SPECIFIED")
            }

            match animal {
                AnimalChoice::Cat => {
                    fs::write("cat_facts.json", serde_json::to_string_pretty(&fact_list).unwrap()).unwrap();
                    HttpResponse::Created().body("CAT FACT ADDED")
                }

                AnimalChoice::Dog => {
                    fs::write("dog_facts.json", serde_json::to_string_pretty(&fact_list).unwrap()).unwrap();
                    HttpResponse::Created().body("DOG FACT ADDED")
                }
           }
            
        } else if action == ModifyAction::Delete && perms.delete { // TODO: REPLICATE CAT ABOVE
            match request.fact_id {
                Some(rem_id) => {
                    match fact_list.iter().find(|fact| fact.id == rem_id) {
                        Some(_) => {
                            fact_list.remove(rem_id as usize);

                            let mut new_fact_list: Vec<Fact> = Vec::with_capacity(fact_list.len() - 1);
                            for fact in fact_list {
                                new_fact_list.push(
                                    Fact {
                                        id: fact.id - 1,
                                        fact: fact.fact
                                    }
                                )
                            }

                            match animal {
                                AnimalChoice::Cat => {
                                    fs::write("cat_facts.json", serde_json::to_string_pretty(&new_fact_list).unwrap()).unwrap();
                                    HttpResponse::Created().body("CAT FACT DELETED")
                                }
                                AnimalChoice::Dog => {
                                    fs::write("dog_facts.json", serde_json::to_string_pretty(&new_fact_list).unwrap()).unwrap();
                                    HttpResponse::Created().body("DOG FACT DELETED")
                                }
                            }
                        }

                        None => HttpResponse::BadRequest().body(format!("INVALID FACT_ID SPECIFIED, MAX: {}", fact_list.len() - 1))
                    }
                }

                None => HttpResponse::BadRequest().body("NO \"fact_id\" SPECIFIED")
            }

        } else {
            HttpResponse::Forbidden().body("INSUFFICENT PERMISSIONS")
        }
    }

    let action = {
        let action_raw = req.to_owned();
        let action_raw = action_raw.path();

        if action_raw.ends_with("add") {
            ModifyAction::Add
        } else if action_raw.ends_with("delete") {
            ModifyAction::Delete
        } else {
            return HttpResponse::NotFound().body("")
        }
    };
    
    let (valid_admin, valid_perms) = check_admin_perms(body.auth.to_owned(), state.admins.to_owned());

    if valid_admin {
        match valid_perms {
            Some(perms) => { // Had some perms, now check if they can perform the requested action
                match body.target.to_lowercase().as_str() {
                    "cat" => modify_fact(body.into_inner(), state.cat_fact_list.to_owned(), action, AnimalChoice::Cat, perms),
                    "dog" => modify_fact(body.into_inner(), state.dog_fact_list.to_owned(), action, AnimalChoice::Dog, perms),

                    &_ => HttpResponse::BadRequest().body("INVALID TARGET")
                }
            }

            None => { // They had a valid account, but had no assigned perms
                HttpResponse::Forbidden()
                    .body("NO PERMISSIONS")
            }
        }
    } else { // Invalid key or username
        HttpResponse::Unauthorized()
            .body("INVALID AUTH")
    }
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

    println!("----------------------");
    println!("Loading API Admins...");
    let admins: Vec<Admin> = match fs::read_to_string("admin_keys.json") {
        Ok(key_file) => {
            // This will never panic if you don't mess up the JSON
            let admins: Vec<Admin> = serde_json::from_str(&key_file).unwrap();
            println!("{} admins registered", admins.len());
            if admins.len() == 0 { println!("No admins will be able to interact with the API via REST!") }
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
                .route(web::get().to(get_cat_fact)))
            
            .service(web::resource("/dog/fact")
                .route(web::get().to(get_dog_fact)))

            .service(web::resource("/").to(index))
            .service(web::resource("/metrics").to(prom_stats))
            .service(web::resource("/geargrinding/revenge").to(sneaky_secret))

            .service(web::resource("/admin/add")
                .route(web::post().to(admin_modify_fact)))
            .service(web::resource("/admin/delete")
                .route(web::post().to(admin_modify_fact)))
    )
    .bind(local_ip).expect("Failed to bind to a port or IP!")
    .run().unwrap();
}
