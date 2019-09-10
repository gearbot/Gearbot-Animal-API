// #![deny(warnings)]
#![deny(unsafe_code)]

use actix_web::{HttpServer, App, web};
use flexi_logger::{Duplicate, Logger};
use log::info;
use prometheus::{IntCounterVec, Registry, Opts, TextEncoder, Encoder};
use toml;

use std::fs;

use animal_api::{
    APIState,
    Config,
    load_fact_lists,
    animal_facts,
    admin
};

fn prom_stats(app_data: web::Data<APIState>) -> String {
    let register = &app_data.stat_register;

    let mut buffer: Vec<u8> = Vec::with_capacity(100);
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
    let fact_count: IntCounterVec = IntCounterVec::new(Opts::new("fact_count", "How many animal facts are currently loaded"), &["animal"]).unwrap();
    let req_count: IntCounterVec = IntCounterVec::new(Opts::new("api_request_count", "How many requests we have served"), &["animal"]).unwrap();

    let reg = Registry::new();
    reg.register(Box::new(fact_count.clone())).unwrap();
    reg.register(Box::new(req_count.clone())).unwrap();

    let config: Config = {
        let config_string = fs::read_to_string("config.toml").expect("The config file couldn't be found!");
        toml::from_str(&config_string).expect("The config was malformed!")
    };

    Logger::with_str(&config.logging_level)
        .log_to_file()
        .directory(&config.logging_dir)
        .duplicate_to_stderr(Duplicate::Info)
        .start()
        .unwrap();

    info!("Loading animal facts...");

    // TODO: Look into making it possible to load a new animal type to be served
    // just based off the config value for animal types?
    let loaded_lists = load_fact_lists(&fact_count, &config);

    let server_binding = (config.server.ip, config.server.port);

    let state_data = web::Data::new(APIState {
        admins: config.admins.clone(),
        config,
        fact_lists: loaded_lists,
        stat_register: reg,
        req_counter: req_count,
    });

    HttpServer::new(move ||
        App::new()
            .register_data(state_data.clone())
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
    .bind(server_binding).expect("Failed to bind to a port or IP!")
    .run().unwrap();
}
