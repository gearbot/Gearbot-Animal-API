use actix_web::{App, web, test, web::Data};
use animal_api::*;

pub mod test_generators {
    use animal_api::*;
    use prometheus::{IntCounterVec, Registry, Opts};

    pub fn gen_admin_add_only() -> Admin {
        Admin {
            name: "Tester".to_string(),
            key: "add_only".to_string(),
            permissions: Perms {
                add: true,
                delete: false,
            }
        }
    }

    pub fn gen_admin_delete_only() -> Admin {
        Admin {
            name: "Tester".to_string(),
            key: "delete_only".to_string(),
            permissions: Perms {
                add: false,
                delete: true,
            }
        }
    }

    pub fn gen_admin_no_perms() -> Admin {
        Admin {
            name: "Tester".to_string(),
            key: "no_perms".to_string(),
            permissions: Perms {
                add: false,
                delete: false,
            }
        }
    }

    pub fn gen_admin_all_perms() -> Admin {
        Admin {
            name: "Tester".to_string(),
            key: "all_perms".to_string(),
            permissions: Perms {
                add: true,
                delete: true,
            }
        }
    }

    pub fn gen_state() -> APIState {
        let config = Config {
            logging_dir: "./test_logs".to_string(),
            logging_level: "info".to_string(),
            facts_dir: "./example_facts/".to_string(),
            animal_fact_types: vec![Animal::Cat, Animal::Dog],
            server: ServerConfig {
                ip: "127.0.0.1".parse().unwrap(),
                port: 8080
            },
            admins: Some(vec![gen_admin_add_only(), gen_admin_delete_only(), gen_admin_no_perms(), gen_admin_all_perms()]),
        };

        let fact_count: IntCounterVec = IntCounterVec::new(Opts::new("fact_count", "How many animal facts are currently loaded"), &["animal"]).unwrap();
        let req_count: IntCounterVec = IntCounterVec::new(Opts::new("api_request_count", "How many requests we have served"), &["animal"]).unwrap();

        let reg = Registry::new();
        reg.register(Box::new(fact_count.clone())).unwrap();
        reg.register(Box::new(req_count.clone())).unwrap();

        APIState {
            admins: config.admins.clone(),
            fact_lists: load_fact_lists(&fact_count, &config),
            config,
            stat_register: reg,
            req_counter: req_count
        }
    }
}

pub fn test_consumer_req(animal: Animal, uri: &str, state: APIState) {
    let mock_state = Data::new(state);

    let endpoint = match animal {
        Animal::Cat => animal_facts::get_cat_fact,
        Animal::Dog => animal_facts::get_dog_fact
    };

    let mut app = test::init_service(
        App::new()
            .register_data(mock_state.clone())
            .service(web::resource(uri)
                .route(web::get().to(endpoint)))
    );

    let req = test::TestRequest::get().uri(uri).to_request();
    let _: Fact = test::read_response_json(&mut app, req);
}

pub fn test_admin_req<'a>(req: ModifyRequest, uri: &str, state: APIState) -> JsonResp<'a> {
    // We always need to pass in a state. We can't have two states with the same
    // Prometheus discriptors at once, or it panics
    let mock_state = Data::new(state);
    let mut app = test::init_service(
        App::new()
            .register_data(mock_state.clone())
            .service(web::resource(uri)
                .route(web::post().to(admin::admin_modify_fact)))
    );
  
    let req = test::TestRequest::post().uri(uri)
        .set_json(&req)
        .to_request();

    test::read_response_json(&mut app, req)    
}

mod cosumption_tests {
    use animal_api::*;
    use super::test_generators::*;
    use super::test_consumer_req;

    #[test]
    fn get_fact_cat() {
        // This will fail if for some reason its not returning a fact
        test_consumer_req(Animal::Cat, "/cat/fact", gen_state())
    }

    #[test]
    fn get_fact_dog() {
        test_consumer_req(Animal::Dog, "/dog/fact", gen_state())
    }

    #[test]
    #[should_panic]
    fn get_unloaded_cat() {
        let mut state = gen_state();
        state.fact_lists.cat_facts = None;
        
        // This will fail because the JSON returned is a JsonResp, not a Fact
        test_consumer_req(Animal::Cat, "/cat/fact", state)
    }

    #[test]
    #[should_panic]
    fn get_unloaded_dog() {
        let mut state = gen_state();
        state.fact_lists.dog_facts = None;
        
        test_consumer_req(Animal::Dog, "/dog/fact", state)
    }
}

mod admin_tests {
    use actix_web::{App, test};
    use actix_web::web::{self, Bytes, Data};

    use serde_json;
    use std::fs;

    use animal_api::*;
    use super::test_generators::*;
    use super::test_admin_req;

    #[test]
    fn no_admins_loaded() {
        let mut state = gen_state();
        state.admins = None;

        let req_json = ModifyRequest {
            fact_id: None,
            fact_content: Some("SpookyFact".to_string()),
            animal_type: Animal::Cat,
            auth: gen_admin_all_perms().key
        };

        assert_eq!(test_admin_req(req_json, "/admin/add", state), Response::InvalidAuth.gen_resp())
    }

    #[test]
    fn invalid_key() {
        let req_json = ModifyRequest {
            fact_id: None,
            fact_content: Some("SpookyFact".to_string()),
            animal_type: Animal::Cat,
            auth: "BadKey".to_string()
        };

        assert_eq!(test_admin_req(req_json, "/admin/add", gen_state()), Response::InvalidAuth.gen_resp())
    }

