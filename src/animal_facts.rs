use actix_web::{web::Json, Result};
use actix_web::web;

use rand::prelude::*;

use crate::{APIState, Fact};

pub fn get_cat_fact(app_data: web::Data<APIState>) -> Result<Json<Fact>> {
    let mut rng = thread_rng();
    let fact_list: &Vec<Fact> = &app_data.cat_fact_list;
    let rand_index = rng.gen_range(0, fact_list.len()); // This should never panic as we will always be in range
    let rand_pick: Fact = fact_list.get(rand_index).unwrap().to_owned();

    app_data.req_counter.with_label_values(&["cat"]).inc();

    Ok(Json(rand_pick))
}

pub fn get_dog_fact(app_data: web::Data<APIState>) -> Result<Json<Fact>> {
    let mut rng = thread_rng();
    let fact_list: &Vec<Fact> = &app_data.dog_fact_list;
    let rand_index = rng.gen_range(0, fact_list.len());
    let rand_pick: Fact = fact_list.get(rand_index).unwrap().to_owned();

    app_data.req_counter.with_label_values(&["dog"]).inc();

    Ok(Json(rand_pick))
}
