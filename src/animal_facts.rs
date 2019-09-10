use actix_web::web::{HttpResponse, Data};
use rand::{thread_rng, Rng};

use crate::{APIState, Animal, Response, generate_response};

pub fn get_cat_fact(app_data: Data<APIState>) -> HttpResponse {
    if let Some(fact_list) = &app_data.fact_lists.cat_facts {
        let mut rng = thread_rng();
        let list_lock = fact_list.read().unwrap();
        // This should never panic as we will always be in range
        let rand_index = rng.gen_range(0, list_lock.len()); 

        let rand_pick = list_lock.get(rand_index).unwrap();
    
        app_data.req_counter.with_label_values(&[&*Animal::Cat]).inc();
        generate_response(rand_pick)
    } else {
        generate_response(&Response::TypeNotLoaded.gen_resp())
    }
}

pub fn get_dog_fact(app_data: Data<APIState>) -> HttpResponse {
    if let Some(fact_list) = &app_data.fact_lists.dog_facts {
        let mut rng = thread_rng();
        let list_lock = fact_list.read().unwrap();
        let rand_index = rng.gen_range(0, list_lock.len());

        let rand_pick = list_lock.get(rand_index).unwrap();

        app_data.req_counter.with_label_values(&[&*Animal::Dog]).inc();
        generate_response(rand_pick)
    } else {
        generate_response(&Response::TypeNotLoaded.gen_resp())
    }
}