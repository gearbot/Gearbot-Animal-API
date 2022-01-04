#![allow(dead_code)]

use actix_web::{test, web, web::Data, App};
use animal_api::*;
use animal_facts::*;
use prometheus::{IntCounter, IntCounterVec, Opts, Registry};
use serde::{Deserialize, Serialize};
use tempdir::TempDir;

// This structure and the below comparision exists because we can't deserialize
// the crate's JsonResp because it uses &'static str's for all the messages.
#[derive(Debug, Deserialize, Serialize)]
pub struct JsonResp {
    code: u16,
    message: String,
}

impl PartialEq<animal_api::JsonResp> for JsonResp {
    fn eq(&self, other: &animal_api::JsonResp) -> bool {
        self.code == other.code && self.message == other.message
    }
}

pub fn gen_admin_add_only() -> Admin {
    Admin {
        name: "Tester".to_string(),
        key: "add_only".to_string(),
        permissions: Perms {
            view_facts: true,
            add_fact: true,
            delete_fact: false,
            view_flags: true,
            add_flag: true,
            delete_flag: false,
        },
    }
}

pub fn gen_admin_delete_only() -> Admin {
    Admin {
        name: "Tester".to_string(),
        key: "delete_only".to_string(),
        permissions: Perms {
            view_facts: true,
            add_fact: false,
            delete_fact: true,
            view_flags: true,
            add_flag: false,
            delete_flag: true,
        },
    }
}

pub fn gen_admin_view_only() -> Admin {
    Admin {
        name: "Tester".to_string(),
        key: "view_only".to_string(),
        permissions: Perms {
            view_facts: true,
            add_fact: false,
            delete_fact: false,
            view_flags: false,
            add_flag: false,
            delete_flag: false,
        },
    }
}

pub fn gen_admin_no_perms() -> Admin {
    Admin {
        name: "Tester".to_string(),
        key: "no_perms".to_string(),
        permissions: Perms {
            view_facts: false,
            add_fact: false,
            delete_fact: false,
            view_flags: false,
            add_flag: false,
            delete_flag: false,
        },
    }
}

pub fn gen_admin_all_perms() -> Admin {
    Admin {
        name: "Tester".to_string(),
        key: "all_perms".to_string(),
        permissions: Perms {
            view_facts: true,
            add_fact: true,
            delete_fact: true,
            view_flags: true,
            add_flag: false,
            delete_flag: true,
        },
    }
}

pub fn gen_flagger() -> Flagger {
    Flagger {
        location: "test_location".to_string(),
        key: "flag_key".to_string(),
    }
}

pub fn gen_state(tmp_dir: &TempDir) -> APIState {
    let config = Config {
        logging_dir: "./test_logs".to_string(),
        logging_level: "info".to_string(),
        facts_dir: tmp_dir.path().as_os_str().to_string_lossy().to_string(),
        animal_fact_types: vec![Animal::Cat, Animal::Dog],
        flagging_enabled: true,
        server: ServerConfig {
            ip: "127.0.0.1".parse().unwrap(),
            port: 8080,
        },
        admins: vec![
            gen_admin_add_only(),
            gen_admin_view_only(),
            gen_admin_delete_only(),
            gen_admin_no_perms(),
            gen_admin_all_perms(),
        ],
        flaggers: vec![gen_flagger()],
    };

    let fact_count: IntCounterVec = IntCounterVec::new(
        Opts::new("fact_count", "How many animal facts are currently loaded"),
        &["animal"],
    )
    .unwrap();
    let flag_count: IntCounter =
        IntCounter::new("flag_count", "How many facts have been flagged").unwrap();
    let req_count: IntCounterVec = IntCounterVec::new(
        Opts::new("api_request_count", "How many requests we have served"),
        &["animal"],
    )
    .unwrap();

    let reg = Registry::new();
    reg.register(Box::new(fact_count.clone())).unwrap();
    reg.register(Box::new(req_count.clone())).unwrap();

    APIState {
        fact_lists: load_fact_lists(&fact_count, &config),
        fact_flags: load_fact_flags(&flag_count, &config),
        config,
        stat_register: reg,
        req_counter: req_count,
    }
}

pub fn make_dir() -> TempDir {
    let dir = tempdir::TempDir::new("facts").unwrap();

    for file in std::fs::read_dir("./example_facts").unwrap() {
        let file = file.unwrap();
        std::fs::copy(
            file.path(),
            dir.path().join(file.path().file_name().unwrap()),
        )
        .unwrap();
    }

    dir
}

pub async fn test_fact_consumer_req(animal: Animal, uri: &str, state: APIState) {
    let mock_state = Data::new(state);

    let endpoint = match animal {
        Animal::Cat => animal_facts::get_cat_fact,
        Animal::Dog => animal_facts::get_dog_fact,
    };

    let app = test::init_service(
        App::new()
            .app_data(mock_state.clone())
            .service(web::resource(uri).route(web::get().to(endpoint))),
    )
    .await;

    let req = test::TestRequest::get().uri(uri).to_request();
    let _: Fact = test::call_and_read_body_json(&app, req).await;
}

pub async fn test_flag_consumer_req(req: FactFlagRequest, uri: &str, state: APIState) -> JsonResp {
    let mock_state = Data::new(state);

    let app = test::init_service(
        App::new()
            .app_data(mock_state.clone())
            .service(web::resource(uri).route(web::post().to(flagging::set_flag))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(uri)
        .set_json(&req)
        .to_request();

    test::call_and_read_body_json(&app, req).await
}

pub async fn test_admin_fact_req<T: Serialize>(req: T, uri: &str, state: APIState) -> JsonResp {
    let mock_state = Data::new(state);
    let app = test::init_service(
        App::new()
            .app_data(mock_state.clone())
            .service(web::resource(uri).route(web::post().to(admin::modify_fact))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(uri)
        .set_json(&req)
        .to_request();

    test::call_and_read_body_json(&app, req).await
}

pub async fn test_admin_flag_req<T: Serialize>(req: T, uri: &str, state: APIState) -> JsonResp {
    let mock_state = Data::new(state);
    let app = test::init_service(
        App::new()
            .app_data(mock_state.clone())
            .service(web::resource(uri).route(web::post().to(admin::modify_flag))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(uri)
        .set_json(&req)
        .to_request();

    test::call_and_read_body_json(&app, req).await
}
