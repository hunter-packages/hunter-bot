//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::mem;
use std::sync::{Arc, Mutex};
use std::thread;

extern crate hyper;
use hyper::Client;
use hyper::client::Body;
use hyper::client::IntoUrl;
use hyper::header::Headers;
use hyper::Url;

extern crate iron;
use self::iron::middleware;
use self::iron::prelude::*;
use self::iron::status;

extern crate openssl;
use self::openssl::crypto::hash::Type;
use self::openssl::crypto::hmac::hmac;

extern crate rand;
use self::rand::Rng;

extern crate serde;
extern crate serde_json;

use config;


////////////////////////////////////////////////////////////
//                        Structs                         //
////////////////////////////////////////////////////////////

pub struct WebhookHandler {
    config: Arc<Mutex<config::ConfigHandler>>
    //mpsc tx
    //logger tx
}


////////////////////////////////////////////////////////////
//                         Impls                          //
////////////////////////////////////////////////////////////

impl WebhookHandler {
    pub fn new(tsconfig: Arc<Mutex<config::ConfigHandler>>) -> WebhookHandler {
        WebhookHandler{
            config: tsconfig.clone()
            //mpsc tx
            //logger tx
        }
    }
}

impl middleware::Handler for WebhookHandler {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {

        //Get a thread local and thread safe (by Mutex) copy of the config
        let config = self.config.clone();

        //Verify webhook HMAC
        match validate_webhook(&config, request) {
            Ok(is_valid)  => {if !is_valid {return Ok(Response::with((status::BadRequest, "Invalid verification hash.")))}}
            Err(response) => return response
        }

        Ok(Response::with((status::Ok, "Received.")))
    }
}


////////////////////////////////////////////////////////////
//                          Funcs                         //
////////////////////////////////////////////////////////////

//Utils

pub fn github_post_request(endpoint: String, body: String) -> Result<(), &'static str>{

    let     http_client   = Client::new();
    let     api_call      = format!("https://api.github.com/{}", endpoint);
    let     api_call_url:   Url;
    let mut header        = Headers::new();
    let mut response:       hyper::client::Response;
    let mut response_body = String::new();

    match api_call.into_url() {
        Ok(url) => api_call_url = url,
        Err(_)  => return Err("Failed to parse the API call url.")
    }

    header.set_raw("User-Agent", vec![b"hunter-bot".to_vec()]);
    let body_len = body.len().clone();
    match http_client.post(api_call_url)
        .headers(header)
        .body(Body::BufBody(&body.into_bytes()[..], body_len))
        .send() {
        Ok(res) => response = res,
        Err(_)  => return Err("Failed to  call the API.")
    }

    if response.status == hyper::status::StatusCode::Unauthorized {
        return Err("Bad Credentials")
    }

    if response.status == hyper::status::StatusCode::NotFound {
        return Err("Endpoint not found or insufficient privileges.")
    }

    match response.read_to_string(&mut response_body){
        Ok(_)  => Ok(()),
        Err(_) => Err("Failed to  convert the API response to a string.")
    }
}

pub fn validate_webhook(tsconfig: &Arc<Mutex<config::ConfigHandler>>, request: &mut Request) -> Result<bool, IronResult<Response>> {

    //Get secret
    let github_webhook_secret: String;
    match tsconfig.lock().unwrap().get_string("state", "github_webhook_secret") {
        Ok(_github_webhook_secret) => github_webhook_secret = _github_webhook_secret,
        Err(_)                     => return Err(Ok(Response::with((status::InternalServerError, "Failed to get \"github_webhook_secret\" from config."))))
    }

    //Extract signature
    let signature_string_header: String;
    match request.headers.get_raw("X-Hub-Signature") {
        Some(signature) => {
            match String::from_utf8(signature[0].clone()) {
                Ok(_signature_string_header) => signature_string_header = _signature_string_header,
                Err(_)                => return Err(Ok(Response::with((status::InternalServerError, "Failed to stringify X-Hub-Signature."))))
            }
        }
        //TODO?: Log this error?
        None            => return Err(Ok(Response::with((status::BadRequest, "X-Hub-Signature missing."))))
    }

    //Extract body
    let mut body_vec:    Vec<u8> = Vec::new();
    let     body_string: String;
    request.body.read_to_end(&mut body_vec).unwrap();
    match String::from_utf8(body_vec.clone()){
        Ok(_body_string) => body_string = _body_string,
        Err(_)           => return Err(Ok(Response::with((status::InternalServerError, "Failed to stringify the request body."))))
    }

    //Compute hmac
    let hmac_array                = hmac(Type::SHA1, github_webhook_secret.as_bytes(), body_string.as_bytes());
    let hmac_strings: Vec<String> = hmac_array.iter().map(|byte| format!("{:02X}", byte)).collect();
    let hmac_string               = hmac_strings.join("").to_lowercase();
    let signature_string_actual   = format!("sha1={}", hmac_string);

    Ok(signature_string_header == signature_string_actual)
}


