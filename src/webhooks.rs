//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.

use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use std::thread;

extern crate hyper;
use hyper::Client;
use hyper::client::Body;
use hyper::client::IntoUrl;
use hyper::header::Headers;

extern crate iron;
use self::iron::middleware;
use self::iron::prelude::*;
use self::iron::status;

extern crate openssl;
use self::openssl::crypto::hash::Type;
use self::openssl::crypto::hmac::hmac;

extern crate rand;
use self::rand::Rng;

extern crate regex;
use self::regex::Regex;

extern crate serde;
extern crate serde_json;

use commands;
use config;

include!("logger_macros.rs");


////////////////////////////////////////////////////////////
//                     WebhookHandler                     //
////////////////////////////////////////////////////////////

pub struct WebhookHandler {
    config:   Arc<Mutex<config::ConfigHandler>>,
    queue_tx: Arc<Mutex<Sender<WebhookEvent>>>
}

impl WebhookHandler {
    pub fn new(tsconfig: Arc<Mutex<config::ConfigHandler>>, queue: Arc<Mutex<Sender<WebhookEvent>>>) -> WebhookHandler {
        trace!("webhooks.rs: WebhookHandler::new(tsconfig, queue)");
        WebhookHandler{
            config:   tsconfig.clone(),
            queue_tx: queue.clone()
        }
    }
}

//TODO: Adapt to work with Appveyor and Travis-CI requests + do trace
impl middleware::Handler for WebhookHandler {
    fn handle(&self, request: &mut Request) -> IronResult<Response> {

        thread_trace!("webhook.rs: WebhookHandler::handle(&self, &mut Request)");
        thread_debug!("Received a webhook");
        thread_debug!("Url:     {:?}", request.url);
        thread_debug!("Headers: {:?}", request.headers);


        //Get a thread local and thread safe (by Mutex) copy of the config
        let config   = self.config.clone();
        let queue_tx = self.queue_tx.clone();

        //Get body
        thread_trace!("  Extract request body");
        let mut body_string: String  = String::new();
        request.body.read_to_string(&mut body_string).unwrap();
        thread_debug!("Body:    {}", body_string);

        //Get signature
        thread_trace!("  Extract X-Hub-Signature header value");
        let signature_string_header: String;
        match extract_header_string(&request.headers, "X-Hub-Signature") {
            Ok(signature) => signature_string_header = signature,
            Err(err)      => return Ok(Response::with((status::InternalServerError, err)))
        }

        //Verify webhook HMAC
        //FIXME: this is kinda messy
        //TODO:  tracing
        match validate_webhook(&config, &signature_string_header, &body_string) {
            Ok(is_valid)  => {if !is_valid {return Ok(Response::with((status::BadRequest, "Invalid verification hash.")))}}
            Err(response) => {
                thread_warn!("Received a github webhook with an invalid HMAC.");
                return response
            }
        }

        //Get X-GitHub-Event header value
        let github_event_string: String;
        match extract_header_string(&request.headers, "X-GitHub-Event") {
            Ok(signature) => github_event_string = signature,
            Err(err)      => {
                error!("Failed to extract the \"X-GitHub-Event\" from a github webhook header: {}", err);
                return Ok(Response::with((status::InternalServerError, err)))
            }
        }

        //Parse the body
        let body_value: serde_json::Value;
        match serde_json::from_str(&body_string[..]) {
            Ok(_body_value) => body_value = _body_value,
            Err(err)        => {
                error!("{}", format!("Failed to parse the request body in a github webhook: {}.", err));
                return Ok(Response::with((status::InternalServerError, format!("Failed to parse the request body: {}.", err))))
            }
        }

        let webhook_event_type = WebhookEventType::from_string(&github_event_string[..]);
        match webhook_event_type {
            WebhookEventType::Ping               => {
                return Ok(Response::with((status::Ok, "Pong.")))
            }
            WebhookEventType::IssueComment       => {
                match WebhookEvent::from_issue_json(&config, &body_value.as_object().unwrap()) {
                    Ok(webhook_event_option) => {
                        match webhook_event_option {
                            Some(webhook_event) => {
                                queue_tx.lock().unwrap().send(webhook_event).unwrap();
                            }
                            None                => return Ok(Response::with((status::Ok, "Skipped.")))
                        }
                    }
                    Err(err)                 => {
                        error!("{}", format!("Failed to parse the request body data in a github webhook: {}.", err));
                        return Ok(Response::with((status::InternalServerError, format!("Failed to parse the request body data: {}.", err))))
                    }
                }
            }
            WebhookEventType::PullRequestComment => {
                match WebhookEvent::from_pull_request_json(&config, &body_value.as_object().unwrap()) {
                    Ok(webhook_event_option) => {
                        match webhook_event_option {
                            Some(webhook_event) => {queue_tx.lock().unwrap().send(webhook_event).unwrap();}
                            None                => return Ok(Response::with((status::Ok, "Skipped.")))
                        }
                    }
                    Err(err)                 => {
                        error!("{}", format!("Failed to parse the request body data in a github webhook: {}.", err));
                        return Ok(Response::with((status::InternalServerError, format!("Failed to parse the request body data: {}.", err))))
                    }
                }
            }
            WebhookEventType::Invalid            => {
                return Ok(Response::with((status::BadRequest, "Invalid event.")))
            }
        }
        return Ok(Response::with((status::Ok, "Received.")))
    }
}


