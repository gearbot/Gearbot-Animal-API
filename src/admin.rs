use actix_web::{HttpRequest, HttpResponse, web::Json};
use actix_web::web::Data;
use rand::RngCore;
use log::info;

use std::fs;
use std::sync::{RwLock, RwLockWriteGuard};

use super::{
    APIState,
    Admin,
    Perms,
    ModifyRequest,
    ModifyAction,
    Fact,
    FactLists,
    Animal,
    Response,
    generate_response
};

fn check_admin_perms(unchecked_auth: &str, admin_list: Option<&Vec<Admin>>) -> Option<(Admin, Option<Perms>)> {
    if let Some(admin_list) = admin_list {
        match admin_list.iter().find(|admin| admin.key == unchecked_auth) {
            Some(admin) => {
                if admin.permissions.add || admin.permissions.delete {
                    Some((
                        admin.clone(),
                        Some(Perms {
                            add: admin.permissions.add,
                            delete: admin.permissions.delete
                        })
                    ))
                } else {
                    Some((admin.clone(), None))
                }
            }
            None => None
        }
    } else {
        None
    }
}

pub fn admin_modify_fact(state: Data<APIState>, req: HttpRequest, body: Json<ModifyRequest>) -> HttpResponse {
    let (action, user) = {
        let action = if req.path().ends_with("add") {
            ModifyAction::Add
        } else {
            // Anything expect /add and /delete will 404 first
            ModifyAction::Delete
        };

        // See if the provided key is valid
        if let Some((user, perms)) = check_admin_perms(&body.auth, state.admins.as_ref()) {
            // Check if they currently have any permissions at all
            if let Some(perms) = perms {
                // Check if they are allowed to perform the desired action
                if (action == ModifyAction::Add && !perms.add) | (action == ModifyAction::Delete && !perms.delete) { 
                    return generate_response(&Response::MissingPermission.gen_resp())
                }
                // Validated for performing their action
                (action, user)
            } else {
                return generate_response(&Response::MissingPermission.gen_resp())
            }
        } else {
           return generate_response(&Response::InvalidAuth.gen_resp())
        }
    };

    // Check if the requested animal list is loaded
    match body.animal_type {
        Animal::Cat => {
            if state.fact_lists.cat_facts.is_none() {
                return generate_response(&Response::TypeNotLoaded.gen_resp())
            }
        }
        Animal::Dog => {
            if state.fact_lists.dog_facts.is_none() {
                return generate_response(&Response::TypeNotLoaded.gen_resp())
            }
        }
    }

    match action {
        ModifyAction::Add => add_fact(body.animal_type, user, body.into_inner(), &state),
        ModifyAction::Delete => delete_fact(body.animal_type, user, body.into_inner(), &state)
    }
}

fn determine_list(animal: Animal, fact_lists: &FactLists) -> &RwLock<Vec<Fact>> {
    match animal {
        // These unwraps are safe due to previous checks
        Animal::Cat => fact_lists.cat_facts.as_ref().unwrap(),
        Animal::Dog => fact_lists.dog_facts.as_ref().unwrap(),
    }
}

fn modify_persistent(animal: Animal, fact_list: RwLockWriteGuard<Vec<Fact>>, state: &APIState) {
    let path = animal.get_filepath(&state.config.facts_dir);
    fs::write(path, serde_json::to_string_pretty(&*fact_list).unwrap()).unwrap() 
}

fn add_fact(animal: Animal, user: Admin, request: ModifyRequest, state: &APIState) -> HttpResponse {
    let id = rand::thread_rng().next_u64();

    let fact_list = determine_list(animal, &state.fact_lists);
    let mut list_lock = fact_list.write().unwrap();

    match request.fact_content {
        Some(content) => {
            list_lock.push(
                Fact {
                    id,
                    content
                }
            );
        }
        None => return generate_response(&Response::NoContentSpecified.gen_resp())
    }
    
    modify_persistent(animal, list_lock, state);

    let message = format!("{} fact added", &*animal);
    info!("{} by {}", message, user.name);

    generate_response(&Response::Created(message).gen_resp())
}

fn delete_fact(animal: Animal, user: Admin, request: ModifyRequest, state: &APIState) -> HttpResponse {
    if let Some(rem_id) = request.fact_id {
        let fact_list = determine_list(animal, &state.fact_lists);
        let mut list_lock = fact_list.write().unwrap();
        if let Some(found) = list_lock.iter().enumerate().find(|(_, fact)| fact.id == rem_id) {
            let pos = found.0;
            list_lock.remove(pos);
            modify_persistent(animal, list_lock, state);

            info!("{} fact removed by {}", &*animal, user.name);

            HttpResponse::NoContent().finish()
        } else {
            generate_response(&Response::BadID.gen_resp())
        }
    } else {
        generate_response(&Response::NoID.gen_resp())
    }
}
