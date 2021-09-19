#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

#[cfg(test)]
pub mod tests;

use std::process::Command;
use std::sync::RwLock;

use clap::{load_yaml, App};
use rocket::config::{Config, Environment};
use rocket_contrib::templates::Template;

mod archive_unpacker;
mod common;
mod config;
mod endpoints;
mod install_task;

use config::ConfigTrait;

fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from(yaml).get_matches();
    let mut config_file = "config.json";
    if let Some(c) = matches.value_of("config") {
        config_file = c;
    }

    let mut config = config::ConfigObject::new(config_file);
    if config.get_mod_list().len() == 0 {
        if let Err(error) = config.rebuild_mod_storage(false) {
            panic!("Error loading {}: {}", config_file, error);
        }
    }

    let routes = endpoints::get_routes();

    let secret_key: String;
    let secret_key_option = &config.config.secret_key;
    if let Some(key) = secret_key_option {
        secret_key = key.to_string();
    } else {
        secret_key = String::from_utf8(
            Command::new("openssl")
                .arg("rand")
                .arg("-base64")
                .arg("32")
                .output()
                .expect("failed to generate secret key")
                .stdout,
        )
        .expect("command returned non-UTF8 output")
        .trim_end()
        .to_string();
        config.set_secret_key(secret_key.clone());
    }

    let rocket_config = Config::build(Environment::Staging)
        .address(&config.config.bind_address)
        .port(config.config.port)
        .secret_key(secret_key)
        .finalize()
        .unwrap();

    let lock = RwLock::new(config);
    rocket::custom(rocket_config)
        .manage(lock)
        .mount("/", routes)
        .attach(Template::fairing())
        .launch();
}