////////////////////////////////////////////////////////////
//                   WebhookEventType                     //
////////////////////////////////////////////////////////////

#[derive(Clone, Debug)]
pub enum WebhookEventType {
    Ping,
    IssueComment,
    PullRequestComment,
    Invalid
}

impl WebhookEventType {
    pub fn from_string(event_str: &str) -> WebhookEventType {
        thread_trace!("webhooks.rs: WebhookEventType::from_string(\"{}\")", event_str);
        match event_str {
            "ping"                        => WebhookEventType::Ping,
            "issue_comment"               => WebhookEventType::IssueComment,
            "pull_request_review_comment" => WebhookEventType::PullRequestComment,
            _                             => WebhookEventType::Invalid
        }
    }
}


////////////////////////////////////////////////////////////
//                     WebhookEvent                       //
////////////////////////////////////////////////////////////

/// WebhookEvent
/// event_type: Type of the event (issue_comment, ping, ...)
/// number:     Issue or PR number
/// id:         Github ID for Issue or PR
/// user:       User that triggered the event
/// command:    Command made by user
#[derive(Clone, Debug)]
pub struct WebhookEvent {
    pub event_type: WebhookEventType,
    pub number:     u64,
    pub id:         u64,
    pub user:       String,
    pub command:    String
}

impl WebhookEvent {

    pub fn new() -> WebhookEvent {
        thread_trace!("webhook.rs: WebhookEvent::new()");
        WebhookEvent{
            event_type: WebhookEventType::Invalid,
            number:     0,
            id:         0,
            user:       String::new(),
            command:    String::new()
        }
    }

    ///Ok:  Option:
    ///        Some: WebhookEvent
    ///        None: Ignore, issue was edited or deleted,
    ///              we only care about created or the
    ///              comment is mentioning someone other
    ///              than the bot (@{github_bot_name})
    ///Err: An error occurred
    pub fn from_issue_json(tsconfig: &Arc<Mutex<config::ConfigHandler>>, json_object: &BTreeMap<String, serde_json::Value>) -> Result<Option<WebhookEvent>, String> {

        thread_trace!("webhook.rs: WebhookEvent::from_issue_json(tsconfig, json_object)");

        let mut config   = tsconfig.lock().unwrap();
        let mut event    = WebhookEvent::new();
        event.event_type = WebhookEventType::IssueComment;

        //Get and validate "action" string
        thread_trace!("  Get \"action\" string (try!)");
        let action_string = try!(extract_json_string(&json_object, "action"));

        thread_trace!("  Is action create");
        if action_string != "created" {
            thread_trace!("    false");
            return Ok(Option::None)
        }
        thread_trace!("    true");

        //Get "comment" Object
        thread_trace!("  Get \"comment\" object (try!)");
        let comment_object = try!(extract_json_object_named(&json_object, "comment"));

        //Check if the bot was mentioned, i.e if the message is directed towards the bot
        thread_trace!("  Get \"github_bot_name\" config string (try!)");
        let github_bot_name     = try!(config.get_string("config", "github_bot_name"));
        thread_trace!("  Get \"body\" string (try!)");
        let comment_body_string = try!(extract_json_string(&comment_object, "body"));
        let regex               = Regex::new(&format!("@{}", github_bot_name)[..]).unwrap();

        thread_trace!("  Is bot mentioned test");
        if regex.find(&comment_body_string[..]).is_some() {
            thread_trace!("    true");
            event.command = String::from(regex.replace(&comment_body_string[..], "").trim());
        } else {
            thread_trace!("    false");
            thread_trace!("Return Ok(None)");
            return Ok(Option::None)
        }

        //Get "user" string
        thread_trace!("  Get \"user\" object (try!)");
        let user_object = try!(extract_json_object_named(&comment_object, "user"));
        thread_trace!("  Get \"login\" string (try!)");
        event.user      = try!(extract_json_string(&user_object, "login"));

        //Get "number" and "id" numbers
        thread_trace!("  Get \"issue\" object (try!)");
        let issue_object = try!(extract_json_object_named(&json_object, "issue"));
        thread_trace!("  Get \"number\" u64 (try!)");
        event.number     = try!(extract_json_u64(&issue_object, "number"));
        thread_trace!("  Get \"id\" u64 (try!)");
        event.id         = try!(extract_json_u64(&issue_object, "id"));

        thread_trace!("Return Ok");
        Ok(Option::Some(event))
    }

