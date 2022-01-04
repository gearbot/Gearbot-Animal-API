use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use rand::RngCore;
use subtle::ConstantTimeEq;

use std::{fs, path::Path};

use crate::*;

fn check_flagger(unchecked_auth: String, flagger_list: &[Flagger]) -> Option<Flagger> {
    if !flagger_list.is_empty() {
        let unchecked_auth = unchecked_auth.as_bytes();
        flagger_list
            .iter()
            .find(|flagger| bool::from(unchecked_auth.ct_eq(flagger.key.as_bytes())))
            .cloned()
    } else {
        None
    }
}

// The user is responsible for ensuring proper ACLs or rate limiting to this
pub fn set_flag(app_data: Data<APIState>, body: Json<FactFlagRequest>) -> HttpResponse {
    if !app_data.config.flagging_enabled {
        return generate_response(&RESP_NOT_LOADED);
    }

    let body = body.into_inner();

    // Make sure the request is allowed
    let location = match check_flagger(body.key, &app_data.config.flaggers) {
        Some(location) => location,
        None => return generate_response(&RESP_BAD_AUTH),
    };

    let flag_list = app_data.fact_flags.as_ref().unwrap();
    {
        // Allow users to submit a name to flag it under or fallback to the submitter location
        // This allows a use case of sending specific user IDs when used by another system
        let flagger = match body.flagger {
            Some(flagger) => flagger,
            None => location.location,
        };

        let FactFlagRequest {
            fact_type,
            fact_id,
            reason,
            ..
        } = body;
        let id = rand::thread_rng().next_u64();

        let mut flag_list = flag_list.write().unwrap();

        // Check to make sure the targeted fact exists
        if !flag_list
            .iter()
            .enumerate()
            .any(|(_, flag)| flag.fact_id == fact_id)
        {
            return generate_response(&RESP_ID_NOT_FOUND);
        }

        flag_list.push(FactFlag {
            id,
            fact_type,
            fact_id,
            reason,
            flagger,
        })
    }

    let file_path = Path::new(&app_data.config.facts_dir).join("fact_flags.json");
    fs::write(file_path, serde_json::to_string_pretty(flag_list).unwrap())
        .expect("Failed writing to flags file!");

    let resp = JsonResp::new(201, CreatedAction::Flag.as_str());
    generate_response(&resp)
}