    #[test]
    fn missing_permission_add() {
        let req_json = ModifyRequest {
            fact_id: None,
            fact_content: Some("SpookyFact".to_string()),
            animal_type: Animal::Cat,
            auth: gen_admin_delete_only().key
        };

        assert_eq!(test_admin_req(req_json, "/admin/add", gen_state()), Response::MissingPermission.gen_resp())
    }

    #[test]
    fn missing_permission_delete() {
        let req_json = ModifyRequest {
            fact_id: Some(6682463169732688062),
            fact_content: None,
            animal_type: Animal::Cat,
            auth: gen_admin_add_only().key
        };

        assert_eq!(test_admin_req(req_json, "/admin/delete", gen_state()), Response::MissingPermission.gen_resp())
    }

    #[test]
    fn missing_permission_all() {
        let req_json = ModifyRequest {
            fact_id: Some(6682463169732688062),
            fact_content: None,
            animal_type: Animal::Cat,
            auth: gen_admin_no_perms().key
        };

        assert_eq!(test_admin_req(req_json, "/admin/delete", gen_state()), Response::MissingPermission.gen_resp())
    }

    #[test]
    fn modify_cat_unloaded() {
        let mut state = gen_state();
        state.fact_lists.cat_facts = None;

        let req_json = ModifyRequest {
            fact_id: Some(6682463169732688062),
            fact_content: None,
            animal_type: Animal::Cat,
            // // Get the key of the 'add_only' admin
            auth: state.config.admins.clone().unwrap()[0].key.clone()
        };

        assert_eq!(test_admin_req(req_json, "/admin/add", state), Response::TypeNotLoaded.gen_resp())
    }

    #[test]
    fn modify_dog_unloaded() {
        let mut state = gen_state();
        state.fact_lists.dog_facts = None;

        let req_json = ModifyRequest {
            fact_id: Some(6682463169732688062),
            fact_content: None,
            animal_type: Animal::Dog,
            // // Get the key of the 'all_perms' admin
            auth: state.config.admins.clone().unwrap()[3].key.clone()
        };

        assert_eq!(test_admin_req(req_json, "/admin/add", state), Response::TypeNotLoaded.gen_resp())
    }

    #[test]
    fn add_fact_no_content() {
        let req_json = ModifyRequest {
            fact_id: None,
            fact_content: None,
            animal_type: Animal::Cat,
            auth: gen_admin_all_perms().key
        };

        assert_eq!(test_admin_req(req_json, "/admin/add", gen_state()), Response::NoContentSpecified.gen_resp())
    }

    #[test]
    fn add_fact_ok() {
        let state = gen_state();
        
        let req_json = ModifyRequest {
            fact_id: None,
            fact_content: Some("Huzaaah, a new fact!".to_string()),
            animal_type: Animal::Dog,
            auth: gen_admin_all_perms().key
        };

        let path = Animal::Dog.get_filepath(&state.config.facts_dir);

        // Reset the list after we add one
        let fact_string = fs::read_to_string(&path).unwrap();
        let fact_list: Vec<Fact> = serde_json::from_str(&fact_string).unwrap();

        let resp = test_admin_req(req_json, "/admin/add", state);

        fs::write(path, serde_json::to_string_pretty(&fact_list).unwrap()).unwrap();

        let message = format!("{} fact added", &*Animal::Dog);
        assert_eq!(resp, Response::Created(message).gen_resp());
    }

    #[test]
    fn delete_fact_no_id() {
        let req_json = ModifyRequest {
            fact_id: None,
            fact_content: None,
            animal_type: Animal::Dog,
            auth: gen_admin_all_perms().key
        };

        assert_eq!(test_admin_req(req_json, "/admin/delete", gen_state()), Response::NoID.gen_resp())
    }

    #[test]
    fn delete_fact_bad_id() {
        let req_json = ModifyRequest {
            // Example of a number that is not *currently* existing 
            fact_id: Some(03),
            fact_content: None,
            animal_type: Animal::Dog,
            auth: gen_admin_all_perms().key
        };

        assert_eq!(test_admin_req(req_json, "/admin/delete", gen_state()), Response::BadID.gen_resp())
    }

    #[test]
    fn delete_fact_ok() {
        let uri = "/admin/delete";
        let req_json = ModifyRequest {
            fact_id: Some(6682463169732688062),
            fact_content: None,
            animal_type: Animal::Cat,
            auth: gen_admin_all_perms().key
        };

        let state = gen_state();
        let path = Animal::Cat.get_filepath(&state.config.facts_dir);
        
        let mock_state = Data::new(state);
        let mut app = test::init_service(
            App::new()
                .register_data(mock_state.clone())
                .service(web::resource(uri)
                    .route(web::post().to(admin::admin_modify_fact)))
        );
  
        let req = test::TestRequest::post().uri(uri)
            .set_json(&req_json)
            .to_request();

        // Regen the list after we delete one
        let fact_string = fs::read_to_string(&path).unwrap();
        let fact_list: Vec<Fact> = serde_json::from_str(&fact_string).unwrap();

        let resp = test::read_response(&mut app, req);

        fs::write(path, serde_json::to_string_pretty(&fact_list).unwrap()).unwrap();

        assert_eq!(resp, Bytes::from_static(b""))
    }
}