    ///Ok:  Option:
    ///        Some: WebhookEvent
    ///        None: Ignore, issue was edited or deleted,
    ///              we only care about created or the
    ///              comment is mentioning someone other
    ///              than the bot (@{github_bot_name})
    ///Err: An error occurred
    pub fn from_pull_request_json(tsconfig: &Arc<Mutex<config::ConfigHandler>>, json_object: &BTreeMap<String, serde_json::Value>) -> Result<Option<WebhookEvent>, String> {

        thread_trace!("webhook.rs: WebhookEvent::from_pull_request_json(tsconfig, json_object)");

        let mut config   = tsconfig.lock().unwrap();
        let mut event    = WebhookEvent::new();
        event.event_type = WebhookEventType::PullRequestComment;

        //Get and validate "action" string
        thread_trace!("  Get \"action\" string (try!)");
        let action_string = try!(extract_json_string(&json_object, "action"));

        thread_trace!("  Is action create");
        if action_string != "created" {
            thread_trace!("    false");
            return Ok(Option::None)
        }
        thread_trace!("    true");

        //Get "comment" Object
        thread_trace!("  Get \"comment\" object (try!)");
        let comment_object = try!(extract_json_object_named(&json_object, "comment"));

        //Check if the bot was mentioned, i.e if the message is directed towards the bot
        thread_trace!("  Get \"github_bot_name\" config string (try!)");
        let github_bot_name     = try!(config.get_string("config", "github_bot_name"));
        thread_trace!("  Get \"body\" string (try!)");
        let comment_body_string = try!(extract_json_string(&comment_object, "body"));
        let regex               = Regex::new(&format!("@{}", github_bot_name)[..]).unwrap();

        thread_trace!("  Is bot mentioned test");
        if regex.find(&comment_body_string[..]).is_some() {
            thread_trace!("    true");
            event.command = String::from(regex.replace(&comment_body_string[..], "").trim());
        } else {
            thread_trace!("    false");
            thread_trace!("Return Ok(None)");
            return Ok(Option::None)
        }

        //Get "user" string
        thread_trace!("  Get \"user\" object (try!)");
        let user_object = try!(extract_json_object_named(&comment_object, "comment"));
        thread_trace!("  Get \"login\" string (try!)");
        event.user      = try!(extract_json_string(&user_object, "login"));

        //Get "number" and "id" numbers
        thread_trace!("  Get \"issue\" object (try!)");
        let pull_request_object = try!(extract_json_object_named(&json_object, "pull_request"));
        thread_trace!("  Get \"number\" u64 (try!)");
        event.number            = try!(extract_json_u64(&pull_request_object, "number"));
        thread_trace!("  Get \"id\" u64 (try!)");
        event.id                = try!(extract_json_u64(&pull_request_object, "id"));

        thread_trace!("Return Ok");
        Ok(Option::Some(event))
    }
}


////////////////////////////////////////////////////////////
//                          Funcs                         //
////////////////////////////////////////////////////////////

//Utils

//TODO: add bad request check
pub fn github_post_request(endpoint: String, body: String) -> Result<(), String>{

    thread_trace!("webhooks.rs: github_post_request({}, {})", endpoint, body);

    let     http_client   = Client::new();
    let     api_call      = format!("https://api.github.com/{}", endpoint);
    let     body_len      = body.len().clone();
    let mut header        = Headers::new();

    thread_trace!("  Api call to url");
    let api_call_url = match api_call.into_url() {
        Ok(url)   => {
            thread_trace!("    Ok");
            url
        },
        Err(err)  => {
            thread_trace!("Return Err");
            return Err(format!("Failed to parse the API call url: {}", err))
        }
    };

    header.set_raw("User-Agent", vec![b"hunter-bot".to_vec()]);
    thread_trace!("  Post request");
    let response = match http_client.post(api_call_url)
        .headers(header)
        .body(Body::BufBody(&body.into_bytes()[..], body_len))
        .send() {
        Ok(res)   => {
            thread_trace!("    Ok");
            res
        },
        Err(err)  => return Err(format!("Failed to  call the API: {}", err))
    };

    thread_trace!("  Status bad credentials check");
    if response.status == hyper::status::StatusCode::Unauthorized {
        thread_trace!("Return err");
        return Err(String::from("Bad Credentials"))
    }
    thread_trace!("    Ok");

    thread_trace!("  Status not found check");
    if response.status == hyper::status::StatusCode::NotFound {
        thread_trace!("Return Err");
        return Err(String::from("Endpoint not found or insufficient privileges."))
    }
    thread_trace!("    Ok");

    thread_trace!("Return Ok");
    Ok(())
}

