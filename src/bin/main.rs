#![deny(warnings)]
#![deny(unsafe_code)]

use actix_web::{web, App, HttpServer};
use flexi_logger::{Duplicate, Logger};
use log::info;
use prometheus::{Encoder, IntCounter, IntCounterVec, Opts, Registry, TextEncoder};

use std::fs;

use animal_api::{
    admin, animal_facts, flagging, load_fact_flags, load_fact_lists, APIState, Config,
};

async fn prom_stats(app_data: web::Data<APIState>) -> String {
    let register = &app_data.stat_register;

    let mut buffer: Vec<u8> = Vec::with_capacity(100);
    let encoder = TextEncoder::new();

    let metrics = register.gather();
    encoder.encode(&metrics, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}

async fn index() -> &'static str {
    "Hello There! This is Gearbot's animal fact API. Head over to /cat/fact or /dog/fact to try it out!"
}

#[actix_web::main]
async fn main() {
    let fact_count = IntCounterVec::new(
        Opts::new("fact_count", "How many animal facts are currently loaded"),
        &["animal"],
    )
    .unwrap();
    let flag_count = IntCounter::new("flag_count", "How many facts have been flagged").unwrap();
    let req_count = IntCounterVec::new(
        Opts::new("api_request_count", "How many requests we have served"),
        &["animal"],
    )
    .unwrap();

    let reg = Registry::new();
    reg.register(Box::new(fact_count.clone())).unwrap();
    reg.register(Box::new(flag_count.clone())).unwrap();
    reg.register(Box::new(req_count.clone())).unwrap();

    let config: Config = {
        let config_string =
            fs::read_to_string("config.toml").expect("The config file couldn't be found!");
        toml::from_str(&config_string).expect("The config was malformed!")
    };

    Logger::with_str(&config.logging_level)
        .log_to_file()
        .directory(&config.logging_dir)
        .duplicate_to_stderr(Duplicate::Info)
        .start()
        .unwrap();

    let loaded_lists = load_fact_lists(&fact_count, &config);
    let flags = load_fact_flags(&flag_count, &config);

    let server_binding = (config.server.ip, config.server.port);

    let state_data = web::Data::new(APIState {
        config,
        fact_lists: loaded_lists,
        fact_flags: flags,
        stat_register: reg,
        req_counter: req_count,
    });

    info!("Facts and configs loaded, starting server...");

    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .service(web::resource("/cat/fact").route(web::get().to(animal_facts::get_cat_fact)))
            .service(web::resource("/dog/fact").route(web::get().to(animal_facts::get_dog_fact)))
            .service(web::resource("/flag").route(web::post().to(flagging::set_flag)))
            .service(web::resource("/").to(index))
            .service(web::resource("/metrics").to(prom_stats))
            .service(web::resource("/admin/fact/list").route(web::post().to(admin::modify_fact)))
            .service(web::resource("/admin/fact/add").route(web::post().to(admin::modify_fact)))
            .service(web::resource("/admin/fact/delete").route(web::post().to(admin::modify_fact)))
            .service(web::resource("/admin/flag/list").route(web::post().to(admin::modify_flag)))
            .service(web::resource("/admin/flag/add").route(web::post().to(admin::modify_flag)))
            .service(web::resource("/admin/flag/delete").route(web::post().to(admin::modify_flag)))
    })
    .bind(server_binding)
    .expect("Failed to bind to a port or IP!")
    .run()
    .await
    .unwrap();
}
