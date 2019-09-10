use actix_web::{http::StatusCode};
use actix_web::web::HttpResponse;
use log::info;
use serde::{Deserialize, Serialize};
use prometheus::{IntCounterVec, Registry};

use std::borrow::Cow;
use std::fs;
use std::net::IpAddr;
use std::ops::Deref;
use std::sync::RwLock;

pub mod admin;
pub mod animal_facts;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub struct Perms {
    pub add: bool,
    pub delete: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Admin {
    pub name: String,
    pub key: String,
    pub permissions: Perms,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct ServerConfig {
    pub ip: IpAddr,
    pub port: u16,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub logging_dir: String,
    pub logging_level: String,
    pub facts_dir: String,
    pub animal_fact_types: Vec<Animal>,
    pub server: ServerConfig,
    pub admins: Option<Vec<Admin>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ModifyRequest {
    // Only used on removals
    pub fact_id: Option<u64>,
    // Only used on additions/updates
    pub fact_content: Option<String>,
    pub animal_type: Animal,
    pub auth: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub enum ModifyAction {
    Add,
    Delete,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub enum Animal {
    Cat,
    Dog,
}

impl Animal {
    pub fn get_filepath(self, dir: &str) -> String {
        match self {
            Animal::Cat => format!("{}cat_facts.json", dir),
            Animal::Dog => format!("{}dog_facts.json", dir)
        }
    }
}

impl Deref for Animal {
    type Target = str;

    fn deref(&self) -> &'static str {
        match self {
            Animal::Cat => "Cat",
            Animal::Dog => "Dog"
        }
    } 
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Fact {
    id: u64,
    content: String,
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

pub struct APIState {
    pub config: Config,
    pub fact_lists: FactLists,
    pub stat_register: Registry,
    pub req_counter: IntCounterVec,
    pub admins: Option<Vec<Admin>>,
}


pub trait HasStatus {
    fn get_code(&self) -> u16;
} 

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum Response {
    TypeNotLoaded,
    MissingPermission,
    InvalidAuth,
    NoContentSpecified,
    BadID,
    NoID,
    Created(String)
}

impl Response {
    pub fn gen_resp<'a>(self) -> JsonResp<'a> {
        match self {
            Response::TypeNotLoaded => JsonResp::new(501, "The requested animal type is not currently loaded!"),
            Response::MissingPermission => JsonResp::new(401, "Missing Permission"),
            Response::InvalidAuth => JsonResp::new(401, "Invalid auth"),
            Response::NoContentSpecified => JsonResp::new(400, "No content was specified"),
            Response::BadID => JsonResp::new(400, "The requested ID doesn't exist"),
            Response::NoID => JsonResp::new(400, "An id was not specified"), 
            Response::Created(message) => JsonResp::new(200, message)
        }
    }
}

// This needs to use a Cow so we don't need to convert
// all &'static str's to Strings, but we can still 
// dynamically generate messages when needed, AND
// be able to deserialize it during testing
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct JsonResp<'a> {
    code: u16,
    message: Cow<'a, str>
}

impl<'a> JsonResp<'a> {
    pub fn new<T: Into<Cow<'a, str>>>(code: u16, message: T) -> Self {
        JsonResp { code, message: message.into() }
    }
}

impl HasStatus for JsonResp<'_> {
    fn get_code(&self) -> u16 {
        self.code
    }
}

impl HasStatus for Fact {
    fn get_code(&self) -> u16 {
        200
    }
}

pub fn load_fact_lists(fact_count: &IntCounterVec, config: &Config) -> FactLists {
    let mut fact_lists = FactLists::default();
    for animal in &config.animal_fact_types {
        let file_name = animal.get_filepath(&config.facts_dir);

        if let Ok(fact_file) = fs::read_to_string(file_name) {
            let facts: Vec<Fact> = serde_json::from_str(&fact_file).unwrap();
            fact_count.with_label_values(&[animal.deref()]).inc_by(facts.len() as i64);

            info!("{} facts loaded!", animal.deref());
            match animal {
                Animal::Cat => fact_lists.cat_facts = Some(RwLock::new(facts)),
                Animal::Dog => fact_lists.dog_facts = Some(RwLock::new(facts))
            }
        }
    }

    fact_lists
}

pub fn generate_response<T: Serialize + HasStatus>(resp: &T) -> HttpResponse {
    let status_code = StatusCode::from_u16(resp.get_code()).unwrap();

    HttpResponse::Ok()
    .status(status_code)
    .json(resp)
}