pub fn validate_webhook(tsconfig: &Arc<Mutex<config::ConfigHandler>>, header_string: &String, body_string: &String) -> Result<bool, IronResult<Response>> {

    thread_trace!("webhooks.rs: validate_webhook(tsconfig, header_string, body_string)");

    //Get secret
    thread_trace!("  Get \"github_webhook_secret\" from config");
    let github_webhook_secret: String = match tsconfig.lock().unwrap().get_string("state", "github_webhook_secret") {
        Ok(secret) => {
            thread_trace!("    Ok");
            secret
        },
        Err(_)                     => {
            thread_trace!("Return Err");
            return Err(Ok(Response::with((status::InternalServerError, "Failed to get \"github_webhook_secret\" from config."))))
        }
    };

    //Extract signature
    //let mut signature_string_header: String;
    //match extract_header_string(&request.headers, "X-Hub-Signature") {
    //    Ok(signature) => signature_string_header = signature,
    //    Err(err)      => return Err(Ok(Response::with((status::InternalServerError, err))))
    //}

    //Compute hmac
    thread_trace!("  Compute HMAC");
    let hmac_array                = match hmac(Type::SHA1, github_webhook_secret.as_bytes(), body_string.as_bytes()) {
        Ok(hmac) => hmac,
        Err(_)   => {
            thread_trace!("Return Err");
            return Err(Ok(Response::with((status::InternalServerError, "Failed to compute HMAC value."))));
        }
    };
    let hmac_strings: Vec<String> = hmac_array.iter().map(|byte| format!("{:02X}", byte)).collect();
    let hmac_string               = hmac_strings.join("").to_lowercase();
    let signature_string_actual   = format!("sha1={}", hmac_string);

    thread_trace!("  HMAC matches: {}", (header_string.clone() == signature_string_actual));
    thread_trace!("Return Ok");
    Ok(header_string.clone() == signature_string_actual)
}

pub fn extract_header_string(header: &iron::Headers, field: &str) -> Result<String, String> {
    thread_trace!("webhook.rs: extract_header_string(header, \"{}\")", field);
    thread_trace!("  Get raw header data");
    match header.get_raw(field) {
        Some(value) => {
            thread_trace!("    Ok");
            thread_trace!("    Stringify header data");
            match String::from_utf8(value[0].clone()) {
                Ok(value_string) => {
                    thread_trace!("return Ok");
                    return Ok(value_string)
                },
                Err(err)         => {
                    thread_trace!("      return Err");
                    return Err(format!("Failed to stringify the \"{}\" field in the header: {}.", field, err))
                }
            }
        }
        None        => {
            thread_trace!("return Err");
            return Err(format!("\"{}\" field in the header is missng.", field))
        }
    }
}

pub fn extract_json_object(value: &serde_json::Value) -> Result<BTreeMap<String, serde_json::Value>, String> {
    thread_trace!("webhook.rs: extract_json_object(value)");
    thread_trace!("  Value to object");
    match value.as_object().ok_or(String::from("JSON data does not describe an object")) {
        Ok(object) => {
            thread_trace!("return Ok");
            return Ok(object.clone())
        },
        Err(err)   => {
            thread_trace!("return Err");
            return Err(err)
        }
    }
}

pub fn extract_json_object_named(object: &BTreeMap<String, serde_json::Value>, field: &'static str) -> Result<BTreeMap<String, serde_json::Value>, String> {
    thread_trace!("webhooks.rs: extract_json_object_named(&object, \"{}\")", field);
    thread_trace!("  Get field from object (try!)");
    let value = try!(object.get(field).ok_or(format!("The \"{}\" field was not found in the JSON object.", field)));
    thread_trace!("Return extract_json_object(&value)");
    return extract_json_object(&value);
}

