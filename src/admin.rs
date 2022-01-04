use actix_web::http::StatusCode;
use actix_web::web::{Data, Json};
use actix_web::{HttpRequest, HttpResponse};
use log::{info, warn};
use rand::RngCore;
use subtle::ConstantTimeEq;

use std::fs;
use std::path::Path;
use std::sync::{RwLock, RwLockWriteGuard};

use crate::animal_facts::{Fact, FactLists};
use crate::*;

fn check_admin_perms<'a>(
    unchecked_auth: &str,
    admin_list: &'a [Admin],
) -> Option<(&'a Admin, Option<Perms>)> {
    if !admin_list.is_empty() {
        let unchecked_auth = unchecked_auth.as_bytes();
        match admin_list
            .iter()
            .find(|admin| bool::from(unchecked_auth.ct_eq(admin.key.as_bytes())))
        {
            Some(admin) => {
                let perms = admin.permissions;
                if perms.add_fact || perms.delete_fact || perms.view_flags || perms.delete_flag {
                    Some((admin, Some(admin.permissions)))
                } else {
                    Some((admin, None))
                }
            }
            None => None,
        }
    } else {
        None
    }
}

fn check_user<'a>(
    action: AdminAction,
    key: &str,
    state: &'a APIState,
) -> Result<&'a Admin, HttpResponse> {
    if let Some((user, perms)) = check_admin_perms(key, &state.config.admins) {
        if let Some(perms) = perms {
            // Check if they are allowed to perform the desired action
            let missing_perms_resp = generate_response(&RESP_MISSING_PERMS);
            match action {
                AdminAction::View => {
                    if !perms.view_facts {
                        return Err(missing_perms_resp);
                    }
                }
                AdminAction::Delete => {
                    if !perms.delete_fact {
                        return Err(missing_perms_resp);
                    }
                }
                AdminAction::Add => {
                    if !perms.add_fact {
                        return Err(missing_perms_resp);
                    }
                }
            }
            // Validated for performing their action
            Ok(user)
        } else {
            warn!(
                "Admin '{}' attempted to {} something, but had no permission to!",
                user.name, action
            );
            Err(generate_response(&RESP_MISSING_PERMS))
        }
    } else {
        Err(generate_response(&RESP_BAD_AUTH))
    }
}

pub fn modify_fact(
    state: Data<APIState>,
    req: HttpRequest,
    body: Json<AdminFactRequest>,
) -> HttpResponse {
    let action = determine_action(req.path());

    let user = match check_user(action, &body.key, &state) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    // Check if the requested animal list is loaded
    match body.animal_type {
        Animal::Cat => {
            if state.fact_lists.cat_facts.is_none() {
                return generate_response(&RESP_NOT_LOADED);
            }
        }
        Animal::Dog => {
            if state.fact_lists.dog_facts.is_none() {
                return generate_response(&RESP_NOT_LOADED);
            }
        }
    }

    match action {
        AdminAction::Add => add_fact(body.animal_type, user, body.into_inner(), &state),
        AdminAction::Delete => delete_fact(body.animal_type, user, body.into_inner(), &state),
        AdminAction::View => view_facts(body.animal_type, &state),
    }
}

fn view_facts(animal: Animal, state: &APIState) -> HttpResponse {
    // Unwrap is already verified before function call
    let FactLists {
        cat_facts,
        dog_facts,
    } = &state.fact_lists;
    let fact_list = match animal {
        Animal::Cat => cat_facts.as_ref().unwrap().read().unwrap(),
        Animal::Dog => dog_facts.as_ref().unwrap().read().unwrap(),
    };

    HttpResponse::Ok().status(StatusCode::OK).json(&*fact_list)
}

fn add_fact(
    animal: Animal,
    user: &Admin,
    request: AdminFactRequest,
    state: &APIState,
) -> HttpResponse {
    let id = rand::thread_rng().next_u64();

    let fact_list = determine_list(animal, &state.fact_lists);
    let mut list_lock = fact_list.write().unwrap();

    match request.fact_content {
        Some(content) => {
            list_lock.push(Fact { id, content });
        }
        None => {
            return generate_response(&RESP_NO_CONTENT_SPECIFIED);
        }
    }

    modify_persistent_fact(animal, list_lock, state);

    let message = CreatedAction::Fact { animal };
    warn!("{} by {}", message.as_str(), user.name);

    let resp = JsonResp::new(201, message.as_str());
    generate_response(&resp)
}

fn delete_fact(
    animal: Animal,
    user: &Admin,
    request: AdminFactRequest,
    state: &APIState,
) -> HttpResponse {
    if let Some(rem_id) = request.fact_id {
        let fact_list = determine_list(animal, &state.fact_lists);
        let mut list_lock = fact_list.write().unwrap();
        if let Some(found) = list_lock
            .iter()
            .enumerate()
            .find(|(_, fact)| fact.id == rem_id)
        {
            let pos = found.0;
            list_lock.remove(pos);
            modify_persistent_fact(animal, list_lock, state);

            warn!("{} fact removed by {}", animal.as_str(), user.name);

            HttpResponse::NoContent().finish()
        } else {
            generate_response(&RESP_ID_NOT_FOUND)
        }
    } else {
        generate_response(&RESP_NO_ID_SUPPLIED)
    }
}

