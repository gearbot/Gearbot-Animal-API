use actix_web::web::{Data, HttpResponse};
use rand::{seq::SliceRandom, thread_rng};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

use crate::{generate_response, APIState, Animal, RESP_NOT_LOADED};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Fact {
    pub id: u64,
    pub content: String,
}

// The system can support all listed fact types, but they aren't required to be present
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FactLists {
    pub cat_facts: Option<RwLock<Vec<Fact>>>,
    pub dog_facts: Option<RwLock<Vec<Fact>>>,
}

pub fn get_cat_fact(app_data: Data<APIState>) -> HttpResponse {
    if let Some(fact_list) = &app_data.fact_lists.cat_facts {
        let mut rng = thread_rng();
        let list_lock = fact_list.read().unwrap();

        // This should never panic since `Some()` means there is >= 1 fact.
        let rand_pick = list_lock.choose(&mut rng).unwrap();

        app_data
            .req_counter
            .with_label_values(&[Animal::Cat.as_str()])
            .inc();

        HttpResponse::Ok().json(rand_pick)
    } else {
        generate_response(&RESP_NOT_LOADED)
    }
}

pub fn get_dog_fact(app_data: Data<APIState>) -> HttpResponse {
    if let Some(fact_list) = &app_data.fact_lists.dog_facts {
        let mut rng = thread_rng();
        let list_lock = fact_list.read().unwrap();

        // This should never panic since `Some()` means there is >= 1 fact.
        let rand_pick = list_lock.choose(&mut rng).unwrap();

        app_data
            .req_counter
            .with_label_values(&[Animal::Dog.as_str()])
            .inc();

        HttpResponse::Ok().json(rand_pick)
    } else {
        generate_response(&RESP_NOT_LOADED)
    }
}
