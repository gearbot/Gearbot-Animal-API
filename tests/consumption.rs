use animal_api::*;
mod generator;
use crate::generator::*;

// Fact tests
#[actix_rt::test]
async fn get_fact_cat() {
    // This will fail if for some reason its not returning a fact
    let dir = make_dir();
    test_fact_consumer_req(Animal::Cat, "/cat/fact", gen_state(&dir)).await
}

#[actix_rt::test]
async fn get_fact_dog() {
    let dir = make_dir();
    test_fact_consumer_req(Animal::Dog, "/dog/fact", gen_state(&dir)).await
}

#[actix_rt::test]
#[should_panic]
async fn get_unloaded_cat() {
    let dir = make_dir();
    let mut state = gen_state(&dir);
    state.fact_lists.cat_facts = None;

    // This will fail because the JSON returned is a JsonResp, not a Fact
    test_fact_consumer_req(Animal::Cat, "/cat/fact", state).await
}

#[actix_rt::test]
#[should_panic]
async fn get_unloaded_dog() {
    let dir = make_dir();
    let mut state = gen_state(&dir);
    state.fact_lists.dog_facts = None;

    test_fact_consumer_req(Animal::Dog, "/dog/fact", state).await
}

// Flag tests
#[actix_rt::test]
async fn set_flag_unloaded() {
    let dir = make_dir();
    let mut state = gen_state(&dir);
    state.config.flagging_enabled = false;

    let req_json = FactFlagRequest {
        fact_type: Animal::Cat,
        fact_id: 6682463169732688062,
        reason: None,
        key: gen_flagger().key,
        flagger: None,
    };

    assert_eq!(
        test_flag_consumer_req(req_json, "/flag", state).await,
        RESP_NOT_LOADED
    )
}

#[actix_rt::test]
async fn set_flag_invalid_auth() {
    let dir = make_dir();
    let req_json = FactFlagRequest {
        fact_type: Animal::Cat,
        fact_id: 6682463169732688062,
        reason: None,
        key: "AGreatPassword".to_string(),
        flagger: None,
    };

    assert_eq!(
        test_flag_consumer_req(req_json, "/flag", gen_state(&dir)).await,
        RESP_BAD_AUTH
    )
}

#[actix_rt::test]
async fn set_flag_bad_factid() {
    let dir = make_dir();
    let req_json = FactFlagRequest {
        fact_type: Animal::Cat,
        fact_id: 18446744073709551615,
        reason: Some("A Reason".to_string()),
        key: gen_flagger().key,
        flagger: None,
    };

    assert_eq!(
        test_flag_consumer_req(req_json, "/flag", gen_state(&dir)).await,
        RESP_ID_NOT_FOUND
    )
}

#[actix_rt::test]
async fn set_flag_valid() {
    let dir = make_dir();
    let state = gen_state(&dir);
    let req_json = FactFlagRequest {
        fact_type: Animal::Cat,
        fact_id: 6682463169732688062,
        reason: Some("A Reason".to_string()),
        key: gen_flagger().key,
        flagger: None,
    };

    let resp = test_flag_consumer_req(req_json, "/flag", state).await;

    let expected = JsonResp::new(201, CreatedAction::Flag.as_str());
    assert_eq!(resp, expected)
}
