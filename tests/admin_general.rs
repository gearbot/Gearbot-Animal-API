// These cover both the admin fact and flag interfaces, they both use `check_user`
use animal_api::*;
use animal_facts::*;
mod generator;
use crate::generator::*;
use actix_web::web::{self, Bytes, Data};
use actix_web::{test, App};

#[actix_rt::test]
async fn no_admins_loaded() {
    let dir = make_dir();
    let mut state = gen_state(&dir);
    state.config.admins = Vec::new();

    let req_json = AdminFactRequest {
        fact_id: None,
        fact_content: Some("SpookyFact".to_string()),
        animal_type: Animal::Cat,
        key: gen_admin_all_perms().key,
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/add", state).await,
        RESP_BAD_AUTH
    )
}

#[actix_rt::test]
async fn invalid_key() {
    let dir = make_dir();
    let req_json = AdminFactRequest {
        fact_id: None,
        fact_content: Some("SpookyFact".to_string()),
        animal_type: Animal::Cat,
        key: "BadKey".to_string(),
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/add", gen_state(&dir)).await,
        RESP_BAD_AUTH
    )
}

#[actix_rt::test]
async fn missing_permission_add() {
    let dir = make_dir();
    let req_json = AdminFactRequest {
        fact_id: None,
        fact_content: Some("SpookyFact".to_string()),
        animal_type: Animal::Cat,
        key: gen_admin_delete_only().key,
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/add", gen_state(&dir)).await,
        RESP_MISSING_PERMS
    )
}

#[actix_rt::test]
async fn missing_permission_delete() {
    let dir = make_dir();
    let req_json = AdminFactRequest {
        fact_id: Some(6682463169732688062),
        fact_content: None,
        animal_type: Animal::Cat,
        key: gen_admin_add_only().key,
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/delete", gen_state(&dir)).await,
        RESP_MISSING_PERMS
    )
}

#[actix_rt::test]
async fn missing_permission_view() {
    let dir = make_dir();
    let req_json = AdminFactRequest {
        fact_id: Some(6682463169732688062),
        fact_content: None,
        animal_type: Animal::Cat,
        key: gen_admin_view_only().key,
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/add", gen_state(&dir)).await,
        RESP_MISSING_PERMS
    )
}

#[actix_rt::test]
async fn missing_permission_all() {
    let dir = make_dir();
    let req_json = AdminFactRequest {
        fact_id: Some(6682463169732688062),
        fact_content: None,
        animal_type: Animal::Cat,
        key: gen_admin_no_perms().key,
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/delete", gen_state(&dir)).await,
        RESP_MISSING_PERMS
    )
}

#[actix_rt::test]
async fn modify_cat_unloaded() {
    let dir = make_dir();
    let mut state = gen_state(&dir);
    state.fact_lists.cat_facts = None;

    let req_json = AdminFactRequest {
        fact_id: Some(6682463169732688062),
        fact_content: None,
        animal_type: Animal::Cat,
        // // Get the key of the 'add_only' admin
        key: state.config.admins[0].key.clone(),
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/add", state).await,
        RESP_NOT_LOADED
    )
}

#[actix_rt::test]
async fn modify_dog_unloaded() {
    let dir = make_dir();
    let mut state = gen_state(&dir);
    state.fact_lists.dog_facts = None;

    let req_json = AdminFactRequest {
        fact_id: Some(6682463169732688062),
        fact_content: None,
        animal_type: Animal::Dog,
        // // Get the key of the 'all_perms' admin
        key: state.config.admins[4].key.clone(),
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/add", state).await,
        RESP_NOT_LOADED
    )
}

#[actix_rt::test]
async fn list_facts() {
    let dir = make_dir();
    let uri = "/admin/fact/list";
    let state = gen_state(&dir);
    let req_json = AdminFactRequest {
        fact_id: None,
        fact_content: None,
        animal_type: Animal::Cat,
        key: gen_admin_all_perms().key,
    };

    let raw = gen_state(&dir).fact_lists.cat_facts.unwrap();
    let expected = raw.read().unwrap();

    let mock_state = Data::new(state);
    let mut app = test::init_service(
        App::new()
            .app_data(mock_state.clone())
            .service(web::resource(uri).route(web::post().to(admin::modify_fact))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(uri)
        .set_json(&req_json)
        .to_request();

    let received: Vec<Fact> = test::read_response_json(&mut app, req).await;

    assert_eq!(received, *expected)
}

#[actix_rt::test]
async fn add_fact_no_content() {
    let dir = make_dir();
    let req_json = AdminFactRequest {
        fact_id: None,
        fact_content: None,
        animal_type: Animal::Cat,
        key: gen_admin_all_perms().key,
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/add", gen_state(&dir)).await,
        RESP_NO_CONTENT_SPECIFIED
    )
}

#[actix_rt::test]
async fn add_fact_ok() {
    let dir = make_dir();
    let state = gen_state(&dir);

    let req_json = AdminFactRequest {
        fact_id: None,
        fact_content: Some("Huzaaah, a new fact!".to_string()),
        animal_type: Animal::Dog,
        key: gen_admin_all_perms().key,
    };

    let resp = test_admin_fact_req(req_json, "/admin/fact/add", state).await;

    let word = CreatedAction::Fact {
        animal: Animal::Dog,
    };
    let expected = JsonResp::new(201, word.as_str());
    assert_eq!(resp, expected);
}

#[actix_rt::test]
async fn delete_fact_no_id() {
    let dir = make_dir();
    let req_json = AdminFactRequest {
        fact_id: None,
        fact_content: None,
        animal_type: Animal::Dog,
        key: gen_admin_all_perms().key,
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/delete", gen_state(&dir)).await,
        RESP_NO_ID_SUPPLIED
    )
}

#[actix_rt::test]
async fn delete_fact_bad_id() {
    let dir = make_dir();
    let req_json = AdminFactRequest {
        // Example of a number that is not *currently* existing
        fact_id: Some(3),
        fact_content: None,
        animal_type: Animal::Dog,
        key: gen_admin_all_perms().key,
    };

    assert_eq!(
        test_admin_fact_req(req_json, "/admin/fact/delete", gen_state(&dir)).await,
        RESP_ID_NOT_FOUND
    )
}

#[actix_rt::test]
async fn delete_fact_ok() {
    let dir = make_dir();
    let uri = "/admin/fact/delete";
    let req_json = AdminFactRequest {
        fact_id: Some(6682463169732688062),
        fact_content: None,
        animal_type: Animal::Cat,
        key: gen_admin_all_perms().key,
    };

    let state = gen_state(&dir);

    let mock_state = Data::new(state);
    let mut app = test::init_service(
        App::new()
            .app_data(mock_state.clone())
            .service(web::resource(uri).route(web::post().to(admin::modify_fact))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(uri)
        .set_json(&req_json)
        .to_request();

    let resp = test::read_response(&mut app, req).await;

    assert_eq!(resp, Bytes::from_static(b""))
}
