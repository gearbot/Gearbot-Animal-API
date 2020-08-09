use actix_web::web::{Data, HttpResponse};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

use crate::{generate_response, APIState, Animal, RESP_NOT_LOADED};

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Fact {
    pub id: u64,
    pub content: String,
}

// The system can support all listed fact types, but they aren't required to be present
#[derive(Serialize, Deserialize, Debug)]
pub struct FactLists {
    pub cat_facts: Option<RwLock<Vec<Fact>>>,
    pub dog_facts: Option<RwLock<Vec<Fact>>>,
}

impl Default for FactLists {
    fn default() -> Self {
        FactLists {
            cat_facts: None,
            dog_facts: None,
        }
    }
}

pub fn get_cat_fact(app_data: Data<APIState>) -> HttpResponse {
    if let Some(fact_list) = &app_data.fact_lists.cat_facts {
        let mut rng = thread_rng();
        let list_lock = fact_list.read().unwrap();

        // This should never panic as we will always be in range
        let rand_index = rng.gen_range(0, list_lock.len());

        let rand_pick = list_lock.get(rand_index).unwrap();

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
        let rand_index = rng.gen_range(0, list_lock.len());

        let rand_pick = list_lock.get(rand_index).unwrap();

        app_data
            .req_counter
            .with_label_values(&[Animal::Dog.as_str()])
            .inc();

        HttpResponse::Ok().json(rand_pick)
    } else {
        generate_response(&RESP_NOT_LOADED)
    }
}
