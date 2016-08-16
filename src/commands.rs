//Copyright (c) 2016, Ruslan Baratov, Alex Frappier Lachapelle
//All rights reserved.


use std::collections::BTreeMap;
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


////////////////////////////////////////////////////////////
//                     CommandHandler                     //
////////////////////////////////////////////////////////////

pub struct CommandHandler {
    config:   Arc<Mutex<config::ConfigHandler>>,
    commands: BTreeMap<String, Command>
}

impl CommandHandler {

    pub fn new(tsconfig: &Arc<Mutex<config::ConfigHandler>>) -> CommandHandler {

        //Register commands
        let mut commands: BTreeMap<String, Command> = BTreeMap::new();
        commands.insert(String::from("ping"), Command::new(false, false, ping));

        CommandHandler {
            config:   tsconfig.clone(),
            commands: commands
        }
    }

    pub fn parse_command(&self, webhook: webhooks::WebhookEvent) {

        let mut tokens: Vec<&str>   = webhook.command.split_whitespace().collect();
        let mut is_please_provided  = false;
        let mut is_user_whitelisted = false;
        let mut next_token_index    = 0;

        //Check if please was said
        if tokens[0].to_lowercase() == "please" {
            is_please_provided = true;
            next_token_index   = 1;
        }

        //Check if user is whitelisted for restricted commands
        //  Restricted scope, we are using the config here but
        //  the callback can also use it so we force the mutex
        //  to unlock before letting the callback lock it
        //  given that locking twice will induce a panic.
        {
            let mut config = self.config.lock().unwrap();
            is_user_whitelisted = config.whitelist_validate_user(webhook.clone().user);
        }

        //Find command among registered commands
        let command = match self.commands.get(tokens[next_token_index]) {
            Some(command) => command,
            None => {
                respond(&self.config, webhook.clone(), String::from("Sorry the command was not found."));
                return;
            }
        };

        //Check if please and whitelist is required
        let mut run_cmd         = false;
        let mut response_prefix = String::new();
        if !(command.whitelist_only && !is_user_whitelisted) {
            if (command.requires_please && is_please_provided) || (!command.requires_please && !is_please_provided) {
                run_cmd = true;
            } else if !command.requires_please && is_please_provided {
                run_cmd         = true;
                response_prefix = String::from("You didn't need to say please but thanks anyways :smiley:\n");
            } else if command.requires_please && !is_please_provided {
                respond(&self.config, webhook.clone(), String::from("Whats the magic word?"));
            }
        } else {
            respond(&self.config, webhook.clone(), String::from("Sorry! That command if for whitelisted people only!"));
        }

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

pub fn ping(tsconfig: &Arc<Mutex<config::ConfigHandler>>, raw_event: webhooks::WebhookEvent, Args: Vec<&str>) -> Result<String, String> {
    return Ok(String::from("Pong"))
}


////////////////////////////////////////////////////////////
//                          Utils                         //
////////////////////////////////////////////////////////////

pub fn respond(tsconfig: &Arc<Mutex<config::ConfigHandler>>, raw_event: webhooks::WebhookEvent, msg: String) {

    //TODO: Don't forget to change this when refactoring config
    //Get repo were following
    let github_follow_repo: String;
    {
        let mut config = tsconfig.lock().unwrap();
        match config.get_string("config", "github_follow_repo") {
            Ok(_github_follow_repo) => github_follow_repo = _github_follow_repo,
            Err(err)        => {
                thread_error!("Failed to acquire \"github_follow_repo\": {}", err);
                panic!("Failed to acquire \"github_follow_repo\": {}", err);
            }
        }
    }

    let endpoint = format!("repos/{}/issues/{}/comments", github_follow_repo, raw_event.number);
    let message  = format!("{{body: \"@{} {}\"}}", raw_event.user, msg);
    match webhooks::github_post_request(endpoint, message) {
        Ok(())   => (),
        Err(err) => {thread_error!("{}", err);}
    }

}