pub fn extract_json_string(object: &BTreeMap<String, serde_json::Value>, field: &'static str) -> Result<String, String> {
    thread_trace!("webhooks.rs: extract_json_string(&object, \"{}\")", field);
    thread_trace!("  Get field from object (try!)");
    let value     = try!(object.get(field).ok_or(format!("The \"{}\" field was not found in the JSON object.", field)));
    thread_trace!("  Get field as string (try!)");
    let value_str = try!(value.as_str().ok_or(format!("The \"{}\" field does not describe a string.", field)));
    thread_trace!("Return Ok");
    return Ok(String::from(value_str));
}

pub fn extract_json_u64(object: &BTreeMap<String, serde_json::Value>, field: &'static str) -> Result<u64, String> {
    thread_trace!("webhooks.rs: extract_json_u64(&object, \"{}\")", field);
    thread_trace!("  Get field from object (try!)");
    let value     = try!(object.get(field).ok_or(format!("The \"{}\" field was not found in the JSON object.", field)));
    thread_trace!("  Get field as u64 (try!)");
    let value_u64 = try!(value.as_u64().ok_or(format!("The \"{}\" field does not describe a string.", field)));
    thread_trace!("Return Ok");
    return Ok(value_u64);
}

//Main funcs

pub fn register(config: &mut config::ConfigHandler) {

    trace!("webhooks.rs: register(config)");

    //List of events to listent for.
    let hooks = vec!["issue_comment","pull_request_review_comment"];
    debug!("hooks: {:?}", hooks);

    info!("Setting up webhooks...");

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
        // Get the system RNG
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
        Err(err)                => {panic!("Error getting  the \"github_owner_token\" value from config: {}", err);}
    }

    //Create JSON data
    let mut json_data:        BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut json_data_config: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut json_data_events: Vec<serde_json::Value>              = Vec::new();

    let mut public_ip_address: String;
    match config.get_string("config", "public_ip_address") {
        Ok(_public_ip_address) => public_ip_address = _public_ip_address,
        Err(err)               => {panic!("Error getting  the \"public_ip_address\" value from config: {}", err);}
    }

    let listen_port: String;
    match config.get_string("config", "listen_port") {
        Ok(_listen_port) => listen_port = _listen_port,
        Err(err)         => {panic!("Error getting  the \"listen_port\" value from config: {}", err);}
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

    info!("Success!");
}


pub fn listen(config: &mut config::ConfigHandler) {

    //Event mpsc queue
    let (tx, rx) = channel::<WebhookEvent>();
    let tsconfig = Arc::new(Mutex::new(config.clone()));
    let tsqueue  = Arc::new(Mutex::new(tx.clone()));
    let handler  = WebhookHandler::new(tsconfig.clone(), tsqueue.clone());

    //Get local_ip_address
    let local_ip_address: String;
    match config.get_string("config", "local_ip_address") {
        Ok(_local_ip_address) => local_ip_address = _local_ip_address,
        Err(err)              => {panic!("Error getting  the \"local_ip_address\" value from config: {}.", err);}
    }

    let mut local_ip_address_num_vec: Vec<u8> = Vec::new();
    for byte in local_ip_address.split(".") {
        let byte_parsed: u8;
        match byte.parse() {
            Ok(_byte_parsed) => byte_parsed = _byte_parsed,
            Err(err)         => {panic!("Error parsing the ip address byte into a u8: {}.", err)}
        }
        local_ip_address_num_vec.push(byte_parsed);
    }


    //Get listen_port
    let listen_port_string: String;
    let listen_port_u16   : u16;
    match config.get_string("config", "listen_port") {
        Ok(_listen_port) => listen_port_string = _listen_port,
        Err(err)         => {panic!("Error getting  the \"listen_port\" value from config: {}.", err);}
    }

    match listen_port_string.parse() {
        Ok(_listen_port_u16) => listen_port_u16 = _listen_port_u16,
        Err(err)             => {panic!("Error parsing the port into a u16: {}.", err)}
    }

    //Start server thread
    let server_thread = thread::spawn(move || {
        Iron::new(handler).http(
            SocketAddr::V4(
                SocketAddrV4::new(
                    Ipv4Addr::new(
                        local_ip_address_num_vec[0],
                        local_ip_address_num_vec[1],
                        local_ip_address_num_vec[2],
                        local_ip_address_num_vec[3]),
                    listen_port_u16
                )
            )
        ).unwrap();
    });

    //Drop the config, we will from now use the thread safe wrapped config.
    drop(config);

    //Process events
    let command_handler = commands::CommandHandler::new(&tsconfig);
    thread_debug!("command_handler: {:?}", command_handler);
    loop {

        //Dequeue
        let webhook_event = rx.recv().unwrap();
        command_handler.parse_command(webhook_event);

    }
}

