//Main funcs

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
    let mut github_webhook_secret: String;
    match config.get_string("state", "github_webhook_secret") {
        Ok(_github_webhook_secret) => github_webhook_secret = _github_webhook_secret,
        Err(err)                   => {panic!("Error getting  the \"github_webhook_secret\" value from config: {}", err);}
    }

    if github_webhook_secret == String::new() {
        // Get the RNG
        let mut rng = match rand::os::OsRng::new() {
            Ok(_rng) => _rng,
            Err(err) => panic!("Failed to obtain OS RNG: {}", err)
        };

        github_webhook_secret = rng.next_u64().to_string();
        config.set_string("state", "github_webhook_secret", &github_webhook_secret.clone()[..]);
        match config.save() {
            Ok(()) => (),
            Err(_) => {panic!("Failed to save the config file.");}
        }
    }

    //Get follow repo
    let github_follow_repo: String;
    match config.get_string("config", "github_follow_repo") {
        Ok(_github_follow_repo) => github_follow_repo = _github_follow_repo,
        Err(err)          => {panic!("Error getting  the \"github_follow_repo\" value from config: {}", err);}
    }

    //Get owner api token
    let github_owner_token: String;
    match config.get_string("config", "github_owner_token") {
        Ok(_github_owner_token) => github_owner_token = _github_owner_token,
        Err(err)          => {panic!("Error getting  the \"github_owner_token\" value from config: {}", err);}
    }

    //Create JSON data
    let mut json_data:        BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut json_data_config: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut json_data_events: Vec<serde_json::Value>              = Vec::new();

    let mut public_ip_address: String;
    match config.get_string("config", "public_ip_address") {
        Ok(_public_ip_address) => public_ip_address = _public_ip_address,
        Err(err)          => {panic!("Error getting  the \"public_ip_address\" value from config: {}", err);}
    }

    let listen_port: String;
    match config.get_string("config", "listen_port") {
        Ok(_listen_port) => listen_port = _listen_port,
        Err(err)          => {panic!("Error getting  the \"listen_port\" value from config: {}", err);}
    }

    public_ip_address.push_str(":");
    public_ip_address.push_str(&listen_port[..]);
    public_ip_address.push_str("/webhook");

    json_data.insert(String::from("name"),                serde_json::Value::String(String::from("web")));
    json_data.insert(String::from("active"),              serde_json::Value::Bool(true));
    json_data_config.insert(String::from("url"),          serde_json::Value::String(public_ip_address));
    json_data_config.insert(String::from("content_type"), serde_json::Value::String(String::from("json")));
    json_data_config.insert(String::from("secret"),       serde_json::Value::String(github_webhook_secret));
    json_data_config.insert(String::from("insecure_ssl"), serde_json::Value::String(String::from("1")));

    for hook in &hooks {
        json_data_events.push(serde_json::Value::String(hook.to_string()));
    }

    json_data.insert(String::from("config"), serde_json::Value::Object(json_data_config));
    json_data.insert(String::from("events"), serde_json::Value::Array(json_data_events));

    let json_data_string: String;
    match serde_json::to_string(&json_data) {
        Ok(_json_data_string) => json_data_string = _json_data_string,
        Err(err)              => {panic!("Faild to create JSON data to initialize webhooks: {}", err.description());}
    }

    //Register webhooks
    let endpoint = format!("repos/{}/hooks?access_token={}", github_follow_repo, github_owner_token);
    match github_post_request(endpoint, json_data_string) {
        Ok(())   => (),
        Err(err) => {panic!("Failed to register webhooks: {}", err)}
    }

    println!("Success!")
}


pub fn listen(config: &mut config::ConfigHandler) {

    //Start server thread
    let handler = WebhookHandler::new(Arc::new(Mutex::new(config.clone())));
    Iron::new(handler).http("0.0.0.0:3000").unwrap();

    //delete config, no longer needed

    //Process events

}

