pub fn modify_flag(
    state: Data<APIState>,
    req: HttpRequest,
    body: Json<AdminFlagRequest>,
) -> HttpResponse {
    if !state.config.flagging_enabled {
        return generate_response(&RESP_NOT_LOADED);
    }

    let action = determine_action(req.path());

    // Check if they have the needed flag related perms
    let user = match check_user(action, &body.key, &state) {
        Ok(user) => user,
        Err(resp) => return resp,
    };

    match action {
        AdminAction::View => list_flags(&state),
        AdminAction::Add => {
            let req = body.into_inner();

            // Make sure that they provided the required values
            if req.fact_type.is_none() {
                return generate_response(&RESP_NO_TYPE_SUPPLIED);
            }
            if req.fact_id.is_none() {
                return generate_response(&RESP_NO_ID_SUPPLIED);
            }

            add_flag(
                &state,
                user,
                (req.fact_type.unwrap(), req.fact_id.unwrap(), req.reason),
            )
        }
        AdminAction::Delete => {
            if let Some(id) = body.into_inner().flag_id {
                delete_flag(&state, id, user)
            } else {
                generate_response(&RESP_NO_ID_SUPPLIED)
            }
        }
    }
}

fn list_flags(state: &APIState) -> HttpResponse {
    let flag_list = state.fact_flags.as_ref().unwrap().read().unwrap();

    HttpResponse::Ok().json(&*flag_list)
}

// This will allow an admin to add a flag and bypass the user-restricted method
fn add_flag(
    state: &APIState,
    user: &Admin,
    set_flag: (Animal, u64, Option<String>),
) -> HttpResponse {
    let flag_list = state.fact_flags.as_ref().unwrap();
    let id = rand::thread_rng().next_u64();

    {
        let mut flag_list = flag_list.write().unwrap();

        if !flag_list
            .iter()
            .enumerate()
            .any(|(_, flag)| flag.fact_id == set_flag.1)
        {
            return generate_response(&RESP_ID_NOT_FOUND);
        }

        flag_list.push(FactFlag {
            id,
            fact_type: set_flag.0,
            fact_id: set_flag.1,
            reason: set_flag.2,
            flagger: user.name.clone(),
        });

        modify_persistent_flag(flag_list, state)
    }

    info!("Flag #{} added by {}", id, user.name);
    let resp = JsonResp::new(201, CreatedAction::Flag.as_str());
    generate_response(&resp)
}

fn delete_flag(state: &APIState, rem_id: u64, user: &Admin) -> HttpResponse {
    let flag_list = state.fact_flags.as_ref().unwrap();

    let mut list_lock = flag_list.write().unwrap();
    if let Some(found) = list_lock
        .iter()
        .enumerate()
        .find(|(_, flag)| flag.id == rem_id)
    {
        let pos = found.0;
        list_lock.remove(pos);

        info!("Flag #{} removed by {}", rem_id, user.name);

        HttpResponse::NoContent().finish()
    } else {
        generate_response(&RESP_ID_NOT_FOUND)
    }
}

fn determine_action(path: &str) -> AdminAction {
    if path.ends_with("list") {
        AdminAction::View
    } else if path.ends_with("delete") {
        AdminAction::Delete
    } else {
        AdminAction::Add
    }
}

fn determine_list(animal: Animal, fact_lists: &FactLists) -> &RwLock<Vec<Fact>> {
    match animal {
        // These unwraps are safe due to previous checks
        Animal::Cat => fact_lists.cat_facts.as_ref().unwrap(),
        Animal::Dog => fact_lists.dog_facts.as_ref().unwrap(),
    }
}

fn modify_persistent_fact(
    animal: Animal,
    fact_list: RwLockWriteGuard<Vec<Fact>>,
    state: &APIState,
) {
    let path = animal.get_filepath(&state.config.facts_dir);
    fs::write(path, serde_json::to_string_pretty(&*fact_list).unwrap()).unwrap()
}

fn modify_persistent_flag(flag_list: RwLockWriteGuard<Vec<FactFlag>>, state: &APIState) {
    let path = Path::new(&state.config.facts_dir).join("fact_flags.json");
    fs::write(path, serde_json::to_string_pretty(&*flag_list).unwrap()).unwrap()
}

#[cfg(test)]
mod permission_tests {
    use super::check_admin_perms;
    use super::{Admin, Perms};

    fn gen_admin_add_only() -> Admin {
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

    fn gen_admin_no_perms() -> Admin {
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

    fn gen_admin_all_perms() -> Admin {
        Admin {
            name: "Tester".to_string(),
            key: "all_perms".to_string(),
            permissions: Perms {
                view_facts: true,
                add_fact: true,
                delete_fact: true,
                view_flags: true,
                add_flag: true,
                delete_flag: true,
            },
        }
    }

    #[test]
    fn no_admin_list() {
        assert_eq!(check_admin_perms("TesterKey", &Vec::new()), None);
    }

    #[test]
    fn invalid_key() {
        let admin_list = vec![gen_admin_no_perms(), gen_admin_all_perms()];
        assert_eq!(check_admin_perms("TesterKey", &admin_list), None);
    }

    #[test]
    fn no_perms() {
        let admin_list = vec![gen_admin_no_perms(), gen_admin_all_perms()];
        let expected = (&gen_admin_no_perms(), None);
        assert_eq!(
            check_admin_perms(&admin_list[0].key, &admin_list),
            Some(expected)
        );
    }

    #[test]
    fn some_perms() {
        let admin_list = vec![gen_admin_add_only(), gen_admin_no_perms()];
        let expected = Some((
            &admin_list[0],
            Some(Perms {
                view_facts: true,
                add_fact: true,
                delete_fact: false,
                view_flags: true,
                add_flag: true,
                delete_flag: false,
            }),
        ));
        assert_eq!(check_admin_perms(&admin_list[0].key, &admin_list), expected);
    }
}
