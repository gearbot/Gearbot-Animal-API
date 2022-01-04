use actix_web::web::{self, Bytes, Data};
use actix_web::{test, App};

use animal_api::*;
mod generator;
use crate::generator::*;

#[actix_rt::test]
async fn flagging_not_loaded() {
    let dir = make_dir();
    let mut state = gen_state(&dir);
    state.config.flagging_enabled = false;

    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: None,
        flag_id: None,
        reason: None,
        fact_type: None,
    };

    assert_eq!(
        test_admin_flag_req(req_json, "/admin/flag/list", state).await,
        RESP_NOT_LOADED
    );
}

// The user permission checks are tested in the general tests

#[actix_rt::test]
async fn view_flags() {
    let dir = make_dir();
    let uri = "/admin/flag/list";

    let (state, state2) = (gen_state(&dir), gen_state(&dir));

    let raw = state.fact_flags.unwrap();
    let expected = raw.read().unwrap();

    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: None,
        flag_id: None,
        reason: None,
        fact_type: None,
    };

    let mock_state = Data::new(state2);
    let app = test::init_service(
        App::new()
            .app_data(mock_state.clone())
            .service(web::resource(uri).route(web::post().to(admin::modify_flag))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(uri)
        .set_json(&req_json)
        .to_request();

    let returned: Vec<FactFlag> = test::call_and_read_body_json(&app, req).await;

    assert_eq!(returned, *expected);
}

#[actix_rt::test]
async fn add_flag_no_type() {
    let dir = make_dir();
    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: None,
        flag_id: None,
        reason: None,
        fact_type: None,
    };

    assert_eq!(
        test_admin_flag_req(req_json, "/admin/flag/add", gen_state(&dir)).await,
        RESP_NO_TYPE_SUPPLIED
    )
}

#[actix_rt::test]
async fn add_flag_no_factid() {
    let dir = make_dir();
    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: None,
        flag_id: None,
        reason: None,
        fact_type: Some(Animal::Cat),
    };

    assert_eq!(
        test_admin_flag_req(req_json, "/admin/flag/add", gen_state(&dir)).await,
        RESP_NO_ID_SUPPLIED
    )
}

#[actix_rt::test]
async fn add_flag_bad_factid() {
    let dir = make_dir();
    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: Some(18446744073709551615),
        flag_id: None,
        reason: None,
        fact_type: Some(Animal::Cat),
    };

    assert_eq!(
        test_admin_flag_req(req_json, "/admin/flag/add", gen_state(&dir)).await,
        RESP_ID_NOT_FOUND
    )
}

#[actix_rt::test]
async fn add_flag_ok() {
    let dir = make_dir();
    let state = gen_state(&dir);

    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: Some(6682463169732688062),
        flag_id: None,
        reason: None,
        fact_type: Some(Animal::Cat),
    };

    let resp = test_admin_flag_req(req_json, "/admin/flag/add", state).await;

    let expected = JsonResp::new(201, CreatedAction::Flag.as_str());
    assert_eq!(resp, expected)
}

#[actix_rt::test]
async fn delete_flag_no_id() {
    let dir = make_dir();
    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: None,
        flag_id: None,
        reason: None,
        fact_type: None,
    };

    assert_eq!(
        test_admin_flag_req(req_json, "/admin/flag/delete", gen_state(&dir)).await,
        RESP_NO_ID_SUPPLIED
    )
}

#[actix_rt::test]
async fn delete_flag_invalid_id() {
    let dir = make_dir();
    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: None,
        flag_id: Some(18446744073709551615),
        reason: None,
        fact_type: None,
    };

    assert_eq!(
        test_admin_flag_req(req_json, "/admin/flag/delete", gen_state(&dir)).await,
        RESP_ID_NOT_FOUND
    )
}

#[actix_rt::test]
async fn delete_flag_ok() {
    let dir = make_dir();
    let uri = "/admin/flag/delete";
    let req_json = AdminFlagRequest {
        key: gen_admin_all_perms().key,
        fact_id: None,
        flag_id: Some(6682463169732628062),
        reason: None,
        fact_type: None,
    };

    let state = gen_state(&dir);

    let mock_state = Data::new(state);
    let app = test::init_service(
        App::new()
            .app_data(mock_state.clone())
            .service(web::resource(uri).route(web::post().to(admin::modify_flag))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri(uri)
        .set_json(&req_json)
        .to_request();

    let resp = test::call_and_read_body(&app, req).await;

    assert_eq!(resp, Bytes::from_static(b""))
}
