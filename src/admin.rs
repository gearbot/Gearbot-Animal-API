use actix_web::{HttpRequest, HttpResponse, web::Json};
use actix_web::web::Data;

use std::fs;

use crate::{
    APIState,
    Admin,
    AuthKeys,
    Perms,
    ModifyRequest,
    ModifyAction,
    Fact,
    AnimalChoice
};

fn check_admin_perms(unchecked_auth: AuthKeys, admin_list: Vec<Admin>) -> (bool, Option<Perms>) {
    match admin_list.iter().find(|admin| admin.auth == unchecked_auth) {
        Some(admin) => {
            if admin.permissions.add || admin.permissions.delete { // If they have no perms, say it
                (true, Some(
                    Perms {
                        add: admin.permissions.add,
                        delete: admin.permissions.delete
                    }
                ))
            } else {
                (true, None)
            }
        }
        None => (false, None)
    }
}

pub fn admin_modify_fact(req: HttpRequest, body: Json<ModifyRequest>) -> HttpResponse {
    let state: &Data<APIState> = req.app_data().unwrap();

    fn modify_fact(request: ModifyRequest, mut fact_list: Vec<Fact>, action: ModifyAction, animal: AnimalChoice, perms: Perms) -> HttpResponse {
        if action == ModifyAction::Add && perms.add {
            let last_id = match fact_list.last() {
                Some(fact) => fact.id,
                None => 0 // It could be a brand new JSON file.. somehow...
            };

            match request.content {
                Some(content) => {
                    fact_list.push(
                        Fact {
                            id: last_id + 1,
                            fact: content
                        }
                    );
                }

                None => return HttpResponse::BadRequest().body("NO \"content\" SPECIFIED")
            }

            match animal {
                AnimalChoice::Cat => {
                    fs::write("cat_facts.json", serde_json::to_string_pretty(&fact_list).unwrap()).unwrap();
                    HttpResponse::Created().body("CAT FACT ADDED")
                }

                AnimalChoice::Dog => {
                    fs::write("dog_facts.json", serde_json::to_string_pretty(&fact_list).unwrap()).unwrap();
                    HttpResponse::Created().body("DOG FACT ADDED")
                }
           }
            
        } else if action == ModifyAction::Delete && perms.delete { // TODO: REPLICATE CAT ABOVE
            match request.fact_id {
                Some(rem_id) => {
                    match fact_list.iter().find(|fact| fact.id == rem_id) {
                        Some(_) => {
                            fact_list.remove(rem_id as usize);

                            let mut new_fact_list: Vec<Fact> = Vec::with_capacity(fact_list.len() - 1);
                            for fact in fact_list {
                                new_fact_list.push(
                                    Fact {
                                        id: fact.id - 1,
                                        fact: fact.fact
                                    }
                                )
                            }

                            match animal {
                                AnimalChoice::Cat => {
                                    fs::write("cat_facts.json", serde_json::to_string_pretty(&new_fact_list).unwrap()).unwrap();
                                    HttpResponse::Created().body("CAT FACT DELETED")
                                }
                                AnimalChoice::Dog => {
                                    fs::write("dog_facts.json", serde_json::to_string_pretty(&new_fact_list).unwrap()).unwrap();
                                    HttpResponse::Created().body("DOG FACT DELETED")
                                }
                            }
                        }

                        None => HttpResponse::BadRequest().body(format!("INVALID FACT_ID SPECIFIED, MAX: {}", fact_list.len() - 1))
                    }
                }

                None => HttpResponse::BadRequest().body("NO \"fact_id\" SPECIFIED")
            }

        } else {
            HttpResponse::Forbidden().body("INSUFFICENT PERMISSIONS")
        }
    }

    let action = {
        let action_raw = req.to_owned();
        let action_raw = action_raw.path();

        if action_raw.ends_with("add") {
            ModifyAction::Add
        } else if action_raw.ends_with("delete") {
            ModifyAction::Delete
        } else {
            return HttpResponse::NotFound().body("")
        }
    };
    
    let (valid_admin, valid_perms) = check_admin_perms(body.auth.to_owned(), state.admins.to_owned());

    if valid_admin {
        match valid_perms {
            Some(perms) => { // Had some perms, now check if they can perform the requested action
                match body.target.to_lowercase().as_str() {
                    "cat" => modify_fact(body.into_inner(), state.cat_fact_list.to_owned(), action, AnimalChoice::Cat, perms),
                    "dog" => modify_fact(body.into_inner(), state.dog_fact_list.to_owned(), action, AnimalChoice::Dog, perms),

                    &_ => HttpResponse::BadRequest().body("INVALID TARGET")
                }
            }

            None => { // They had a valid account, but had no assigned perms
                HttpResponse::Forbidden()
                    .body("NO PERMISSIONS")
            }
        }
    } else { // Invalid key or username
        HttpResponse::Unauthorized()
            .body("INVALID AUTH")
    }
}
