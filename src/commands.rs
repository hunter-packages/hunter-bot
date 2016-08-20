//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.


use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use config;
use webhooks;

include!("logger_macros.rs");


////////////////////////////////////////////////////////////
//                         Command                        //
////////////////////////////////////////////////////////////

//                            Config,                             Raw webhook,            Arguments
pub type CommandCallback = fn(&Arc<Mutex<config::ConfigHandler>>, webhooks::WebhookEvent, Vec<&str>) -> Result<String, String>;

pub struct Command {
    pub requires_please: bool,
    pub whitelist_only:  bool,
    pub callback:        CommandCallback
}

impl Command {
    pub fn new(requires_please: bool, whitelist_only: bool, callback: CommandCallback) -> Command {
        Command{
            requires_please: requires_please,
            whitelist_only:  whitelist_only,
            callback:        callback
        }
    }
}

impl fmt::Debug for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Command {{requires_please: {}, whitelist_only: {}, callback: fn(&Arc<Mutex<config::ConfigHandler>>, webhooks::WebhookEvent, Vec<&str>) -> Result<String, String>}}", self.requires_please, self.whitelist_only)
    }
}


////////////////////////////////////////////////////////////
//                     CommandHandler                     //
////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct CommandHandler {
    config:   Arc<Mutex<config::ConfigHandler>>,
    commands: BTreeMap<String, Command>
}

impl CommandHandler {

    pub fn new(tsconfig: &Arc<Mutex<config::ConfigHandler>>) -> CommandHandler {

        thread_trace!("commands.rs: CommandHandler::new(tsconfig)");

        //Register commands
        let mut commands: BTreeMap<String, Command> = BTreeMap::new();
        commands.insert(String::from("ping"), Command::new(false, false, ping));
        commands.insert(String::from("help"), Command::new(false, false, help));

        CommandHandler {
            config:   tsconfig.clone(),
            commands: commands
        }
    }

    pub fn parse_command(&self, webhook: webhooks::WebhookEvent) {

        thread_trace!("commands.rs: CommandHandler::parse_command(&self, webhook)");

        let mut tokens: Vec<&str>   = webhook.command.split_whitespace().collect();
        let mut is_please_provided  = false;
        let mut is_user_whitelisted = false;
        let mut next_token_index    = 0;
        let mut bot_name            = String::new();

        thread_debug!("Command tokens: {:?}", tokens);

        //Check if please was said
        if tokens[0].to_lowercase() == "please" {
            is_please_provided = true;
            next_token_index   = 1;
        }
        thread_debug!("Please was said: {}", is_please_provided);

        //Check if user is whitelisted for restricted commands
        //  Restricted scope, we are using the config here but
        //  the callback can also use it so we force the mutex
        //  to unlock before letting the callback lock it
        //  given that locking twice will induce a panic.
        {
            let mut config      = self.config.lock().unwrap();
            is_user_whitelisted = config.whitelist_validate_user(webhook.clone().user);
            bot_name            = config.get_string_required("config", "github_bot_name");
        }

        //Ignore commands/responses from the bot
        thread_trace!("Check if command is from bot.");
        if bot_name == webhook.user {
            thread_trace!("Command is from bot, return.");
            return;
        }

        //Find command among registered commands
        thread_trace!("Check if command exists.");
        thread_debug!("Looking for command: {}", tokens[next_token_index]);
        let command = match self.commands.get(tokens[next_token_index]) {
            Some(command) => {
                thread_trace!("Command exists.");
                command
            },
            None          => {
                thread_trace!("Command does not exists, send response.");
                respond(&self.config, webhook.clone(), String::from("Sorry the command was not found. Please visit [https://hunterbot.readthedocs.io](https://hunterbot.readthedocs.io) for available commands."));
                return;
            }
        };

        //Check if please and whitelist is required
        let mut run_cmd         = false;
        let mut response_prefix = String::new();

        thread_trace!("Check for whitelist and please.");
        thread_debug!("is_please_provided:  {}", is_please_provided);
        thread_debug!("is_user_whitelisted: {}", is_user_whitelisted);

        if !(command.whitelist_only && !is_user_whitelisted) {
            if (command.requires_please && is_please_provided) || (!command.requires_please && !is_please_provided) {
                run_cmd = true;
            } else if !command.requires_please && is_please_provided {
                run_cmd         = true;
                response_prefix = String::from("You didn't need to say please but thanks anyways :smiley: \\r\\n\\r\\nOhh and: \\r\\n");
            } else if command.requires_please && !is_please_provided {
                //TODO: keep please state
                respond(&self.config, webhook.clone(), String::from("Whats the magic word?"));
            }
        } else {
            respond(&self.config, webhook.clone(), String::from("Sorry! That command if for whitelisted people only!"));
        }

        thread_debug!("run_cmd: {}", run_cmd);

        if run_cmd {
            match (command.callback)(&self.config, webhook.clone(), tokens.split_off(next_token_index)) {
                Ok(msg)  => {
                    respond(&self.config, webhook.clone(), format!("{}{}", response_prefix, msg));
                }
                Err(msg) => {
                    respond(&self.config, webhook.clone(), format!("An error occurred while executing the command: {}", msg));
                }
            }
        }
    }
}


////////////////////////////////////////////////////////////
//                        Callbacks                       //
////////////////////////////////////////////////////////////

#[allow(unused_variables)]
pub fn ping(tsconfig: &Arc<Mutex<config::ConfigHandler>>, raw_event: webhooks::WebhookEvent, args: Vec<&str>) -> Result<String, String> {
    return Ok(String::from("Pong"))
}

#[allow(unused_variables)]
pub fn help(tsconfig: &Arc<Mutex<config::ConfigHandler>>, raw_event: webhooks::WebhookEvent, args: Vec<&str>) -> Result<String, String> {
    return Ok(String::from("Documentation related to the bot including available commands are at [https://hunterbot.readthedocs.io](https://hunterbot.readthedocs.io)"))
}


////////////////////////////////////////////////////////////
//                          Utils                         //
////////////////////////////////////////////////////////////

pub fn respond(tsconfig: &Arc<Mutex<config::ConfigHandler>>, raw_event: webhooks::WebhookEvent, msg: String) {

    thread_trace!("commands.rs: respond(tsconfig, raw_event, msg)");

    //TODO: Don't forget to change this when refactoring config
    //Get repo were following
    let github_follow_repo: String;
    let github_bot_token: String;
    {
        let mut config = tsconfig.lock().unwrap();
        github_follow_repo = config.get_string_required("config", "github_follow_repo");
        github_bot_token = config.get_string_required("config", "github_bot_token");
    }

    let endpoint = format!("repos/{}/issues/{}/comments?access_token={}", github_follow_repo, raw_event.number, github_bot_token);
    let message  = format!("{{\"body\": \"@{} {}\"}}", raw_event.user, msg);
    match webhooks::github_post_request(endpoint, message) {
        Ok(())   => (),
        Err(err) => {thread_error!("{}", err);}
    }
}
