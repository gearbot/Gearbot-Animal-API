#![deny(warnings)]
#![deny(unsafe_code)]

use actix_web::http::StatusCode;
use actix_web::web::HttpResponse;
use log::{info, warn};
use prometheus::{IntCounter, IntCounterVec, Registry};
use serde::{Deserialize, Serialize};

use std::fmt;
use std::fs;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

pub mod admin;
pub mod animal_facts;
pub mod flagging;

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub struct Perms {
    pub view_facts: bool,
    pub add_fact: bool,
    pub delete_fact: bool,
    pub view_flags: bool,
    pub add_flag: bool,
    pub delete_flag: bool,
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
    pub flagging_enabled: bool,
    pub flaggers: Vec<Flagger>,
    pub server: ServerConfig,
    pub admins: Vec<Admin>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AdminFactRequest {
    // Only used on removals
    pub fact_id: Option<u64>,
    // Only used on additions/updates
    pub fact_content: Option<String>,
    pub animal_type: Animal,
    pub key: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub enum AdminAction {
    Add,
    Delete,
    View,
}

impl fmt::Display for AdminAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdminAction::Add => write!(f, "add"),
            AdminAction::Delete => write!(f, "delete"),
            AdminAction::View => write!(f, "view"),
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct AdminFlagRequest {
    pub key: String,
    pub fact_id: Option<u64>,
    pub flag_id: Option<u64>,
    pub reason: Option<String>,
    pub fact_type: Option<Animal>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Flagger {
    pub location: String,
    pub key: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FactFlagRequest {
    pub fact_type: Animal,
    pub fact_id: u64,
    pub reason: Option<String>,
    pub key: String,
    // This shouldn't be abusable because it still requires auth from a known flagger
    pub flagger: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct FactFlag {
    pub id: u64,
    pub fact_type: Animal,
    pub fact_id: u64,
    pub reason: Option<String>,
    pub flagger: String,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
pub enum Animal {
    Cat,
    Dog,
}

impl Animal {
    pub fn get_filepath(self, dir: &str) -> PathBuf {
        match self {
            Animal::Cat => Path::new(dir).join("cat_facts.json"),
            Animal::Dog => Path::new(dir).join("dog_facts.json"),
        }
    }
}

impl Animal {
    fn as_str(self) -> &'static str {
        match self {
            Animal::Cat => "Cat",
            Animal::Dog => "Dog",
        }
    }
}

pub struct APIState {
    pub config: Config,
    pub fact_lists: animal_facts::FactLists,
    pub fact_flags: Option<RwLock<Vec<FactFlag>>>,
    pub stat_register: Registry,
    pub req_counter: IntCounterVec,
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub enum CreatedAction {
    Fact { animal: Animal },
    Flag,
}

impl CreatedAction {
    pub fn as_str(self) -> &'static str {
        match self {
            CreatedAction::Fact { animal } => match animal {
                Animal::Cat => "Cat fact added",
                Animal::Dog => "Dog fact added",
            },
            CreatedAction::Flag => "Flag set",
        }
    }
}

pub const RESP_NOT_LOADED: JsonResp =
    JsonResp::new(501, "The requested feature is not currently loaded!");
pub const RESP_MISSING_PERMS: JsonResp = JsonResp::new(401, "Missing Permission");
pub const RESP_BAD_AUTH: JsonResp = JsonResp::new(401, "Invalid authorization");
pub const RESP_NO_CONTENT_SPECIFIED: JsonResp = JsonResp::new(400, "No content was specified");
pub const RESP_ID_NOT_FOUND: JsonResp = JsonResp::new(404, "The requested ID doesn't exist");
pub const RESP_NO_TYPE_SUPPLIED: JsonResp = JsonResp::new(400, "The animal type was not specified");
pub const RESP_NO_ID_SUPPLIED: JsonResp = JsonResp::new(400, "An ID was not specified");

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct JsonResp {
    pub code: u16,
    pub message: &'static str,
}

impl JsonResp {
    pub const fn new(code: u16, message: &'static str) -> Self {
        JsonResp { code, message }
    }
}

pub fn load_fact_flags(flag_count: &IntCounter, config: &Config) -> Option<RwLock<Vec<FactFlag>>> {
    let file_name = Path::new(&config.facts_dir).join("fact_flags.json");

    if config.flagging_enabled {
        match fs::read_to_string(&file_name) {
            Ok(contents) => {
                let flags: Vec<FactFlag> =
                    serde_json::from_str(&contents).expect("The flags file was malformed!");
                flag_count.inc_by(flags.len() as i64);
                Some(RwLock::new(flags))
            }
            Err(_) => {
                warn!("Fact flagging was enabled, but the flagging history couldn't be found!");
                None
            }
        }
    } else {
        None
    }
}

pub fn load_fact_lists(fact_count: &IntCounterVec, config: &Config) -> animal_facts::FactLists {
    let mut fact_lists = animal_facts::FactLists::default();
    for animal in &config.animal_fact_types {
        let file_name = animal.get_filepath(&config.facts_dir);

        if let Ok(fact_file) = fs::read_to_string(file_name) {
            let facts: Vec<animal_facts::Fact> = serde_json::from_str(&fact_file).unwrap();

            if facts.is_empty() {
                warn!(
                    "While loading {} facts, none were found in the file!",
                    animal.as_str()
                );
                continue;
            }

            fact_count
                .with_label_values(&[animal.as_str()])
                .inc_by(facts.len() as i64);

            info!("{} facts loaded!", animal.as_str());
            match animal {
                Animal::Cat => fact_lists.cat_facts = Some(RwLock::new(facts)),
                Animal::Dog => fact_lists.dog_facts = Some(RwLock::new(facts)),
            }
        } else {
            warn!(
                "The facts file for the {} facts couldn't be found!",
                animal.as_str()
            );
        }
    }

    fact_lists
}

pub fn generate_response(resp: &JsonResp) -> HttpResponse {
    let status = StatusCode::from_u16(resp.code).unwrap();

    if status.is_server_error() {
        warn!("A request to an unloaded part of the server occured!")
    }

    match status {
        StatusCode::CREATED => HttpResponse::Created().json(resp),
        StatusCode::NOT_IMPLEMENTED => HttpResponse::NotImplemented().json(resp),
        StatusCode::UNAUTHORIZED => HttpResponse::Unauthorized().json(resp),
        StatusCode::BAD_REQUEST => HttpResponse::BadRequest().json(resp),
        StatusCode::NOT_FOUND => HttpResponse::NotFound().json(resp),
        _ => unreachable!(),
    }
}
