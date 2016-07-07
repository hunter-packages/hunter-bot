//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::collections::BTreeMap;
use std::error::Error;

extern crate rand;
use self::rand::Rng;

extern crate serde;
extern crate serde_json;

use config;
use utilities;

////////////////////////////////////////////////////////////
//                          Funcs                         //
////////////////////////////////////////////////////////////

pub fn register(config: &mut config::ConfigHandler) {

    //List of events to listent for.
    let hooks = vec!["issue_comment","pull_request_review_comment"];

    println!("Setting up webhooks...");

    //Create a random number for the "secret"
    //which will be used for verifying that
    //github is the actual sender of the webhook
    //via SHA1 HMAC

    //Skip secret number generation if we already made one before
    //Get webhook secret
    let mut github_webhook_secret = String::new();
    match config.get_string("config", "github_webhook_secret") {
        Ok(_github_webhook_secret) => github_webhook_secret = _github_webhook_secret,
        Err(err)          => {panic!("Error getting  the \"github_webhook_secret\" value from config: {}", err);}
    }

    if github_webhook_secret == String::new() {
        // Get the RNG
        let mut rng = match rand::os::OsRng::new() {
            Ok(_rng) => _rng,
            Err(err) => panic!("Failed to obtain OS RNG: {}", err)
        };

        let github_webhook_secret = rng.next_u64().to_string();
        config.set_string("state", "github_webhook_secret", &github_webhook_secret[..]);
        match config.save() {
            Ok(()) => (),
            Err(_) => {panic!("Failed to save the config file.");}
        }
    }

    //Get follow repo
    let mut github_follow_repo = String::new();
    match config.get_string("config", "github_follow_repo") {
        Ok(_github_follow_repo) => github_follow_repo = _github_follow_repo,
        Err(err)          => {panic!("Error getting  the \"github_follow_repo\" value from config: {}", err);}
    }

    //Get owner api token
    let mut github_owner_token = String::new();
    match config.get_string("config", "github_owner_token") {
        Ok(_github_owner_token) => github_owner_token = _github_owner_token,
        Err(err)          => {panic!("Error getting  the \"github_owner_token\" value from config: {}", err);}
    }

    //Create JSON data
    let mut json_data:        BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut json_data_config: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut json_data_events: Vec<serde_json::Value>              = Vec::new();

    let mut self_address = String::new();
    match config.get_string("config", "self_address") {
        Ok(_self_address) => self_address = _self_address,
        Err(err)          => {panic!("Error getting  the \"self_address\" value from config: {}", err);}
    }

    let mut listen_port = String::new();
    match config.get_string("config", "listen_port") {
        Ok(_listen_port) => listen_port = _listen_port,
        Err(err)          => {panic!("Error getting  the \"listen_port\" value from config: {}", err);}
    }

    self_address.push_str(":");
    self_address.push_str(&listen_port[..]);
    self_address.push_str("/webhook");

    json_data.insert(String::from("name"),                serde_json::Value::String(String::from("web")));
    json_data.insert(String::from("active"),              serde_json::Value::Bool(true));
    json_data_config.insert(String::from("url"),          serde_json::Value::String(self_address));
    json_data_config.insert(String::from("content_type"), serde_json::Value::String(String::from("json")));
    json_data_config.insert(String::from("secret"),       serde_json::Value::String(github_webhook_secret));
    json_data_config.insert(String::from("insecure_ssl"), serde_json::Value::String(String::from("1")));

    for hook in &hooks {
        json_data_events.push(serde_json::Value::String(hook.to_string()));
    }

    json_data.insert(String::from("config"), serde_json::Value::Object(json_data_config));
    json_data.insert(String::from("events"), serde_json::Value::Array(json_data_events));

    let mut json_data_string = String::new();
    match serde_json::to_string(&json_data) {
        Ok(_json_data_string) => json_data_string = _json_data_string,
        Err(err)              => {panic!("Faild to create JSON data to initialize webhooks: {}", err.description());}
    }

    //Register webhooks
    let endpoint = format!("repos/{}/hooks?access_token={}", github_follow_repo, github_owner_token);
    match utilities::github_post_request(endpoint, json_data_string) {
        Ok(())   => (),
        Err(err) => {panic!("Failed to register webhooks: {}", err)}
    }

    println!("Success!")
}